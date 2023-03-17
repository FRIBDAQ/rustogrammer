//! Provides the message and reply structures that allow for manipulation
//! of a condition dictionary via messaging.
//! Note that for each new condition type it'll be necessary to add
//! a NewXXXX enum for the request message.
//!

use super::MessageType;
use super::Reply;
use super::Request;
use crate::conditions::*;

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
        dependent: String,
    },
    CreateAnd {
        name: String,
        dependents: Vec<String>,
    },
    CreateOr {
        name: String,
        dependents: Vec<String>,
    },
    CreateCut {
        name: String,
        param_id: u32,
        low: f64,
        high: f64,
    },
    CreateBand {
        name: String,
        x_id: u32,
        y_id: u32,
        points: Vec<(f64, f64)>,
    },
    CreateContour {
        name: String,
        x_id: u32,
        y_id: u32,
        points: Vec<(f64, f64)>,
    },
    DeleteCondition(String),
    List(String),
    GetProperties(String),
}
/// This structure provides condition properties:
#[derive(Clone, Debug, PartialEq)]
pub struct ConditionProperties {
    gate_name: String,
    type_name: String,
    points: Vec<(f64, f64)>,
    gates: Vec<String>,
    parameters: Vec<u32>,
}
///
/// These are replies that can be sent from the condition manager
/// part of the histograming thread:
///
#[derive(Clone, Debug, PartialEq)]
pub enum ConditionReply {
    Error(String),
    Created,
    Replaced,
    Deleted,
    Listing(Vec<ConditionProperties>),
}
// Having learned our lessons from parameter_messages.rs our
// private helper messages wil make ConditionRequest objects not
// MessageType objects.. It will be up to the API to wrap those
// into Request objects:

fn make_true_creation(name: &str) -> ConditionRequest {
    ConditionRequest::CreateTrue(String::from(name))
}
fn make_false_creation(name: &str) -> ConditionRequest {
    ConditionRequest::CreateFalse(String::from(name))
}
fn make_not_creation(name: &str, dependent: &str) -> ConditionRequest {
    ConditionRequest::CreateNot {
        name: String::from(name),
        dependent: String::from(dependent),
    }
}
fn make_and_creation(name: &str, dependents: &Vec<String>) -> ConditionRequest {
    ConditionRequest::CreateAnd {
        name: String::from(name),
        dependents: dependents.clone(),
    }
}
fn make_or_creation(name: &str, dependents: &Vec<String>) -> ConditionRequest {
    ConditionRequest::CreateOr {
        name: String::from(name),
        dependents: dependents.clone(),
    }
}
fn make_cut_creation(name: &str, param_id: u32, low: f64, high: f64) -> ConditionRequest {
    ConditionRequest::CreateCut {
        name: String::from(name),
        param_id,
        low,
        high,
    }
}
fn make_band_creation(
    name: &str,
    x_id: u32,
    y_id: u32,
    points: &Vec<(f64, f64)>,
) -> ConditionRequest {
    ConditionRequest::CreateBand {
        name: String::from(name),
        x_id,
        y_id,
        points: points.clone(),
    }
}
fn make_contour_creation(
    name: &str,
    x_id: u32,
    y_id: u32,
    points: &Vec<(f64, f64)>,
) -> ConditionRequest {
    ConditionRequest::CreateContour {
        name: String::from(name),
        x_id,
        y_id,
        points: points.clone(),
    }
}
fn make_delete(name: &str) -> ConditionRequest {
    ConditionRequest::DeleteCondition(String::from(name))
}
fn make_list(pattern: &str) -> ConditionRequest {
    ConditionRequest::List(String::from(pattern))
}

fn make_request(reply_channel: mpsc::Sender<Reply>, req: ConditionRequest) -> Request {
    Request {
        reply_channel,
        message: MessageType::Condition(req),
    }
}
fn transaction(
    req_send: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_read: mpsc::Receiver<Reply>,
    req: ConditionRequest,
) -> ConditionReply {
    let req = make_request(rep_send, req);
    let raw_reply = req.transaction(req_send, rep_read);
    if let Reply::Condition(reply) = raw_reply {
        reply
    } else {
        panic!("Condition transaction expected a condition reply but got something different");
    }
}

// Client API:

