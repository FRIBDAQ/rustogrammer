//! This module implements Rocket handlers for
//! the the /spectcl/gate domain of URLS.
//! The name follows the SpecTcl name for conditions while
//! Rustogrammer knows that there are conditions which are
//! just objects that can be evaluated, as needed for each event
//! which return a true or false value.  
//!
//! A condition can gate (verb) a spectrum to determine which
//! events are allowed to increment it.
//!
//! A nasty concern is that the condition type names supported
//! by Rustogramer have more useful names like True, And, Cut
//! where those in SpecTcl (and therefore the type-names expected
//! by REST clients) have simpler names like T, F, s, * (slice).
//! it is therefore necessary to map from Rustogramer
//! condition types to SpecTcl gate types in this domain of URLs.

use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;

use super::*;

use crate::messaging::condition_messages::{ConditionMessageClient, ConditionReply};

// Private mappings between SpecTcl <-> Rustogramer condition types:
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
        "MultiCut" => String::from("gs"),
        "MultiContour" => String::from("gc"),
        _ => String::from("-unsupported-"),
    }
}
//----------------------------------------------------------------
// Stuff to handle listing conditions(gates):

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct GatePoint {
    x: f64,
    y: f64,
}

#[derive(Serialize, Deserialize, Clone)]
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
    // value : u32            // Note Rustogrammer has no support for mask conditions.
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct ListReply {
    status: String,
    detail: Vec<GateProperties>,
}

// Private utility to turn a vector of parameter ids to
// a vector of parameter string names.
// The ids should all be valid.

