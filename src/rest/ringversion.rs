//! Support for the ringversion domain of URLs
//! We extend the SpecTcl REST API to support not only setting the
//! ring item version format but also by querying the format currently in
//! use:
//!
//! *  /spectcl/ringformat - sets the ring item format.
//! *  /spectcl/ringformat/get - returns the current ring format.

use rocket::serde::{json::Json, Serialize};
use rocket::State;

use super::*;
use crate::ring_items::RingVersion;

/// Set the ring item version.
///
/// ### Parameters  
/// *   major - Major version required.
/// *   minor - Minor version (optional and actually ignored).
///
/// ### Returns:
///  *  Json encoded GenericResponse.
///      - On success detail is empty.
///      - On failure, status is _Unable to set ring format version_ and  
/// detail is the reason for the failure.
///
#[get("/?<major>")]
pub fn ringversion_set(major: usize, state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = state.inner().processing.lock().unwrap();

    let result = match major {
        11 => api.set_ring_version(RingVersion::V11),
        12 => api.set_ring_version(RingVersion::V12),
        _ => Err(String::from("Invalid ring item version number")),
    };
    Json(match result {
        Ok(_) => GenericResponse::ok(""),
        Err(reason) => GenericResponse::err("Unable to set ring format version", &reason),
    })
}

//------------------------------------------------------------------------
// Getting the ring version:

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct VersionDetail {
    major: usize,
    minor: usize,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct VersionResponse {
    status: String,
    detail: VersionDetail,
}

/// Returns the ring format currently in use.
///
/// ### Parameters
/// *  The state reference which allows us to get the processing api.
///
/// ### Returns
/// *  Json encoded VersionResponse - note that for Rustogramer, the minor
/// version is always zero - theoretically NSCLDAQ is not allowed to have
/// minor versions in the format as formats are only allowed to change
/// when major versions change.
///
#[get("/get")]
pub fn ringversion_get(state: &State<HistogramState>) -> Json<VersionResponse> {
    let api = state.inner().processing.lock().unwrap();
    let result = api.get_ring_version();

    let mut response = VersionResponse {
        status: String::from("OK"),
        detail: VersionDetail { major: 0, minor: 0 },
    };
    match result {
        Ok(v) => match v {
            RingVersion::V11 => response.detail.major = 11,
            RingVersion::V12 => response.detail.major = 12,
        },
        Err(s) => response.status = format!("Unable to get the ring item format: {}", s),
    };

    Json(response)
}
