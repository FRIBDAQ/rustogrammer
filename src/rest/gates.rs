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

use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;

use super::*;

use crate::messaging::condition_messages::{ConditionMessageClient, ConditionReply};

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
//----------------------------------------------------------------
// Stuff to handle listing conditions(gates):

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
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
    // value : u32            // Note Rustogrammer has no support for mask gates.
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
/// *   points for 2-d gates an array of {x,y} objects.
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

    let api = ConditionMessageClient::new(&state.inner().histogramer.lock().unwrap());
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
//--------------------------------------------------------------------
// Delete condition

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
    let api = ConditionMessageClient::new(&state.inner().histogramer.lock().unwrap());
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

// Validate the query parameters needed to make a slice gate and extract them
//
fn validate_slice_parameters(
    parameter: OptionalString,
    low: Option<f64>,
    high: Option<f64>,
    state: &State<HistogramState>,
) -> Result<(u32, f64, f64), String> {
    if parameter.is_none() {
        return Err(String::from(
            "The parameter query parameter is required for slice gates",
        ));
    }
    if low.is_none() || high.is_none() {
        return Err(String::from(
            "Both the low and high query parameters are requried for slice gates",
        ));
    }
    let low = low.unwrap();
    let high = high.unwrap();
    let parameter_name = parameter.unwrap();
    let pid = find_parameter_by_name(&parameter_name, state);
    if pid.is_none() {
        return Err(format!("Parameter {} does not exist", parameter_name));
    }

    Ok((pid.unwrap(), low, high))
}

