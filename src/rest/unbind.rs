//!  This module provides the handlers for the /spectcl/unbind
//!  domain of URLs.  This URLs take care of removing a spectrum
//!  from a binding it might have in shared memory.
//!  
//!  The URLS are:
//!
//! *  /spectcl/unbind/byname - unbind a single spectrum given its name.
//! *  /spectcl/unbind/byid - This is unimplemented as Rustogramer doesn't
//! assign and id to spectra.
//! *  /spectcl/unbind/all - all obund spectra are unbound.
//!
//!  For more information see the documentation for the specific
//!  function that implements each REST handler.
//!

use rocket::serde::json::Json;
use rocket::State;

use super::*;
use crate::sharedmem::binder;

//-------------------------------------------------------------
// unbind by name.

/// Unbind given a spectrum name.
///
/// ### Parameters  
/// *  name - Name of the spectrum to unbind.
/// *  state - the REST server state struct.
///
/// ### Returns
/// *   Json encoded GenericResponse.  On success, the detail
/// is an empty string.  On failure. the status is something like
/// _Failed to unbind {spectrum name} and the detail is the
/// reason given for the failure.
///
#[get("/byname?<name>")]
pub fn unbind_byname(name: String, state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = binder::BindingApi::new(&state.inner().binder.lock().unwrap());

    let response = if let Err(s) = api.unbind(&name) {
        GenericResponse::err(&format!("Failed to unbind {}", name), &s)
    } else {
        GenericResponse::ok("")
    };
    Json(response)
}
//---------------------------------------------------------------
// unbind by id.

/// Unbind by id is not possible in the Rustogramer because
/// ids are not assigned to spectra.
///
/// ### Parameters
/// (none)
/// ### Returns:
/// * Json encoded Generic Response with status _unbinding by id is not implemented_
/// and detail _This is not SpecTcl_
#[get("/byid")]
pub fn unbind_byid() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Unbind by id is not implemented",
        "This is not SpecTcl",
    ))
}
//------------------------------------------------------------------
// unbind all bindings.

/// Unbind all.  
///
/// ### Parameters
/// * state - the REST server state.
///
/// ### Returns
/// * Json encoded Generic response.  In case of error,
///  status is _Failed to unbind all spectra_ and detail is the
/// specific error message returned by the server.
///
#[get("/all")]
pub fn unbind_all(state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = binder::BindingApi::new(&state.inner().binder.lock().unwrap());

    let response = if let Err(s) = api.unbind_all() {
        GenericResponse::err("Failed to unbind all spectra", &s)
    } else {
        GenericResponse::ok("")
    };
    Json(response)
}
#[cfg(test)]
mod unbind_tests {
    use super::*;
    use crate::histogramer;
    use crate::messaging;
    use crate::messaging::{parameter_messages, spectrum_messages}; // to interrogate.

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
        let (_, hg_sender) = histogramer::start_server();

        let (binder_req, _jh) = binder::start_server(&hg_sender, 8 * 1024 * 1024);

        // Construct the state:

        let state = HistogramState {
            histogramer: Mutex::new(hg_sender.clone()),
            binder: Mutex::new(binder_req),
            processing: Mutex::new(processing::ProcessingApi::new(&hg_sender)),
            portman_client: None,
        };

        // Note we have two domains here because of the SpecTcl
        // divsion between tree parameters and raw parameters.

        rocket::build()
            .manage(state)
            .mount("/", routes![unbind_byname, unbind_byid, unbind_all])
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
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
        (chan, papi, binder_api)
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        let backing_file = b.exit().expect("Forcing binding thread to exit");
        thread::sleep(time::Duration::from_millis(100));
        fs::remove_file(Path::new(&backing_file)).expect(&format!(
            "Failed to remove shared memory file {}",
            backing_file
        ));
        histogramer::stop_server(&c);
        p.stop_thread().expect("Stopping processing thread");
    }

    #[test]
    fn unbindid_1() {
        // THis is always an error retur JSON:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating Rocket test client");
        let req = client.get("/byid?id=1");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("Unbind by id is not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bind_api);
    }
}
