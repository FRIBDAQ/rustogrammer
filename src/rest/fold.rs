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
use rocket::serde::{json::Json, Serialize};
use super::*;

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
        "This is not SpecTcl"
    ))
}

//
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FoldInfo {
    spectrum : String,
    gate : String
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FoldListResponse {
    status : String,
    detail : Vec<FoldInfo>
}
/// list - unimplemented
///  If implemented the _pattern_ query  parameter will filter out
/// the listing to only inlcude the spectra with names that match the
/// pattern.  The reply is a FoldListResponse shown above.
#[get("/list")]
pub fn list() -> Json<FoldListResponse> {
    Json(FoldListResponse {
        status : String::from("/spectcl/fold/list is not implemented - this is not SpecTcl"),
        detail : vec![]
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
        "This is not SpecTcl"
    ))
}