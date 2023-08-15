//!  Provides support for the /spectcl/sbind domain of REST URIs.
//!  This set of URIs allows clients ot bind spectra into shared
//!  memory.  In Rustogramer this is done via interactions with both
//!  the histogramer and bindings threads via channels stored in
//!  the State of the server.
//!  
//!  Note that sbind is used historically because in SpecTcl,
//!  **bind** is a command in Tcl that binds events to a widget.
//!  so the SpecTcl command to do a binding was **sbind** or
//!  **s**pectrum**bind**.
//!
//!  URis we support are:
//!
//! *  /spectcl/sbind/all - attempt to bind all spectra to shared
//! memory.
//! *  /spectcl/sbind/sbind - Bind a list of spectra to shared memory
//! by name.
//! *  /spectcl/sbind/list - list the bindings.  See, however
//! the documentation for sbind_list below.

// Imports.
use super::*;
use crate::messaging::spectrum_messages;
use crate::sharedmem::binder;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;

use std::collections::HashSet;

//--------------------------------------------------------------
// /sbind/all

// Create the hash of binding names:

fn make_binding_hash(bindings: &Vec<(usize, String)>) -> HashSet<String> {
    // Put the names of the bound spectra into a HashSet so the
    // lookup is O(1):
    let mut binding_hash = HashSet::<String>::new();
    for binding in bindings {
        binding_hash.insert(binding.1.clone());
    }

    binding_hash
}
// Given a list of spectrum definitions makes a list of spectrum names:

fn make_spectrum_names(spectra: &Vec<spectrum_messages::SpectrumProperties>) -> Vec<String> {
    let mut result = Vec::<String>::new();
    for spectrum in spectra {
        result.push(spectrum.name.clone());
    }
    result
}
// Given a vec of spectrum names and a binding has, return the names
// not in the hash:
fn remove_bound_spectra(names: &Vec<String>, bindings: &HashSet<String>) -> Vec<String> {
    let mut result = Vec::<String>::new();
    for name in names {
        if !bindings.contains(name) {
            result.push(name.clone());
        }
    }

    result
}

// Given a list of the spectrum defs. and the bindings return a list of
// the spectrum names that are not bound.

fn list_unbound_spectra(
    spectra: &Vec<spectrum_messages::SpectrumProperties>,
    bindings: &Vec<(usize, String)>,
) -> Vec<String> {
    // Put the names of the bound spectra into a HashSet so the
    // lookup is O(1):

    let binding_hash = make_binding_hash(&bindings);
    let spectrum_names = make_spectrum_names(spectra);

    remove_bound_spectra(&spectrum_names, &binding_hash)
}

// This function binds a set of spectra and returns the response to
// be Json'd.  It is used by sbind_all and sbind_list

fn bind_spectrum_list(
    spectra_to_bind: &Vec<String>,
    binding_api: &binder::BindingApi,
) -> GenericResponse {
    for name in spectra_to_bind {
        if let Err(s) = binding_api.bind(&name) {
            return GenericResponse::err(&format!("Unable to bind spectrum {}", name), &s);
        }
    }

    GenericResponse::ok("")
}

