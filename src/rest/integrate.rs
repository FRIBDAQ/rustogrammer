//! 'Implements' the /spectcl/integrate domain.
//! This is not implemented in this version of Rustogramer.
//! This is a candidate for implementation in a future release
//! *However*, it seems to me that this functionality really belongs
//!  in a displayer such as CutiePie where users can interact with
//!  a visualization of the spectrum to peform their integrations.
//!
//!  There is only /spectcl/integrate, nothing underneath it.
//!
use rocket::serde::{json::Json, Serialize};

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct IntegrationDetail {
    centroid: f64,
    fwhm: f64,
    counts: u64,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct IntegrationResponse {
    status: String,
    detail: IntegrationDetail,
}
/// integrate (unimplemented)
///
/// When implemented this would accept the following
/// query parameters depending on the type of integration being performed
///
/// * spectrum (mandatory) - The spectrum to be integrated.
/// * gate (optional) - If the gate can appear drawn on the spectrum,
/// the integration will be over the interior of the gate.
/// * low - If the spectrum is one dimensional and the integration is
/// not in a gate this is the low limit of the range of channels
/// over which to integrate.
/// * high - if the spectrum is 1d the high limit over which to integerate.
/// * xcoord - If the
/// integration is not in a gate and in a 2d spectrum, these are
/// the X coordinates of a contour within which an integration is performed.
/// * ycoord - if the integrations is not in a gate and  in a 2d spectrum,
/// these are the set of y coordinates of points that describe the
/// contour within which the integration will be done.
///
/// The reply is an IntegrationResponse.
///
#[get("/")]
pub fn integrate() -> Json<IntegrationResponse> {
    Json(IntegrationResponse {
        status: String::from("/spectcl/integrate is not supported - this is not SpecTcl"),
        detail: IntegrationDetail {
            centroid: 0.0,
            fwhm: 0.0,
            counts: 0,
        },
    })
}
