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
/// delete parameters).  Note that changed is a new trace.
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

// Tests for the ReST interface.

#[cfg(test)]
mod trace_rest_tests {
    use super::*;
    use crate::messaging;
    use crate::test::rest_common;
    use crate::trace;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;

    fn setup() -> Rocket<Build> {
        rest_common::setup().mount("/", routes![establish_trace, trace_done, fetch_traces])
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
        trace::SharedTraceStore,
    ) {
        let common_state = rest_common::get_state(r);
        let tracedb = r
            .state::<trace::SharedTraceStore>()
            .expect("Getting state")
            .clone();
        (common_state.0, common_state.1, common_state.2, tracedb)
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        rest_common::teardown(c, p, b);
    }
    fn get_token(client: &Client, retention: usize) -> usize {
        let uri = format!("/establish?retention={}", retention);
        let req = client.get(&uri);
        let reply = req
            .dispatch()
            .into_json::<EstablishResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", reply.status);
        reply.detail
    }
    fn free_token(client: &Client, token: usize) {
        let uri = format!("/done?token={}", token);
        let req = client.get(&uri);
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", reply.status);
    }
    #[test]
    fn establish_1() {
        // establish a single client - our token will be 0.

        let rocket = setup();
        let (msg_chan, papi, binder_api, _tracdb) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket client");

        assert_eq!(0, get_token(&client, 10));

        teardown(msg_chan, &papi, &binder_api);
    }
    #[test]
    fn establish_2() {
        // Establishing two clients should give different tokens:

        let rocket = setup();
        let (msg_chan, papi, binder_api, _tracdb) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket client");
        let tok1 = get_token(&client, 10);
        assert_eq!(0, tok1);

        let tok2 = get_token(&client, 10);
        assert!(tok1 != tok2); // This is actually the only required test.
        assert_eq!(1, tok2); // White box knowing how they're supposed to be allocated

        teardown(msg_chan, &papi, &binder_api);
    }
    #[test]
    fn done_1() {
        // Done on a token we don't have is an error:

        let rocket = setup();
        let (msg_chan, papi, binder_api, _tracdb) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let request = client.get("/done?token=0");
        let reply = request
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing JSON");

        assert!("OK" != reply.status);

        teardown(msg_chan, &papi, &binder_api);
    }
    #[test]
    fn done_2() {
        // can successfully be done with an allocated token.

        let rocket = setup();
        let (msg_chan, papi, binder_api, _tracedb) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");

        let token = get_token(&client, 10);

        let free_uri = format!("/done?token={}", token);
        let free = client.get(&free_uri);
        let free_response = free
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", free_response.status);

        teardown(msg_chan, &papi, &binder_api);
    }
    #[test]
    fn done_3() {
        // Can't be done with the same token twice.

        let rocket = setup();
        let (msg_chan, papi, binder_api, _tracedb) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");

        let token = get_token(&client, 10);
        free_token(&client, token);

        let free_req = format!("/done?token={}", token);
        let free = client.get(&free_req);
        let free_response = free
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert!("OK" != free_response.status);

        teardown(msg_chan, &papi, &binder_api);
    }
    // In the tests to get traces, the simplest way to get traces inserted
    // initially is to just put them in the tracedb ourself.
    //
    #[test]
    fn get_1() {
        // Nothing to trace - so we get an empty set  of arrays:

        let rocket = setup();
        let (msg_chan, papi, binder_api, _tracedb) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making client");
        let token = get_token(&client, 10);

        let uri = format!("/fetch?token={}", token);
        let req = client.get(&uri);
        let response = req
            .dispatch()
            .into_json::<TraceGetResponse>()
            .expect("Parsing JSon");

        assert_eq!("OK", response.status);
        assert_eq!(0, response.detail.parameter.len());
        assert_eq!(0, response.detail.spectrum.len());
        assert_eq!(0, response.detail.gate.len());
        assert_eq!(0, response.detail.binding.len());

        free_token(&client, token);
        teardown(msg_chan, &papi, &binder_api);
    }
    #[test]
    fn get_2() {
        // one of each type of parameter trace:

        let rocket = setup();
        let (msg_chan, papi, binder_api, tracedb) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making client");
        let token = get_token(&client, 10); // Need this to save traces:

        tracedb.add_event(trace::TraceEvent::NewParameter(String::from("newpar")));
        tracedb.add_event(trace::TraceEvent::ParameterModified(String::from("newpar")));

        // Now fetch our traces.

        let uri = format!("/fetch?token={}", token);
        let req = client.get(&uri);
        let response = req
            .dispatch()
            .into_json::<TraceGetResponse>()
            .expect("Parsing JSon");

        assert_eq!("OK", response.status);
        assert_eq!(2, response.detail.parameter.len());
        assert_eq!(0, response.detail.spectrum.len());
        assert_eq!(0, response.detail.gate.len());
        assert_eq!(0, response.detail.binding.len());

        // THe first one should have "add newpar"
        // the second "changed newpar"
        assert_eq!("add newpar", response.detail.parameter[0]);
        assert_eq!("changed newpar", response.detail.parameter[1]);

        free_token(&client, token);
        teardown(msg_chan, &papi, &binder_api);
    }
    #[test]
    fn get_3() {
        // test spectrum trace handling:

        // one of each type of parameter trace:

        let rocket = setup();
        let (msg_chan, papi, binder_api, tracedb) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making client");
        let token = get_token(&client, 10); // Need this to save traces:

        tracedb.add_event(trace::TraceEvent::SpectrumCreated(String::from("newpar")));
        tracedb.add_event(trace::TraceEvent::SpectrumDeleted(String::from("newpar")));

        // Now fetch our traces.

        let uri = format!("/fetch?token={}", token);
        let req = client.get(&uri);
        let response = req
            .dispatch()
            .into_json::<TraceGetResponse>()
            .expect("Parsing JSon");

        assert_eq!("OK", response.status);
        assert_eq!(0, response.detail.parameter.len());
        assert_eq!(2, response.detail.spectrum.len());
        assert_eq!(0, response.detail.gate.len());
        assert_eq!(0, response.detail.binding.len());

        // THe first one should have "add newpar"
        // the second "changed newpar"
        assert_eq!("add newpar", response.detail.spectrum[0]);
        assert_eq!("delete newpar", response.detail.spectrum[1]);

        free_token(&client, token);
        teardown(msg_chan, &papi, &binder_api);
    }
    #[test]
    fn get_4() {
        // one of each type of parameter trace:

        let rocket = setup();
        let (msg_chan, papi, binder_api, tracedb) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making client");
        let token = get_token(&client, 10); // Need this to save traces:

        tracedb.add_event(trace::TraceEvent::ConditionCreated(String::from("newpar")));
        tracedb.add_event(trace::TraceEvent::ConditionModified(String::from("newpar")));
        tracedb.add_event(trace::TraceEvent::ConditionDeleted(String::from("newpar")));

        // Now fetch our traces.

        let uri = format!("/fetch?token={}", token);
        let req = client.get(&uri);
        let response = req
            .dispatch()
            .into_json::<TraceGetResponse>()
            .expect("Parsing JSon");

        assert_eq!("OK", response.status);
        assert_eq!(0, response.detail.parameter.len());
        assert_eq!(0, response.detail.spectrum.len());
        assert_eq!(3, response.detail.gate.len());
        assert_eq!(0, response.detail.binding.len());

        // THe first one should have "add newpar"
        // the second "changed newpar"
        assert_eq!("add newpar", response.detail.gate[0]);
        assert_eq!("changed newpar", response.detail.gate[1]);
        assert_eq!("delete newpar", response.detail.gate[2]);

        free_token(&client, token);
        teardown(msg_chan, &papi, &binder_api);
    }
    #[test]
    fn get_5() {
        //binding trace handling:

        let rocket = setup();
        let (_msg_chan, _papi, _binder_api, tracedb) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making client");
        let token = get_token(&client, 10); // Need this to save traces:

        tracedb.add_event(trace::TraceEvent::SpectrumBound {
            name: String::from("aspec"),
            binding_id: 123,
        });
        tracedb.add_event(trace::TraceEvent::SpectrumUnbound {
            name: String::from("aspec"),
            binding_id: 123,
        });

        // Now fetch our traces.

        let uri = format!("/fetch?token={}", token);
        let req = client.get(&uri);
        let response = req
            .dispatch()
            .into_json::<TraceGetResponse>()
            .expect("Parsing JSon");

        assert_eq!("OK", response.status);
        assert_eq!(0, response.detail.parameter.len());
        assert_eq!(0, response.detail.spectrum.len());
        assert_eq!(0, response.detail.gate.len());
        assert_eq!(2, response.detail.binding.len());

        // THe first one should have "add newpar"
        // the second "changed newpar"

        assert_eq!("add aspec 123", response.detail.binding[0]);
        assert_eq!("remove aspec 123", response.detail.binding[1]);

        free_token(&client, token);
    }
}
