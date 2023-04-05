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

use rocket::serde::ser::SerializeStruct;
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

#[get("/version")]
pub fn parameter_version() -> Json<TreeParameterVersion> {
    let version = TreeParameterVersion {
        status: String::from("OK"),
        detail: String::from("2.0"),
    };

    Json(version)
}
