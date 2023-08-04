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
    state: &State<SharedHistogramChannel>,
) -> Json<GateApplicationResponse> {
    let mut response = GateApplicationResponse {
        status: String::from("OK"),
        detail: Vec::new(),
    };
    let api = SpectrumMessageClient::new(&state.inner().lock().unwrap());
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

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Application {
    spectrum: String,
    gate: String,
}
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ApplicationListing {
    status: String,
    detail: Vec<Application>,
}

#[get("/list?<pattern>")]
pub fn apply_list(
    pattern: OptionalString,
    state: &State<SharedHistogramChannel>,
) -> Json<ApplicationListing> {
    let mut pat = String::from("*"); // Default pattern
    if let Some(s) = pattern {
        pat = s; // User supplied pattern.
    }

    let api = SpectrumMessageClient::new(&state.inner().lock().unwrap());
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
    state: &State<SharedHistogramChannel>,
) -> Json<GateApplicationResponse> {
    let api = SpectrumMessageClient::new(&state.inner().lock().unwrap());
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
    use crate::messaging::condition_messages;
    use crate::messaging::parameter_messages;
    use crate::messaging::spectrum_messages;
    use crate::processing;
    use crate::sharedmem::binder;
    use crate::trace;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::{mpsc, Arc, Mutex};

    fn setup() -> Rocket<Build> {
        let tracedb = trace::SharedTraceStore::new();
        let (_, hg_sender) = histogramer::start_server(tracedb.clone());
        let (binder_req, _rx): (
            mpsc::Sender<binder::Request>,
            mpsc::Receiver<binder::Request>,
        ) = mpsc::channel();
        let state = HistogramState {
            processing: Mutex::new(processing::ProcessingApi::new(&hg_sender.clone())),
            portman_client: None,
            mirror_exit: Arc::new(Mutex::new(mpsc::channel::<bool>().0)),
            mirror_port: 0,
        };
        rocket::build()
            .manage(state)
            .manage(Mutex::new(hg_sender.clone()))
            .manage(Mutex::new(binder_req))
            .manage(tracedb.clone())
            .mount("/", routes![apply_gate, apply_list, ungate_spectrum])
    }
    fn teardown(c: mpsc::Sender<messaging::Request>, p: &processing::ProcessingApi) {
        histogramer::stop_server(&c);
        p.stop_thread().expect("Stopping processing thread");
    }
    fn get_state(
        r: &Rocket<Build>,
    ) -> (mpsc::Sender<messaging::Request>, processing::ProcessingApi) {
        let chan = r
            .state::<SharedHistogramChannel>()
            .expect("Valid state")
            .lock()
            .unwrap()
            .clone();
        let papi = r
            .state::<HistogramState>()
            .expect("Valid State")
            .processing
            .lock()
            .unwrap()
            .clone();

        (chan, papi)
    }

    #[test]
    fn apply_gate_1() {
        let rocket = setup();
        let (chan, papi) = get_state(&rocket);

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
        assert_eq!(1, json.detail.len());
        assert_eq!(String::from("spec"), json.detail[0].0);

        teardown(chan, &papi);
    }
    #[test]
    fn apply_gate_2() {
        // need to make a parameter a spectrum and a gate to
        // test success.

        let rocket = setup();
        //

        let (chan, papi) = get_state(&rocket);

        // Use the channel to make a parameter, spectrum  and
        // condition api which we'll use to create what we need to test
        // application:

        let param_api = parameter_messages::ParameterMessageClient::new(&chan);
        let cnd_api = condition_messages::ConditionMessageClient::new(&chan);
        let spec_api = spectrum_messages::SpectrumMessageClient::new(&chan);

        param_api
            .create_parameter("test")
            .expect("Making parameter");
        assert!(if let condition_messages::ConditionReply::Created =
            cnd_api.create_true_condition("True")
        {
            true
        } else {
            false
        });
        spec_api
            .create_spectrum_1d("test_spec", "test", 0.0, 1024.0, 1024)
            .expect("making spectrum");

        // Now apply the True condition to test_spec.

        let c = Client::tracked(rocket).expect("client created");
        let r = c.get("/apply?gate=True&spectrum=test_spec");
        let reply = r.dispatch();

        // Should get success and the gate should be applied:

        let json = reply
            .into_json::<GateApplicationResponse>()
            .expect("Valid JSON back");
        assert_eq!(String::from("OK"), json.status);
        assert_eq!(0, json.detail.len());

        // Check that the spectrum is gated on True:

        let spectra = spec_api.list_spectra("test_spec").expect("Listing");
        assert_eq!(1, spectra.len());
        let gate = spectra[0].clone().gate.expect("Gated").clone();
        assert_eq!("True", gate.as_str());

        teardown(chan, &papi);
    }
    #[test]
    fn apply_list_1() {
        // Empty list:

        let rocket = setup();
        let (chan, papi) = get_state(&rocket);

        // No spectra so applying a gate will fail:

        let c = Client::tracked(rocket).unwrap();
        let r = c.get("/list"); // no pattern/
        let reply = r.dispatch();

        let json = reply
            .into_json::<ApplicationListing>()
            .expect("Failed Json decode");
        assert_eq!("OK", json.status.as_str());
        assert_eq!(0, json.detail.len());

        teardown(chan, &papi);
    }
    #[test]
    fn apply_list_2() {
        // List no pattern but one listing.

        // need to make a parameter a spectrum and a gate to
        // test success.

        let rocket = setup();
        //

        let (chan, papi) = get_state(&rocket);

        // Use the channel to make a parameter, spectrum  and
        // condition api which we'll use to create what we need to test
        // application:

        let param_api = parameter_messages::ParameterMessageClient::new(&chan);
        let cnd_api = condition_messages::ConditionMessageClient::new(&chan);
        let spec_api = spectrum_messages::SpectrumMessageClient::new(&chan);

        param_api
            .create_parameter("test")
            .expect("Making parameter");
        assert!(if let condition_messages::ConditionReply::Created =
            cnd_api.create_true_condition("True")
        {
            true
        } else {
            false
        });
        spec_api
            .create_spectrum_1d("test_spec", "test", 0.0, 1024.0, 1024)
            .expect("making spectrum");

        // apply the gate the easy way:

        spec_api
            .gate_spectrum("test_spec", "True")
            .expect("Failed to gate spectrum");

        //  Get the listing:

        let c = Client::tracked(rocket).unwrap();
        let r = c.get("/list"); // no pattern/
        let reply = r.dispatch();

        let json = reply
            .into_json::<ApplicationListing>()
            .expect("Failed Json decode");
        assert_eq!("OK", json.status.as_str());
        assert_eq!(1, json.detail.len());
        assert_eq!("test_spec", json.detail[0].spectrum.as_str());
        assert_eq!("True", json.detail[0].gate.as_str());

        teardown(chan, &papi);
    }
    #[test]
    fn apply_list_3() {
        // list pattern but does not match

        // need to make a parameter a spectrum and a gate to
        // test success.

        let rocket = setup();
        //

        let (chan, papi) = get_state(&rocket);

        // Use the channel to make a parameter, spectrum  and
        // condition api which we'll use to create what we need to test
        // application:

        let param_api = parameter_messages::ParameterMessageClient::new(&chan);
        let cnd_api = condition_messages::ConditionMessageClient::new(&chan);
        let spec_api = spectrum_messages::SpectrumMessageClient::new(&chan);

        param_api
            .create_parameter("test")
            .expect("Making parameter");
        assert!(if let condition_messages::ConditionReply::Created =
            cnd_api.create_true_condition("True")
        {
            true
        } else {
            false
        });
        spec_api
            .create_spectrum_1d("test_spec", "test", 0.0, 1024.0, 1024)
            .expect("making spectrum");

        // apply the gate the easy way:

        spec_api
            .gate_spectrum("test_spec", "True")
            .expect("Failed to gate spectrum");

        //  Get the listing:

        let c = Client::tracked(rocket).unwrap();
        let r = c.get("/list?pattern=test_spork"); // no pattern/
        let reply = r.dispatch();

        let json = reply
            .into_json::<ApplicationListing>()
            .expect("Failed Json decode");
        assert_eq!("OK", json.status.as_str());
        assert_eq!(0, json.detail.len());

        teardown(chan, &papi);
    }
    #[test]
    fn apply_list_4() {
        // List with pattern which does match.

        // need to make a parameter a spectrum and a gate to
        // test success.

        let rocket = setup();
        //

        let (chan, papi) = get_state(&rocket);

        // Use the channel to make a parameter, spectrum  and
        // condition api which we'll use to create what we need to test
        // application:

        let param_api = parameter_messages::ParameterMessageClient::new(&chan);
        let cnd_api = condition_messages::ConditionMessageClient::new(&chan);
        let spec_api = spectrum_messages::SpectrumMessageClient::new(&chan);

        param_api
            .create_parameter("test")
            .expect("Making parameter");
        assert!(if let condition_messages::ConditionReply::Created =
            cnd_api.create_true_condition("True")
        {
            true
        } else {
            false
        });
        spec_api
            .create_spectrum_1d("test_spec", "test", 0.0, 1024.0, 1024)
            .expect("making spectrum");

        // apply the gate the easy way:

        spec_api
            .gate_spectrum("test_spec", "True")
            .expect("Failed to gate spectrum");

        //  Get the listing:

        let c = Client::tracked(rocket).unwrap();
        let r = c.get("/list?pattern=test_spec"); // no pattern/
        let reply = r.dispatch();

        let json = reply
            .into_json::<ApplicationListing>()
            .expect("Failed Json decode");
        assert_eq!("OK", json.status.as_str());
        assert_eq!(1, json.detail.len());
        assert_eq!("test_spec", json.detail[0].spectrum.as_str());
        assert_eq!("True", json.detail[0].gate.as_str());
        teardown(chan, &papi);
    }
    #[test]
    fn ungate_1() {
        // no such spectrum.
        let rocket = setup();
        //

        let (chan, papi) = get_state(&rocket);

        let c = Client::tracked(rocket).unwrap();
        let r = c.get("/?name=george");
        let reply = r.dispatch();
        let json = reply
            .into_json::<GateApplicationResponse>()
            .expect("Invalid json");

        assert_eq!(
            "Unable to ungate at least one spectrum",
            json.status.as_str()
        );
        assert_eq!(1, json.detail.len());
        assert_eq!("george", json.detail[0].0.as_str());

        teardown(chan, &papi);
    }
    #[test]
    fn ungate_2() {
        // Sucessful ungate - we're going to test he ungating as
        // well:

        let rocket = setup();
        //

        let (chan, papi) = get_state(&rocket);

        // Use the channel to make a parameter, spectrum  and
        // condition api which we'll use to create what we need to test
        // application:

        let param_api = parameter_messages::ParameterMessageClient::new(&chan);
        let cnd_api = condition_messages::ConditionMessageClient::new(&chan);
        let spec_api = spectrum_messages::SpectrumMessageClient::new(&chan);

        param_api
            .create_parameter("test")
            .expect("Making parameter");
        assert!(if let condition_messages::ConditionReply::Created =
            cnd_api.create_true_condition("True")
        {
            true
        } else {
            false
        });
        spec_api
            .create_spectrum_1d("test_spec", "test", 0.0, 1024.0, 1024)
            .expect("making spectrum");

        // apply the gate the easy way:

        spec_api
            .gate_spectrum("test_spec", "True")
            .expect("Failed to gate spectrum");

        //  Get the listing:

        let c = Client::tracked(rocket).unwrap();
        let r = c.get("/?name=test_spec"); // no pattern/
        let reply = r.dispatch();

        let json = reply
            .into_json::<ApplicationListing>()
            .expect("Failed Json decode");
        assert_eq!("OK", json.status.as_str());
        assert_eq!(0, json.detail.len());

        // The spectrum should not be gated now:

        let listing = spec_api.list_spectra("*").expect("Failed listing");
        assert_eq!(1, listing.len());
        assert!(listing[0].gate.is_none());

        teardown(chan, &papi);
    }
}
