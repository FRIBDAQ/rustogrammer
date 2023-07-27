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

        for (_, v) in self.store.lock().unwrap().client_traces.iter_mut() {
            v.trace_store.push(stamped_event.clone());
        }
    }

    /// Given a client token,
    /// Return its traces and clear them:

    pub fn get_traces(&mut self, token: usize) -> Result<Vec<StampedTraceEvent>, String> {
        let mut store = self.store.lock().unwrap();

        if !store.client_traces.contains_key(&token) {
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
        let mut thread_copy = self.clone();
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
