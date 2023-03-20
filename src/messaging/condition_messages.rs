//! Provides the message and reply structures that allow for manipulation
//! of a condition dictionary via messaging.
//! Note that for each new condition type it'll be necessary to add
//! a NewXXXX enum for the request message.
//!

use super::MessageType;
use super::Reply;
use super::Request;
use crate::conditions::*;

use glob::Pattern;
use std::cell::RefCell;
use std::rc::Rc;
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
}
/// This structure provides condition properties:
#[derive(Clone, Debug, PartialEq)]
pub struct ConditionProperties {
    cond_name: String,
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
    // Private methods:

    fn add_condition<T: Condition + Sized + 'static>(
        &mut self,
        name: &str,
        cond: T,
    ) -> ConditionReply {
        let b = Box::new(cond);
        match self.dict.get(&String::from(name)) {
            Some(prior) => {
                prior.replace(b);
                ConditionReply::Replaced
            }
            None => {
                self.dict
                    .insert(String::from(name), Rc::new(RefCell::new(b)));
                ConditionReply::Created
            }
        }
    }

    fn add_true(&mut self, name: &str) -> ConditionReply {
        let t = True {};
        self.add_condition(name, t)
    }
    fn add_false(&mut self, name: &str) -> ConditionReply {
        let f = False {};
        self.add_condition(name, f)
    }
    fn add_not(&mut self, name: &str, dependent: &str) -> ConditionReply {
        // Get the depdent condition:

        let d = self.dict.get(&String::from(dependent));
        if let Some(d) = d {
            let n = Not::new(d);
            self.add_condition(name, n)
        } else {
            ConditionReply::Error(format!("Dependent gate {} not found", dependent))
        }
    }
    fn add_and(&mut self, name: &str, dependents: Vec<String>) -> ConditionReply {
        let mut a = And::new();
        // now try to add all of the dependencies:

        for n in dependents {
            if let Some(c) = self.dict.get(&n) {
                a.add_condition(c);
            } else {
                return ConditionReply::Error(format!("Dependent gate {} not found", name));
            }
        }
        self.add_condition(name, a)
    }
    fn add_or(&mut self, name: &str, dependents: Vec<String>) -> ConditionReply {
        let mut o = Or::new();

        // add the dependecies:

        for n in dependents {
            if let Some(c) = self.dict.get(&n) {
                o.add_condition(c);
            } else {
                return ConditionReply::Error(format!("Dependent gate {} not found", name));
            }
        }
        self.add_condition(name, o)
    }
    fn add_cut(&mut self, name: &str, param_id: u32, low: f64, high: f64) -> ConditionReply {
        let c = Cut::new(param_id, low, high);
        self.add_condition(name, c)
    }

    // Turn the points as tuples into Vec<Point>

    fn convert_points(points: Vec<(f64, f64)>) -> Vec<Point> {
        let mut result = Vec::<Point>::new();
        for p in points {
            result.push(Point::new(p.0, p.1));
        }
        result
    }

    fn add_band(
        &mut self,
        name: &str,
        x_id: u32,
        y_id: u32,
        points: Vec<(f64, f64)>,
    ) -> ConditionReply {
        let b = Band::new(x_id, y_id, Self::convert_points(points));
        if let Some(b) = b {
            self.add_condition(name, b)
        } else {
            ConditionReply::Error(String::from("Too few points for  band"))
        }
    }

    fn add_contour(
        &mut self,
        name: &str,
        x_id: u32,
        y_id: u32,
        points: Vec<(f64, f64)>,
    ) -> ConditionReply {
        let c = Contour::new(x_id, y_id, Self::convert_points(points));
        if let Some(c) = c {
            self.add_condition(name, c)
        } else {
            ConditionReply::Error(String::from("Too few points for a contour"))
        }
    }
    fn remove_condition(&mut self, name: &str) -> ConditionReply {
        if let Some(_) = self.dict.remove(&String::from(name)) {
            ConditionReply::Deleted
        } else {
            ConditionReply::Error(format!("No such condition {}", name))
        }
    }
    // make CondtionPropreties from a condition and its name.

    fn make_props(&self, name: &str, c: &Container) -> ConditionProperties {
        // Need to make the dependent gates:
        let dependencies = c.borrow().dependent_gates();
        let mut d_names = Vec::<String>::new();
        for d in dependencies.iter() {
            if let Some(s) = gate_name_from_ref(&self.dict, d) {
                d_names.push(s)
            } else {
                d_names.push(String::from("-deleted-"));
            }
        }

        ConditionProperties {
            cond_name: String::from(name),
            type_name: c.borrow().gate_type(),
            points: c.borrow().gate_points(),
            gates: d_names,
            parameters: c.borrow().dependent_parameters(),
        }
    }

    fn list_conditions(&self, pattern: &str) -> ConditionReply {
        // compile the pattern if that fails return an error:

        let patt = Pattern::new(pattern);
        if patt.is_err() {
            return ConditionReply::Error(String::from(patt.unwrap_err().msg));
        }
        let patt = patt.unwrap();

        let mut props = Vec::<ConditionProperties>::new();
        for (name, cond) in self.dict.iter() {
            if patt.matches(&name) {
                props.push(self.make_props(&name, cond))
            }
        }
        ConditionReply::Listing(props)
    }
    /// Constructor:
    pub fn new() -> ConditionProcessor {
        ConditionProcessor {
            dict: ConditionDictionary::new(),
        }
    }
    /// Invalidates all the cached condition evaulations
    /// in our dict.
    ///
    pub fn invalidate_cache(&mut self) {
        invalidate_cache(&mut self.dict);
    }
    /// Process a request returning a reply:
    ///
    pub fn process_request(&mut self, req: ConditionRequest) -> ConditionReply {
        match req {
            ConditionRequest::CreateTrue(name) => self.add_true(&name),
            ConditionRequest::CreateFalse(name) => self.add_false(&name),
            ConditionRequest::CreateNot { name, dependent } => self.add_not(&name, &dependent),
            ConditionRequest::CreateAnd { name, dependents } => self.add_and(&name, dependents),
            ConditionRequest::CreateOr { name, dependents } => self.add_or(&name, dependents),
            ConditionRequest::CreateCut {
                name,
                param_id,
                low,
                high,
            } => self.add_cut(&name, param_id, low, high),
            ConditionRequest::CreateBand {
                name,
                x_id,
                y_id,
                points,
            } => self.add_band(&name, x_id, y_id, points),
            ConditionRequest::CreateContour {
                name,
                x_id,
                y_id,
                points,
            } => self.add_contour(&name, x_id, y_id, points),
            ConditionRequest::DeleteCondition(name) => self.remove_condition(&name),
            ConditionRequest::List(pattern) => self.list_conditions(&pattern),
        }
    }
}
// Tests of request message generators.

