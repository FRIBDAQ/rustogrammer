//! Provides the message and reply structures that allow for manipulation
//! of a condition dictionary via messaging.
//! Note that for each new condition type it'll be necessary to add
//! a NewXXXX enum for the request message.
//!

use super::MessageType;
use super::Reply;
use super::Request;

use glob;
use std::sync::mpsc;

///
/// ConditionRequest Defines all of the requests that can be made of the
/// condition dictionary manager part of the histograming thread.
///
#[derive(Clone, Debug, PartialEq)]
pub enum ConditionRequest {
    CreateTrue(String),
    CreateFalse(String),
    CreateNot {
        name: String,
        dependent : String
    },
    CreateAnd {
        name : String,
        dependents : Vec<String>
    },
    CreateOr {
        name : String,
        dependents : Vec<String>
    },
    CreateCut {
        name : String,
        param_id : u32,
        low : f64,
        high : f64
    },
    CreateBand {
        name : String,
        x_id : u32,
        y_id : u32,
        points : Vec<(f64, f64)>
    },
    CreateContour {
        name : String,
        x_id : u32,
        y_id : u32,
        points : Vec<(f64, f64)>
    },
    DeleteCondition(String),
    List(String),
    GetProperties(String)
}
///
/// These are replies that can be sent from the condition manager
/// part of the histograming thread:
/// 
#[derive(Clone, Debug, PartialEq)]
pub enum ConditionReply {
    Error(String),
    Created,
    Replacead,
    Deleted,
    Properties {
        type_name : String,
        points : Vec<(f64, f64)>,
        gates : Vec<String>,
        parameters : Vec<u32>
    }
}
// Having learned our lessons from parameter_messages.rs our
// private helper messages wil make ConditionRequest objects not
// MessageType objects.. It will be up to the API to wrap those
// into Request objects:

fn make_true_creation(name : &str) -> ConditionRequest {
    ConditionRequest::CreateTrue(String::from(name))
}
fn make_false_creation(name : &str) -> ConditionRequest {
    ConditionRequest::CreateFalse(String::from(name))
}
fn make_not_creation(name : &str, dependent : &str) -> ConditionRequest {
    ConditionRequest::CreateNot {name : String::from(name), dependent: String::from(dependent)}
}
fn make_and_creation(name: &str, dependents: &Vec<String>) -> ConditionRequest {
    ConditionRequest::CreateAnd {name: String::from(name), dependents: dependents.clone()}
}
fn make_or_creation(name: &str, dependents: &Vec<String>) -> ConditionRequest {
    ConditionRequest::CreateOr {name: String::from(name), dependents: dependents.clone()}
}
fn make_cut_creation(name : &str, param_id : u32, low : f64, high : f64) -> ConditionRequest {
    ConditionRequest::CreateCut {name: String::from(name), param_id, low, high}
}
fn make_band_creation(name : &str, x_id : u32, y_id : u32, points : &Vec<(f64, f64)>) -> ConditionRequest {
    ConditionRequest::CreateBand {name: String::from(name), x_id, y_id, points: points.clone()}
}
fn make_contour_creation(name : &str, x_id : u32, y_id: u32, points : &Vec<(f64, f64)>) -> ConditionRequest {
    ConditionRequest::CreateContour {name: String::from(name), x_id, y_id, points: points.clone()}
}
fn make_delete(name : &str) -> ConditionRequest {
    ConditionRequest::DeleteCondition(String::from(name))
}
fn make_list(pattern : &str) -> ConditionRequest {
    ConditionRequest::List(String::from(pattern))
}
fn make_get_properties(name : &str) -> ConditionRequest {
    ConditionRequest::GetProperties(String::from(name))
}
fn make_request(reply_channel : mpsc::Sender<Reply>, req : ConditionRequest) -> Request {
    Request {
        reply_channel,
        message: MessageType::Condition(req)
    }
}
fn transaction(req_send : mpsc::Sender<Request>, rep_send :mpsc::Sender<Reply>, rep_read : mpsc::Receiver<Reply>, req : ConditionRequest) -> ConditionReply {
    let req = make_request(rep_send, req);
    let raw_reply = req.transaction(req_send, rep_read);
    if let Reply::Condition(reply) = raw_reply {
        reply
    } else {
        panic!("Condition transaction expected a condition reply but got something different");
    }
}
