//!  Folds are a concept specific to the analysis of sequential
//!  decays by gamma emission.  The idea is that you can create a
//!  condition that involves the parameters of a multiply incremented
//!  spetrum.  Normally, this codition is a specific decay peak in
//!  the spectrum.  
//!
//!  A fold then increments this spectrum for all parameters that
//!  don't make this condition true (folds could be one or 2-d).
//!  What remains in the spectrum are the peaks that correspond
//!  to gamma rays in the same sequence of decays.
//!
//! The initial version of Rustogramer does not implement folds.
//! Therefore we report to the client that all /spectcl/fold URIs
//! defined by the REST interface are not supported.  I'll note that
//! of all unsupported elements of the REST specification, this
//! one is most likely to be eventually supported.
//!  
//! /spectcl/fold has the following URIs under this domain:
//!
//! *   apply - applies a condition to a spectrum as a fold.
//! *   list  - lists the fold applications
//! *   remove - Removes a fold from the spectrum.
//!
use super::*;
use rocket::serde::{json::Json, Deserialize, Serialize};

/// apply - unimplemented
///  If implemented the following query parameters would be required:
///
/// *  gate - the condition that defines the fold.
/// *  spectrum - the spectrum to be folded.
///
/// A GenericResponse is perfectly appropriate.
///
#[get("/apply")]
pub fn apply() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/fold/apply is not implemented",
        "This is not SpecTcl",
    ))
}

//
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct FoldInfo {
    spectrum: String,
    gate: String,
}
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct FoldListResponse {
    status: String,
    detail: Vec<FoldInfo>,
}
/// list - unimplemented
///  If implemented the _pattern_ query  parameter will filter out
/// the listing to only inlcude the spectra with names that match the
/// pattern.  The reply is a FoldListResponse shown above.
#[get("/list")]
pub fn list() -> Json<FoldListResponse> {
    Json(FoldListResponse {
        status: String::from("/spectcl/fold/list is not implemented - this is not SpecTcl"),
        detail: vec![],
    })
}
/// remove - unimplemented
///
/// Requires one query parameter _spectrum_ Any fold will be removed
/// from that spectrum.
///
/// GenericResponse is appropriate.
///
#[get("/remove")]
pub fn remove() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/fold/remove is not implemented",
        "This is not SpecTcl",
    ))
}
#[cfg(test)]
mod fold_tests {
    use super::*;
    use crate::histogramer;
    use crate::messaging;
    use crate::processing;
    use crate::sharedmem::binder;
    use crate::trace;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::{mpsc, Arc, Mutex};
    // note these are all unimplemented URLS so...

    fn setup() -> Rocket<Build> {
        let tracedb = trace::SharedTraceStore::new();
        let (_, hg_sender) = histogramer::start_server(tracedb.clone());
        let (binder_req, _rx): (
            mpsc::Sender<binder::Request>,
            mpsc::Receiver<binder::Request>,
        ) = mpsc::channel();

        // Construct the state:

        let state = HistogramState {
            portman_client: None,
            mirror_exit: Arc::new(Mutex::new(mpsc::channel::<bool>().0)),
            mirror_port: 0,
        };

        rocket::build()
            .manage(state)
            .manage(Mutex::new(hg_sender.clone()))
            .manage(Mutex::new(binder_req))
            .manage(Mutex::new(processing::ProcessingApi::new(&hg_sender)))
            .manage(tracedb.clone())
            .mount("/", routes![crate::fold::apply, list, remove])
    }
    fn teardown(c: mpsc::Sender<messaging::Request>, p: &processing::ProcessingApi) {
        histogramer::stop_server(&c);
        p.stop_thread().expect("Stopping processing thread");
    }
    fn get_state(
        r: &Rocket<Build>,
    ) -> (mpsc::Sender<messaging::Request>, processing::ProcessingApi) {
        let chan = r
            .state::<SharedHistogramChannel>()
            .expect("Valid state")
            .lock()
            .unwrap()
            .clone();
        let papi = r
            .state::<SharedProcessingApi>()
            .expect("Valid State")
            .lock()
            .unwrap()
            .clone();

        (chan, papi)
    }
    // Note none of thes URIS are implemented so the tests are
    // simple and don't need any other setup.  These tests are
    // actually placeholders against the eventual implementation
    // of folds in rustogramer.

    #[test]
    fn apply_1() {
        let rocket = setup();
        let (c, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/apply");

        let response = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Decoding JSON");

        assert_eq!(
            "/spectcl/fold/apply is not implemented",
            response.status.as_str()
        );
        assert_eq!("This is not SpecTcl", response.detail.as_str());

        teardown(c, &papi);
    }
    #[test]
    fn list_1() {
        let rocket = setup();
        let (c, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/list");

        let response = req
            .dispatch()
            .into_json::<FoldListResponse>()
            .expect("Decoding JSON");
        assert_eq!(
            "/spectcl/fold/list is not implemented - this is not SpecTcl",
            response.status.as_str()
        );
        assert_eq!(0, response.detail.len());

        teardown(c, &papi);
    }
    #[test]
    fn remove_1() {
        let rocket = setup();
        let (c, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/remove");

        let response = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Decoding JSON");

        assert_eq!(
            "/spectcl/fold/remove is not implemented",
            response.status.as_str()
        );
        assert_eq!("This is not SpecTcl", response.detail.as_str());

        teardown(c, &papi);
    }
}
