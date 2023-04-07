//! This module implements Rocket handlers for
//! the the /spectcl/gate domain of URLS.
//! The name follows the SpecTcl name for conditions while
//! Rustogrammer knows that there are conditions which are
//! just objects that can be evaluated, as needed for each event
//! which return a true or false value.  
//!
//! A condition can the gate (verb) a spectrum to determine which
//! events are allowed to increment it.
//!
//! A nasty concern is that the condition type names supported
//! by Rustogramer have more useful names like True, And, Cut
//! where those in SpecTcl (and therefore the type-names expected
//! by REST clients) have simpler names like T, F, s, * (slice).
//! it is therefore necessary to map from Rustogramer
//! Gate types to SpecTcl gate types in this domain of URLs.

use rocket::serde::{json::Json, Serialize};
use rocket::State;

use super::*;

use crate::messaging::condition_messages::{ConditionMessageClient, ConditionReply};
use crate::messaging::parameter_messages::ParameterMessageClient;

// Private mappings between SpecTcl <-> Rustogramer gate types:
// Note making a static hashmap is possible but requires unsafe to access.
// Making the hashmap each time is possible but slower
// so we'll just use if chains.
//
fn rg_condition_to_spctl(rg_type: &str) -> String {
    match rg_type {
        "True" => String::from("T"),
        "False" => String::from("F"),
        "And" => String::from("*"),
        "Or" => String::from("+"),
        "Not" => String::from("-"),
        "Band" => String::from("b"),
        "Contour" => String::from("c"),
        "Cut" => String::from("s"),
        _ => String::from("-unsupported-"),
    }
}
fn spc_gate_to_rg(spc_type: &str) -> String {
    match spc_type {
        "T" => String::from("True"),
        "F" => String::from("False"),
        "*" => String::from("And"),
        "+" => String::from("Or"),
        "-" => String::from("Not"),
        "b" => String::from("Band"),
        "c" => String::from("Contour"),
        "s" => String::from("Slice"),
        _ => String::from("Unsupported"),
    }
}
//----------------------------------------------------------------
// Stuff to handle listing conditions(gates):

