//!  Implements the /spectcl/fit domain of URIs.
//!  This entire domain is implemented to return to the
//!  client that this functionality is not supported by Rustogramer.
//!  With the development of the, e.g. CutiePie, visualizer,
//!  it seems that doing fitting in the visualizer is the appropriate
//!  place for this functionality.
//!
//!  The /spectcl/fit domain has the following URIs that will
//!  have handlers:
//!
//!  *  create - creates a new fit object.
//!  *  update - Update fit parameters based on current data.
//!  *  delete - Delete a fit object.
//!  *  list   - list the fit objects that exist.
//!  *  proc   - Returns the name of the fit proc associated with the fit.
//! (In SpecTcl this allowed evaulation of the fit).
//!  
use super::*;
use rocket::serde::{json::Json, Serialize};

/// create - create a new fit object. (unimplemented).
/// If this is implemented the following query parameters
/// would required:
///
///  name - name of the fit object.
///  spectrum - Name of the spectrum on which the fit is evaulated.
///  low  - Low channel limit of the fitted region.
///  high - high channel limit of the fitted region.
///  type - Type of the fit (e.g. 'gaussian')
///
/// Idf implemented a GenericResponse is perfectly appropriate.
///
#[get("/create")]
pub fn create() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/fit/create is not supported",
        "This is not Spectcl",
    ))
}

/// update (not implemented) Give a set of fits that match a pattern,
/// the fit paramaeters are re-computed using the current spectrum
/// data.  The concept is that as the data are processed,fit parameters
/// will shift both because
///
/// * Additional statisitcs may shift slightly the fit parameters.
/// * After clearing the spectra and attaching a different data file,
/// the data could change significantly (consider an experimental
/// data set that includes an energy scan or multiple beam species
/// for example).
///
/// The query parameter that would be accepted if implemented would be
/// _pattern_ which is a glob pattern.  Fits with matching names
/// only will be recomputed.
///
#[get("/update")]
pub fn update() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/fit/update is not supported",
        "This is not Spectcl",
    ))
}

/// delete (unimplemented)
/// Deletes an existing fit object.  The only query parameter is
/// _name_ which specifies the the name of the fit to delete.
///
/// A GenericResponse is perfectly ok for any future implementation
/// of this URI.
///
#[get("/delete")]
pub fn delete() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/fit/delete is not supported",
        "This is not Spectcl",
    ))
}
//
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FitParameter {
    name: String,
    value: f64,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FitDescription {
    name: String,
    spectrum: String,
    r#type: String,
    low: f64,
    high: f64,
    parameters: Vec<FitParameter>,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FitListReply {
    status: String,
    detail: Vec<FitDescription>,
}

///
/// list (unimplemented).
///
/// Lists the set of fits that match the optional _pattern_ query
/// parameter (defaults to "*").  The returned reply will be of the
/// form described by FitListReply above.  Note that the
/// FitParameter is different from what SpecTcl would produce which
/// is just a set of name/value pairs... which I don't quite know how
/// to produce (Maybe a tuple would be better?).
///
/// Not important at this time since we're going to
/// return an unimplemented reply.'
///
#[get("/list")]
pub fn list() -> Json<FitListReply> {
    Json(FitListReply {
        status: String::from("spectcl/fit/list is not implemented - this is not SpecTcl"),
        detail: vec![],
    })
}
///
/// proc (unimplemented)
///
/// This would, in SpecTcl return the name of a proc that can be
/// invoked to evaulate a fit at a specific channel.  This would be
/// done bia the script interface.
///
/// A GenericResponse is fine if implemented as the detail is just
/// the string proc name.
///
#[get("/proc")]
pub fn proc() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/fit/proc is not supported",
        "This is not Spectcl",
    ))
}
