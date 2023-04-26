//!  Supplies the spectcl/apply domain of URIs.
//!  This set of URIs has to do with the application of gates
//!  (conditions) to spectra and provides the following:
//!
//!  *  apply - applies a condition to a spectrum so that it can only
//! be incremented for events that make that condition true.
//!  *  list - lists the gates applied to a set of spectra that match
//! the pattern supplied in the request.
//!

use rocket::serde::{json::Json, Serialize};
use rocket::State;

use super::*;
use crate::messaging::spectrum_messages::SpectrumMessageClient;

//---------------------------------------------------------------
// Stuff needed to implement apply:

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct GateApplicationResponse {
    status: String,
    detail: Vec<(String, String)>,
}

///  Apply a gate to a spectrum.
///  Query parameters are:
///
/// *   gate (mandatory) - name of the condition
/// *   spectrum (mandatory) - name of the spectrum to which
/// to apply the gate.  The SpecTcl version of this only accepts a
/// single spectrum.   We accept any number of spectra, applying the
/// gate to all.
///
/// On success a GateApplicationResponse is returned. With an empty
/// array in the detail (status of course is _OK_).  On failure
/// the message is "Failed to apply {gatename} to some spectra"
/// and the detail is an array of the spectrum for which we could not
/// apply the gate.
///
#[get("/apply?<gate>&<spectrum>")]
pub fn apply_gate(
    gate: String,
    spectrum: Vec<String>,
    state: &State<HistogramState>,
) -> Json<GateApplicationResponse> {
    let mut response = GateApplicationResponse {
        status: String::from("OK"),
        detail: Vec::new(),
    };
    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);
    for name in spectrum {
        if let Err(s) = api.gate_spectrum(&name, &gate) {
            response.status = format!("Failed to apply {} to some spectra", gate);
            response.detail.push((name, s));
        }
    }
    Json(response)
}
//---------------------------------------------------------------------
// Stuff needed to provde the application list.

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Application {
    spectrum: String,
    gate: String,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ApplicationListing {
    status: String,
    detail: Vec<Application>,
}

#[get("/list?<pattern>")]
pub fn apply_list(
    pattern: OptionalString,
    state: &State<HistogramState>,
) -> Json<ApplicationListing> {
    let mut pat = String::from("*"); // Default pattern
    if let Some(s) = pattern {
        pat = s; // User supplied pattern.
    }

    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);
    let listing = api.list_spectra(&pat);
    if listing.is_err() {
        return Json(ApplicationListing {
            status: format!("Failed to get spectrum listing: {}", listing.unwrap_err()),
            detail: Vec::new(),
        });
    }
    let listing = listing.unwrap();
    let mut result = ApplicationListing {
        status: String::from("OK"),
        detail: Vec::new(),
    };
    for spectrum in listing {
        let gate_name = if let Some(g) = spectrum.gate {
            g
        } else {
            String::from("-none-")
        };

        result.detail.push(Application {
            spectrum: spectrum.name,
            gate: gate_name,
        });
    }
    Json(result)
}