fn validate_2d_parameters(
    xpname: OptionalString,
    ypname: OptionalString,
    xcoord: OptionalF64Vec,
    ycoord: OptionalF64Vec,
    state: &State<HistogramState>,
) -> Result<(u32, u32, Vec<(f64, f64)>), String> {
    if xpname.is_none() {
        return Err(String::from(
            "xparameter is a mandatory query parameter for this gate type",
        ));
    }
    if ypname.is_none() {
        return Err(String::from(
            "yparameter is a mandatory query parameter for this gate type",
        ));
    }
    if xcoord.is_none() {
        return Err(String::from(
            "xcoord is a mandatory query parameter for this gate type",
        ));
    }
    if ycoord.is_none() {
        return Err(String::from(
            "ycoord is a mandatory query parameter for this gate type",
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
    let api = ConditionMessageClient::new(&state.inner().histogramer.lock().unwrap());

    let raw_result = match r#type.as_str() {
        "T" => api.create_true_condition(&name),
        "F" => api.create_false_condition(&name),
        "-" => {
            // There must be exactly one gate:

            if gate.is_some() {
                let gate = gate.unwrap();
                if gate.len() == 1 {
                    api.create_not_condition(&name, &gate[0])
                } else {
                    ConditionReply::Error(String::from(
                        "Not gates can have at most one dependent gate",
                    ))
                }
            } else {
                ConditionReply::Error(String::from(
                    "gate is a required query parameter for not gates",
                ))
            }
        }
        "*" => {
            // There must be at least one gate:

            if gate.is_some() {
                let gate = gate.unwrap();
                if gate.len() >= 1 {
                    api.create_and_condition(&name, &gate)
                } else {
                    ConditionReply::Error(String::from(
                        "And gates require at least one dependent gate",
                    ))
                }
            } else {
                ConditionReply::Error(String::from(
                    "And gates require the 'gate' query parameters",
                ))
            }
        }
        "+" => {
            // There must be at least one gate:

            if gate.is_some() {
                let gate = gate.unwrap();
                if gate.len() >= 1 {
                    api.create_or_condition(&name, &gate)
                } else {
                    ConditionReply::Error(String::from(
                        "Or gates require at least one dependent gate",
                    ))
                }
            } else {
                ConditionReply::Error(String::from("Or gates require the 'gate' query parameters"))
            }
        }
        "s" => {
            // There must be a parameter, low and high.

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
        _ => ConditionReply::Error(format!("Unsupported gate type: {}", r#type)),
    };

    let reply = match raw_result {
        ConditionReply::Created => GenericResponse::ok("Created"),
        ConditionReply::Replaced => GenericResponse::ok("Replaced"),
        ConditionReply::Error(s) => {
            GenericResponse::err(&format!("Could not create/edit gate {}", name), &s)
        }
        _ => GenericResponse::err(
            &format!("Could not create/edit gate {}", name),
            "Unexpected respones type from server",
        ),
    };
    Json(reply)
}

#[cfg(test)]
mod gate_tests {
    use super::*;
    use crate::histogramer;
    use crate::messaging;
    use crate::messaging::{condition_messages, parameter_messages};
    use crate::processing;
    use crate::sharedmem::binder;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;
    use std::sync::Mutex;
    // note these are all unimplemented URLS so...

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

        rocket::build()
            .manage(state)
            .mount("/", routes![list_gates, delete_gate, edit_gate])
    }
    fn teardown(c: mpsc::Sender<messaging::Request>, p: &processing::ProcessingApi) {
        histogramer::stop_server(&c);
        p.stop_thread().expect("Stopping processing thread");
    }
    fn get_state(
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
    // Create parameters p1, p2
    // which will be used to create gates that need parameters.
    //
    fn make_test_objects(c: &mpsc::Sender<messaging::Request>) {
        let api = parameter_messages::ParameterMessageClient::new(c);
        api.create_parameter("p1").expect("Creating p1");
        api.create_parameter("p2").expect("Creating p2");
    }

    #[test]
    fn list_1() {
        // empty list:

        let rocket = setup();
        let (c, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/list");

        let reply = req
            .dispatch()
            .into_json::<ListReply>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status.as_str());
        assert_eq!(0, reply.detail.len());

        teardown(c, &papi);
    }
    #[test]
    fn list_2() {
        // Make a T gate and make sure the right properties are present.

        let rocket = setup();
        let (c, papi) = get_state(&rocket);
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

        teardown(c, &papi);
    }
    #[test]
    fn list_3() {
        // Make an F gate ...

        let rocket = setup();
        let (c, papi) = get_state(&rocket);
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

        teardown(c, &papi);
    }
    #[test]
    fn list_4() {
        // Not condition:

        let rocket = setup();
        let (c, papi) = get_state(&rocket);
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

        teardown(c, &papi);
    }
    #[test]
    fn list_5() {
        // and condtion:

        let rocket = setup();
        let (c, papi) = get_state(&rocket);
        make_test_objects(&c);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_false_condition("FALSE");
        api.create_true_condition("TRUE");
        api.create_and_condition("AND", &vec![String::from("FALSE"), String::from("TRUE")]);

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

        teardown(c, &papi);
    }
    #[test]
    fn list_6() {
        // list or condition:
        let rocket = setup();
        let (c, papi) = get_state(&rocket);
        make_test_objects(&c);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_false_condition("FALSE");
        api.create_true_condition("TRUE");
        api.create_or_condition("OR", &vec![String::from("FALSE"), String::from("TRUE")]);

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

        teardown(c, &papi);
    }
    #[test]
    fn list_7() {
        // Cut condition:

        let rocket = setup();
        let (c, papi) = get_state(&rocket);
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

        teardown(c, &papi);
    }
    #[test]
    fn list_8() {
        // Band condition:

        let rocket = setup();
        let (c, papi) = get_state(&rocket);
        make_test_objects(&c);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_band_condition(
            "band",
            1,
            2,
            &vec![(10.0, 10.0), (15.0, 20.0), (100.0, 15.0)],
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

        teardown(c, &papi);
    }
    #[test]
    fn list_9() {
        // contour conditions:

        let rocket = setup();
        let (c, papi) = get_state(&rocket);
        make_test_objects(&c);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_contour_condition(
            "contour",
            1,
            2,
            &vec![(10.0, 10.0), (15.0, 20.0), (100.0, 15.0)],
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

        teardown(c, &papi);
    }
    // Gate deletion:

    #[test]
    fn delete_1() {
        // Delete a nonexistent gate:

        let rocket = setup();
        let (c, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/delete?name=george");
        let response = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing json");

        assert_eq!("Failed to delete condition george", response.status);
        assert_eq!("No such condition george", response.detail);
        teardown(c, &papi);
    }
    #[test]
    fn delete_2() {
        let rocket = setup();
        let (c, papi) = get_state(&rocket);

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

        teardown(c, &papi);
    }

    // Note that edit is used to both create and modify gates.
    // Except for the last test we'll be creating gates.
    // The final edit_n test will modify an existing gate.

    #[test]
    fn edit_1() {
        // Create True gate:

        let rocket = setup();
        let (c, papi) = get_state(&rocket);

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

        teardown(c, &papi);
    }
    #[test]
    fn edit_2() {
        // create a False gate:

        let rocket = setup();
        let (c, papi) = get_state(&rocket);

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

        teardown(c, &papi);
    }
    // Test not gates and error scenarios.  Note we assume that
    // dependent gate existence is checked by the tests in condition_messages.

    #[test]
    fn edit_3() {
        // make  a not gate:

        let rocket = setup();
        let (c, papi) = get_state(&rocket);

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_true_condition("true"); // dependent gate:

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

        teardown(c, &papi);
    }
    #[test]
    fn edit_4() {
        // fail creation of not gate -- need a dependent gate:

        let rocket = setup();
        let (c, papi) = get_state(&rocket);
        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=not&type=-");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit gate not", reply.status);
        assert_eq!(
            "gate is a required query parameter for not gates",
            reply.detail
        );

        teardown(c, &papi);
    }
    #[test]
    fn edit_5() {
        // Fail creation of not gate - must have only 1 dependent gate:

        let rocket = setup();
        let (c, papi) = get_state(&rocket);
        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=not&type=-&gate=g1&gate=g2");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit gate not", reply.status);
        assert_eq!(
            "Not gates can have at most one dependent gate",
            reply.detail
        );

        teardown(c, &papi);
    }
    // Test and gates and error scenarios.

    #[test]
    fn edit_6() {
        // Good creation.

        let rocket = setup();
        let (c, papi) = get_state(&rocket);

        // Make dependent gates:

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_true_condition("true"); // dependent gate:
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

        teardown(c, &papi);
    }
    #[test]
    fn edit_7() {
        // no dependent gates provided.

        let rocket = setup();
        let (c, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=and&type=*");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit gate and", reply.status);
        assert_eq!(
            "And gates require the 'gate' query parameters",
            reply.detail
        );

        teardown(c, &papi);
    }
    // Tests for Or gates. Note the literal + is a stand-in for
    // ' ' so we need to use the escap %2B instead.

    #[test]
    fn edit_8() {
        // Good creation

        let rocket = setup();
        let (c, papi) = get_state(&rocket);

        // Make dependent gates:

        let api = condition_messages::ConditionMessageClient::new(&c);
        api.create_true_condition("true"); // dependent gate:
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

        teardown(c, &papi);
    }
    #[test]
    fn edit_9() {
        // failed for missing dependent gates;

        let rocket = setup();
        let (c, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let req = client.get("/edit?name=or&type=%2B");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Could not create/edit gate or", reply.status);
        assert_eq!("Or gates require the 'gate' query parameters",
            
            reply.detail
        );

        teardown(c, &papi);
    }
}