fn marshall_parameter_names(ids: &Vec<u32>, state: &State<SharedHistogramChannel>) -> Vec<String> {
    let mut result = Vec::<String>::new();
    for id in ids {
        result.push(
            find_parameter_by_id(*id, state)
                .unwrap_or_else(|| panic!("BUG Failed to find gate parameter {} by id", id)),
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
/// the default pattern, if not supplied is "*" which match all conditions.
/// The resulting Json has a status field, which is *OK* on success
/// and an error string on faiure, and a detail string which is an
///  array of structs that contain the following fields:
///
/// *   name - name of a condition.
/// *   type - Condition type in SpecTcl notation e.g. a Rustogramer *BAND*
/// has type *b*
/// *   gates - Possibly empty array of dependent condition names.  This will only
/// be nonempty if the type string is one of _+_, _-_, or _*_
/// *   parameters - Possibly empty array of parameters that must be
/// present in order for the condition to be evaluated (does not include
/// parameters in dependent conditions).  This will only be nonempty for
/// types: _s_, _b_ or _c_ though see low, high below for _s_ conditions.
/// *   low - The low limit of a _s_ condition - this is just the x coordinate of
/// the first point in points.
/// *   high - the high limit of a _s_ condition - this is just the x coordinate
/// of the second point in points.
/// *   points for 2-d conditions an array of {x,y} objects.
///
/// The simplistic manner in which each GateProperties struct is filled in
/// provides for the presence of data in fields where the SpecTcl REST
/// implementation might not provide the field e.g. low, high will
/// be filled in for _c_ and _b_ conditions.
///
#[get("/list?<pattern>")]
pub fn list_gates(
    pattern: Option<String>,
    state: &State<SharedHistogramChannel>,
) -> Json<ListReply> {
    // figure out the pattern:

    let pat = if let Some(p) = pattern {
        p
    } else {
        String::from("*")
    };

    let api = ConditionMessageClient::new(&state.inner().lock().unwrap());
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
            status: format!("Failed to list conditions matching '{}' : {}", pat, s),
            detail: Vec::<GateProperties>::new(),
        },
        _ => ListReply {
            status: String::from("Unexpeced return type from list_conditions"),
            detail: Vec::<GateProperties>::new(),
        },
    };
    Json(reply)
}
//--------------------------------------------------------------------
// Delete condition

/// Delete a condition.
///
/// Requires the name of the condition as a query parameter.
///
/// * Successful response has status = "OK" and detail an empty string.
/// * Failure respons has status something like "Failed to delete conditions {}"
/// with the detail the actual messagse from the internal Histogram server.
///
#[get("/delete?<name>")]
pub fn delete_gate(name: String, state: &State<SharedHistogramChannel>) -> Json<GenericResponse> {
    let api = ConditionMessageClient::new(&state.inner().lock().unwrap());
    let response = match api.delete_condition(&name) {
        ConditionReply::Deleted => GenericResponse::ok(""),
        ConditionReply::Error(s) => {
            GenericResponse::err(format!("Failed to delete condition {}", name).as_str(), &s)
        }
        _ => GenericResponse::err(
            &format!("Failed to delete condition {}", name),
            "Invalid response from server",
        ),
    };
    Json(response)
}
//--------------------------------------------------------------
// Edit/create conditions:

// Validate the query parameters needed to make a slice condition and extract them
//
fn validate_slice_parameters(
    parameter: OptionalStringVec,
    low: Option<f64>,
    high: Option<f64>,
    state: &State<SharedHistogramChannel>,
) -> Result<(u32, f64, f64), String> {
    if parameter.is_none() {
        return Err(String::from(
            "The parameter query parameter is required for slice conditions",
        ));
    }
    let parameter = parameter.unwrap();
    if parameter.len() != 1 {
        return Err(String::from("Slices must have exactly one parameter"));
    }
    let parameter_name = &parameter[0];
    if low.is_none() || high.is_none() {
        return Err(String::from(
            "Both the low and high query parameters are requried for slice conditions",
        ));
    }
    let low = low.unwrap();
    let high = high.unwrap();
    let pid = find_parameter_by_name(parameter_name, state);
    if pid.is_none() {
        return Err(format!("Parameter {} does not exist", parameter_name));
    }

    Ok((pid.unwrap(), low, high))
}

type TwodParameters = (u32, u32, Vec<(f64, f64)>);

fn validate_2d_parameters(
    xpname: OptionalString,
    ypname: OptionalString,
    xcoord: OptionalF64Vec,
    ycoord: OptionalF64Vec,
    state: &State<SharedHistogramChannel>,
) -> Result<TwodParameters, String> {
    if xpname.is_none() {
        return Err(String::from(
            "xparameter is a mandatory query parameter for this condition type",
        ));
    }
    if ypname.is_none() {
        return Err(String::from(
            "yparameter is a mandatory query parameter for this condition type",
        ));
    }
    if xcoord.is_none() {
        return Err(String::from(
            "xcoord is a mandatory query parameter for this condition type",
        ));
    }
    if ycoord.is_none() {
        return Err(String::from(
            "ycoord is a mandatory query parameter for this condition type",
        ));
    }
    // unwrap the parametesr from their options:

    let xpname = xpname.unwrap();
    let ypname = ypname.unwrap();
    let xcoord = xcoord.unwrap();
    let ycoord = ycoord.unwrap();

    // The xcoord and y coords arrays must be the same length:

    if xcoord.len() != ycoord.len() {
        return Err(format!(
            "xcoord array has {} entries but ycoord array has {} -they must be the same length",
            xcoord.len(),
            ycoord.len()
        ));
    }
    // Translate xpname and ypname to parameter ids if possible:

    let xpid = find_parameter_by_name(&xpname, state);
    let ypid = find_parameter_by_name(&ypname, state);

    if xpid.is_none() {
        return Err(format!("Parameter {} does not exist", xpname));
    }
    if ypid.is_none() {
        return Err(format!("Parameter {} does not exist", ypname));
    }
    let xpid = xpid.unwrap();
    let ypid = ypid.unwrap();

    // Marshall the coordinats:

    let mut points = Vec::<(f64, f64)>::new();
    for (i, x) in xcoord.iter().enumerate() {
        points.push((*x, ycoord[i]));
    }
    Ok((xpid, ypid, points))
}

// Validate the parameters for  multi slice:
// - There must be a parameter array.
// -  There must be a low, and a high.
// - The parameters must be converted into ids.
fn validate_multi1_parameters(
    parameter: OptionalStringVec,
    low: Option<f64>,
    high: Option<f64>,
    state: &State<SharedHistogramChannel>,
) -> Result<(Vec<u32>, f64, f64), String> {
    if low.is_none() || high.is_none() {
        return Err(String::from(
            "Both low and high must be present to make a multi  slice (gs)",
        ));
    }
    if parameter.is_none() {
        return Err(String::from("Multi 1d (gs) conditions require parameters"));
    }
    let mut ids = Vec::<u32>::new();

    for name in parameter.unwrap().iter() {
        if let Some(id) = find_parameter_by_name(name, state) {
            ids.push(id);
        } else {
            return Err(format!("Parameter: {} does not exist", name));
        }
    }
    Ok((ids, low.unwrap(), high.unwrap()))
}
// Validate the parameters for a multi parameter contour:

type ParameterIdAndCoords = (Vec<u32>, Vec<(f64, f64)>);

fn validate_multi2_parameters(
    parameter: OptionalStringVec,
    xcoords: OptionalF64Vec,
    ycoords: OptionalF64Vec,
    state: &State<SharedHistogramChannel>,
) -> Result<ParameterIdAndCoords, String> {
    // THere must be parameers, x and y coordinates:

    if parameter.is_none() {
        return Err(String::from(
            "Parameters are required for a multi parameter contour",
        ));
    }
    let parameter = parameter.unwrap();

    if xcoords.is_none() {
        return Err(String::from(
            "xcoords are required for multi parameter contours",
        ));
    }
    if ycoords.is_none() {
        return Err(String::from(
            "ycoords are required for multi parameters contous",
        ));
    }
    let x = xcoords.unwrap();
    let y = ycoords.unwrap();
    if x.len() != y.len() {
        return Err(String::from(
            "There must be the same number of x and y coordinates.",
        ));
    }
    // Marshall the points:

    let mut pts = vec![];
    for (i, x) in x.iter().enumerate() {
        pts.push((*x, y[i]));
    }
    // Marshall the parameters -> ids:

    let mut ids = vec![];
    for name in parameter.iter() {
        if let Some(id) = find_parameter_by_name(name, state) {
            ids.push(id);
        } else {
            return Err(format!("Parameter: {} does not exist", name));
        }
    }

    Ok((ids, pts))
}
///
/// Create/edit a condition.  Note that creating a new condition and editing
/// an existing condition are the same.  If we 'edit' a new condition the condition is created
/// and saved in the condition dictionary.  If we 'edit' an existing condition,
/// the condition replaces the old one and the server side
/// return value indicates this.
/// The required query parameters are:
///
/// *   name - the name of the condition to create/edit.
/// *   type - The SpecTcl type of the condition to create/edit.
///
/// The other parameters required depend on the condition type:
///
/// *  T, F conditions require nothing else.
/// *  + - * conditions require condition - a list of conditions the condition depends on.
///These conditions must already be defined.
/// *  c, b require:
///     -   xparameter, yparameter - the parameters the condition is set on.
///     -   xcoord, ycoord - the x/y coordinates of the points that make up the condition.
/// * s requires:
///     - parameter for the parameter the condition is set on.
///     - low - low limit of the slice.
///     - high - high limit of the slice.
/// Other condition types are not supported.
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
    parameter: OptionalStringVec,
    xcoord: OptionalF64Vec,
    ycoord: OptionalF64Vec,
    low: Option<f64>,
    high: Option<f64>,
    state: &State<SharedHistogramChannel>,
) -> Json<GenericResponse> {
    let api = ConditionMessageClient::new(&state.inner().lock().unwrap());

    let raw_result = match r#type.as_str() {
        "T" => api.create_true_condition(&name),
        "F" => api.create_false_condition(&name),
        "-" => {
            // There must be exactly one condition:

            if let Some(gate) = gate {
                if gate.len() == 1 {
                    api.create_not_condition(&name, &gate[0])
                } else {
                    ConditionReply::Error(String::from(
                        "Not conditions can have at most one dependent condition",
                    ))
                }
            } else {
                ConditionReply::Error(String::from(
                    "gate is a required query parameter for not conditions",
                ))
            }
        }
        "*" => {
            // There must be at least one condition:

            if let Some(gate) = gate {
                if !gate.is_empty() {
                    api.create_and_condition(&name, &gate)
                } else {
                    ConditionReply::Error(String::from(
                        "And conditions require at least one dependent condition",
                    ))
                }
            } else {
                ConditionReply::Error(String::from(
                    "And conditions require the 'gate' query parameters",
                ))
            }
        }
        "+" => {
            // There must be at least one condition:

            if let Some(condition) = gate {
                if !condition.is_empty() {
                    api.create_or_condition(&name, &condition)
                } else {
                    ConditionReply::Error(String::from(
                        "Or conditions require at least one dependent condition",
                    ))
                }
            } else {
                ConditionReply::Error(String::from(
                    "Or conditions require the 'gate' query parameters",
                ))
            }
        }
        "s" => {
            // There must be one parameter, low and high.

            match validate_slice_parameters(parameter, low, high, state) {
                Ok((pid, low, high)) => api.create_cut_condition(&name, pid, low, high),
                Err(s) => ConditionReply::Error(s),
            }
        }
        "b" => match validate_2d_parameters(xparameter, yparameter, xcoord, ycoord, state) {
            Err(s) => ConditionReply::Error(s),
            Ok((xid, yid, points)) => api.create_band_condition(&name, xid, yid, &points),
        },
        "c" => match validate_2d_parameters(xparameter, yparameter, xcoord, ycoord, state) {
            Err(s) => ConditionReply::Error(s),
            Ok((xid, yid, points)) => api.create_contour_condition(&name, xid, yid, &points),
        },
        "gs" => match validate_multi1_parameters(parameter, low, high, state) {
            Err(s) => ConditionReply::Error(s),
            Ok((ids, low, high)) => api.create_multicut_condition(&name, &ids, low, high),
        },
        "gc" => match validate_multi2_parameters(parameter, xcoord, ycoord, state) {
            Err(s) => ConditionReply::Error(s),
            Ok((ids, points)) => api.create_multicontour_condition(&name, &ids, &points),
        },
        _ => ConditionReply::Error(format!("Unsupported condition type: {}", r#type)),
    };

    let reply = match raw_result {
        ConditionReply::Created => GenericResponse::ok("Created"),
        ConditionReply::Replaced => GenericResponse::ok("Replaced"),
        ConditionReply::Error(s) => {
            GenericResponse::err(&format!("Could not create/edit condition {}", name), &s)
        }
        _ => GenericResponse::err(
            &format!("Could not create/edit condition {}", name),
            "Unexpected respones type from server",
        ),
    };
    Json(reply)
}

#[cfg(test)]
mod gate_tests {
    use super::*;
    use crate::messaging;
    use crate::messaging::{condition_messages, parameter_messages};
    use crate::processing;
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;
    // note these are all unimplemented URLS so...

    fn setup() -> Rocket<Build> {
        rest_common::setup().mount("/", routes![list_gates, delete_gate, edit_gate])
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
    // Create parameters p1, p2
    // which will be used to create conditions that need parameters.
    //
    fn make_test_objects(c: &mpsc::Sender<messaging::Request>) {
        let api = parameter_messages::ParameterMessageClient::new(c);
        api.create_parameter("p1").expect("Creating p1");
        api.create_parameter("p2").expect("Creating p2");
        api.create_parameter("p3").expect("creating p3");
    }

    #[test]
    fn list_1() {
        // empty list:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/list");

        let reply = req
            .dispatch()
            .into_json::<ListReply>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status.as_str());
        assert_eq!(0, reply.detail.len());

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_2() {
        // Make a T condition and make sure the right properties are present.

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_true_condition("TRUE");

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/list");
        let reply = req
            .dispatch()
            .into_json::<ListReply>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status.as_str());
        assert_eq!(1, reply.detail.len());

        let l = &reply.detail[0];
        assert_eq!("TRUE", l.name.as_str());
        assert_eq!("T", l.type_name);
        assert_eq!(0, l.gates.len());
        assert_eq!(0, l.points.len());

        // low/high are unimportant.

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_3() {
        // Make an F condition ...

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_false_condition("FALSE");

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/list");
        let reply = req
            .dispatch()
            .into_json::<ListReply>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status.as_str());
        assert_eq!(1, reply.detail.len());

        let l = &reply.detail[0];
        assert_eq!("FALSE", l.name);
        assert_eq!("F", l.type_name);
        assert_eq!(0, l.gates.len());
        assert_eq!(0, l.points.len());

        // low/high are unimportant.

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_4() {
        // Not condition:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_false_condition("FALSE");
        api.create_not_condition("NOT", "FALSE");

        // Note this will, to some extent, test filtering too:

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/list?pattern=NOT");
        let reply = req
            .dispatch()
            .into_json::<ListReply>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status.as_str());
        assert_eq!(1, reply.detail.len());

        let l = &reply.detail[0];
        assert_eq!("NOT", l.name.as_str());
        assert_eq!("-", l.type_name);
        assert_eq!(1, l.gates.len());
        assert_eq!("FALSE", l.gates[0]);
        assert_eq!(0, l.points.len());

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_5() {
        // and condtion:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_false_condition("FALSE");
        api.create_true_condition("TRUE");
        api.create_and_condition("AND", &[String::from("FALSE"), String::from("TRUE")]);

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/list?pattern=AND");
        let reply = req
            .dispatch()
            .into_json::<ListReply>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status.as_str());
        assert_eq!(1, reply.detail.len());

        let l = &reply.detail[0];
        assert_eq!("AND", l.name.as_str());
        assert_eq!("*", l.type_name);
        assert_eq!(2, l.gates.len());
        assert_eq!("FALSE", l.gates[0]);
        assert_eq!("TRUE", l.gates[1]);
        assert_eq!(0, l.points.len());

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_6() {
        // list or condition:
        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_false_condition("FALSE");
        api.create_true_condition("TRUE");
        api.create_or_condition("OR", &[String::from("FALSE"), String::from("TRUE")]);

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/list?pattern=OR");
        let reply = req
            .dispatch()
            .into_json::<ListReply>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status.as_str());
        assert_eq!(1, reply.detail.len());

        let l = &reply.detail[0];
        assert_eq!("OR", l.name.as_str());
        assert_eq!("+", l.type_name);
        assert_eq!(2, l.gates.len());
        assert_eq!("FALSE", l.gates[0]);
        assert_eq!("TRUE", l.gates[1]);
        assert_eq!(0, l.points.len());

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_7() {
        // Cut condition:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_cut_condition("cut", 1, 10.0, 20.0); //1 is p1 I think?

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/list");
        let reply = req
            .dispatch()
            .into_json::<ListReply>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status.as_str());
        assert_eq!(1, reply.detail.len());

        let l = &reply.detail[0];
        assert_eq!("cut", l.name);
        assert_eq!("s", l.type_name);
        assert_eq!(0, l.gates.len());
        assert_eq!(1, l.parameters.len());
        assert_eq!(2, l.points.len());
        assert_eq!(10.0, l.points[0].x);
        assert_eq!(20.0, l.points[1].x);
        assert_eq!("p1", l.parameters[0]);
        assert_eq!(10.0, l.low);
        assert_eq!(20.0, l.high);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_8() {
        // Band condition:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_band_condition("band", 1, 2, &[(10.0, 10.0), (15.0, 20.0), (100.0, 15.0)]);

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/list");
        let reply = req
            .dispatch()
            .into_json::<ListReply>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status.as_str());
        assert_eq!(1, reply.detail.len());
        let l = &reply.detail[0];

        assert_eq!("band", l.name);
        assert_eq!("b", l.type_name);
        assert_eq!(0, l.gates.len());
        assert_eq!(2, l.parameters.len());
        assert_eq!("p1", l.parameters[0]);
        assert_eq!("p2", l.parameters[1]);
        assert_eq!(3, l.points.len());
        assert_eq!(GatePoint { x: 10.0, y: 10.0 }, l.points[0]);
        assert_eq!(GatePoint { x: 15.0, y: 20.0 }, l.points[1]);
        assert_eq!(GatePoint { x: 100.0, y: 15.0 }, l.points[2]);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_9() {
        // contour conditions:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_contour_condition(
            "contour",
            1,
            2,
            &[(10.0, 10.0), (15.0, 20.0), (100.0, 15.0)],
        );

        let client = Client::tracked(rocket).expect("making client");
        let req = client.get("/list");
        let reply = req
            .dispatch()
            .into_json::<ListReply>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status.as_str());
        assert_eq!(1, reply.detail.len());
        let l = &reply.detail[0];

        assert_eq!("contour", l.name);
        assert_eq!("c", l.type_name);
        assert_eq!(0, l.gates.len());
        assert_eq!(2, l.parameters.len());
        assert_eq!("p1", l.parameters[0]);
        assert_eq!("p2", l.parameters[1]);
        assert_eq!(3, l.points.len());
        assert_eq!(GatePoint { x: 10.0, y: 10.0 }, l.points[0]);
        assert_eq!(GatePoint { x: 15.0, y: 20.0 }, l.points[1]);
        assert_eq!(GatePoint { x: 100.0, y: 15.0 }, l.points[2]);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_10() {
        // Make a multislice condition and see that it is listed:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_multicut_condition("test", &[1, 2, 3], 100.0, 200.0);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/list");
        let reply = req
            .dispatch()
            .into_json::<ListReply>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(1, reply.detail.len());

        let l = &reply.detail[0];
        assert_eq!("test", l.name);
        assert_eq!("gs", l.type_name);
        assert!(l.gates.is_empty());
        assert_eq!(
            vec![String::from("p1"), String::from("p2"), String::from("p3")],
            l.parameters
        );
        assert_eq!(
            vec![
                GatePoint { x: 100.0, y: 0.0 },
                GatePoint { x: 200.0, y: 0.0 }
            ],
            l.points
        );
        assert_eq!(100.0, l.low);
        assert_eq!(200.0, l.high);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn list_11() {
        // List a gc (MultiContour) condition:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_multicontour_condition(
            "test",
            &[1, 2, 3],
            &[(100.0, 50.0), (200.0, 50.0), (150.0, 75.0)],
        );

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/list");
        let reply = req
            .dispatch()
            .into_json::<ListReply>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(1, reply.detail.len());

        let l = &reply.detail[0];
        assert_eq!("test", l.name);
        assert_eq!("gc", l.type_name);
        assert!(l.gates.is_empty());
        assert_eq!(
            vec![String::from("p1"), String::from("p2"), String::from("p3")],
            l.parameters
        );
        assert_eq!(
            vec![
                GatePoint { x: 100.0, y: 50.0 },
                GatePoint { x: 200.0, y: 50.0 },
                GatePoint { x: 150.0, y: 75.0 }
            ],
            l.points
        );

        teardown(c, &papi, &bapi);
    }
    // condition deletion:

    #[test]
    fn delete_1() {
        // Delete a nonexistent condition:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/delete?name=george");
        let response = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing json");

        assert_eq!("Failed to delete condition george", response.status);
        assert_eq!("No such condition george", response.detail);
        teardown(c, &papi, &bapi);
    }
    #[test]
    fn delete_2() {
        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        // Make a condition to delete:

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_true_condition("george");

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/delete?name=george");
        let response = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing json");
        assert_eq!("OK", response.status);
        assert_eq!("", response.detail);

        teardown(c, &papi, &bapi);
    }

    // Note that edit is used to both create and modify conditions.
    // Except for the last test we'll be creating conditions.
    // The final edit_n test will modify an existing condition.

    #[test]
    fn edit_1() {
        // Create True condition:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=true&type=T");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!("Created", reply.detail);

        // ah but was it really created:

        let api = condition_messages::ConditionMessageClient::new(&c);
        let gates = api.list_conditions("*");
        assert!(if let ConditionReply::Listing(l) = gates {
            assert_eq!(1, l.len());
            let cond = &l[0];
            assert_eq!("true", cond.cond_name);
            assert_eq!("True", cond.type_name);
            true
        } else {
            false
        });

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_2() {
        // create a False condition:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=false&type=F");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!("Created", reply.detail);

        // ah but was it really created:

        let api = condition_messages::ConditionMessageClient::new(&c);
        let gates = api.list_conditions("*");

        assert!(if let ConditionReply::Listing(l) = gates {
            assert_eq!(1, l.len());
            let cond = &l[0];
            assert_eq!("false", cond.cond_name);
            assert_eq!("False", cond.type_name);
            true
        } else {
            false
        });

        teardown(c, &papi, &bapi);
    }
    // Test not conditions and error scenarios.  Note we assume that
    // dependent condition existence is checked by the tests in condition_messages.

    #[test]
    fn edit_3() {
        // make  a not condition:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_true_condition("true"); // dependent condition:

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=not&type=-&gate=true");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!("Created", reply.detail);

        // Check the 'not' gate:

        let gates = api.list_conditions("not");

        assert!(if let ConditionReply::Listing(l) = gates {
            assert_eq!(1, l.len());
            let cond = &l[0];
            assert_eq!("not", cond.cond_name);
            assert_eq!("Not", cond.type_name);
            assert_eq!(1, cond.gates.len());
            assert_eq!("true", cond.gates[0]);
            true
        } else {
            false
        });

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_4() {
        // fail creation of not condition -- need a dependent condition:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=not&type=-");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit condition not", reply.status);
        assert_eq!(
            "gate is a required query parameter for not conditions",
            reply.detail
        );

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_5() {
        // Fail creation of not condition - must have only 1 dependent condition:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=not&type=-&gate=g1&gate=g2");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit condition not", reply.status);
        assert_eq!(
            "Not conditions can have at most one dependent condition",
            reply.detail
        );

        teardown(c, &papi, &bapi);
    }
    // Test and conditions and error scenarios.

    #[test]
    fn edit_6() {
        // Good creation.

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        // Make dependent conditions:

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_true_condition("true"); // dependent condition:
        api.create_false_condition("false");

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=and&type=*&gate=true&gate=false");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!("Created", reply.detail);

        let listing = api.list_conditions("and");
        assert!(if let ConditionReply::Listing(l) = listing {
            assert_eq!(1, l.len());
            let cond = &l[0];
            assert_eq!("and", cond.cond_name);
            assert_eq!("And", cond.type_name);
            assert_eq!(2, cond.gates.len());
            assert_eq!("true", cond.gates[0]);
            assert_eq!("false", cond.gates[1]);
            true
        } else {
            false
        });

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_7() {
        // no dependent conditions provided.

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=and&type=*");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit condition and", reply.status);
        assert_eq!(
            "And conditions require the 'gate' query parameters",
            reply.detail
        );

        teardown(c, &papi, &bapi);
    }
    // Tests for Or conditions. Note the literal + is a stand-in for
    // ' ' so we need to use the escap %2B instead.

    #[test]
    fn edit_8() {
        // Good creation

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        // Make dependent conditions:

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_true_condition("true"); // dependent condition:
        api.create_false_condition("false");

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=or&type=%2B&gate=true&gate=false");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!("Created", reply.detail);

        let listing = api.list_conditions("or");
        assert!(if let ConditionReply::Listing(l) = listing {
            assert_eq!(1, l.len());
            let cond = &l[0];
            assert_eq!("or", cond.cond_name);
            assert_eq!("Or", cond.type_name);
            assert_eq!(2, cond.gates.len());
            assert_eq!("true", cond.gates[0]);
            assert_eq!("false", cond.gates[1]);
            true
        } else {
            false
        });

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_9() {
        // failed for missing dependent gates;

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=or&type=%2B");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit condition or", reply.status);
        assert_eq!(
            "Or conditions require the 'gate' query parameters",
            reply.detail
        );

        teardown(c, &papi, &bapi);
    }
    // Slice condition tests:

    #[test]
    fn edit_10() {
        // Test success.

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=slice&type=s&parameter=p1&low=10&high=100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!("Created", reply.detail);

        // check the condition:

        let api = condition_messages::ConditionMessageClient::new(&c);
        let listing = api.list_conditions("*");
        assert!(if let ConditionReply::Listing(l) = listing {
            assert_eq!(1, l.len());
            let cond = &l[0];
            assert_eq!("slice", cond.cond_name);
            assert_eq!("Cut", cond.type_name);
            assert_eq!(2, cond.points.len());
            assert_eq!(10.0, cond.points[0].0);
            assert_eq!(100.0, cond.points[1].0);
            assert_eq!(1, cond.parameters.len());
            assert_eq!(1, cond.parameters[0]); // p1 parameter
            true
        } else {
            false
        });
        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_11() {
        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=slice&type=s&low=10&high=100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit condition slice", reply.status);
        assert_eq!(
            "The parameter query parameter is required for slice conditions",
            reply.detail
        );
        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_12() {
        // missing low:

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=slice&type=s&parameter=p1&high=100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit condition slice", reply.status);
        assert_eq!(
            "Both the low and high query parameters are requried for slice conditions",
            reply.detail
        );
        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_13() {
        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=slice&type=s&parameter=p1&low=100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit condition slice", reply.status);
        assert_eq!(
            "Both the low and high query parameters are requried for slice conditions",
            reply.detail
        );
        teardown(c, &papi, &bapi);
    }
    // Tests for making new Bands.

    #[test]
    fn edit_14() {
        // good creation.
        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let client = Client::tracked(rocket).expect("Making client.");
        let request = client.get("/edit?name=band&type=b&xparameter=p1&yparameter=p2&xcoord=100&ycoord=50&xcoord=200&ycoord=60");
        let reply = request
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing json");

        assert_eq!("OK", reply.status);

        // Check the condition was proprly made:

        let api = condition_messages::ConditionMessageClient::new(&c);
        let l = api.list_conditions("*");
        assert!(if let ConditionReply::Listing(gates) = l {
            assert_eq!(1, gates.len());
            let gate = &gates[0];
            assert_eq!("band", gate.cond_name);
            assert_eq!("Band", gate.type_name);
            assert_eq!(2, gate.points.len());
            assert_eq!(100.0, gate.points[0].0);
            assert_eq!(50.0, gate.points[0].1);
            assert_eq!(200.0, gate.points[1].0);
            assert_eq!(60.0, gate.points[1].1);
            true
        } else {
            false
        });

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_15() {
        // missing x parameter

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let client = Client::tracked(rocket).expect("Making client.");
        let request = client
            .get("/edit?name=band&type=b&yparameter=p2&xcoord=100&ycoord=50&xcoord=200&ycoord=60");
        let reply = request
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing json");

        assert_eq!("Could not create/edit condition band", reply.status);
        assert_eq!(
            "xparameter is a mandatory query parameter for this condition type",
            reply.detail
        );

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_16() {
        // missing y parameter.
        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let client = Client::tracked(rocket).expect("Making client.");
        let request = client
            .get("/edit?name=band&type=b&xparameter=p1&xcoord=100&ycoord=50&xcoord=200&ycoord=60");
        let reply = request
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing json");

        assert_eq!("Could not create/edit condition band", reply.status);
        assert_eq!(
            "yparameter is a mandatory query parameter for this condition type",
            reply.detail
        );

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_17() {
        // xcoords

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let client = Client::tracked(rocket).expect("Making client.");
        let request =
            client.get("/edit?name=band&type=b&xparameter=p1&yparameter=p2&ycoord=50&ycoord=60");
        let reply = request
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing json");

        assert_eq!("Could not create/edit condition band", reply.status);
        assert_eq!(
            "xcoord is a mandatory query parameter for this condition type",
            reply.detail
        );

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_18() {
        // No ycoords

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let client = Client::tracked(rocket).expect("Making client.");
        let request =
            client.get("/edit?name=band&type=b&xparameter=p1&yparameter=p2&xcoord=100&xcoord=200");
        let reply = request
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing json");

        assert_eq!("Could not create/edit condition band", reply.status);
        assert_eq!(
            "ycoord is a mandatory query parameter for this condition type",
            reply.detail
        );

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_19() {
        // differing lengths of xcoord/ycoords.

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let client = Client::tracked(rocket).expect("Making client.");
        let request = client.get(
            "/edit?name=band&type=b&xparameter=p1&yparameter=p2&xcoord=100&ycoord=50&xcoord=200",
        );
        let reply = request
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing json");

        assert_eq!("Could not create/edit condition band", reply.status);
        assert_eq!(
            "xcoord array has 2 entries but ycoord array has 1 -they must be the same length",
            reply.detail
        );

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_20() {
        // only one point.

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let client = Client::tracked(rocket).expect("Making client.");
        let request =
            client.get("/edit?name=band&type=b&xparameter=p1&yparameter=p2&xcoord=100&ycoord=50");
        let reply = request
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing json");

        assert_eq!("Could not create/edit condition band", reply.status);

        teardown(c, &papi, &bapi);
    }
    // Tests for contours.
    // A bit of white box-ness.  The same parameter validation is
    // done for contours as bands so we can reduce the number of
    // tests dramatically:

    #[test]
    fn edit_21() {
        // Good contour creation.

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let client = Client::tracked(rocket).expect("Making client");
        let request = client.get("/edit?name=contour&type=c&xparameter=p1&yparameter=p2&xcoord=100&ycoord=50&xcoord=200&ycoord=60&xcoord=100&ycoord=100");
        let reply = request
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing json");

        assert_eq!("OK", reply.status);

        // Check the condition was made:

        let api = condition_messages::ConditionMessageClient::new(&c);
        let l = api.list_conditions("*");
        assert!(if let ConditionReply::Listing(gates) = l {
            assert_eq!(1, gates.len());
            let g = &gates[0];
            assert_eq!("contour", g.cond_name);
            assert_eq!("Contour", g.type_name);
            assert_eq!(3, g.points.len());
            assert_eq!(100.0, g.points[0].0);
            assert_eq!(50.0, g.points[0].1);
            assert_eq!(200.0, g.points[1].0);
            assert_eq!(60.0, g.points[1].1);
            assert_eq!(100.0, g.points[2].0);
            assert_eq!(100.0, g.points[2].1);
            true
        } else {
            false
        });

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_22() {
        // Not enough points for a contour.

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let client = Client::tracked(rocket).expect("Making client");
        let request = client.get("/edit?name=contour&type=c&xparameter=p1&yparameter=p2&xcoord=100&ycoord=50&xcoord=200&ycoord=60");
        let reply = request
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing json");

        assert_eq!("Could not create/edit condition contour", reply.status);

        teardown(c, &papi, &bapi);
    }
    // Edit can modify an existing condition:

    #[test]
    fn edit_23() {
        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        // Create a true condition:

        let api = condition_messages::ConditionMessageClient::new(&c);
        let cr = api.create_true_condition("existing");
        assert!(matches!(cr, ConditionReply::Created));

        let client = Client::tracked(rocket).expect("Creating client");
        let request = client.get("/edit?name=existing&type=F"); // flip to false condition.
        let response = request
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", response.status);
        assert_eq!("Replaced", response.detail);

        let l = api.list_conditions("*");
        assert!(if let ConditionReply::Listing(gates) = l {
            assert_eq!(1, gates.len());
            let g = &gates[0];
            assert_eq!("existing", g.cond_name);
            assert_eq!("False", g.type_name);
            true
        } else {
            false
        });
        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_24() {
        // Good createion of multislice.

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);
        make_test_objects(&c);

        let client = Client::untracked(rocket).expect("Creating rocket client");
        let req = client
            .get("/edit?name=test&type=gs&parameter=p1&parameter=p2&parameter=p3&low=100&high=200");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing Json");
        assert_eq!("OK", reply.status);
        assert_eq!("Created", reply.detail);

        let api = condition_messages::ConditionMessageClient::new(&c);
        let l = api.list_conditions("test");

        assert_eq!(
            condition_messages::ConditionReply::Listing(vec![
                condition_messages::ConditionProperties {
                    cond_name: String::from("test"),
                    type_name: String::from("MultiCut"),
                    points: vec![(100.0, 0.0), (200.0, 0.0)],
                    gates: vec![],
                    parameters: vec![1, 2, 3]
                },
            ]),
            l
        );

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_25() {
        // Missing parameters for multislice.

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::untracked(rocket).expect("Creating rocket client");
        let req = client.get("/edit?name=test&type=gs&low=100&high=200");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit condition test", reply.status);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_26() {
        // Bad parameter for multi slice

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::untracked(rocket).expect("Creating rocket client");
        let req = client.get(
            "/edit?name=test&type=gs&parameter=p1&parameter=p2&parameter=p333&low=100&high=200",
        );
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit condition test", reply.status);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_27() {
        // missing high

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::untracked(rocket).expect("Creating rocket client");
        let req =
            client.get("/edit?name=test&type=gs&parameter=p1&parameter=p2&parameter=p3&low=100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit condition test", reply.status);

        teardown(c, &papi, &bapi);
    }
    #[test]
    fn edit_28() {
        // missing low.

        let rocket = setup();
        let (c, papi, bapi) = get_state(&rocket);

        let client = Client::untracked(rocket).expect("Creating rocket client");
        let req =
            client.get("/edit?name=test&type=gs&parameter=p1&parameter=p2&parameter=p3&high=100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit condition test", reply.status);

        teardown(c, &papi, &bapi);
    }
}
