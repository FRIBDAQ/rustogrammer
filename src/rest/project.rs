//!  This ReST interface implements projection spectra.
//!  In fact, projection spectra are no different than
//!  ordinary spectra.  A projection just describes how they
//!  come into being and get their initial contents.
//!
//!  Only spectra with 'sensible' underlying 2d histoigrams can
//!  be projected and these include:
//!  *  Ordinary 2-d spectra.
//!  *  Multiply incremented 2-d spectra (g2 in SpecTcl parlance).
//!  *  Particle Gamma spectra (gd in SpecTcl parlance).
//!  *  Twod sum spectra (m2 in SpecTcl parlance).
//!
//!  The resulting spectrum is populated by summing rows or columns
//!  In the direction other than the projection direction.  For example,
//!  In the case of an X projectsion, columns (y axis) are summed
//!  To project the image down onto the  X axis. Similarly, in the case of a
//!  Y projection, rows (x  axis) are summed onto the Y axis.  The resulting
//!  spectrum type depends on the original spectrum:
//!
//!  * 2d -> A 1d spectrum on the parameter of the appropriate axis
//! (e.g x parameter for x projection)
//!  * 2dmulti -> A multi1d spectrum on all of the parameters of the original
//! spectrum.
//!  * pgamma -> A 1d multi spectrum - The parameters are the parameters appropriate to
//! the axis but each parameter is repeated once per opposite parameter.  For example,
//! Suppose the original spectrum has 3 y parameters and we project in X,  The resultiing
//! Spectrum has the original X parameters but repeated 3 times (to maintain the appropriate projection).
//! * twodsum - a 1d multi spectrum.  The parmaeters are the parameters of the approprite axis.  For
//! example an X projection will have all of the X parameters of the original spectrum.
//!
//!  Projections can also be within any contour that can be displayed on the underlying spectrum.
//!
//!  The final gate of the spectrum will depend:
//!
//! * Snapshot spectra get the false gate _projection_snapshot_condition which, if necessary is created.
//! * Ungated  source spectra result in ungated non-snapshot projections... but see below:
//! * Gated Source spectra result in non-snapshots gated on the same condition that gates the original spectrum
//! * Spectra projected within a contour will retain that contour as a gate.  If the source spectrum
//! is gated, the gate used is the And of the original spectrum's gate and the contour.
//!

use super::*;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;
//------------------------------------------------------------------
// project:
#[allow(unused_variables)]
#[get("/?<snapshot>&<source>&<newname>&<direction>&<contour>")]
pub fn project(
    snapshot: String,
    source: String,
    newname: String,
    direction: String,
    contour: OptionalString,
) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Projections are not implemented",
        "This is not SpecTcl",
    ))
}
