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
pub fn list_parameters(filter: Option<String>, state: &State<HistogramState>) -> Json<Parameters> {
    let mut result = Parameters {
        status: String::from("OK"),
        detail: Vec::<ParameterDefinition>::new(),
    };
    let api = ParameterMessageClient::new(&state.inner().histogramer.lock().unwrap());

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
    state: &State<HistogramState>,
) -> Json<GenericResponse> {
    let mut response = GenericResponse {
        status: String::from("OK"),
        detail: String::new(),
    };

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

        let api = ParameterMessageClient::new(&state.inner().histogramer.lock().unwrap());
        let reply = api.create_parameter(&name);
        match reply {
            Ok(_) => {
                // Attempt to set the metadata:

                let status = api.modify_parameter_metadata(&name, bins, limits, units, description);
                if let Err(s) = status {
                    response.status = String::from("Failed set parameter metadata: ");
                    response.detail = s;
                }
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
    state: &State<HistogramState>,
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

        let api = ParameterMessageClient::new(&state.inner().histogramer.lock().unwrap());
        if let Err(s) = api.modify_parameter_metadata(&name, bins, limits, units, description) {
            response.status = String::from("Could not modify metadata");
            response.detail = s;
        }
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
    state: &State<HistogramState>,
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

fn check_uncheck_common_code(name: &str, state: &State<HistogramState>) -> CheckResponse {
    let mut response = CheckResponse {
        status: String::from("OK"),
        detail: Some(0),
    };
    let api = ParameterMessageClient::new(&state.inner().histogramer.lock().unwrap());
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
pub fn check_parameter(name: String, state: &State<HistogramState>) -> Json<CheckResponse> {
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
pub fn uncheck_parameter(name: String, state: &State<HistogramState>) -> Json<CheckResponse> {
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
    state: &State<HistogramState>,
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
    state: &State<HistogramState>,
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
    use crate::histogramer;
    use crate::messaging;
    use crate::messaging::parameter_messages;
    use crate::processing;
    use crate::rest::HistogramState;
    use crate::sharedmem::binder;
    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;
    use std::sync::Mutex;

    fn setup() -> Rocket<Build> {
        let (_, hg_sender) = histogramer::start_server();
        let (binder_req, _rx): (
            mpsc::Sender<binder::Request>,
            mpsc::Receiver<binder::Request>,
        ) = mpsc::channel();

        // Construct the state:

        let state = HistogramState {
            histogramer: Mutex::new(hg_sender.clone()),
            binder: Mutex::new(binder_req),
            processing: Mutex::new(processing::ProcessingApi::new(&hg_sender)),
            portman_client: None,
        };
        // Note we have two domains here because of the SpecTcl
        // divsion between tree parameters and raw parameters.

        rocket::build()
            .manage(state)
            .mount("/par", routes![list_parameters, parameter_version])
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
    }
    fn teardown(c: mpsc::Sender<messaging::Request>, p: &processing::ProcessingApi) {
        histogramer::stop_server(&c);
        p.stop_thread().expect("Stopping processing thread");
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (mpsc::Sender<messaging::Request>, processing::ProcessingApi) {
        let chan = r
            .state::<HistogramState>()
            .expect("Valid state")
            .histogramer
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
}
