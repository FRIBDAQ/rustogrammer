//! This module implements, the /spectcl/filter domain of URIS,
//! or rather documents to the client that these are no implemented for Rustogramer
//! These URIS are not implemented because Rustogramer does not,
//! and probably never will, implement filters.  It operates on what is,
//! essentially, already filtered data.
//!
//!  Filters in SpecTcl perform two functions:
//!  
//! *   Provide data in an already decoded format for speedy playback.
//! *   Provide some subset of the  full data set (in SpecTcl this subset is
//! defined by events that satisfy a gate and parameter list).
//!
//!  The first of these function is provided already by the fact that
//! Rustogramer operates on data that is the output of the analysis
//! pipeline (that is already decoded data).  
//!
//! The second of these functions would be provided by using
//! The analysis pipeline to filter out events you don't want rustogramer
//! to see.
//!
//!  The following URIS are caught within the /spectcl/filter domain:
//!
//! *  new - would create a new filter.
//! *  delete - Would delete an existing filter.
//! *  enable - would enable an existing filter to output data.
//! *  disable - would disable an existing filter from outputting data.
//! *  regate - would replace the gate on an existing filter that determines
//! which subset it writes.
//! *  file - Defines the file an existing filter writes data to.
//! *  list - lists the set of filters that match an optional Glob pattern.
//! *  format - selects the output format for an existing filter.
//!
use super::*;
use rocket::serde::{json::Json, Serialize};

/// new - would create a new filter.  Parameters, if implemented
/// are:
///
/// *   name the name of the new filter.
/// *   gate - the gate that will select the event the filter outputs.
/// *   parameter - can repeat as many times as needed -the set of parameters
/// that will be output.
///
/// In the implementation a GenericResponse would also likely be
/// returned.
///
#[get("/new")]
pub fn new() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/filter/new is not implemented",
        "This is not SpecTcl",
    ))
}
/// delete - deletes an existing filter.  The only parameter
/// required is the name of the filter.  Note that a Generic
/// response would be used if this is implemented.
///
#[get("/delete")]
pub fn delete() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/filter/delete is not implemented",
        "This is not SpecTcl",
    ))
}
/// enable - would enable an existing filter.  The only
/// query parameter is the name of the filter to enable.
/// If implemented a GenericResponse would be used.
///
#[get("/enable")]
pub fn enable() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/filter/enable is not implemented",
        "This is not SpecTcl",
    ))
}
/// disable - would disable an existing filter.  Only the
/// name of the filter is required as a query parameter.
///
#[get("/disable")]
pub fn disable() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/filter/disable is not implemented",
        "This is not SpecTcl",
    ))
}
/// regate - would specify a new gate be used to select the
/// set of events written by the filter.
/// Query parameters;
///
/// *   name - Name of the filter to modify.
/// *   gate - gate to use to select output events
///
/// A GenericResponse is fine for this if it were implemented.
///
#[get("/regate")]
pub fn regate() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/filter/regate is not implemented",
        "This is not SpecTcl",
    ))
}
/// file - set the otput file for the filter.
/// The query parameters would be:
///
/// *  name -filter name.
/// * file - name of the new output file for the filter.
///
#[get("/file")]
pub fn file() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/filter/file is not implemented",
        "This is not SpecTcl",
    ))
}

//----------------------------------------------------------------

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FilterDetail {
    name: String,
    gate: String,
    file: String,
    parameters: Vec<String>,
    enabled: bool,
    format: String,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FilterListResponse {
    status: String,
    detail: Vec<FilterDetail>,
}

/// list - lists the filters that match an optional
/// _pattern_ query parameter.  The FilterListResponse
/// struct defined above describes what the successful listing
/// response would be if this were ever implemented.
///
#[get("/list")]
pub fn list() -> Json<FilterListResponse> {
    Json(FilterListResponse {
        status: String::from("/spectcl/filter/list is not implemented - this is not SpecTcl"),
        detail: vec![],
    })
}
