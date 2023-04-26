//!  Implements the /spectcl/channel domain.  Note that in Rustogrammer,
//!  this is not implemented.  While implemented in SpecTcl I believe
//!  it's not used.  If it is required, it is possible to implement it
//!  in the spectrum server code (I think).
//!
//!  We have handlers for
//!
//!  set - sets a channel value.
//!  get - gets a channel value.
//!
//!  Both of these just return a GenericResponse::err.
//!

use rocket::serde::{json::Json, Serialize};
use rocket::State;

use super::*;

// We don't even bother with query parameters.
#[get("/set")]
pub fn set_chan() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Unsupported /spectcl/channel/set",
        "This is not SpecTcl",
    ))
}
#[get("/get")]
pub fn get_chan() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Unsupported /spectcl/channel/get",
        "This is not SpecTcl",
    ))
}