#[cfg(test)]
mod cond_msg_tests {
    use super::*;

    // let's be sure the ConditionRequest builders make valid requests:

    #[test]
    fn make_true_1() {
        let mtr = make_true_creation("a-condition");
        if let ConditionRequest::CreateTrue(t) = mtr {
            assert_eq!(String::from("a-condition"), t)
        } else {
            panic!("make_true_creation did not make ConditionRequest::CreateTrue");
        }
    }
    #[test]
    fn make_false_1() {
        let mfr = make_false_creation("false-cond");
        if let ConditionRequest::CreateFalse(n) = mfr {
            assert_eq!(String::from("false-cond"), n);
        } else {
            panic!("make_false_creation did not make a ConditionRequest::CreateFalse");
        }
    }
    #[test]
    fn make_not_1() {
        let mr = make_not_creation("not-cond", "dependent-cond");
        if let ConditionRequest::CreateNot { name, dependent } = mr {
            assert_eq!(String::from("not-cond"), name);
            assert_eq!(String::from("dependent-cond"), dependent);
        } else {
            panic!("make_not_creation did not make a ConditionRequest::CreateNot");
        }
    }
    #[test]
    fn make_and_1() {
        let dependent_conds = vec![
            String::from("cond1"),
            String::from("cond2"),
            String::from("cond3"),
            String::from("cond4"),
        ];
        let mr = make_and_creation("test", &dependent_conds);
        if let ConditionRequest::CreateAnd { name, dependents } = mr {
            assert_eq!(String::from("test"), name);
            assert_eq!(dependent_conds.len(), dependents.len());
            for (i, dep) in dependent_conds.iter().enumerate() {
                assert_eq!(*dep, dependents[i]);
            }
        } else {
            panic!("make_and_creation did not return a ConditionRequest::CrateAnd");
        }
    }
    #[test]
    fn make_or_1() {
        let dependent_conds = vec![
            String::from("cond1"),
            String::from("cond2"),
            String::from("cond3"),
            String::from("cond4"),
        ];
        let mr = make_or_creation("test", &dependent_conds);
        if let ConditionRequest::CreateOr { name, dependents } = mr {
            assert_eq!(String::from("test"), name);
            assert_eq!(dependent_conds.len(), dependents.len());
            for (i, dep) in dependent_conds.iter().enumerate() {
                assert_eq!(*dep, dependents[i]);
            }
        } else {
            panic!("make_and_creation did not return a ConditionRequest::CrateAnd");
        }
    }
    #[test]
    fn make_cut_1() {
        let mr = make_cut_creation("a-cut", 12, 100.0, 200.0);
        if let ConditionRequest::CreateCut {
            name,
            param_id,
            low,
            high,
        } = mr
        {
            assert_eq!(String::from("a-cut"), name);
            assert_eq!(12, param_id);
            assert_eq!(100.0, low);
            assert_eq!(200.0, high);
        } else {
            panic!("make_cut_creation did not return a ConditionRequest::CreateCut");
        }
    }
    #[test]
    fn make_band_1() {
        let pts = vec![(0.0, 100.0), (10.0, 50.0), (50.0, 25.0), (75.0, 0.0)];
        let mr = make_band_creation("band", 2, 5, &pts);
        if let ConditionRequest::CreateBand {
            name,
            x_id,
            y_id,
            points,
        } = mr
        {
            assert_eq!(String::from("band"), name);
            assert_eq!(2, x_id);
            assert_eq!(5, y_id);
            assert_eq!(pts.len(), points.len());
            for (i, p) in pts.iter().enumerate() {
                assert_eq!(p.0, points[i].0);
                assert_eq!(p.1, points[i].1);
            }
        } else {
            panic!("make_band_creation did not return a ConditionRequest::CreateBand");
        }
    }
    #[test]
    fn make_contour_1() {
        let pts = vec![(0.0, 100.0), (10.0, 50.0), (50.0, 25.0), (75.0, 0.0)];
        let mr = make_contour_creation("cont", 2, 5, &pts);
        if let ConditionRequest::CreateContour {
            name,
            x_id,
            y_id,
            points,
        } = mr
        {
            assert_eq!(String::from("cont"), name);
            assert_eq!(2, x_id);
            assert_eq!(5, y_id);
            assert_eq!(pts.len(), points.len());
            for (i, p) in pts.iter().enumerate() {
                assert_eq!(p.0, points[i].0);
                assert_eq!(p.1, points[i].1);
            }
        } else {
            panic!("make_contour_creation did not return a ConditionRequest::CreateContour");
        }
    }
    #[test]
    fn make_delete_1() {
        let mr = make_delete("junk");
        if let ConditionRequest::DeleteCondition(s) = mr {
            assert_eq!(String::from("junk"), s);
        } else {
            panic!("make_delete did not create DeleteCondition request");
        }
    }
    #[test]
    fn make_list_1() {
        let mr = make_list("*");
        if let ConditionRequest::List(p) = mr {
            assert_eq!(String::from("*"), p);
        } else {
            panic!("make_list did not create a List request");
        }
    }
}
#[cfg(test)]
mod cnd_processor_tests {
    use super::*;

    #[test]
    fn new_1() {
        // Construction makes a valid empty dict:

        let cp = ConditionProcessor::new();
        assert_eq!(0, cp.dict.len())
    }
    #[test]
    fn make_true_1() {
        let mut cp = ConditionProcessor::new();
        let rep = cp.process_request(make_true_creation("true-cond"));
        assert_eq!(ConditionReply::Created, rep);

        let item = cp.dict.get("true-cond");
        assert!(item.is_some());
        assert_eq!(String::from("True"), item.unwrap().borrow().gate_type());
    
    }
}
