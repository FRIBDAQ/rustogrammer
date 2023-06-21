//!  This module provides the REST interface to the procesing
//!  thread.  The assumption is that he field _processing_ in the
//!  HistogramState object contains a Mutex wrapped
//!  ProcessingApi object, and the analysis thread has already
//!  been started.
//!  
//! Two mount points are provided:
//!  
//!  *  /attach which provides the attach, detach and list methods.
//!  *  /analyze which provides the start, stop and eventchunk
//! methods.

// Imports:

use rocket::serde::json::Json;
use rocket::State;

use super::*;

//---------------------------------------------------------------
// The /attach mount point:

/// Attach a data source.
/// Note that this version of rustogrammer only support type=file
/// Query parameters:
///
/// *  type - the type of attach (file is the only one supported).
/// *  source - in this case the name of the data file to attach.
/// *  size (ignored) - for compatiblity with SpecTcl's API.
///
/// The response is a generic resposne with the detail empty on
/// success and containing more detailed error message on failure
/// than that in status.
#[allow(unused_variables)]
#[get("/attach?<type>&<source>&<size>")]
pub fn attach_source(
    r#type: String,
    source: String,
    size: OptionalString,
    state: &State<HistogramState>,
) -> Json<GenericResponse> {
    let reply = if r#type == "file" {
        let api = state.inner().processing.lock().unwrap();
        if let Err(s) = api.attach(&source) {
            GenericResponse::err("Attach failed", &s)
        } else {
            GenericResponse::ok("")
        }
    } else {
        GenericResponse::err(
            &format!("Data source type '{}' is not supported", r#type),
            "This is Rustogramer not SpecTcl",
        )
    };
    Json(reply)
}

/// list the current data source.
/// this has no query parameters:
///  On success, detail contains the data source.
///  on failure, the error from the api.
#[get("/list")]
pub fn list_source(state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = state.inner().processing.lock().unwrap();
    Json(match api.list() {
        Ok(s) => GenericResponse::ok(&s),
        Err(s) => GenericResponse::err("Failed to get data source", &s),
    })
}
/// Detach from the current data source.
///  This is specific to Rustogramer.
///
#[get("/detach")]
pub fn detach_source(state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = state.inner().processing.lock().unwrap();
    Json(match api.detach() {
        Ok(s) => GenericResponse::ok(&s),
        Err(s) => GenericResponse::err("Failed to detach", &s),
    })
}

//--------------------------------------------------------------
// The /analyze mount point.
//

/// start - starts analyzing data on the currently attached
/// data source.  No query parameters are required/accepted.
#[get("/start")]
pub fn start_processing(state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = state.inner().processing.lock().unwrap();
    Json(match api.start_analysis() {
        Ok(_) => GenericResponse::ok(""),
        Err(s) => GenericResponse::err("Failed to start analysis", &s),
    })
}
///
/// stop stops analyzing data on the current data source.
/// No query parameters are required.
///
#[get("/stop")]
pub fn stop_processing(state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = state.inner().processing.lock().unwrap();
    Json(match api.stop_analysis() {
        Ok(_) => GenericResponse::ok(""),
        Err(s) => GenericResponse::err("Failed to stop analysis", &s),
    })
}
/// Set the analysis block size.  This is the number of events that
/// will be sent to the histograming thread for each analysis request.
///
/// The query parameter _events_ must be the number of events.
///
#[get("/size?<events>")]
pub fn set_event_batch(events: usize, state: &State<HistogramState>) -> Json<GenericResponse> {
    let mut api = state.inner().processing.lock().unwrap();
    Json(match api.set_batching(events) {
        Ok(_) => GenericResponse::ok(""),
        Err(s) => GenericResponse::err("Failed to set event processing batch size", &s),
    })
}
#[cfg(test)]
mod processing_tests {
    use super::*;
    use crate::histogramer;
    use crate::messaging;
    use crate::processing;
    use crate::sharedmem::binder;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;
    use std::sync::Mutex;

    // Setup needs to set a state for Rocket that includes valid
    // histogramer request channel and thread.
    // binder channel (no need for thread).
    // processing channel and thread.
    // No port manager instance.

    fn setup() -> Rocket<Build> {
        let (_, hg_sender) = histogramer::start_server();
        let (binder_req, _rx): (
            mpsc::Sender<binder::Request>,
            mpsc::Receiver<binder::Request>,
        ) = mpsc::channel();

        // Construct the state:

        let state = HistogramState {
            histogramer: Mutex::new(hg_sender.clone()),
            binder: Mutex::new(binder_req),
            processing: Mutex::new(processing::ProcessingApi::new(&hg_sender.clone())),
            portman_client: None,
        };

        rocket::build().manage(state).mount(
            "/",
            routes![
                attach_source,
                list_source,
                detach_source,
                start_processing,
                stop_processing,
                set_event_batch
            ],
        )
    }
    fn teardown(c: mpsc::Sender<messaging::Request>, p: &processing::ProcessingApi) {
        histogramer::stop_server(&c);
        p.stop_thread().expect("Stopping processing thread");
    }
    fn get_state(
        r: &Rocket<Build>,
    ) -> (mpsc::Sender<messaging::Request>, processing::ProcessingApi) {
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

        (chan, papi)
    }
    #[test]
    fn attach_1() {
        // fail attach because the type is bad:

        let rocket = setup();
        let (chan, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/attach?type=pipe&source=ring2stdout");
        let reply = req.dispatch();

        let json = reply
            .into_json::<GenericResponse>()
            .expect("Bad Json returned");

        assert_eq!(
            "Data source type 'pipe' is not supported",
            json.status.as_str()
        );

        teardown(chan, &papi);
    }
    #[test]
    fn attach_2() {
        // fail attach b/c no such file:

        let rocket = setup();
        let (chan, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/attach?type=file&source=no-such_file.par");
        let reply = req.dispatch();

        let json = reply.into_json::<GenericResponse>().expect("Bad JSON");
        assert_eq!("Attach failed", json.status.as_str());

        teardown(chan, &papi);
    }
    #[test]
    fn attach_3() {
        // success

        let rocket = setup();
        let (chan, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/attach?type=file&source=run-0000-00.par");
        let json = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Bad JSON");

        assert_eq!("OK", json.status.as_str());

        // double check the file is attached in the processing
        // thread:

        let reply = papi.list().expect("Getting attchment");
        assert_eq!("file:run-0000-00.par", reply);

        teardown(chan, &papi);
    }
    #[test]
    fn list_1() {
        // not attached:

        let rocket = setup();
        let (chan, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating clent");
        let req = client.get("/list");
        let reply = req.dispatch().into_json::<GenericResponse>().expect("Bad JSON");

        assert_eq!("OK", reply.status.as_str());
        assert_eq!("Not Attached", reply.detail.as_str());

        teardown(chan, &papi);
    }
    #[test]
    fn list_2() {
        // attached
        let rocket = setup();
        let (chan, papi)= get_state(&rocket);

        papi.attach("run-0000-00.par");  // attach the easy way
        
        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/list");
        let reply = req.dispatch().into_json::<GenericResponse>().expect("Bad JSON");

        assert_eq!("OK", reply.status.as_str());
        assert_eq!("file:run-0000-00.par", reply.detail.as_str());

        teardown(chan, &papi);
    }
}
