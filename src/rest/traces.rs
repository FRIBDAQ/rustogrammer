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

// Tests for the ReST interface that, by their nature are integration tests
// for the entire tracing subsystem.

#[cfg(test)]
mod trace_rest_tests {
    use super::*;
    use crate::histogramer;
    use crate::messaging;
    use crate::trace;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::fs;
    use std::path::Path;
    use std::sync::mpsc;
    use std::sync::Mutex;
    use std::thread;
    use std::time;
    fn setup() -> Rocket<Build> {
        let tracedb = trace::SharedTraceStore::new();
        let (_, hg_sender) = histogramer::start_server(tracedb.clone());

        let (binder_req, _jh) = binder::start_server(&hg_sender, 8 * 1024 * 1024, &tracedb);

        // Construct the state:

        let state = HistogramState {
            histogramer: Mutex::new(hg_sender.clone()),
            binder: Mutex::new(binder_req),
            processing: Mutex::new(processing::ProcessingApi::new(&hg_sender)),
            portman_client: None,
            mirror_exit: Arc::new(Mutex::new(mpsc::channel::<bool>().0)),
            mirror_port: 0,
        };

        // Note we have two domains here because of the SpecTcl
        // divsion between tree parameters and raw parameters.

        rocket::build()
            .manage(state)
            .manage(tracedb.clone())
            .mount("/", routes![establish_trace, trace_done, fetch_traces])
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
        trace::SharedTraceStore,
    ) {
        let chan = r
            .state::<HistogramState>()
            .expect("Valid state")
            .histogramer
            .lock()
            .unwrap()
            .clone();
        let papi = r
            .state::<HistogramState>()
            .expect("Valid State")
            .processing
            .lock()
            .unwrap()
            .clone();
        let binder_api = binder::BindingApi::new(
            &r.state::<HistogramState>()
                .expect("Valid State")
                .binder
                .lock()
                .unwrap(),
        );
        let tracedb = r
            .state::<trace::SharedTraceStore>()
            .expect("Valid tracedb")
            .clone();
        (chan, papi, binder_api, tracedb)
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        let backing_file = b.exit().expect("Forcing binding thread to exit");
        thread::sleep(time::Duration::from_millis(100));
        let _ = fs::remove_file(Path::new(&backing_file)); // faliure is ok.
        histogramer::stop_server(&c);
        p.stop_thread().expect("Stopping processing thread");
    }
    #[test]
    fn establish_1() {
        // establish a single client - our token will be 0.

        let rocket = setup();
        let (msg_chan, papi, binder_api, _tracdb) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket client");
        let request = client.get("/establish?retention=10");
        let response = request
            .dispatch()
            .into_json::<EstablishResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", response.status);
        assert_eq!(0, response.detail);

        teardown(msg_chan, &papi, &binder_api);
    }
    #[test]
    fn establish_2() {
        // Establishing two clients should give different tokens:

        let rocket = setup();
        let (msg_chan, papi, binder_api, _tracdb) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket client");
        let request = client.get("/establish?retention=10");
        let response = request
            .dispatch()
            .into_json::<EstablishResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", response.status);
        assert_eq!(0, response.detail);
        let tok1 = response.detail;

        let request = client.get("/establish?retention=10");
        let response = request
            .dispatch()
            .into_json::<EstablishResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", response.status);
        assert!(tok1 != response.detail); // This is actually the only required test.
        assert_eq!(1, response.detail); // White box knowing how they're supposed to be allocated

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
        let establish = client.get("/establish?retention=10");
        let est_response = establish
            .dispatch()
            .into_json::<EstablishResponse>()
            .expect("parsing JSON");

        assert_eq!("OK", est_response.status);
        let token = est_response.detail;

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
        let establish = client.get("/establish?retention=10");
        let est_response = establish
            .dispatch()
            .into_json::<EstablishResponse>()
            .expect("parsing JSON");

        assert_eq!("OK", est_response.status);
        let token = est_response.detail;

        let free_req = format!("/done?token={}", token);
        let free = client.get(&free_req);
        let free_response = free
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", free_response.status);

        let free = client.get(&free_req);
        let free_response = free
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert!("OK" != free_response.status);

        teardown(msg_chan, &papi, &binder_api);
    }
}
