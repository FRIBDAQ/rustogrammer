//!  This module provides code that retains
//!  trace information that supports the trace
//!  REST interface that provides a SpecTcl compatible
//!  trace interface.  See src/rest/trace_info.rs for that.
//!
//!   This is all accomplished via an Arc/Mutex proteced
//! struct that contains all of the data and provides
//! an api that allows the appropriate threads to declare
//! tracable events, register clients and manage per client
//! trace stores and, finally,  a thread that handles triming the
//! trace store.  
//!
//! This later is needed because ReST interfaces don't
//! allow us to push traces to the client.  Furthermore,
//! we can't gaurantee that each trace client will properly
//! tell us it's done.  Therefore each trace client specifies
//! a lifetime for its traces. The trace events are stamped
//! with when they were declared and the prune thread
//! will go over all stored traces removing the expired ones.
//! this prevents the trace store from growing without bounds.
//!
//!
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

/// The various types of traces
/// If the payload for an enumerated type
/// is just a string, that's the name of the affected
/// object (e.g. the name of the new parameter for NewParameter below).
#[derive(Clone)]
pub enum TraceEvent {
    NewParameter(String),
    SpectrumCreated(String),
    SpectrumDeleted(String),
    ConditionCreated(String),
    ConditionModified(String),
    ConditionDeleted(String),
    /// For bind traces we also provide the
    /// binding id of the affected spectrum.
    SpectrumBound {
        name: String,
        binding_id: usize,
    },
    SpectrumUnbound {
        name: String,
        binding_id: usize,
    },
}
/// Traces are timestamped with when they are logged.
/// as descsribed above, this enables aging:

#[derive(Clone)]
pub struct StampedTraceEvent {
    stamp: time::Instant,
    event: TraceEvent,
}
impl StampedTraceEvent {
    pub fn event(&self) -> TraceEvent {
        self.event.clone()
    }
}

/// Each client has several things associated with it:
///
/// * A client token.
/// * A trace lifetime
/// * A time orderd vector of traces.
///
pub struct ClientTraces {
    token: usize,
    trace_lifetime: time::Duration,
    trace_store: Vec<StampedTraceEvent>,
}
impl ClientTraces {
    pub fn new(token: usize, lifetime: time::Duration) -> ClientTraces {
        ClientTraces {
            token: token,
            trace_lifetime: lifetime,
            trace_store: Vec::new(),
        }
    }
}

/// This struct provides the trace store.  We need
///
/// * next_client - the token to be given to the next
/// client.
/// * A hash of ClientTraces.
///
pub struct TraceStore {
    next_client: usize,
    stop_prune_thread: bool,
    client_traces: HashMap<usize, ClientTraces>,
}

/// A shared TraceStore just holds a TraceStore in an Arc/Mutex
/// container.  The idea is that rather than implementing
/// the TraceStore API, I can implement the SharedTraceStore
/// API instead and hide all the lock().unwrap.crap() needed to
/// get me the trace store to operate on _and) maybe the
/// result can be Sendable.
///

pub struct SharedTraceStore {
    store: Arc<Mutex<TraceStore>>,
}

impl SharedTraceStore {
    fn add_to_all(&mut self, stamped_event: StampedTraceEvent) {
        for (_, v) in self.store.lock().unwrap().client_traces.iter_mut() {
            v.trace_store.push(stamped_event.clone());
        }
    }
    //
    pub fn new() -> SharedTraceStore {
        SharedTraceStore {
            store: Arc::new(Mutex::new(TraceStore {
                next_client: 0,
                stop_prune_thread: false,
                client_traces: HashMap::new(),
            })),
        }
    }
    pub fn clone(&self) -> SharedTraceStore {
        SharedTraceStore {
            store: Arc::clone(&self.store),
        }
    }
    /// Allocate a new token for a new trace client
    /// And add a trace store for it.
    pub fn new_client(&mut self, lifetime: time::Duration) -> usize {
        let mut store = self.store.lock().unwrap();
        let result = store.next_client;
        store.next_client += 1;

        store
            .client_traces
            .insert(result, ClientTraces::new(result, lifetime));

        result
    }
    /// Prune the client trace stores.
    /// for each client, we only retain those elements for which
    /// their timestamp is newer than the lifetime specified by
    /// that client.
    ///
    pub fn prune(&mut self) {
        let mut store = self.store.lock().unwrap();
        let now = time::Instant::now();
        for (_, v) in store.client_traces.iter_mut() {
            v.trace_store.retain(|x| {
                let age = now.duration_since(x.stamp);
                age < v.trace_lifetime
            });
        }
    }
    /// Add a new event to client traces.
    ///

