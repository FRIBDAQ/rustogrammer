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

use crate::messaging::condition_messages::ConditionMessageClient;
use crate::messaging::parameter_messages::ParameterMessageClient;

// Private mappings between SpecTcl <-> Rustogramer gate types:
// Note making a static hashmap is possible but requires unsafe to access.
// Making the hashmap each time is possible but slower
// so we'll just use if chains.
//
fn rg_condition_to_spctl(rg_type : &str) -> String {
    match rg_type {
        "True" => String::from("T"),
        "False" => String::from("F"),
        "And"  => String::from("*"),
        "Or"  => String::from("+"),
        "Not" => String::from("-"),
        "Band" => String::from("b"),
        "Contour" => String::from("c"),
        "Cut" => String::from("s"),
        _ => String::from("-unsupported-")
        
    }
}
fn  spc_gate_to_rg(spc_type : &str) -> String  {
    match spc_type {
        "T" => String::from("True"),
        "F" => String::from("False"),
        "*" => String::from("And"),
        "+" => String::from("Or"),
        "-" => String::from("Not"),
        "b" => String::from("Band"),
        "c" => String::from("Contour"),
        "s" => String::from("Slice"),
        _ => String::from("Unsupported")
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct GatePoint {
    x : f64,
    y : f64
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct GateProperties {
    name : String,
    #[serde(rename = "type")]
    type_name : String,
    gates : Vec<String>,     // Dependencies.
    parameters : Vec<String>,
    points : Vec<GatePoint>,
    low : f64,
    high : f64,
    // value : u32            // Note Rustogrammer has no support for mask gates.
}

