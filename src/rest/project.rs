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

use crate::messaging::{condition_messages, spectrum_messages};
use crate::projections;
use crate::sharedmem::binder;
//------------------------------------------------------------------
// project:
#[allow(unused_variables)]
#[get("/?<snapshot>&<source>&<newname>&<direction>&<contour>&<bind>")]
pub fn project(
    snapshot: String,
    source: String,
    newname: String,
    direction: String,
    contour: OptionalString,
    bind: OptionalFlag,
    hgchannel: &State<SharedHistogramChannel>,
    bchannel: &State<SharedBinderChannel>,
) -> Json<GenericResponse> {
    // Make the spectrum and condition APIs:

    let sapi = spectrum_messages::SpectrumMessageClient::new(&(hgchannel.inner().lock().unwrap()));
    let capi =
        condition_messages::ConditionMessageClient::new(&(hgchannel.inner().lock().unwrap()));

    // Figure out direction:

    let projection_direction = match direction.as_str() {
        "x" | "X" => projections::ProjectionDirection::X,
        "y" | "Y" => projections::ProjectionDirection::Y,
        _ => {
            return Json(GenericResponse::err(
                "Invalid projection direction",
                "Must be 'X' or 'Y'",
            ));
        }
    };
    // Snapshot text to bool:

    let snapshot = match snapshot.as_str() {
        "Yes" | "yes" | "True" | "true" => true,
        "No" | "no" | "False" | "false" => false,
        _ => {
            return Json(GenericResponse::err(
                "Invalid value for 'snapshot'",
                "Must be in {yes, no, true, false} e.g.",
            ));
        }
    };

    // Can we make the spectrum?

    let mut reply = if let Err(s) = projections::project(
        &sapi,
        &capi,
        &source,
        projection_direction,
        &newname,
        snapshot,
        contour,
    ) {
        GenericResponse::err("Failed to crate projection spectrum", &s)
    } else {
        GenericResponse::ok("")
    };
    // On success, bind if requested:

    if "OK" == reply.status.as_str() {
        let do_bind = if let Some(b) = bind {
            b
        } else {
            false // SpecTcl does not support this flag and does not bind
        };
        if do_bind {
            let bapi = binder::BindingApi::new(&bchannel.inner().lock().unwrap());
            reply = match bapi.bind(&newname) {
                Ok(()) => GenericResponse::ok(""),
                Err(s) => GenericResponse::err("Could not bind projected spectrum", &s),
            };
        }
    }

    Json(reply)
}