/// Bind all unbound spectra to the shared memory.  To avoid
/// errors and minimize transaction with the binding thread,
/// we get a list of all spectra and a list of all bound spectra.
/// we form a list of the spectra that are not yet bound and,
/// since the binding thread only support binding one at a time,
/// loop over the spectra attempting to bind them all.
/// If there's an error we stop right there and report the error
/// for that binding attempt.
///
/// ### Parameters
/// *  state - Histogrammer REST interface state.
///
/// ### Returns
/// * Json encoded GenericResponse.  On success, detail is empty.
/// There are three types of failures to consider:
///      -  Unable to get the spectrum list - in which case the status
/// is _Unable to obtain list of spectra to bind_ detail is empty
///      -  Unable to get the list of currently bound spectr - in which
/// case the status is _Unable to get list of currently bound spectra_ and
/// detail is empty.
///      -  A binding attempt failed, in which case the status is
/// _Could not bind spectrum {spectrum name}_ and the detail is
/// the reason the binding failed (usually this is because either
/// the slot count is exceeded or the spectrum memory pool did not
/// have a chunk big enough for the spectrum).
///
#[get("/all")]
pub fn sbind_all(
    hg_state: &State<SharedHistogramChannel>,
    b_state: &State<SharedBinderChannel>,
) -> Json<GenericResponse> {
    let spectrum_api =
        spectrum_messages::SpectrumMessageClient::new(&hg_state.inner().lock().unwrap());
    let binding_api = binder::BindingApi::new(&b_state.inner().lock().unwrap());

    // Get the spectra:

    let spectrum_list = match spectrum_api.list_spectra("*") {
        Ok(l) => l,
        Err(s) => {
            return Json(GenericResponse::err("Unable to get spectrum list", &s));
        }
    };
    let binding_list = match binding_api.list_bindings("*") {
        Ok(l) => l,
        Err(s) => return Json(GenericResponse::err("Unable to get bindings list", &s)),
    };
    // Now get the unique bindings:
    let spectra_to_bind = list_unbound_spectra(&spectrum_list, &binding_list);
    let response = bind_spectrum_list(&spectra_to_bind, &binding_api);
    Json(response)
}
//----------------------------------------------------------------
// bind a list of spectra (note uses bind_spectrum_list)

// Only use unique spectra:

fn remove_duplicates(in_names: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    let mut scoreboard: HashSet<String> = HashSet::new();

    for s in in_names {
        if !scoreboard.contains(&s) {
            scoreboard.insert(s.clone());
            result.push(s);
        }
    }

    return result;
}

/// Implements the /spectcl/sbind/sbind REST interface.
///
/// ### Parameters
/// *  spectrum - Can be supplied as many times as needed to specify
/// the spectra to be bound.  Note that in SpecTcl, attempts to
/// bind an existing binding are just ignored.
/// * state - the state of the REST server, which allows us to get the
/// API we need.
///
/// ### Returns
/// *  GenericResponse encoded as Json. On success, the detail is empty.
/// There are several failures to consider:
///     - Unable to get the list of bindings: status is _Failed to get spectrum bindings_
/// and the detail is the reason given by the bindings API>
///     - Unable to bind a spectrum: status is _Unable to bind {spectrum name} and
/// the detail is the reason given by the binding api.
///
#[get("/sbind?<spectrum>")]
pub fn sbind_list(
    spectrum: Vec<String>,
    state: &State<SharedBinderChannel>,
) -> Json<GenericResponse> {
    // We need the bindings api.

    let api = binder::BindingApi::new(&state.inner().lock().unwrap());
    let binding_list = match api.list_bindings("*") {
        Ok(l) => l,
        Err(s) => {
            return Json(GenericResponse::err("Unable to get bindings", &s));
        }
    };
    let spectrum = remove_duplicates(spectrum);
    let binding_hash = make_binding_hash(&binding_list);
    let to_bind = remove_bound_spectra(&spectrum, &binding_hash);
    let response = bind_spectrum_list(&to_bind, &api);
    Json(response)
}
//------------------------------------------------------------------
// /spectcl/sbind/list[?pattern=glob-pattern]
//

