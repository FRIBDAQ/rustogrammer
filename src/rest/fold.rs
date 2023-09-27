//!  Folds are a concept specific to the analysis of sequential
//!  decays by gamma emission.  The idea is that you can create a
//!  condition that involves the parameters of a multiply incremented
//!  spetrum.  Normally, this codition is a specific decay peak in
//!  the spectrum.  
//!
//!  A fold then increments this spectrum for all parameters that
//!  don't make this condition true (folds could be one or 2-d).
//!  What remains in the spectrum are the peaks that correspond
//!  to gamma rays in the same sequence of decays.
//!
//! The initial version of Rustogramer does not implement folds.
//! Therefore we report to the client that all /spectcl/fold URIs
//! defined by the REST interface are not supported.  I'll note that
//! of all unsupported elements of the REST specification, this
//! one is most likely to be eventually supported.
//!  
//! /spectcl/fold has the following URIs under this domain:
//!
//! *   apply - applies a condition to a spectrum as a fold.
//! *   list  - lists the fold applications
//! *   remove - Removes a fold from the spectrum.
//!
use super::*;
use crate::messaging::spectrum_messages;
use rocket::serde::{json::Json, Deserialize, Serialize};

/// apply - unimplemented
///  If implemented the following query parameters would be required:
///
/// *  gate - the condition that defines the fold.
/// *  spectrum - the spectrum to be folded.
///
/// A GenericResponse is perfectly appropriate.
///
#[get("/apply?<gate>&<spectrum>")]
pub fn apply(
    gate: String,
    spectrum: String,
    state: &State<SharedHistogramChannel>,
) -> Json<GenericResponse> {
    let client = spectrum_messages::SpectrumMessageClient::new(&state.inner().lock().unwrap());
    let reply = if let Err(s) = client.fold_spectrum(&spectrum, &gate) {
        GenericResponse::err("Could not fold spectrum", &s)
    } else {
        GenericResponse::ok("")
    };
    Json(reply)
}

//
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct FoldInfo {
    spectrum: String,
    gate: String,
}
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct FoldListResponse {
    status: String,
    detail: Vec<FoldInfo>,
}
/// list - unimplemented
///  If implemented the _pattern_ query  parameter will filter out
/// the listing to only inlcude the spectra with names that match the
/// pattern.  The reply is a FoldListResponse shown above.
#[get("/list?<pattern>")]
pub fn list(
    pattern: OptionalString,
    msg_chan: &State<SharedHistogramChannel>,
) -> Json<FoldListResponse> {
    let hapi = spectrum_messages::SpectrumMessageClient::new(&msg_chan.inner().lock().unwrap());

    let p = if let Some(pp) = pattern {
        pp
    } else {
        String::from("*")
    };
    let response = match hapi.list_spectra(&p) {
        Ok(l) => {
            let mut result = FoldListResponse {
                status: String::from("OK"),
                detail: vec![],
            };

            for props in l {
                if props.fold.is_some() {
                    result.detail.push(FoldInfo {
                        spectrum: props.name.clone(),
                        gate: props.fold.clone().unwrap().clone(),
                    });
                }
            }
            result
        }
        Err(s) => FoldListResponse {
            status: format!("Failed to fetch spectrum list: {}", s),
            detail: vec![],
        },
    };

    Json(response)
}
/// remove - unimplemented
///
/// Requires one query parameter _spectrum_ Any fold will be removed
/// from that spectrum.
///
/// GenericResponse is appropriate.
///
#[get("/remove?<spectrum>")]
pub fn remove(spectrum: String, msg_chan: &State<SharedHistogramChannel>) -> Json<GenericResponse> {
    let sapi = spectrum_messages::SpectrumMessageClient::new(&msg_chan.inner().lock().unwrap());

    let reply = if let Err(s) = sapi.unfold_spectrum(&spectrum) {
        GenericResponse::err("Failed to remove fold", &s)
    } else {
        GenericResponse::ok("")
    };
    Json(reply)
}
#[cfg(test)]
mod fold_tests {
    use super::*;
    use crate::messaging;
    use crate::messaging::{condition_messages, parameter_messages, spectrum_messages};
    use crate::processing;
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;
    // note these are all unimplemented URLS so...

