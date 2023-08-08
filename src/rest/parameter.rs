//! The rest::parameter module contains handlers for the
//! spectcl/parameter set of URLs.  These URLs provide
//! REST interfaces to the parameter subsystem of the
//! histogram server.
//! Specifically:
//!
//! *   ../list - list all or some of the parameters.
//! *   ../edit - modify the metadata properties of a parameter.
//! *   ../promote - provide metadata properties of a parmaeter that may have none.
//! for rustogramer this is the same as edit.
//! *   ../create - Create a new parameter
//! *   ../listnew - This is routed to list for now.
//! *   ../check - Checks the flag for parameter changes (always true for rustogramer).
//! *   ../uncheck - uncheks the parameter change flag (NO_OP).
//! *   ../version - Returns a tree parameter version string which
//!will be 2.0 for rustogramer.

//#[macro_use]
//extern crate rocket;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;

use super::*;

use crate::messaging::parameter_messages::ParameterMessageClient;

//------------------------- List operation ---------------------
// These define structs that will be serialized.
// to Json:
// And, where needed their implementation of traits required.
//
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ParameterDefinition {
    name: String,
    id: u32,
    bins: Option<u32>,
    low: Option<f64>,
    high: Option<f64>,
    units: Option<String>,
    description: Option<String>, // New in rustogramer.
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Parameters {
    status: String,
    detail: Vec<ParameterDefinition>,
}

/// List the parameters:
///
/// The URL is
///
///    spectcl/parameter/list[?filter=pattern]
///
/// If the filter pattern is supplied it must be a valid glob
/// pattern used to select the names of the parameters
/// actually listed.  detail on success is an array of
/// ParameterDefinition values JSON encoded.
///
#[get("/list?<filter>")]
pub fn list_parameters(
    filter: Option<String>,
    state: &State<SharedHistogramChannel>,
) -> Json<Parameters> {
    let mut result = Parameters {
        status: String::from("OK"),
        detail: Vec::<ParameterDefinition>::new(),
    };
    let api = ParameterMessageClient::new(&state.inner().lock().unwrap());

    let pattern = if let Some(p) = filter {
        p
    } else {
        String::from("*")
    };
    let list = api.list_parameters(&pattern);
    match list {
        Ok(listing) => {
            for p in listing {
                result.detail.push(ParameterDefinition {
                    name: p.get_name(),
                    id: p.get_id(),
                    bins: p.get_bins(),
                    low: p.get_limits().0,
                    high: p.get_limits().1,
                    units: p.get_units(),
                    description: p.get_description(),
                })
            }
        }
        Err(s) => {
            result.status = s;
        }
    }
    Json(result)
}

//---------------------------------------------------------
// What we need to provide the version:

/// Fetch the tree parameter version number.
/// The URL is of the form:
///
///      spectcl/parameter/version
///
/// No query parameters are allowed.  The detail on success
/// is a version string.
///
#[get("/version")]
pub fn parameter_version() -> Json<GenericResponse> {
    let version = GenericResponse::ok("2.0");

    Json(version)
}
//-----------------------------------------------------
// What we need to provide the /create method.
// We're going to allow low, high and bis all to be
// optional..only requiring name.

// This function is common between the /edit and /create methods:
// It sets the metadata associated with the parameter:

fn set_metadata(
    name: &str,
    bins: Option<u32>,
    limits: Option<(f64, f64)>,
    units: Option<String>,
    description: Option<String>,
    state: &State<SharedHistogramChannel>,
) -> GenericResponse {
    let mut response = GenericResponse::ok("");

    let api = ParameterMessageClient::new(&state.inner().lock().unwrap());
    if let Err(s) = api.modify_parameter_metadata(name, bins, limits, units, description) {
        response.status = String::from("Could not modify metadata");
        response.detail = s;
    }

    response
}

///
/// Implement the create operations for parameters.
///  The url in general is of the form:
///
///    spectcl/parameter/create?name=param_name[&bins=num_bins] \
///        [&low=low_lim&high=hi_lim][&units=uom][&description=a description]
///
/// Note from the above that either both low and high must be
/// supplied or neither.   The only required parameter is the
/// parameter name. All others are optional.
///
/// The description parameter is an extension from SpecTcl and supports
/// providing a human readable description of the parameter.
///
/// On success, the detail is empty.  On failure the status
/// provides a top level description of what was being attempted
/// the detail is a string that describes how it failed.
///
/// There are actually two requests made of the internal histogram
/// server.  The first creates the parameter and the second
/// then sets any metadata that has been supplied in the URL query
/// parameters.
///
#[get("/create?<name>&<low>&<high>&<bins>&<units>&<description>")]
pub fn create_parameter(
    name: String,
    low: Option<f64>,
    high: Option<f64>,
    bins: Option<u32>,
    units: Option<String>,
    description: Option<String>,
    state: &State<SharedHistogramChannel>,
) -> Json<GenericResponse> {
    let mut response = GenericResponse::ok("");

    // Both low and high must be supplied, not just one:

    if (low.is_none() && high.is_some()) || (low.is_some() && high.is_none()) {
        response.status = String::from("invalid request");
        response.detail = String::from("Either low and high must be provided or neither");
    } else {
        // Fish out low/high given that either both are there or none:

        let limits = if low.is_some() {
            Some((low.unwrap(), high.unwrap()))
        } else {
            None
        };

        // Make the API so we can create and, if needed,
        // modify the metadata:

        let api = ParameterMessageClient::new(&state.inner().lock().unwrap());
        let reply = api.create_parameter(&name);
        match reply {
            Ok(_) => {
                // Attempt to set the metadata:
                response = set_metadata(&name, bins, limits, units, description, state);
            }
            Err(s) => {
                response.status = String::from("'treeparameter -create' failed: ");
                response.detail = s;
            }
        }
    }
    Json(response)
}
//------------------------------------------------------------------
// Edit the metadata associated with a parameter:

///
/// Modify the metadata associated with a parameter.
/// Base URL is spectcl/parameter/edit
/// Query parameters are:
///
/// *  name - required -name of the parameter to modify.
/// *  bins - optional - number of bins metadata
/// *  low  - optional - Low limit metadata
/// *  high  - optional - high limit metadata.
/// *  units - optional - units of measure metadata.
/// *  descdription - optional - parameter description.  This is
/// new metadata with Rustogramer.
///
/// The reply on success as status "OK" and detail an empty thing.
/// On failure status is a top level error string with additional
/// information in detail.
///
#[get("/edit?<name>&<bins>&<low>&<high>&<units>&<description>")]
pub fn edit_parameter(
    name: String,
    bins: Option<u32>,
    low: Option<f64>,
    high: Option<f64>,
    units: Option<String>,
    description: Option<String>,
    state: &State<SharedHistogramChannel>,
) -> Json<GenericResponse> {
    let mut response = GenericResponse::ok("");

    if (low.is_none() && high.is_some()) || (low.is_some() && high.is_none()) {
        response.status = String::from("invalid request");
        response.detail = String::from("Either low and high must be provided or neither");
    } else {
        // Fish out low/high given that either both are there or none:

        let limits = if low.is_some() {
            Some((low.unwrap(), high.unwrap()))
        } else {
            None
        };

        // Make the API so we can create and, if needed,
        // modify the metadata:

        response = set_metadata(&name, bins, limits, units, description, state);
    }
    Json(response)
}
// Note that Promote is the same as edit since all parameters in
// rustogrammer have implicit metadata

/// See edit for information about the query parameters asnd
/// return data - this just calls that method.
///
#[get("/promote?<name>&<bins>&<low>&<high>&<units>&<description>")]
pub fn promote_parameter(
    name: String,
    bins: Option<u32>,
    low: Option<f64>,
    high: Option<f64>,
    units: Option<String>,
    description: Option<String>,
    state: &State<SharedHistogramChannel>,
) -> Json<GenericResponse> {
    edit_parameter(name, bins, low, high, units, description, state)
}
//--------------------------------------------------------------------
// CHeck status

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CheckResponse {
    status: String,
    detail: Option<u8>,
}
// This method is used by check and uncheck to factor out their
// mostly similar code:

fn check_uncheck_common_code(name: &str, state: &State<SharedHistogramChannel>) -> CheckResponse {
    let mut response = CheckResponse {
        status: String::from("OK"),
        detail: Some(0),
    };
    let api = ParameterMessageClient::new(&state.inner().lock().unwrap());
    let result = api.list_parameters(name);
    match result {
        Ok(listing) => {
            if listing.len() == 0 {
                response.status = format!("No such parameter {}", name);
                response.detail = None;
            }
        }
        Err(s) => {
            response.status = format!("Check of parameter failed: {}", s);
            response.detail = None;
        }
    }
    response
}

///
/// /spectcl/parameter/check
///
/// There is no check flag on Rustogramer status items so
/// this always return false.
///
/// The sole query parameter is _name_ - the name of the parameter
/// to modify.
/// We do go through the trouble of ensuring that parameter
/// actually exists first.  Returns:
///
/// Success:  status is OK and detail is 0
/// Failure:  Status is a top level error message and
/// Empty.
#[get("/check?<name>")]
pub fn check_parameter(name: String, state: &State<SharedHistogramChannel>) -> Json<CheckResponse> {
    let response = check_uncheck_common_code(&name, state);
    Json(response)
}
//----------------------------------------------------------------
// uncheck

///
/// This is a no-op
///
/// Query parameter _name_ specifies the name of the parameter to
/// 'check'.
///
/// Successful reply:  status is OK, detail is Null.
/// Failed repsly:  Status is a detailed error message detail is null.
///
#[get("/uncheck?<name>")]
pub fn uncheck_parameter(
    name: String,
    state: &State<SharedHistogramChannel>,
) -> Json<CheckResponse> {
    let mut response = check_uncheck_common_code(&name, state);
    response.detail = None; // Fix up resposne.

    Json(response)
}
//------------------------------------------------------------
// Rawparameters has some similar properties to
// parameters and, therefore, can share some code.
//

///
/// new is essentially create.
/// the SpecTcl interface supports an id query parameter which
/// we just ignore as IDs get assigned by the parameter dictionary
/// in the histogramer server:
#[get("/new?<name>&<low>&<high>&<bins>&<units>&<description>")]
pub fn new_rawparameter(
    name: String,
    low: Option<f64>,
    high: Option<f64>,
    bins: Option<u32>,
    units: Option<String>,
    description: Option<String>,
    state: &State<SharedHistogramChannel>,
) -> Json<GenericResponse> {
    create_parameter(name, low, high, bins, units, description, state)
}

///
/// list is a front end to the list_parameters method:
///
///  1.  The user must only supply either a pattern or an id:
/// else we throw an error back.
///  2. If the user supplies a pattern we can directly delegate to
/// list_parameters.
///  3. If the user supplies an id, we get the list of all parameters
/// and find the one with the correct id - if we find it, we can
/// us its name as a pattern to call list_parameters otherwise
/// toss an error back
///
#[get("/list?<pattern>&<id>")]
pub fn list_rawparameter(
    pattern: Option<String>,
    id: Option<u32>,
    state: &State<SharedHistogramChannel>,
) -> Json<Parameters> {
    if pattern.is_some() && id.is_some() {
        Json(Parameters {
            status: String::from("Only id or pattern can be supplied, not both"),
            detail: Vec::<ParameterDefinition>::new(),
        })
    } else if pattern.is_none() && id.is_none() {
        Json(Parameters {
            status: String::from("One of name or id must be supplied neither were"),
            detail: Vec::<ParameterDefinition>::new(),
        })
    } else {
        if let Some(_) = pattern {
            list_parameters(pattern, state)
        } else {
            let name = find_parameter_by_id(id.unwrap(), state);
            if name.is_some() {
                list_parameters(name, state)
            } else {
                Json(Parameters {
                    status: format!("No parameter with id {} exists", id.unwrap()),
                    detail: Vec::<ParameterDefinition>::new(),
                })
            }
        }
    }
}
/// delete is not supported and will always return an error:

#[get("/delete")]
pub fn delete_rawparameter() -> Json<GenericResponse> {
    let result = GenericResponse::err(
        "Deletion of parameters is not supported",
        "This is rustogrammer not SpecTcl",
    );
    Json(result)
}

#[cfg(test)]
mod parameter_tests {
    use super::*;
    use crate::messaging;
    use crate::messaging::parameter_messages;
    use crate::processing;
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;

    fn setup() -> Rocket<Build> {
        rest_common::setup()
            .mount(
                "/tree",
                routes![
                    create_parameter,
                    edit_parameter,
                    promote_parameter,
                    check_parameter,
                    uncheck_parameter,
                    new_rawparameter,
                    list_rawparameter,
                    delete_rawparameter
                ],
            )
            .mount("/par", routes![list_parameters, parameter_version,])
    }
    fn teardown(c: mpsc::Sender<messaging::Request>, p: &processing::ProcessingApi) {
        rest_common::teardown(c, p);
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (mpsc::Sender<messaging::Request>, processing::ProcessingApi) {
        rest_common::get_state(r)
    }
    #[test]
    fn listp_1() {
        // list_parameters - none existing and no filter.

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/par/list");
        let reply = req
            .dispatch()
            .into_json::<Parameters>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(0, reply.detail.len());

        teardown(c, &papi);
    }
    #[test]
    fn listp_2() {
        // Make a parameter then list with no filter:
        // no metadata:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        param_api
            .create_parameter("a_parameter")
            .expect("Creating test parmaeter");

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/par/list");
        let reply = req
            .dispatch()
            .into_json::<Parameters>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(1, reply.detail.len());
        let pinfo = &reply.detail[0];
        assert_eq!("a_parameter", pinfo.name);
        assert_eq!(1, pinfo.id);
        assert!(pinfo.bins.is_none());
        assert!(pinfo.low.is_none());
        assert!(pinfo.high.is_none());
        assert!(pinfo.units.is_none());
        assert!(pinfo.description.is_none());

        teardown(c, &papi);
    }
    #[test]
    fn listp_3() {
        // Filter only lists the paramters that match:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        param_api.create_parameter("param1").expect("making param1");
        param_api.create_parameter("param2").expect("making param2");

        let client = Client::tracked(rocket).expect("Making client'");
        let req = client.get("/par/list?filter=*2");
        let reply = req
            .dispatch()
            .into_json::<Parameters>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(1, reply.detail.len());
        assert_eq!("param2", reply.detail[0].name);
        assert_eq!(2, reply.detail[0].id);

        teardown(c, &papi);
    }
    #[test]
    fn listp_4() {
        // List parameter that has metadata:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let param_api = parameter_messages::ParameterMessageClient::new(&c);

        param_api.create_parameter("param1").expect("param1");
        param_api
            .modify_parameter_metadata(
                "param1",
                Some(1024),
                Some((0.0, 1024.0)),
                Some(String::from("furlong/fortnight")),
                Some(String::from("this is a description")),
            )
            .expect("Setting param1's metadata");

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/par/list");
        let reply = req
            .dispatch()
            .into_json::<Parameters>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(1, reply.detail.len());
        let info = &reply.detail[0];
        assert_eq!("param1", info.name);
        assert_eq!(1, info.id);
        assert!(if let Some(bins) = info.bins {
            assert_eq!(1024, bins);
            true
        } else {
            false
        });
        assert!(if let Some(low) = info.low {
            assert_eq!(0.0, low);
            true
        } else {
            false
        });
        assert!(if let Some(high) = info.high {
            assert_eq!(1024.0, high);
            true
        } else {
            false
        });
        assert!(if let Some(units) = &info.units {
            assert_eq!("furlong/fortnight", units);
            true
        } else {
            false
        });
        assert!(if let Some(desc) = &info.description {
            assert_eq!("this is a description", desc);
            true
        } else {
            false
        });

        teardown(c, &papi);
    }
    #[test]
    fn version_1() {
        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/par/version");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!("2.0", reply.detail);

        teardown(c, &papi);
    }
    #[test]
    fn pcreate_1() {
        // create a parameter with no metadata:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/tree/create?name=param1");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        // Now let's be sure the parameter go created (with no metadata):

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        let listing = param_api.list_parameters("*");
        assert!(listing.is_ok());
        let listing = listing.unwrap();
        assert_eq!(1, listing.len());
        let info = &listing[0];
        assert_eq!("param1", info.get_name());
        assert_eq!(1, info.get_id());
        assert!(info.get_limits().0.is_none());
        assert!(info.get_limits().1.is_none());
        assert!(info.get_bins().is_none());
        assert!(info.get_units().is_none());
        assert!(info.get_description().is_none());

        teardown(c, &papi);
    }
    #[test]
    fn pcreate_2() {
        // Making a duplicate parameter is an error:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);
        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        param_api
            .create_parameter("p1")
            .expect("Making existing parameter");

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/tree/create?name=p1");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing JSON");

        assert_eq!("'treeparameter -create' failed: ", reply.status);

        teardown(c, &papi);
    }
    #[test]
    fn pcreate_3() {
        // Make aparameter with limits

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/tree/create?name=p1&low=0.0&high=1024.0");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        // See what got created:

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        let listing = param_api
            .list_parameters("*")
            .expect("Listing parameters via API");
        assert_eq!(1, listing.len());
        let info = &listing[0];
        assert_eq!("p1", info.get_name());
        assert_eq!(1, info.get_id());
        let limits = info.get_limits();
        assert!(limits.0.is_some());
        assert_eq!(0.0, limits.0.unwrap());
        assert!(limits.1.is_some());
        assert_eq!(1024.0, limits.1.unwrap());
        assert!(info.get_bins().is_none());
        assert!(info.get_units().is_none());
        assert!(info.get_description().is_none());

        teardown(c, &papi);
    }
    #[test]
    fn pcreate_4() {
        // If we're giving limits we need both of them:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/tree/create?name=p1&low=0.0");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("invalid request", reply.status);
        assert_eq!(
            "Either low and high must be provided or neither",
            reply.detail
        );

        let req = client.get("/tree/create?name=p1&high=0.0");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("invalid request", reply.status);
        assert_eq!(
            "Either low and high must be provided or neither",
            reply.detail
        );

        teardown(c, &papi);
    }
    #[test]
    fn pcreate_5() {
        // Set bins:
        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/tree/create?name=p1&low=0.0&high=1024.0&bins=512");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        // see what got made:

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        let listing = param_api
            .list_parameters("*")
            .expect("Listing parameters via API");
        assert_eq!(1, listing.len());
        let info = &listing[0];
        assert_eq!("p1", info.get_name());
        assert_eq!(1, info.get_id());
        let limits = info.get_limits();
        assert!(limits.0.is_some());
        assert_eq!(0.0, limits.0.unwrap());
        assert!(limits.1.is_some());
        assert_eq!(1024.0, limits.1.unwrap());
        assert!(info.get_bins().is_some());
        assert_eq!(512, info.get_bins().unwrap());
        assert!(info.get_units().is_none());
        assert!(info.get_description().is_none());

        teardown(c, &papi);
    }
    #[test]
    fn pcreate_6() {
        // Set the units of measure:
        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/tree/create?name=p1&low=0.0&high=1024.0&bins=512&units=cm");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        // see what got made:

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        let listing = param_api
            .list_parameters("*")
            .expect("Listing parameters via API");
        assert_eq!(1, listing.len());
        let info = &listing[0];
        assert_eq!("p1", info.get_name());
        assert_eq!(1, info.get_id());
        let limits = info.get_limits();
        assert!(limits.0.is_some());
        assert_eq!(0.0, limits.0.unwrap());
        assert!(limits.1.is_some());
        assert_eq!(1024.0, limits.1.unwrap());
        assert!(info.get_bins().is_some());
        assert_eq!(512, info.get_bins().unwrap());
        assert!(info.get_units().is_some());
        assert_eq!("cm", info.get_units().unwrap());
        assert!(info.get_description().is_none());

        teardown(c, &papi);
    }
    #[test]
    fn pcreate_7() {
        // Create with a description.

        // Set the units of measure:
        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/tree/create?name=p1&low=0.0&high=1024.0&bins=512&units=cm&description=This%20is%20a%20parameter");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        // see what got made:

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        let listing = param_api
            .list_parameters("*")
            .expect("Listing parameters via API");
        assert_eq!(1, listing.len());
        let info = &listing[0];
        assert_eq!("p1", info.get_name());
        assert_eq!(1, info.get_id());
        let limits = info.get_limits();
        assert!(limits.0.is_some());
        assert_eq!(0.0, limits.0.unwrap());
        assert!(limits.1.is_some());
        assert_eq!(1024.0, limits.1.unwrap());
        assert!(info.get_bins().is_some());
        assert_eq!(512, info.get_bins().unwrap());
        assert!(info.get_units().is_some());
        assert_eq!("cm", info.get_units().unwrap());
        assert!(info.get_description().is_some());
        assert_eq!("This is a parameter", info.get_description().unwrap());

        teardown(c, &papi);
    }
    // Tests to edit the metadata for an existing parameter.

    #[test]
    fn edit_1() {
        // Parameter must exist. Else an error

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/tree/edit?name=p1");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing json");

        assert_eq!("Could not modify metadata", reply.status);

        teardown(c, &papi);
    }
    #[test]
    fn edit_2() {
        // Set bins:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        param_api
            .create_parameter("param")
            .expect("Creating parameter");

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/tree/edit?name=param&bins=1024");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        let listing = param_api.list_parameters("*").expect("Getting list");
        assert_eq!(1, listing.len());
        let info = &listing[0];
        assert_eq!("param", info.get_name());
        let bins = info.get_bins().expect("should be bins");
        assert_eq!(1024, bins);

        teardown(c, &papi);
    }
    #[test]
    fn edit_3() {
        // set low and high:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        param_api
            .create_parameter("param")
            .expect("Creating parameter");

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/tree/edit?name=param&bins=1024&low=0&high=512");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        let listing = param_api.list_parameters("*").expect("Getting list");
        assert_eq!(1, listing.len());
        let info = &listing[0];
        assert_eq!("param", info.get_name());
        let bins = info.get_bins().expect("should be bins");
        assert_eq!(1024, bins);
        let limits = info.get_limits();
        assert_eq!(0.0, limits.0.expect("Low not here"));
        assert_eq!(512.0, limits.1.expect("HIgh not here"));

        teardown(c, &papi);
    }
    #[test]
    fn edit_4() {
        // both low and high must be present if either is:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        param_api
            .create_parameter("param")
            .expect("Creating parameter");

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/tree/edit?name=param&bins=1024&low=0");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("invalid request", reply.status);
        assert_eq!(
            "Either low and high must be provided or neither",
            reply.detail
        );

        let req = client.get("/tree/edit?name=param&bins=1024&high=0");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("invalid request", reply.status);
        assert_eq!(
            "Either low and high must be provided or neither",
            reply.detail
        );

        teardown(c, &papi);
    }
    #[test]
    fn edit_5() {
        // Set units of measure:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        param_api
            .create_parameter("param")
            .expect("Creating parameter");

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/tree/edit?name=param&bins=1024&low=0&high=512&units=furlongs");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        let listing = param_api.list_parameters("*").expect("Getting list");
        assert_eq!(1, listing.len());
        let info = &listing[0];
        assert_eq!("param", info.get_name());
        let bins = info.get_bins().expect("should be bins");
        assert_eq!(1024, bins);
        let limits = info.get_limits();
        assert_eq!(0.0, limits.0.expect("Low not here"));
        assert_eq!(512.0, limits.1.expect("HIgh not here"));
        let units = info.get_units().expect("No units!");
        assert_eq!("furlongs", units);

        teardown(c, &papi);
    }
    #[test]
    fn edit_6() {
        // set the description:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        param_api
            .create_parameter("param")
            .expect("Creating parameter");

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/tree/edit?name=param&bins=1024&low=0&high=512&units=furlongs&description=This%20is%20a%20description");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        let listing = param_api.list_parameters("*").expect("Getting list");
        assert_eq!(1, listing.len());
        let info = &listing[0];
        assert_eq!("param", info.get_name());
        let bins = info.get_bins().expect("should be bins");
        assert_eq!(1024, bins);
        let limits = info.get_limits();
        assert_eq!(0.0, limits.0.expect("Low not here"));
        assert_eq!(512.0, limits.1.expect("HIgh not here"));
        let units = info.get_units().expect("No units!");
        assert_eq!("furlongs", units);
        let desc = info.get_description().expect("No description");
        assert_eq!("This is a description", desc);

        teardown(c, &papi);
    }
    // Note that the 'check' flag does not exit in rustogramer
    // so return values are fixed -- if there are matching parameters.

    #[test]
    fn check_1() {
        // Parameter does not exist:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/tree/check?name=p1");
        let reply = req
            .dispatch()
            .into_json::<CheckResponse>()
            .expect("Parsing JSON");

        assert_eq!("No such parameter p1", reply.status);
        assert!(reply.detail.is_none());

        teardown(c, &papi);
    }
    #[test]
    fn check_2() {
        // Parameter does exist:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        param_api
            .create_parameter("p1")
            .expect("Making parameter with API");

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/tree/check?name=p1");
        let reply = req
            .dispatch()
            .into_json::<CheckResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert!(reply.detail.is_some());
        assert_eq!(0, reply.detail.unwrap());

        teardown(c, &papi);
    }
    #[test]
    fn uncheck_1() {
        // Parameter does not exist:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/tree/uncheck?name=p1");
        let reply = req
            .dispatch()
            .into_json::<CheckResponse>()
            .expect("Parsing JSON");

        assert_eq!("No such parameter p1", reply.status);
        assert!(reply.detail.is_none());

        teardown(c, &papi);
    }
    #[test]
    fn uncheck_2() {
        // Parameter does exist:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        param_api
            .create_parameter("p1")
            .expect("Making parameter with API");

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/tree/uncheck?name=p1");
        let reply = req
            .dispatch()
            .into_json::<CheckResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert!(reply.detail.is_none());

        teardown(c, &papi);
    }
    // list_rawparameters is mostly a front end to list cases to test:
    //
    // - neither name nor id suppllied.
    // - both name and id supplied.
    // - id supplied and found.
    // - id supplied and  not found.

    #[test]
    fn rawlist_1() {
        // Name _and_ id missing.
        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/tree/list");
        let reply = req
            .dispatch()
            .into_json::<Parameters>()
            .expect("Decoding JSON");

        assert_eq!(
            "One of name or id must be supplied neither were",
            reply.status
        );
        assert_eq!(0, reply.detail.len());

        teardown(c, &papi);
    }
    #[test]
    fn rawlist_2() {
        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/tree/list?pattern=*&id=12");
        let reply = req
            .dispatch()
            .into_json::<Parameters>()
            .expect("Decoding JSON");

        assert_eq!("Only id or pattern can be supplied, not both", reply.status);
        assert_eq!(0, reply.detail.len());

        teardown(c, &papi);
    }
    #[test]
    fn rawlist_3() {
        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        param_api
            .create_parameter("abcd")
            .expect("Creating parameter");

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/tree/list?id=1");
        let reply = req
            .dispatch()
            .into_json::<Parameters>()
            .expect("Decoding Json");

        assert_eq!("OK", reply.status);
        assert_eq!(1, reply.detail.len());
        let info = &reply.detail[0];
        assert_eq!("abcd", info.name);
        assert_eq!(1, info.id);
        assert!(info.bins.is_none());
        assert!(info.low.is_none());
        assert!(info.high.is_none());
        assert!(info.units.is_none());
        assert!(info.description.is_none());

        teardown(c, &papi);
    }
    #[test]
    fn rawlist_4() {
        // id supplied and not found.

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let param_api = parameter_messages::ParameterMessageClient::new(&c);
        param_api
            .create_parameter("abcd")
            .expect("Creating parameter");

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/tree/list?id=2");
        let reply = req
            .dispatch()
            .into_json::<Parameters>()
            .expect("Decoding Json");

        assert_eq!("No parameter with id 2 exists", reply.status);
        assert_eq!(0, reply.detail.len());

        teardown(c, &papi);
    }
    #[test]
    fn delete_1() {
        // parameters can't be deleted:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/tree/delete");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Decoding JSON");

        assert_eq!("Deletion of parameters is not supported", reply.status);
        assert_eq!("This is rustogrammer not SpecTcl", reply.detail);

        teardown(c, &papi);
    }
}