///  Create a true condition:
///  *  req_send - channel  to which the request should be sent.
///  *  rep_send - channel the server should use to send the reply.
///  *  rep_read - channel from which our thread should read that reply.
///  *  name - name of the true condition to create.
///
/// Returns ConditionReply.   On success this is either
/// *   Created - this was a new gate.
/// *   Replaced - An exsting gate by that name was replaced by
/// this true gate.
///
/// Other returns are errors.
pub fn create_true_condition(
    req_send: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_read: mpsc::Receiver<Reply>,
    name: &str,
) -> ConditionReply {
    transaction(req_send, rep_send, rep_read, make_true_creation(name))
}
///  Create a false condition:
///  *  req_send - channel  to which the request should be sent.
///  *  rep_send - channel the server should use to send the reply.
///  *  rep_read - channel from which our thread should read that reply.
///  *  name - name of the false condition to create.
///
/// Returns ConditionReply.   On success this is either
/// *   Created - this was a new gate.
/// *   Replaced - An exsting gate by that name was replaced by
/// this true gate.
///
/// Other returns are errors.
pub fn create_false_condition(
    req_send: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_read: mpsc::Receiver<Reply>,
    name: &str,
) -> ConditionReply {
    transaction(req_send, rep_send, rep_read, make_false_creation(name))
}
/// Create a Not condition.
///
///  *  req_send - channel  to which the request should be sent.
///  *  rep_send - channel the server should use to send the reply.
///  *  rep_read - channel from which our thread should read that reply.
///  *  name - name of the Not condition to create.
///  *  dependent - name of the condition that will be negated by this
/// condition.
/// Returns ConditionReply.   On success this is either
/// *   Created - this was a new gate.
/// *   Replaced - An exsting gate by that name was replaced by
/// this true gate.
///
/// Other returns are errors.  Note that a very simple error is that the
/// dependent condition does not yet exist.
///
pub fn create_not_condition(
    req_send: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_read: mpsc::Receiver<Reply>,
    name: &str,
    dependent: &str,
) -> ConditionReply {
    transaction(
        req_send,
        rep_send,
        rep_read,
        make_not_creation(name, dependent),
    )
}
/// Create a condition that is true if all dependent conditions are
/// true (And condition).
///
///  *  req_send - channel  to which the request should be sent.
///  *  rep_send - channel the server should use to send the reply.
///  *  rep_read - channel from which our thread should read that reply.
///  *  name - name of the Not condition to create.
///  *  dependents - names of the conditions that must all be true to make
/// the new condition true.
///
/// Returns ConditionReply.   On success this is either
/// *   Created - this was a new gate.
/// *   Replaced - An exsting gate by that name was replaced by
/// this true gate.
///
/// Other returns are errors.  Note that a very simple error is that the
/// one or more of the dependent conditions does not exist.
///
pub fn create_and_condition(
    req_send: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_read: mpsc::Receiver<Reply>,
    name: &str,
    dependents: &Vec<String>,
) -> ConditionReply {
    transaction(
        req_send,
        rep_send,
        rep_read,
        make_and_creation(name, dependents),
    )
}
/// Create a condition that is true if any of its dependenbt conditions is
/// true (Or condition).
///
///  *  req_send - channel  to which the request should be sent.
///  *  rep_send - channel the server should use to send the reply.
///  *  rep_read - channel from which our thread should read that reply.
///  *  name - name of the Not condition to create.
///  *  dependents - names of the conditions for which at least one must
/// be true to make the new condition true.
///
/// Returns ConditionReply.   On success this is either
/// *   Created - this was a new gate.
/// *   Replaced - An exsting gate by that name was replaced by
/// this new gate.
///
/// Other returns are errors.  Note that a very simple error is that the
/// one or more of the dependent conditions does not exist.
///
pub fn create_or_condition(
    req_send: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_read: mpsc::Receiver<Reply>,
    name: &str,
    dependents: &Vec<String>,
) -> ConditionReply {
    transaction(
        req_send,
        rep_send,
        rep_read,
        make_or_creation(name, dependents),
    )
}
/// Create a condition that is a cut on a parameter.
///
///  *  req_send - channel  to which the request should be sent.
///  *  rep_send - channel the server should use to send the reply.
///  *  rep_read - channel from which our thread should read that reply.
///  *  name - name of the Not condition to create.
///  *  param_id - The id of the parameter that is checked against the cut limits.
///  *  low  - Cut low limit.
///  *  high - Cut high limit.
///
/// Returns ConditionReply.   On success this is either
/// *   Created - this was a new gate.
/// *   Replaced - An exsting gate by that name was replaced by
/// this new gate.
///
/// Other returns are errors.  Note that the caller must have gotten the parameter_id
/// in some way that makes it valid (e.g. from a list request to the
/// histogram parameter handling module).  It is harmless for the parameter id
/// to be invalid -- the condition will, most likely never be true in that
/// case.
///
pub fn create_cut_condition(
    req_send: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_read: mpsc::Receiver<Reply>,
    name: &str,
    param_id: u32,
    low: f64,
    high: f64,
) -> ConditionReply {
    transaction(
        req_send,
        rep_send,
        rep_read,
        make_cut_creation(name, param_id, low, high),
    )
}
/// create a band condition.  This checks to see if events are below
/// some polyline in the 2d plane defined by a pair of parameters.
///  
///  *  req_send - channel  to which the request should be sent.
///  *  rep_send - channel the server should use to send the reply.
///  *  rep_read - channel from which our thread should read that reply.
///  *  name - name of the Not condition to create.
///  *  x_id  - Id of the X parameter the condition is checked against.
///  *  y_id  - Id of the Y parameter the condition is checked against.
///  *  points - The points that define the polyline events are checked against.
///
/// Returns ConditionReply.   On success this is either
/// *   Created - this was a new gate.
/// *   Replaced - An exsting gate by that name was replaced by
/// this new gate.
///
/// Other returns are errors.  Note that the caller must have gotten parameer ids
/// in some way that makes them valid (e.g. from a list request to the
/// histogram parameter handling module).  It is harmless for the parameter ids
/// to be invalid -- the condition will, most likely never be true in that
/// case.
///
pub fn create_band_condition(
    req_send: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_read: mpsc::Receiver<Reply>,
    name: &str,
    x_id: u32,
    y_id: u32,
    points: &Vec<(f64, f64)>,
) -> ConditionReply {
    transaction(
        req_send,
        rep_send,
        rep_read,
        make_band_creation(name, x_id, y_id, &points),
    )
}
///
/// create a contour condition.  Contours are closed figures in a plane
/// defined by two parameters.  The condition is true if the
/// event lives inside the contour where 'inside' is defined by the odd
/// crossing rule:
///
/// Odd Crossing Rule:   A point is inside a figure if a ray drawn from that
/// point in any direction crosses an odd number of figure boundary segments.
///
///  *  req_send - channel  to which the request should be sent.
///  *  rep_send - channel the server should use to send the reply.
///  *  rep_read - channel from which our thread should read that reply.
///  *  name - name of the Not condition to create.
///  *  x_id  - Id of the X parameter the condition is checked against.
///  *  y_id  - Id of the Y parameter the condition is checked against.
///  *  points - The points that define the closed figure the event is
/// checked against.
///
/// Returns ConditionReply.   On success this is either
/// *   Created - this was a new gate.
/// *   Replaced - An exsting gate by that name was replaced by
/// this new gate.
///
/// Other returns are errors.  Note that the caller must have gotten parameer ids
/// in some way that makes them valid (e.g. from a list request to the
/// histogram parameter handling module).  It is harmless for the parameter ids
/// to be invalid -- the condition will, most likely never be true in that
/// case.
///
pub fn create_contour_condition(
    req_send: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_read: mpsc::Receiver<Reply>,
    name: &str,
    x_id: u32,
    y_id: u32,
    points: &Vec<(f64, f64)>,
) -> ConditionReply {
    transaction(
        req_send,
        rep_send,
        rep_read,
        make_contour_creation(name, x_id, y_id, &points),
    )
}
///
/// Deletes a condition.  The condition is removed fromt he dictionary.
/// All remaining references are 'weak' by definition and will fail to promote
/// to a strong reference when use is attemped.
///
///  *  req_send - channel  to which the request should be sent.
///  *  rep_send - channel the server should use to send the reply.
///  *  rep_read - channel from which our thread should read that reply.
///  *  name - name of the condition to delete.
//
/// Returns ConditionReply.   On success this is Deleted.
///  Other returns are errors.  A simple error condition is that the
/// name is not a condition that is defined.
///
pub fn delete_condition(
    req_send: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_read: mpsc::Receiver<Reply>,
    name: &str,
) -> ConditionReply {
    transaction(req_send, rep_send, rep_read, make_delete(name))
}
///
/// Get a list of all conditions and their properties that match
/// a glob pattern.
///
///  *  req_send - channel  to which the request should be sent.
///  *  rep_send - channel the server should use to send the reply.
///  *  rep_read - channel from which our thread should read that reply.
///  *  pattern - glob pattern the condition names have to match
///to be included in the list.
///
/// Returns ConditionReply.   On success this is Listing and the payload
/// is a vector of the properties of the conditions that match the pattern.
/// Should never return errors.
///
pub fn list_conditions(
    req_send: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_read: mpsc::Receiver<Reply>,
    pattern: &str,
) -> ConditionReply {
    transaction(req_send, rep_send, rep_read, make_list(pattern))
}
///
/// Given a single condition name, get its properties.  Note this
/// is really list_conditions with a name rather than a pattern.
///
///  *  req_send - channel  to which the request should be sent.
///  *  rep_send - channel the server should use to send the reply.
///  *  rep_read - channel from which our thread should read that reply.
///  *  name - name of the condition whose properties will be gotten.
//
/// Returns ConditionProperties or panics on errors.
///
pub fn get_properties(
    req_send: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_read: mpsc::Receiver<Reply>,
    name: &str,
) -> ConditionProperties {
    let result = transaction(req_send, rep_send, rep_read, make_list(name));

    if let ConditionReply::Listing(properties) = result {
        assert_eq!(1, properties.len()); // Shold only be one item:
        properties[0].clone()
    } else {
        panic!("Expected Listing in get_properties but got somethjing different");
    }
}

// Sever side stuff.

/// ConditionProperites encapsulates a ConditionDictionary
/// and provides a public method that allows a ConditionRequest
/// to be processed returning the appropriate ConditionReply
///
/// The actual communication is assumed to have happened external to this
/// module.
///
struct ConditionProcessor {
    dict: ConditionDictionary,
}
impl ConditionProcessor {
    /// Invalidates all the cached condition evaulations
    /// in our dict.

    pub fn invalidate_cache(&mut self) {
        invalidate_cache(&mut self.dict);
    }
}