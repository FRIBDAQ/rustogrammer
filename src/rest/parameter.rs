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
use rocket::serde::{json::Json, Serialize};
use rocket::State;

use super::*;

use crate::messaging::parameter_messages::ParameterMessageClient;

//------------------------- List operation ---------------------
// These define structs that will be serialized.
// to Json:
// And, where needed their implementation of traits required.
//
#[derive(Serialize)]
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

#[derive(Serialize)]
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
    let api = ParameterMessageClient::new(&state.inner().state.lock().unwrap().1);

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

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct TreeParameterVersion {
    status: String,
    detail: String,
}

/// Fetch the tree parameter version number.
/// The URL is of the form:
///
///      spectcl/parameter/version
///
/// No query parameters are allowed.  The detail on success
/// is a version string.
///
#[get("/version")]
pub fn parameter_version() -> Json<TreeParameterVersion> {
    let version = TreeParameterVersion {
        status: String::from("OK"),
        detail: String::from("2.0"),
    };

    Json(version)
}
//-----------------------------------------------------
// What we need to provide the /create method.
// We're going to allow low, high and bis all to be
// optional..only requiring name.

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct GenericResponse {
    status: String,
    detail: String,
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

        let api = ParameterMessageClient::new(&state.inner().state.lock().unwrap().1);
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
    let mut response = GenericResponse {
        status: String::from("OK"),
        detail: String::from(""),
    };

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

        let api = ParameterMessageClient::new(&state.inner().state.lock().unwrap().1);
        if let Err(s) = api.modify_parameter_metadata(&name, bins, limits, units, description) {
            response.status = String::from("Could not modify metadata");
            response.detail = s;
        }
    }
    Json(response)
}