// The structure we will return in the detail:

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Binding {
    spectrumid: usize,
    name: String,
    binding: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BindingsResponse {
    status: String,
    detail: Vec<Binding>,
}
/// Handles the /spectcl/sbind/list REST request.
///
/// ### Parameters
/// *  pattern - optional glob pattern.  Only bindings for spectra
/// whose names match the pattern are provided.  By default, if not provided,
/// the pattern used is _*_ which matches everything.
/// *  state - the REST interface State which includes the channel
/// that allows us to create a bindings API>
///
/// ### Returns
///  * A Json encoded instance of a BindingsResponse.
///
/// #### Note
/// Rustogramer does not assign ids to spectra.  THerefore
/// all spectra will be given the id 0.
///
#[get("/list?<pattern>")]
pub fn sbind_bindings(
    pattern: OptionalString,
    state: &State<SharedBinderChannel>,
) -> Json<BindingsResponse> {
    let api = binder::BindingApi::new(&state.inner().lock().unwrap());
    let p = if let Some(pat) = pattern {
        pat
    } else {
        String::from("*")
    };
    let mut response = BindingsResponse {
        status: String::from(""),
        detail: vec![],
    };
    match api.list_bindings(&p) {
        Ok(l) => {
            response.status = String::from("OK");
            for b in l {
                response.detail.push(Binding {
                    spectrumid: 0,
                    name: b.1,
                    binding: b.0,
                });
            }
        }
        Err(s) => {
            response.status = format!("Could not get bindings list {}", s);
        }
    };

    Json(response)
}
/// Set the upate rate. In SpecTcl the shared memory region directly
/// contains the contents of bound spectra.  In rustoramer, the data are
/// a copy that must be periodically updated.  This ReST method
/// sets that update period in seconds
#[get("/set_update?<seconds>")]
pub fn set_update(seconds: u64, state: &State<SharedBinderChannel>) -> Json<GenericResponse> {
    let bapi = binder::BindingApi::new(&state.inner().lock().unwrap());
    let response = if let Err(s) = bapi.set_update_period(seconds) {
        GenericResponse::err("Could not set update rate", &s)
    } else {
        GenericResponse::ok("")
    };
    Json(response)
}
/// Retrieve the update rate for the shared memory:
#[get("/get_update")]
pub fn get_update(state: &State<SharedBinderChannel>) -> Json<UnsignedResponse> {
    let bapi = binder::BindingApi::new(&state.inner().lock().unwrap());

    let response = match bapi.get_update_period() {
        Ok(i) => UnsignedResponse::new("OK", i),
        Err(s) => UnsignedResponse::new(&format!("Failed to get update rate: {}", s), 0),
    };
    Json(response)
}

#[cfg(test)]
mod sbind_tests {
    use super::*;
    use crate::histogramer;
    use crate::messaging;
    use crate::messaging::{parameter_messages, spectrum_messages};
    use crate::processing;
    use crate::sharedmem::binder;
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::fs;
    use std::path::Path;
    use std::sync::mpsc;
    use std::thread;
    use std::time;

    fn setup() -> Rocket<Build> {
        let result = rest_common::setup().mount(
            "/",
            routes![
                sbind_all,
                sbind_list,
                sbind_bindings,
                set_update,
                get_update
            ],
        );

        let hg_sender = result
            .state::<SharedHistogramChannel>()
            .expect("getting state");
        make_test_objects(&hg_sender.lock().unwrap());

        result
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
            .state::<SharedProcessingApi>()
            .expect("Valid State")
            .lock()
            .unwrap()
            .clone();
        let binder_api = binder::BindingApi::new(
            &r.state::<SharedBinderChannel>()
                .expect("Valid State")
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
    // Make some spectra.. which means making parameters as well:

    fn make_test_objects(req: &mpsc::Sender<messaging::Request>) {
        let param_api = parameter_messages::ParameterMessageClient::new(req);

        param_api.create_parameter("p1").expect("making p1");
        param_api.create_parameter("p2").expect("Making p2");

        let spec_api = spectrum_messages::SpectrumMessageClient::new(&req);

        spec_api
            .create_spectrum_1d("oned", "p1", 0.0, 1024.0, 1024)
            .expect("Making 1d spectrum");
        spec_api
            .create_spectrum_2d("twod", "p1", "p2", -1.0, 1.0, 100, -2.0, 4.0, 100)
            .expect("Making 2d specttrum");
    }
    #[test]
    fn set_update_1() {
        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making client");
        let req = client.get("/set_update?seconds=12");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing JSONG");

        assert_eq!("OK", reply.status);
        let period = bapi.get_update_period().expect("Could not get period");
        assert_eq!(12, period);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn get_update_1() {
        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Failed ot make client");
        let req = client.get("/get_update");
        let response = req
            .dispatch()
            .into_json::<UnsignedResponse>()
            .expect("Failed to parse JSON");
        assert_eq!("OK", response.status);
        assert_eq!(binder::DEFAULT_TIMEOUT, response.detail);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn sbindall_1() {
        // Bind all spectra:

        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/all");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!("", reply.detail);

        // Check that both 'one' and 'twod' are bound:

        let mut bindings = bapi.list_bindings("*").expect("API List of bindings");
        assert_eq!(2, bindings.len());

        // Sort by name so that we have known ordering:

        bindings.sort_by(|a, b| a.1.cmp(&b.1));

        // We don't know the binding indices so we just ensure
        // both names are there:

        assert_eq!("oned", bindings[0].1);
        assert_eq!("twod", bindings[1].1);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn sbindlist_1() {
        // Bind a single spectrum that's not bound.
        // Should end up with a single bound spectrum.

        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/sbind?spectrum=oned");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!("", reply.detail);

        let bindings = bapi.list_bindings("*").expect("API list of bindings");
        assert_eq!(1, bindings.len());
        assert_eq!("oned", bindings[0].1);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn sbindlist_2() {
        // Bind a single spectrum that is bound - should not get
        // double bound:

        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        bapi.bind("oned").expect("bound oned via api");

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/sbind?spectrum=oned");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!("", reply.detail);

        let bindings = bapi.list_bindings("*").expect("API list of bindings");
        assert_eq!(1, bindings.len());
        assert_eq!("oned", bindings[0].1);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn sbindlist_3() {
        // Bind a spectrum when there's a different one bound..
        // should get an extra binding:

        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        bapi.bind("oned").expect("bound oned via api");

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/sbind?spectrum=twod");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!("", reply.detail);

        let mut bindings = bapi.list_bindings("*").expect("API list of bindings");
        assert_eq!(2, bindings.len());

        bindings.sort_by(|a, b| a.1.cmp(&b.1)); // sort by name.
        assert_eq!("oned", bindings[0].1);
        assert_eq!("twod", bindings[1].1);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn sbindlist_4() {
        // Make a list of bindings... none of them done yet:

        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/sbind?spectrum=twod&spectrum=oned");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!("", reply.detail);

        let mut bindings = bapi.list_bindings("*").expect("API list of bindings");
        assert_eq!(2, bindings.len());

        bindings.sort_by(|a, b| a.1.cmp(&b.1)); // sort by name.
        assert_eq!("oned", bindings[0].1);
        assert_eq!("twod", bindings[1].1);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn sbindlist_5() {
        // Duplicate bindings requests in the list get filtered out:

        // Make a list of bindings... none of them done yet:

        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/sbind?spectrum=twod&spectrum=oned&spectrum=twod&spectrum=oned");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!("", reply.detail);

        let mut bindings = bapi.list_bindings("*").expect("API list of bindings");
        assert_eq!(2, bindings.len());

        bindings.sort_by(|a, b| a.1.cmp(&b.1)); // sort by name.
        assert_eq!("oned", bindings[0].1);
        assert_eq!("twod", bindings[1].1);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_1() {
        // list bindings when there aren't any.

        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/list");
        let reply = req
            .dispatch()
            .into_json::<BindingsResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(0, reply.detail.len());

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_2() {
        // list bindings when there is one:

        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        bapi.bind("oned").expect("Binding oned");

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/list");
        let reply = req
            .dispatch()
            .into_json::<BindingsResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(1, reply.detail.len());

        assert_eq!("oned", reply.detail[0].name);
        assert_eq!(0, reply.detail[0].spectrumid);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_3() {
        // list bindings when both are bound:
        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        bapi.bind("oned").expect("binding oned with api");
        bapi.bind("twod").expect("binding twod with api");

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/list");
        let reply = req
            .dispatch()
            .into_json::<BindingsResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(2, reply.detail.len());

        let mut bind_list = reply.detail.clone();
        bind_list.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!("oned", bind_list[0].name);
        assert_eq!("twod", bind_list[1].name);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_4() {
        // test list with pattern

        // list bindings when both are bound:
        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        bapi.bind("oned").expect("binding oned with api");
        bapi.bind("twod").expect("binding twod with api");

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/list?pattern=t*");
        let reply = req
            .dispatch()
            .into_json::<BindingsResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(1, reply.detail.len());
        assert_eq!("twod", reply.detail[0].name);

        teardown(c, &papi, &bapi);
    }
}