    pub fn add_event(&mut self, event: TraceEvent) {
        let stamped_event = StampedTraceEvent {
            stamp: time::Instant::now(),
            event: event,
        };
        self.add_to_all(stamped_event);
    }

    /// Given a client token,
    /// Return its traces and clear them:

    pub fn get_traces(&mut self, token: usize) -> Result<Vec<StampedTraceEvent>, String> {
        let mut store = self.store.lock().unwrap();

        if store.client_traces.contains_key(&token) {
            let traces = store.client_traces.get_mut(&token).unwrap();
            let result = traces.trace_store.clone();
            traces.trace_store.clear();
            Ok(result)
        } else {
            Err(String::from("No such client token"))
        }
    }

    /// Once a shared trace store is created, this should be called
    /// to start the prune thread.
    /// It passes a clone of self to a thread that runs every second
    /// pruning the events.  Note that this is separate so that testing does not
    /// have to deal with asynchronous pruning.

    pub fn start_prune_thread(&mut self) -> thread::JoinHandle<()> {
        self.store.lock().unwrap().stop_prune_thread = false; // in case one was previously stopped.
        let mut thread_copy = self.clone();
        //
        // Note I _think_ I recall reading that a:
        //   while !thread_copy.store.lock().unwrap().stop_prune_thread {
        //    ...
        //   }
        // Would hold the lock for the duration of the loop effectively deadlocking
        // I'd love to be wrong as that's a moe natural formulation.
        //
        thread::spawn(move || loop {
            thread::sleep(time::Duration::from_secs(1));
            if thread_copy.store.lock().unwrap().stop_prune_thread {
                break;
            }
            thread_copy.prune();
        })
    }
    /// Schedule the prune thread to stop.
    /// Note that if the caller retains/has access to the join handle returned by
    /// start_prune_thread, it can synchronize with the stop otherwise,
    /// sleeping a couple of seconds should do it.
    pub fn stop_prune(&mut self) {
        self.store.lock().unwrap().stop_prune_thread = false;
    }
}

