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
use rocket::serde::{json::Json, Serialize};
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
pub fn sbind_all(state: &State<HistogramState>) -> Json<GenericResponse> {
    let spectrum_api =
        spectrum_messages::SpectrumMessageClient::new(&state.inner().histogramer.lock().unwrap());
    let binding_api = binder::BindingApi::new(&state.inner().binder.lock().unwrap().0);

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
pub fn sbind_list(spectrum: Vec<String>, state: &State<HistogramState>) -> Json<GenericResponse> {
    // We need the bindings api.

    let api = binder::BindingApi::new(&state.inner().binder.lock().unwrap().0);
    let binding_list = match api.list_bindings("*") {
        Ok(l) => l,
        Err(s) => {
            return Json(GenericResponse::err("Unable to get bindings", &s));
        }
    };
    let binding_hash = make_binding_hash(&binding_list);
    let to_bind = remove_bound_spectra(&spectrum, &binding_hash);
    let response = bind_spectrum_list(&to_bind, &api);
    Json(response)
}
//------------------------------------------------------------------
// /spectcl/sbind/list[?pattern=glob-pattern]
//

// The structure we will return in the detail:

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Binding {
    spectrumid: usize,
    name: String,
    binding: usize,
}

#[derive(Serialize)]
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
    state: &State<HistogramState>,
) -> Json<BindingsResponse> {
    let api = binder::BindingApi::new(&state.inner().binder.lock().unwrap().0);
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