    fn setup() -> Rocket<Build> {
        rest_common::setup().mount("/", routes![crate::fold::apply, list, remove])
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        rest_common::teardown(c, p, b);
    }
    fn get_state(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
    ) {
        rest_common::get_state(r)
    }
    // Note none of thes URIS are implemented so the tests are
    // simple and don't need any other setup.  These tests are
    // actually placeholders against the eventual implementation
    // of folds in rustogramer.

    #[test]
    fn apply_1() {
        // Successful application.

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        // Make a set of parameters, a multicut and Multi1d:
        let parapi = parameter_messages::ParameterMessageClient::new(&c);
        let capi = condition_messages::ConditionMessageClient::new(&c);
        let sapi = spectrum_messages::SpectrumMessageClient::new(&c);

        let mut params = vec![];
        let mut param_ids = vec![];
        for i in 0..10 {
            let name = format!("param.{}", i);
            parapi.create_parameter(&name).expect("Making a parameter");
            params.push(name);
            param_ids.push(i);
        }
        assert!(matches!(
            capi.create_multicut_condition("mcut", &param_ids, 100.0, 200.0),
            condition_messages::ConditionReply::Created
        ));

        sapi.create_spectrum_multi1d("test", &params, 0.0, 1024.0, 1024)
            .expect("Making spectrum");

        let client = Client::untracked(rocket).expect("Making rocket client");
        let req = client.get("/apply?spectrum=test&gate=mcut");
        let response = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", response.status);

        // See that we do have a fold applied to "test"

        let l = sapi.list_spectra("test").expect("Listing spectra");

        assert_eq!(1, l.len());
        let desc = &l[0];
        assert!(desc.fold.is_some());
        assert_eq!("mcut", desc.fold.clone().unwrap());
        // Need to test apply.

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn apply_2() {
        // Ensure error handling works:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let cl = Client::untracked(rocket).expect("Making rocket client");
        let r = cl.get("/apply?spectrum=junk&gate=trash");
        let reply = r
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not fold spectrum", reply.status);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_1() {
        // Nothing to list.

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/list");

        let response = req
            .dispatch()
            .into_json::<FoldListResponse>()
            .expect("Decoding JSON");
        assert_eq!("OK", response.status);
        assert_eq!(0, response.detail.len());

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_2() {
        // make a folded spectrum and list:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        // Make a set of parameters, a multicut and Multi1d:
        let parapi = parameter_messages::ParameterMessageClient::new(&c);
        let capi = condition_messages::ConditionMessageClient::new(&c);
        let sapi = spectrum_messages::SpectrumMessageClient::new(&c);

        let mut params = vec![];
        let mut param_ids = vec![];
        for i in 0..10 {
            let name = format!("param.{}", i);
            parapi.create_parameter(&name).expect("Making a parameter");
            params.push(name);
            param_ids.push(i);
        }
        assert!(matches!(
            capi.create_multicut_condition("mcut", &param_ids, 100.0, 200.0),
            condition_messages::ConditionReply::Created
        ));

        sapi.create_spectrum_multi1d("test", &params, 0.0, 1024.0, 1024)
            .expect("Making spectrum");

        sapi.fold_spectrum("test", "mcut")
            .expect("Folding spectrum");

        let client = Client::untracked(rocket).expect("Making rocket client");
        let req = client.get("/list");
        let result = req
            .dispatch()
            .into_json::<FoldListResponse>()
            .expect("parsing json");

        assert_eq!("OK", result.status);
        assert_eq!(1, result.detail.len());
        let props = &result.detail[0];

        assert_eq!(
            FoldInfo {
                spectrum: String::from("test"),
                gate: String::from("mcut")
            },
            *props
        );

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_3() {
        // listing honors pattern when there's a match:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        // Make a set of parameters, a multicut and Multi1d:
        let parapi = parameter_messages::ParameterMessageClient::new(&c);
        let capi = condition_messages::ConditionMessageClient::new(&c);
        let sapi = spectrum_messages::SpectrumMessageClient::new(&c);

        let mut params = vec![];
        let mut param_ids = vec![];
        for i in 0..10 {
            let name = format!("param.{}", i);
            parapi.create_parameter(&name).expect("Making a parameter");
            params.push(name);
            param_ids.push(i);
        }
        assert!(matches!(
            capi.create_multicut_condition("mcut", &param_ids, 100.0, 200.0),
            condition_messages::ConditionReply::Created
        ));

        sapi.create_spectrum_multi1d("test", &params, 0.0, 1024.0, 1024)
            .expect("Making spectrum");

        sapi.fold_spectrum("test", "mcut")
            .expect("Folding spectrum");

        let client = Client::untracked(rocket).expect("Making rocket client");
        let req = client.get("/list?pattern=t*");
        let result = req
            .dispatch()
            .into_json::<FoldListResponse>()
            .expect("parsing json");

        assert_eq!("OK", result.status);
        assert_eq!(1, result.detail.len());
        let props = &result.detail[0];

        assert_eq!(
            FoldInfo {
                spectrum: String::from("test"),
                gate: String::from("mcut")
            },
            *props
        );

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_4() {
        // listing when there's no pattern match:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        // Make a set of parameters, a multicut and Multi1d:
        let parapi = parameter_messages::ParameterMessageClient::new(&c);
        let capi = condition_messages::ConditionMessageClient::new(&c);
        let sapi = spectrum_messages::SpectrumMessageClient::new(&c);

        let mut params = vec![];
        let mut param_ids = vec![];
        for i in 0..10 {
            let name = format!("param.{}", i);
            parapi.create_parameter(&name).expect("Making a parameter");
            params.push(name);
            param_ids.push(i);
        }
        assert!(matches!(
            capi.create_multicut_condition("mcut", &param_ids, 100.0, 200.0),
            condition_messages::ConditionReply::Created
        ));

        sapi.create_spectrum_multi1d("test", &params, 0.0, 1024.0, 1024)
            .expect("Making spectrum");

        sapi.fold_spectrum("test", "mcut")
            .expect("Folding spectrum");

        let client = Client::untracked(rocket).expect("Making rocket client");
        let req = client.get("/list?pattern=q*");
        let result = req
            .dispatch()
            .into_json::<FoldListResponse>()
            .expect("parsing json");

        assert_eq!("OK", result.status);
        assert_eq!(0, result.detail.len());

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn remove_1() {
        // Success when there's a fold applied:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        // Make a set of parameters, a multicut and Multi1d:
        let parapi = parameter_messages::ParameterMessageClient::new(&c);
        let capi = condition_messages::ConditionMessageClient::new(&c);
        let sapi = spectrum_messages::SpectrumMessageClient::new(&c);

        let mut params = vec![];
        let mut param_ids = vec![];
        for i in 0..10 {
            let name = format!("param.{}", i);
            parapi.create_parameter(&name).expect("Making a parameter");
            params.push(name);
            param_ids.push(i);
        }
        assert!(matches!(
            capi.create_multicut_condition("mcut", &param_ids, 100.0, 200.0),
            condition_messages::ConditionReply::Created
        ));

        sapi.create_spectrum_multi1d("test", &params, 0.0, 1024.0, 1024)
            .expect("Making spectrum");

        sapi.fold_spectrum("test", "mcut")
            .expect("Folding spectrum");

        let client = Client::untracked(rocket).expect("Making rocket client");
        let req = client.get("/remove?spectrum=test");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", reply.status);

        let l = sapi
            .list_spectra("test")
            .expect("Getting spectrum properties");
        assert_eq!(1, l.len());
        assert!(l[0].fold.is_none()); // Actually unfolded.

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn remove_2() {
        // Failure get mapped:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/remove?spectrum=junk");
        let resp = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Failed to remove fold", resp.status);

        teardown(c, &papi, &bapi);
    }
}
