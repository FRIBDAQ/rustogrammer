//! Support for the ringversion domain of URLs
//! We extend the SpecTcl REST API to support not only setting the
//! ring item version format but also by querying the format currently in
//! use:
//!
//! *  /spectcl/ringformat - sets the ring item format.
//! *  /spectcl/ringformat/get - returns the current ring format.

use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;

use super::*;
use rust_ringitem_format::RingVersion;

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
pub fn ringversion_set(major: String, state: &State<SharedProcessingApi>) -> Json<GenericResponse> {
    let api = state.inner().lock().unwrap();

    let result = major.parse::<RingVersion>();
    if let Err(r) = result {
        Json(GenericResponse::err(
            "Unable to set ring format version",
            &r,
        ))
    } else {
        let v = result.unwrap();
        let result = api.set_ring_version(v);
        Json(match result {
            Ok(_) => GenericResponse::ok(""),
            Err(reason) => GenericResponse::err("Unable to set ring format version", &reason),
        })
    }
}

//------------------------------------------------------------------------
// Getting the ring version:

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct VersionDetail {
    major: usize,
    minor: usize,
}
#[derive(Serialize, Deserialize)]
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
pub fn ringversion_get(state: &State<SharedProcessingApi>) -> Json<VersionResponse> {
    let api = state.inner().lock().unwrap();
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

#[cfg(test)]
mod ringversion_tests {
    use super::*;
    use crate::messaging;
    use crate::processing;
    use crate::sharedmem::binder;
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;

    fn setup() -> Rocket<Build> {
        rest_common::setup().mount("/", routes![ringversion_set, ringversion_get])
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        rest_common::teardown(c, p, b);
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
    ) {
        rest_common::get_state(r)
    }

    #[test]
    fn set_1() {
        // Legal version:

        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/?major=12");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn set_2() {
        // in valid version:

        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/?major=xyzzy");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Unable to set ring format version", reply.status);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn get_1() {
        // get 11.0:

        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        // Set it to 11:

        papi.set_ring_version(RingVersion::V11)
            .expect("Setting ringversion");

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/get");
        let reply = req
            .dispatch()
            .into_json::<VersionResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(11, reply.detail.major);
        assert_eq!(0, reply.detail.minor);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn get_2() {
        // get 12.0:

        let rocket = setup();
        let (c, papi, bapi) = getstate(&rocket);

        // Set it to 11:

        papi.set_ring_version(RingVersion::V12)
            .expect("Setting ringversion");

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/get");
        let reply = req
            .dispatch()
            .into_json::<VersionResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(12, reply.detail.major);
        assert_eq!(0, reply.detail.minor);

        teardown(c, &papi, &bapi);
    }
}
