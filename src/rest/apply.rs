//!  Supplies the spectcl/apply domain of URIs.
//!  This set of URIs has to do with the application of gates
//!  (conditions) to spectra and provides the following:
//!
//!  *  apply - applies a condition to a spectrum so that it can only
//! be incremented for events that make that condition true.
//!  *  list - lists the gates applied to a set of spectra that match
//! the pattern supplied in the request.
//!

use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;

use super::*;
use crate::messaging::spectrum_messages::SpectrumMessageClient;

//---------------------------------------------------------------
// Stuff needed to implement apply:

#[derive(Serialize, Deserialize)]
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
    let api = SpectrumMessageClient::new(&state.inner().histogramer.lock().unwrap());
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

    let api = SpectrumMessageClient::new(&state.inner().histogramer.lock().unwrap());
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
//-----------------------------------------------------------------
// what we need for /spectcl/ungate.

///
/// Remove gate from spectra.   The name parameter is the only
/// allowed parameter.  Unlike SpecTcl it can be specified
/// more than once and the handler attempts to remove gates
/// from all named spectra.  The returned JSON Is a
/// GateApplicationResponse.  On success, the detail is an empty
/// vector.  If unable to remove the gate from any of the
/// specified spectra, the status will be
/// _Unable to ungate at least one spectrum_
/// and the detail will be a vector of 2 String element tuples with
/// the first element the name of the spectrum that could not be
/// ungated and the second the reason given by the spectrum
/// messaging API.
///
#[get("/?<name>")]
pub fn ungate_spectrum(
    name: Vec<String>,
    state: &State<HistogramState>,
) -> Json<GateApplicationResponse> {
    let api = SpectrumMessageClient::new(&state.inner().histogramer.lock().unwrap());
    let mut result = GateApplicationResponse {
        status: String::from("OK"),
        detail: Vec::new(),
    };
    for spectrum in name {
        if let Err(s) = api.ungate_spectrum(&spectrum) {
            result.status = String::from("Unable to ungate at least one spectrum");
            result.detail.push((spectrum, s));
        }
    }
    Json(result)
}
#[cfg(test)]
mod apply_tests {
    use super::*;
    use crate::histogramer;
    use crate::messaging;
    use crate::processing;
    use crate::sharedmem::binder;
    use rocket;
    use rocket::http::Status;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Config;
    use rocket::Rocket;
    use rocket::Shutdown;

    use std::sync::mpsc;
    use std::sync::Mutex;

    fn setup() -> Rocket<Build> {
        let (_, hg_sender) = histogramer::start_server();
        let (binder_req, _rx): (
            mpsc::Sender<binder::Request>,
            mpsc::Receiver<binder::Request>,
        ) = mpsc::channel();
        let state = HistogramState {
            histogramer: Mutex::new(hg_sender.clone()),
            binder: Mutex::new(binder_req),
            processing: Mutex::new(processing::ProcessingApi::new(&hg_sender.clone())),
            portman_client: None,
        };
        rocket::build()
            .manage(state)
            .mount("/", routes![apply_gate, apply_list, ungate_spectrum])
    }
    fn teardown(c: mpsc::Sender<messaging::Request>) {
        histogramer::stop_server(&c);
    }
    #[test]
    fn apply_gate() {
        let rocket = setup();
        let chan = rocket
            .state::<HistogramState>()
            .expect("Valid state")
            .histogramer
            .lock()
            .unwrap()
            .clone();
        // No spectra so applying a gate will fail:

        let c = Client::tracked(rocket).unwrap();
        let r = c.get("/apply?gate=g&spectrum=spec");
        let reply = r.dispatch();

        let json = reply.into_json::<GateApplicationResponse>();
        assert!(json.is_some());
        let json = json.unwrap();
        assert_eq!(
            format!("Failed to apply {} to some spectra", "g"),
            json.status
        );

        teardown(chan);
    }
}
