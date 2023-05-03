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

use rocket::serde::{json::Json, Serialize};
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
    let api = binder::BindingApi::new(&state.inner().binder.lock().unwrap().0);

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
    let api = binder::BindingApi::new(&state.inner().binder.lock().unwrap().0);

    let response = if let Err(s) = api.unbind_all() {
        GenericResponse::err("Failed to unbind all spectra", &s)
    } else {
        GenericResponse::ok("")
    };
    Json(response)
}