#[cfg(test)]
mod trace_store_tests {
    use super::*;
    use std::time;
    #[test]
    fn event_check() {
        let event = StampedTraceEvent {
            stamp: time::Instant::now(),
            event: TraceEvent::NewParameter(String::from("junk")),
        };

        assert!(match event.event() {
            TraceEvent::NewParameter(s) => {
                assert_eq!("junk", s);
                true
            }
            _ => false,
        });
    }
    #[test]
    fn ts_new_1() {
        // Make a new shared trace store and check that it's right:

        let store = SharedTraceStore::new();
        let inner = store.store.lock().unwrap();
        assert_eq!(0, inner.next_client);
        assert_eq!(false, inner.stop_prune_thread);
        assert!(inner.client_traces.is_empty());
    }
    #[test]
    fn ts_clone_1() {
        // a clone is actually a new reference to the same underlying data:

        let store = SharedTraceStore::new();
        let cloned = store.clone();
        store.store.lock().unwrap().next_client = 1234;
        store.store.lock().unwrap().stop_prune_thread = true;

        let c = cloned.store.lock().unwrap();
        assert_eq!(1234, c.next_client);
        assert_eq!(true, c.stop_prune_thread);
    }
    #[test]
    fn ts_add_client_1() {
        // adding a client increments the next_client field:
        // and adds the client to the hashmap:
        let mut store = SharedTraceStore::new();
        let token = store.new_client(time::Duration::from_secs(10));
        let s = store.store.lock().unwrap();

        assert_eq!(token + 1, s.next_client);
        assert!(s.client_traces.contains_key(&token));

        let c = s
            .client_traces
            .get(&token)
            .expect("Token not found in hashmap");
        assert_eq!(token, c.token);
        assert_eq!(time::Duration::from_secs(10), c.trace_lifetime);
        assert!(c.trace_store.is_empty());
    }
    #[test]
    pub fn ts_add_event_1() {
        // Adding an event with  no clients does nothing:

        let mut store = SharedTraceStore::new();
        store.add_event(TraceEvent::NewParameter(String::from("george")));

        assert!(store.store.lock().unwrap().client_traces.is_empty());
    }
    #[test]
    pub fn ts_add_event_2() {
        //  adding an event adds it to all cilents:

        let mut store = SharedTraceStore::new();
        let tok1 = store.new_client(time::Duration::from_secs(10));
        let tok2 = store.new_client(time::Duration::from_secs(11));
        assert!(tok1 != tok2);

        store.add_event(TraceEvent::NewParameter(String::from("george")));

        let s = store.store.lock().unwrap();
        assert_eq!(2, s.client_traces.len());
        let c1_traces = s.client_traces.get(&tok1).expect("Getting tok1 traces");
        assert_eq!(1, c1_traces.trace_store.len());
        assert!(match &c1_traces.trace_store[0].event {
            TraceEvent::NewParameter(s) => {
                assert_eq!("george", s);
                true
            }
            _ => false,
        });

        let c2_traces = s.client_traces.get(&tok2).expect("Getting tok1 traces");
        assert_eq!(1, c2_traces.trace_store.len());
        assert!(match &c2_traces.trace_store[0].event {
            TraceEvent::NewParameter(s) => {
                assert_eq!("george", s);
                true
            }
            _ => false,
        });
    }
    #[test]
    fn ts_prune_1() {
        // Prune things older than the expiration date.
        // THere's an assumption that the
        let mut store = SharedTraceStore::new();
        let tok1 = store.new_client(time::Duration::from_secs(10));
        let tok2 = store.new_client(time::Duration::from_secs(10));

        // add an event now so that will not go away:

        store.add_event(TraceEvent::NewParameter(String::from("George")));

        // dirty add of an event that's 25 seconds old:

        store.add_to_all(StampedTraceEvent {
            stamp: time::Instant::now()
                .checked_sub(time::Duration::from_secs(25))
                .unwrap(),
            event: TraceEvent::NewParameter(String::from("Ringo")),
        });
        store.prune(); // the 25 second old event pruned the now events should remain:

        let locked_store = store.store.lock().unwrap();

        let tok1_events = locked_store
            .client_traces
            .get(&tok1)
            .expect("Getting events for tok1");
        assert_eq!(1, tok1_events.trace_store.len());
        assert!(match &tok1_events.trace_store[0].event {
            TraceEvent::NewParameter(s) => {
                assert_eq!("George", s);
                true
            }
            _ => false,
        });

        let tok2_events = locked_store
            .client_traces
            .get(&tok2)
            .expect("Getting events for tok2");
        assert_eq!(1, tok2_events.trace_store.len());
        assert!(match &tok2_events.trace_store[0].event {
            TraceEvent::NewParameter(s) => {
                assert_eq!("George", s);
                true
            }
            _ => false,
        });
    }
    #[test]
    fn ts_get_1() {
        // get traces from a bad token is an error:

        let mut store = SharedTraceStore::new();
        assert!(store.get_traces(12345).is_err());
    }
    #[test]
    fn ts_get_2() {
        // Get traces from a valid token but no trces:

        let mut store = SharedTraceStore::new();
        let tok1 = store.new_client(time::Duration::from_secs(10));
        let traces = store
            .get_traces(tok1)
            .expect("Unable to get traces for tok1");
        assert_eq!(0, traces.len());
    }
}