#[derive(Serialize, Clone, Copy)]
#[serde(crate = "rocket::serde")]
pub struct GatePoint {
    x: f64,
    y: f64,
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct GateProperties {
    name: String,
    #[serde(rename = "type")]
    type_name: String,
    gates: Vec<String>, // Dependencies.
    parameters: Vec<String>,
    points: Vec<GatePoint>,
    low: f64,
    high: f64,
    // value : u32            // Note Rustogrammer has no support for mask gates.
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct ListReply {
    status: String,
    detail: Vec<GateProperties>,
}

// Private utility to turn a vector of parameter ids to
// a vector of parameter string names.
// The ids should all be valid.

fn marshall_parameter_names(ids: &Vec<u32>, state: &State<HistogramState>) -> Vec<String> {
    let mut result = Vec::<String>::new();
    for id in ids {
        result.push(
            find_parameter_by_id(*id, state)
                .expect(format!("BUG Failed to find gate parameter {} by id", id).as_str()),
        );
    }
    result
}

// Private utility to marshall any points from a raw
// codition record to the points, low and high fields of
// a GateProperties struct:
//
fn marshall_points(p: &mut GateProperties, raw_pts: &Vec<(f64, f64)>) {
    for raw in raw_pts {
        let pt = GatePoint { x: raw.0, y: raw.1 };
        p.points.push(pt);
    }
    // Sufficient points to fill in low/high?
    //
    if p.points.len() >= 2 {
        p.low = p.points[0].x;
        p.high = p.points[1].x;
    }
}

/// list conditions that match an optional _pattern_ string.
/// the default pattern, if not supplied is "*" which match all gates.
/// The resulting Json has a status field, which is *OK* on success
/// and an error string on faiure, and a detail string which is an
///  array of structs that contain the following fields:
///
/// *   name - name of a condition.
/// *   type - Condition type in SpecTcl notation e.g. a Rustogramer *BAND*
/// has type *b*
/// *   gates - Possibly empty array of dependent gate names.  This will only
/// be nonempty if the type string is one of _+_, _-_, or _*_
/// *   parameters - Possibly empty array of parameters that must be
/// present in order for the condition to be evaluated (does not include
/// parameters in dependent conditions).  This will only be nonempty for
/// types: _s_, _b_ or _c_ though see low, high below for _s_ conditions.
/// *   low - The low limit of a _s_ gate - this is just the x coordinate of
/// the first point in points.
/// *   high - the high limit of a _s_ gate - this is just the x coordinate
/// of the second point in points.
///
/// The simplistic manner in which each GateProperties struct is filled in
/// provides for the presence of data in fields where the SpecTcl REST
/// implementation might not provide the field e.g. low, high will
/// be filled in for _c_ and _b_ conditions.
///
#[get("/list?<pattern>")]
pub fn list_gates(pattern: Option<String>, state: &State<HistogramState>) -> Json<ListReply> {
    // figure out the pattern:

    let pat = if let Some(p) = pattern {
        p
    } else {
        String::from("*")
    };

    let api = ConditionMessageClient::new(&state.inner().state.lock().unwrap().1);
    let reply = match api.list_conditions(&pat) {
        ConditionReply::Listing(l) => {
            let mut r = ListReply {
                status: String::from("OK"),
                detail: Vec::<GateProperties>::new(),
            };
            for condition in l.iter() {
                let mut p = GateProperties {
                    name: condition.cond_name.clone(),
                    type_name: rg_condition_to_spctl(&condition.type_name),
                    gates: condition.gates.clone(),
                    parameters: Vec::<String>::new(),
                    points: Vec::<GatePoint>::new(),
                    low: 0.0,
                    high: 0.0,
                };
                // Marshall the parameters:

                p.parameters = marshall_parameter_names(&condition.parameters, state);
                marshall_points(&mut p, &condition.points);
                r.detail.push(p);
            }
            r
        }
        ConditionReply::Error(s) => ListReply {
            status: format!("Failed to list gates matching '{}' : {}", pat, s),
            detail: Vec::<GateProperties>::new(),
        },
        _ => ListReply {
            status: format!("Unexpeced return type from list_conditions"),
            detail: Vec::<GateProperties>::new(),
        },
    };
    Json(reply)
}

/// Delete a gate.
///
/// Requires the name of the gate as a query parameter.
///
/// * Successful response has status = "OK" and detail an empty string.
/// * Failure respons has status something like "Failed to delete conditions {}"
/// with the detail the actual messagse from the internal Histogram server.
///
#[get("/delete?<name>")]
pub fn delete_gate(name: String, state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = ConditionMessageClient::new(&state.inner().state.lock().unwrap().1);
    let response = match api.delete_condition(&name) {
        ConditionReply::Deleted => GenericResponse {
            status: String::from("OK"),
            detail: String::from(""),
        },
        ConditionReply::Error(s) => GenericResponse {
            status: format!("Failed to delete condition {}", name),
            detail: s,
        },
        _ => GenericResponse {
            status: format!("Failed to delete condition {}", name),
            detail: String::from("Invalid repsonse from server"),
        },
    };
    Json(response)
}
///
/// Create/edit a gate.  Note that creating a new gate and editing
/// an existing gate.  If we 'edit' a new gate the gate is created
/// and saved in the condition dictionary.  If we 'edit' an existing gate,
/// the condition replaces the old one and the server side
/// return value indicates this.
/// The required query parameters are:
///
/// *   name - the name of the gate to create/edit.
/// *   type - The SpecTcl type of the gate to create/edit.
///
/// The other parameters required depend on the gate type:
///
/// *  T, F gates require nothing else.
/// *  + - * gates require gate - a list of gates the gate depends on.
///These gates must already be defined.
/// *  c, b require:
///     -   xparameter, yparameter - the parameters the gate is set on.
///     -   xcoord, ycoord - the x/y coordinates of the points that make up the gate.
/// * s requires:
///     - parameter for the parameter the condition is set on.
///     - low - low limit of the slice.
///     - high - high limit of the slice.
/// Other gate types are not supported.
///
/// The response is a GenericResponse.  On success,
///
///  *  status - is _OK_
///  *  detail is one of _Created_ for a new condition or _Replaced_
/// if the condition previously existed.
///
/// In the event of a failure:
///
/// * status is a top level error e.g. _bad parameter_
/// * detail provides more information about the error e.g
///   _only one name allowed_ or _parameter {} does not exist_
///
#[get("/edit?<name>&<type>&<gate>&<xparameter>&<yparameter>&<parameter>&<xcoord>&<ycoord>&<low>&<high>")]
pub fn edit_gate(
    name: String,
    r#type: String,
    gate: OptionalStringVec,
    xparameter: OptionalString,
    yparameter: OptionalString,
    parameter: OptionalString,
    xcoord: OptionalF64Vec,
    ycoord: OptionalF64Vec,
    low: Option<f64>,
    high: Option<f64>,
    state: &State<HistogramState>,
) -> Json<GenericResponse> {
    let api = ConditionMessageClient::new(
        &state.inner().state.lock().unwrap().1
    );

    let raw_result = match r#type.as_str() {
        "T" => api.create_true_condition(&name),
        "F" => api.create_false_condition(&name),
        "-" => {
            // There must be exactly one gate:

            if gate.is_some() {
                let gate = gate.unwrap();
                if gate.len()  == 1 {
                    api.create_not_condition(&name, &gate[0])
                } else {
                    ConditionReply::Error(String::from("Not gates can have at most one dependent gate"))
                }
            } else {
                ConditionReply::Error(String::from("gate is a required query parameter for not gatess"))
            }
        }
        _ => ConditionReply::Error(format!("Unsupported gate type: {}", r#type)),
    };

    let reply = match raw_result {
        ConditionReply::Created => GenericResponse {
            status: String::from("OK"),
            detail: String::from("Created"),
        },
        ConditionReply::Replaced => GenericResponse {
            status: String::from("OK"),
            detail: String::from("Replaced"),
        },
        ConditionReply::Error(s) => GenericResponse {
            status: format!("Could not create/edit gate {}", name),
            detail: s,
        },
        _ => GenericResponse {
            status: format!("Could not create/edit gate {}", name),
            detail: String::from("Unexpected respones type from server"),
        },
    };
    Json(reply)
}
