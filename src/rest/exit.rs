//! Supports the /spectcl/exit URL.
//! this provides the ability to, in order:
//!
//! * Shutdown the rustogramer threads via their APIs.
//! * Reply to the caller that, yes we're shutting down.
//! * Notify rocket that when the request is complete it too should
//! shutdown which, in turn exits th main program.
//!

use rocket::serde::{json::Json, Serialize};
use rocket::Shutdown;

use super::*; // For generic response.

/// This performs the shutdown:
///
#[get["/exit"]]
pub fn shutdown(shutdown: Shutdown, state: &State<HistogramState>) -> Json<GenericResponse> {
    Json(GenericResponse::ok(""))
}
