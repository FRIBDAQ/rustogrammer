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
    use crate::trace;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::fs;
    use std::path::Path;
    use std::sync::{mpsc, Arc, Mutex};
    use std::thread;
    use std::time;
    fn setup() -> Rocket<Build> {
        let tracedb = trace::SharedTraceStore::new();
        let (_, hg_sender) = histogramer::start_server(tracedb.clone());

        let (binder_req, _jh) = binder::start_server(&hg_sender, 8 * 1024 * 1024, &tracedb);

        // Construct the state:

        let state = HistogramState {
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
            .manage(Mutex::new(hg_sender.clone()))
            .manage(tracedb.clone())
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
            .state::<SharedHistogramChannel>()
            .expect("Valid state")
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
        let _ = fs::remove_file(Path::new(&backing_file)); // faliure is ok.
        histogramer::stop_server(&c);
        p.stop_thread().expect("Stopping processing thread");
    }
    fn make_some_spectra(chan: &mpsc::Sender<messaging::Request>) {
        // Make parameters p.0 .. p.9 and a 1d for each.  The spectrum
        // type doesn't really matter as that is/was tested in the sharedmem
        // tests.
        let papi = parameter_messages::ParameterMessageClient::new(&chan);
        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        for i in 0..10 {
            let name = format!("p.{}", i);
            papi.create_parameter(&name).expect("making a parameter");
            sapi.create_spectrum_1d(&name, &name, 0.0, 512.0, 512)
                .expect("Making a spectrum");
        }
    }
    fn bind_spectrum_list(api: &binder::BindingApi, names: Vec<String>) {
        for n in names {
            api.bind(&n).expect("attempting to bind a spectrum");
        }
    }

    #[test]
    fn byid_1() {
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
    #[test]
    fn byname_1() {
        // NO matching spectrum to unbind:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating Rocket test client");
        let req = client.get("/byname?name=nosuch");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Failed to unbind nosuch", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn byname_2() {
        // Just one spectrum so no selectivity needed:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        make_some_spectra(&chan);
        bind_spectrum_list(&bind_api, vec![String::from("p.0")]);

        let client = Client::untracked(rocket).expect("Creating rocket test client");
        let req = client.get("/byname?name=p.0");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        // the be no bindings now:

        assert_eq!(
            0,
            bind_api.list_bindings("*").expect("Listing bindings").len()
        );

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn byname_3() {
        // THe correct spectrum is selected for deletion from a
        // bunch:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        make_some_spectra(&chan);
        bind_spectrum_list(
            &bind_api,
            vec![
                String::from("p.0"),
                String::from("p.1"),
                String::from("p.2"),
                String::from("p.3"),
                String::from("p.4"),
                String::from("p.5"),
                String::from("p.6"),
            ],
        );

        let client = Client::untracked(rocket).expect("Creating rocket test client");
        let req = client.get("/byname?name=p.1");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        let bindings = bind_api.list_bindings("*").expect("listing bindings");
        assert_eq!(6, bindings.len()); // one fewer than before.
        for (_, name) in bindings {
            assert!("p1" != name);
        }

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn all_1() {
        // Bind a bunch, unbind all:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        make_some_spectra(&chan);
        bind_spectrum_list(
            &bind_api,
            vec![
                String::from("p.0"),
                String::from("p.1"),
                String::from("p.2"),
                String::from("p.3"),
                String::from("p.4"),
                String::from("p.5"),
                String::from("p.6"),
                String::from("p.7"),
                String::from("p.8"),
                String::from("p.9"),
            ],
        );

        let client = Client::untracked(rocket).expect("Creating rocket test client");
        let req = client.get("/all");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status, "Reason for failure: {}", reply.detail);

        let bindings = bind_api.list_bindings("*").expect("listing bindings");
        assert_eq!(0, bindings.len()); // all gone.

        teardown(chan, &papi, &bind_api);
    }
}
