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
use std::sync::{Arc, Mutex, MutexGuard};
use std::time;

/// The various types of traces
/// If the payload for an enumerated type
/// is just a string, that's the name of the affected
/// object (e.g. the name of the new parameter for NewParameter below).
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

pub struct StampedTraceEvent {
    stamp: time::SystemTime,
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
    trace_store: Vec<TraceEvent>,
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
    client_traces: HashMap<usize, ClientTraces>,
}

/// A shared TraceStore just holds a TraceStore in an Arc/Mutex
/// container.  The idea is that rather than implementing
/// the TraceSTore API, I can implement the SharedTraceStore
/// API instead an hide all the lock().unwrap.crap() needed to
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
}
