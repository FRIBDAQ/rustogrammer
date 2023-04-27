//!  Implements handlers for the /spectcl/evbunpack domain.
//!  In SpecTcl, this sets up event decoders for event built data
//!  In our case, however, this is unimplemented/not necessary
//!  because we deal only with data that has already been broken down
//!  into decoded parameter data.  That is the parameters extracted
//!  from event built data.
//!
//!  subdomains are:
//!
//! *   create - create a new event built data unpacker.,
//! *   add - Add a processor for a source id to an event built
//! unpacker.
//! *   list - List the event built data unpackers that have been
//! created.
//!
//!  All of this is, in SpecTcl coupled to the dynamically controlled
//! event processing pipeline which is not needed in Rustogramer
//! because data are already  unpacked into a mechanically
//! usable form.
use rocket::serde::{json::Json, Serialize};

use super::*; // For GenericResponse.

/// create.   If this were implemented, it would require the
/// following parameters
///
/// * name (mandatory) - name of the new unpacker.
/// * frequency (mandatory) - event builder clock frequency in MHz,
/// this would be used to generate diagnostic parameters.
/// * basename - parameter base name for the diagnostic parameters.
///
/// A GenericResponse would likely be generated if implemented.
#[get("/create")]
pub fn create_evbunpack() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/evbunpack/create is not implemented",
        "This is not SpecTcl",
    ))
}
/// add.   If this were implemented; it would require the following
/// query parameters:
///
/// *   name - name of the event processor being manipulated.
/// *   source - source id we're adding a processing pipeline for.
/// *   pipe - Name of the event processing pipeline that will handle
/// data from that source.
///
/// A GenericResponse would likely still be returned.
///
#[get("/add")]
pub fn add_evbunpack() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/evbunpack/add is not implemented",
        "This is not SpecTcl",
    ))
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ListResponse {
    status: String,
    detail: Vec<String>,
}
/// list.  If this were implemented; it would take an optional
/// query parameter _pattern_ which would restrict the names of
/// the patterns matched to add to the listing.
///
/// If implemented this would return Json that is
/// has a detail that constis of an array of evb event processor
/// names that have been created, or otherwise known to the system.
///
#[get("/list")]
pub fn list_evbunpack() -> Json<ListResponse> {
    Json(ListResponse {
        status: String::from("/spectcl/evb/unpack/list is not implemented - this is not SpecTcl"),
        detail: vec![],
    })
}
