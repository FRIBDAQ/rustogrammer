/// This module provides a set of ReST URIS to support client traces
/// of interesting events within Rustogramer.  These events include:
///
/// * Parameter creation.
/// * Spectrum creation.
/// * Spectrum deletion.
/// * Condition creation.
/// * Condition deletion.
/// * Condition modification.
/// * Addition of spectra to the bound set.
/// * Removal of spectra from the bound set.
///
/// See the src/trace/mod.rs module for the guts of the rustogramer trace
/// internals.
///
/// What we suppor is:
///
/// * Clients registering interest in traces - along with a maximum retention
///lifetime for its traces in the trace database.
/// * Clients unregistering interest in traces.
/// * Clients fetching the set of unexpired traces that were created since the
/// last time it fetched them.
///
///  We are depending on the main to have set the Rocket State to include
/// an trace::SharedTraceStore so that we can access the trace database created
/// by all of the trace producers.
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;

use super::*;
use crate::trace;
use std::time;

//---------------------------------------------------------------------------
// what's needed for the trace/establish interface:

///
/// This is the struct that's JSON encoded and returned by a
/// trace/establish call.  The detail is just a isize that is a token
/// that must be used in subsequent calls;

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct EstablishResponse {
    status: String,
    detail: usize,
}

/// handler for trace/establish
///  Get the trace database from the server state and invoke
///  new client to get a token to return.
/// this cannot fail.
///
/// #### Query Parameters:
/// *  retention - the number of seconds that traces for this client
/// will be retained before aging out.
///
#[get("/establish?<retention>")]
pub fn establish_trace(
    retention: u64,
    state: &State<trace::SharedTraceStore>,
) -> Json<EstablishResponse> {
    let lifetime = time::Duration::from_secs(retention);
    let token = state.inner().new_client(lifetime);

    Json(EstablishResponse {
        status: String::from("OK"),
        detail: token,
    })
}
///  When done tracing, or before exiting, a client should do a
/// call to trace/done - this releases all storage associted
/// with the trace.  If this is not done, a very small memory leak will
/// occur for the client.  Small because over time any trace data itself
/// will be pruned out as it hits its age.
///
/// #### Query Parameters:
/// *   token = the value of the detail field of the response from
/// the /trace/estabslih call that initiated trace data collection for this
/// client.
///
#[get("/done?<token>")]
pub fn trace_done(token: usize, state: &State<trace::SharedTraceStore>) -> Json<GenericResponse> {
    match state.inner().delete_client(token) {
        Ok(()) => Json(GenericResponse::ok("")),
        Err(s) => Json(GenericResponse::err("Unable to delete client", &s)),
    }
}

//-----------------------------------------------------------------------------
//  Stuff for trace/fetch

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct TraceDetail {
    parameter: Vec<String>,
    spectrum: Vec<String>,
    gate: Vec<String>,
    binding: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct TraceGetResponse {
    status: String,
    detail: TraceDetail,
}
///
/// A client can ask for its traces using /trace/fetch and providing its token
///
///  The result is odd and really intended for consumption by
///  Tcl clients.  The detail is of type TraceDetail which has a
///  field for each trace type and each trace is then an string in a vector of
///  strings (one for each type).  The strings are valid Tcl lists which
///  whose elements define the trace details as follows:
///
///  * parameter - each list contains:
///      *  The reason for the trace "add" or "changed" (note that Rustogramer cannot
/// delete parameters).
///      *  The name of the parameter affected.
///  * spectrum - Each list contains:
///      *  The trace reason ("add", or "delete"),
///      *  The name of the spectrum added or deleted.
///  * gate - Each list contains:
///      *  The trace reason:  "add", "delete", "changed"
///      *  The name of the gate that was affected.
///  * binding - Each list contains:
///      *  The trace reason which is one of "add" (bind), "remove" (unbind)
///      *  Then name of the affected spectrum.
///      *  The binding id of the affected spectrum.
///  
#[get("/fetch?<token>")]
pub fn fetch_traces(
    token: usize,
    state: &State<trace::SharedTraceStore>,
) -> Json<TraceGetResponse> {
    let mut result = TraceGetResponse {
        status: String::from("OK"),
        detail: TraceDetail {
            parameter: Vec::new(),
            spectrum: Vec::new(),
            gate: Vec::new(),
            binding: Vec::new(),
        },
    };
    match state.inner().get_traces(token) {
        Ok(traces) => {
            // Process the traces:
            for trace in traces {
                match trace.event() {
                    trace::TraceEvent::NewParameter(name) => {
                        result.detail.parameter.push(format!("add {}", name))
                    }
                    trace::TraceEvent::ParameterModified(name) => {
                        result.detail.parameter.push(format!("changed {}", name))
                    }
                    trace::TraceEvent::SpectrumCreated(name) => {
                        result.detail.spectrum.push(format!("add {}", name))
                    }
                    trace::TraceEvent::SpectrumDeleted(name) => {
                        result.detail.spectrum.push(format!("delete {}", name))
                    }
                    trace::TraceEvent::ConditionCreated(name) => {
                        result.detail.gate.push(format!("add {}", name))
                    }
                    trace::TraceEvent::ConditionModified(name) => {
                        result.detail.gate.push(format!("changed {}", name))
                    }
                    trace::TraceEvent::ConditionDeleted(name) => {
                        result.detail.gate.push(format!("delete {}", name))
                    }
                    trace::TraceEvent::SpectrumBound { name, binding_id } => result
                        .detail
                        .binding
                        .push(format!("add {} {}", name, binding_id)),
                    trace::TraceEvent::SpectrumUnbound { name, binding_id } => result
                        .detail
                        .binding
                        .push(format!("remove {} {}", name, binding_id)),
                }
            }
        }
        Err(msg) => result.status = format!("Unable to fetch traces for token {}: {}", token, msg),
    }

    Json(result)
}
