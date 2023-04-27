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

/// We don't even bother with query parameters.
/// If we implement this the query parameters would be:
///
/// * spectrum (mandatory)- name of the spectrum.
/// * xchannel (mandatory)- xchannel number to set.
/// * ychannel (optional)- y channel number to set.
/// only makes sense for 2 d spectra.  Defaults to 0.0
/// if not supplied.
/// * value - value to set the selected channel to.
///
#[get("/set")]
pub fn set_chan() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Unsupported /spectcl/channel/set",
        "This is not SpecTcl",
    ))
}
/// If this were implemented,
/// the query paramters would be:
///
/// *   spectrum (mandatory) - name of the spectrum being queried.
/// *   xchannel (mandatory) - X channel to get
/// *   ychannel (optional) - required only for 2d spectra. The
/// Y channel to get.
///
/// The return value on success would then be
/// *   status : _OK_
/// *   detail : the value in that channel.
///
/// Note that channels out of range would, unlike SpecTcl likely
/// fetch the over/underflow value depending.
///
#[get("/get")]
pub fn get_chan() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Unsupported /spectcl/channel/get",
        "This is not SpecTcl",
    ))
}
