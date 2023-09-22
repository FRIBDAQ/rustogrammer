//! Provides the message and reply structures that allow for manipulation
//! of a condition dictionary via messaging.
//! Note that for each new condition type it'll be necessary to add
//! a NewXXXX enum for the request message.
//!

use super::MessageType;
use super::Reply;
use super::Request;
use crate::conditions::*;
use crate::trace;

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
    CreateMultiCut {
        name: String,
        ids: Vec<u32>,
        low: f64,
        high: f64,
    },
    CreateMultiContour {
        name: String,
        ids: Vec<u32>,
        points: Vec<(f64, f64)>,
    },
    DeleteCondition(String),
    List(String),
}
/// This structure provides condition properties:
#[derive(Clone, Debug, PartialEq)]
pub struct ConditionProperties {
    pub cond_name: String,
    pub type_name: String,
    pub points: Vec<(f64, f64)>,
    pub gates: Vec<String>,
    pub parameters: Vec<u32>,
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

/// Per issue 23,  We produce a message client class.
/// It encapsulates the requesting channel and the public methods
/// will all generate the reply channels as per request
/// simplifying the public call signatures and logic
///
pub struct ConditionMessageClient {
    req_send: mpsc::Sender<Request>,
}

impl ConditionMessageClient {
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
    fn make_and_creation(name: &str, dependents: &[String]) -> ConditionRequest {
        ConditionRequest::CreateAnd {
            name: String::from(name),
            dependents: dependents.to_owned(),
        }
    }
    fn make_or_creation(name: &str, dependents: &[String]) -> ConditionRequest {
        ConditionRequest::CreateOr {
            name: String::from(name),
            dependents: dependents.to_owned(),
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
        points: &[(f64, f64)],
    ) -> ConditionRequest {
        ConditionRequest::CreateBand {
            name: String::from(name),
            x_id,
            y_id,
            points: points.to_owned(),
        }
    }
    fn make_contour_creation(
        name: &str,
        x_id: u32,
        y_id: u32,
        points: &[(f64, f64)],
    ) -> ConditionRequest {
        ConditionRequest::CreateContour {
            name: String::from(name),
            x_id,
            y_id,
            points: points.to_owned(),
        }
    }
    fn make_multicut_creation(name: &str, ids: &[u32], low: f64, high: f64) -> ConditionRequest {
        ConditionRequest::CreateMultiCut {
            name: String::from(name),
            ids: ids.to_owned(),
            low,
            high,
        }
    }
    fn make_multicontour_creation(
        name: &str,
        ids: &[u32],
        points: &[(f64, f64)],
    ) -> ConditionRequest {
        ConditionRequest::CreateMultiContour {
            name: String::from(name),
            ids: ids.to_owned(),
            points: points.to_owned(),
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

    // This method isolates all the messaging skulduggery from the rest of the
    // code.

    fn transaction(&self, req: ConditionRequest) -> ConditionReply {
        let (rep_send, rep_read) = mpsc::channel::<Reply>();
        let req_send = self.req_send.clone();
        let req = Self::make_request(rep_send, req);
        let raw_reply = req.transaction(req_send, rep_read);
        if let Reply::Condition(reply) = raw_reply {
            reply
        } else {
            panic!("Condition transaction expected a condition reply but got something different");
        }
    }

    // Client API:

    /// Create a new client:

    pub fn new(chan: &mpsc::Sender<Request>) -> ConditionMessageClient {
        ConditionMessageClient {
            req_send: chan.clone(),
        }
    }

    ///  Create a true condition:
    ///  *  name - name of the true condition to create.
    ///
    /// Returns ConditionReply.   On success this is either
    /// *   Created - this was a new condition.
    /// *   Replaced - An exsting condition by that name was replaced by
    /// this true condition.
    ///
    /// Other returns are errors.
    pub fn create_true_condition(&self, name: &str) -> ConditionReply {
        self.transaction(Self::make_true_creation(name))
    }
    ///  Create a false condition:
    ///  *  name - name of the false condition to create.
    ///
    /// Returns ConditionReply.   On success this is either
    /// *   Created - this was a new condition.
    /// *   Replaced - An exsting condition by that name was replaced by
    /// this true condition.
    ///
    /// Other returns are errors.
    pub fn create_false_condition(&self, name: &str) -> ConditionReply {
        self.transaction(Self::make_false_creation(name))
    }
    /// Create a Not condition.
    ///
    ///  *  name - name of the Not condition to create.
    ///  *  dependent - name of the condition that will be negated by this
    /// condition.
    /// Returns ConditionReply.   On success this is either
    /// *   Created - this was a new condition.
    /// *   Replaced - An exsting condition by that name was replaced by
    /// this true condition.
    ///
    /// Other returns are errors.  Note that a very simple error is that the
    /// dependent condition does not yet exist.
    ///
    pub fn create_not_condition(&self, name: &str, dependent: &str) -> ConditionReply {
        self.transaction(Self::make_not_creation(name, dependent))
    }
    /// Create a condition that is true if all dependent conditions are
    /// true (And condition).
    ///
    ///  *  name - name of the Not condition to create.
    ///  *  dependents - names of the conditions that must all be true to make
    /// the new condition true.
    ///
    /// Returns ConditionReply.   On success this is either
    /// *   Created - this was a new condition.
    /// *   Replaced - An exsting condition by that name was replaced by
    /// this true condition.
    ///
    /// Other returns are errors.  Note that a very simple error is that the
    /// one or more of the dependent conditions does not exist.
    ///
    pub fn create_and_condition(&self, name: &str, dependents: &[String]) -> ConditionReply {
        self.transaction(Self::make_and_creation(name, dependents))
    }
    /// Create a condition that is true if any of its dependenbt conditions is
    /// true (Or condition).
    ///
    ///  *  name - name of the Not condition to create.
    ///  *  dependents - names of the conditions for which at least one must
    /// be true to make the new condition true.
    ///
    /// Returns ConditionReply.   On success this is either
    /// *   Created - this was a new condition.
    /// *   Replaced - An exsting condition by that name was replaced by
    /// this new condition.
    ///
    /// Other returns are errors.  Note that a very simple error is that the
    /// one or more of the dependent conditions does not exist.
    ///
    pub fn create_or_condition(&self, name: &str, dependents: &[String]) -> ConditionReply {
        self.transaction(Self::make_or_creation(name, dependents))
    }
    /// Create a condition that is a cut on a parameter.
    ///
    ///  *  name - name of the Not condition to create.
    ///  *  param_id - The id of the parameter that is checked against the cut limits.
    ///  *  low  - Cut low limit.
    ///  *  high - Cut high limit.
    ///
    /// Returns ConditionReply.   On success this is either
    /// *   Created - this was a new condition.
    /// *   Replaced - An exsting condition by that name was replaced by
    /// this new condition.
    ///
    /// Other returns are errors.  Note that the caller must have gotten the parameter_id
    /// in some way that makes it valid (e.g. from a list request to the
    /// histogram parameter handling module).  It is harmless for the parameter id
    /// to be invalid -- the condition will, most likely never be true in that
    /// case.
    ///
    pub fn create_cut_condition(
        &self,
        name: &str,
        param_id: u32,
        low: f64,
        high: f64,
    ) -> ConditionReply {
        self.transaction(Self::make_cut_creation(name, param_id, low, high))
    }
    /// create a band condition.  This checks to see if events are below
    /// some polyline in the 2d plane defined by a pair of parameters.
    ///  
    ///  *  name - name of the Not condition to create.
    ///  *  x_id  - Id of the X parameter the condition is checked against.
    ///  *  y_id  - Id of the Y parameter the condition is checked against.
    ///  *  points - The points that define the polyline events are checked against.
    ///
    /// Returns ConditionReply.   On success this is either
    /// *   Created - this was a new condition.
    /// *   Replaced - An exsting condition by that name was replaced by
    /// this new condition.
    ///
    /// Other returns are errors.  Note that the caller must have gotten parameer ids
    /// in some way that makes them valid (e.g. from a list request to the
    /// histogram parameter handling module).  It is harmless for the parameter ids
    /// to be invalid -- the condition will, most likely never be true in that
    /// case.
    ///
    pub fn create_band_condition(
        &self,
        name: &str,
        x_id: u32,
        y_id: u32,
        points: &[(f64, f64)],
    ) -> ConditionReply {
        self.transaction(Self::make_band_creation(name, x_id, y_id, points))
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
    ///  *  name - name of the Not condition to create.
    ///  *  x_id  - Id of the X parameter the condition is checked against.
    ///  *  y_id  - Id of the Y parameter the condition is checked against.
    ///  *  points - The points that define the closed figure the event is
    /// checked against.
    ///
    /// Returns ConditionReply.   On success this is either
    /// *   Created - this was a new condition.
    /// *   Replaced - An exsting condition by that name was replaced by
    /// this new condition.
    ///
    /// Other returns are errors.  Note that the caller must have gotten parameer ids
    /// in some way that makes them valid (e.g. from a list request to the
    /// histogram parameter handling module).  It is harmless for the parameter ids
    /// to be invalid -- the condition will, most likely never be true in that
    /// case.
    ///
    pub fn create_contour_condition(
        &self,
        name: &str,
        x_id: u32,
        y_id: u32,
        points: &[(f64, f64)],
    ) -> ConditionReply {
        self.transaction(Self::make_contour_creation(name, x_id, y_id, points))
    }
    /// create_multicut_condition
    ///
    /// Multi cuts are what SpecTcl called gamma slices.  They get an array
    /// of parameters, and low, high limits:
    ///
    /// ###  Parameters
    /// *  name - name of the new condition.
    /// *  ids  - Array of parameter ids for the condition is set on.
    /// *  low, high - the condition limits.
    ///
    /// ### Returns
    ///    ConditionReply - this should be either Created or Replaced or Error.
    ///
    pub fn create_multicut_condition(
        &self,
        name: &str,
        ids: &[u32],
        low: f64,
        high: f64,
    ) -> ConditionReply {
        self.transaction(Self::make_multicut_creation(name, ids, low, high))
    }
    ///
    /// Creaet a multicontour
    ///   MulitContours are analagous to SpecTcl gamma-contours.  They get an
    /// array of ids and 2-d points:
    ///
    /// ### Parameters
    ///  *   name - name of the new condition.
    ///  *   ids - array of parameter ids.
    ///  *   points - array of points.
    ///
    /// ### Returns:
    ///   Condition reply which is hopefully either Created or Replaced
    ///
    pub fn create_multicontour_condition(
        &self,
        name: &str,
        ids: &[u32],
        points: &[(f64, f64)],
    ) -> ConditionReply {
        self.transaction(Self::make_multicontour_creation(name, ids, points))
    }
    ///
    /// Deletes a condition.  The condition is removed fromt he dictionary.
    /// All remaining references are 'weak' by definition and will fail to promote
    /// to a strong reference when use is attemped.
    ///
    ///  *  name - name of the condition to delete.
    //
    /// Returns ConditionReply.   On success this is Deleted.
    ///  Other returns are errors.  A simple error condition is that the
    /// name is not a condition that is defined.
    ///
    pub fn delete_condition(&self, name: &str) -> ConditionReply {
        self.transaction(Self::make_delete(name))
    }
    ///
    /// Get a list of all conditions and their properties that match
    /// a glob pattern.
    ///
    ///  *  pattern - glob pattern the condition names have to match
    ///to be included in the list.
    ///
    /// Returns ConditionReply.   On success this is Listing and the payload
    /// is a vector of the properties of the conditions that match the pattern.
    /// Should never return errors.
    ///
    pub fn list_conditions(&self, pattern: &str) -> ConditionReply {
        self.transaction(Self::make_list(pattern))
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
pub struct ConditionProcessor {
    dict: ConditionDictionary,
}
impl ConditionProcessor {
    // Private methods:

    fn add_condition<T: Condition + Sized + 'static>(
        &mut self,
        name: &str,
        cond: T,
        tracedb: &trace::SharedTraceStore,
    ) -> ConditionReply {
        let b = Box::new(cond);
        match self.dict.get(&String::from(name)) {
            Some(prior) => {
                prior.replace(b);
                tracedb.add_event(trace::TraceEvent::ConditionModified(String::from(name)));
                ConditionReply::Replaced
            }
            None => {
                self.dict
                    .insert(String::from(name), Rc::new(RefCell::new(b)));
                tracedb.add_event(trace::TraceEvent::ConditionCreated(String::from(name)));
                ConditionReply::Created
            }
        }
    }

    fn add_true(&mut self, name: &str, tracedb: &trace::SharedTraceStore) -> ConditionReply {
        let t = True {};
        self.add_condition(name, t, tracedb)
    }
    fn add_false(&mut self, name: &str, tracedb: &trace::SharedTraceStore) -> ConditionReply {
        let f = False {};
        self.add_condition(name, f, tracedb)
    }
    fn add_not(
        &mut self,
        name: &str,
        dependent: &str,
        tracedb: &trace::SharedTraceStore,
    ) -> ConditionReply {
        // Get the depdent condition:

        let d = self.dict.get(&String::from(dependent));
        if let Some(d) = d {
            let n = Not::new(d);
            self.add_condition(name, n, tracedb)
        } else {
            ConditionReply::Error(format!("Dependent condition {} not found", dependent))
        }
    }
    fn add_and(
        &mut self,
        name: &str,
        dependents: Vec<String>,
        tracedb: &trace::SharedTraceStore,
    ) -> ConditionReply {
        let mut a = And::new();
        // now try to add all of the dependencies:

        for n in dependents {
            if let Some(c) = self.dict.get(&n) {
                a.add_condition(c);
            } else {
                return ConditionReply::Error(format!("Dependent condition {} not found", name));
            }
        }
        self.add_condition(name, a, tracedb)
    }
    fn add_or(
        &mut self,
        name: &str,
        dependents: Vec<String>,
        tracedb: &trace::SharedTraceStore,
    ) -> ConditionReply {
        let mut o = Or::new();

        // add the dependecies:

        for n in dependents {
            if let Some(c) = self.dict.get(&n) {
                o.add_condition(c);
            } else {
                return ConditionReply::Error(format!("Dependent condition {} not found", name));
            }
        }
        self.add_condition(name, o, tracedb)
    }
    fn add_cut(
        &mut self,
        name: &str,
        param_id: u32,
        low: f64,
        high: f64,
        tracedb: &trace::SharedTraceStore,
    ) -> ConditionReply {
        let c = Cut::new(param_id, low, high);
        self.add_condition(name, c, tracedb)
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
        tracedb: &trace::SharedTraceStore,
    ) -> ConditionReply {
        let b = Band::new(x_id, y_id, Self::convert_points(points));
        if let Some(b) = b {
            self.add_condition(name, b, tracedb)
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
        tracedb: &trace::SharedTraceStore,
    ) -> ConditionReply {
        let c = Contour::new(x_id, y_id, Self::convert_points(points));
        if let Some(c) = c {
            self.add_condition(name, c, tracedb)
        } else {
            ConditionReply::Error(String::from("Too few points for a contour"))
        }
    }

    fn add_multicut(
        &mut self,
        name: &str,
        ids: &[u32],
        low: f64,
        high: f64,
        tracedb: &trace::SharedTraceStore,
    ) -> ConditionReply {
        self.add_condition(name, MultiCut::new(ids, low, high), tracedb)
    }
    fn add_multicontour(
        &mut self,
        name: &str,
        ids: &[u32],
        points: &[(f64, f64)],
        tracedb: &trace::SharedTraceStore,
    ) -> ConditionReply {
        let mut pts = vec![];
        for pt in points {
            pts.push(Point::new(pt.0, pt.1));
        }
        if let Some(c) = MultiContour::new(ids, pts) {
            self.add_condition(name, c, tracedb)
        } else {
            ConditionReply::Error(String::from("Unable to create multicontour"))
        }
    }
    fn remove_condition(
        &mut self,
        name: &str,
        tracedb: &trace::SharedTraceStore,
    ) -> ConditionReply {
        if self.dict.remove(&String::from(name)).is_some() {
            tracedb.add_event(trace::TraceEvent::ConditionDeleted(String::from(name)));
            ConditionReply::Deleted
        } else {
            ConditionReply::Error(format!("No such condition {}", name))
        }
    }
    // make CondtionPropreties from a condition and its name.

    fn make_props(&self, name: &str, c: &Container) -> ConditionProperties {
        // Need to make the dependent conditions:
        let dependencies = c.borrow().dependent_gates();
        let mut d_names = Vec::<String>::new();
        for d in dependencies.iter() {
            if let Some(s) = condition_name_from_ref(&self.dict, d) {
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
        if let Err(e) = patt {
            return ConditionReply::Error(String::from(e.msg));
        }
        let patt = patt.unwrap();

        let mut props = Vec::<ConditionProperties>::new();
        for (name, cond) in self.dict.iter() {
            if patt.matches(name) {
                props.push(self.make_props(name, cond))
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

    /// Process a request returning a reply:
    ///
    pub fn process_request(
        &mut self,
        req: ConditionRequest,
        tracedb: &trace::SharedTraceStore,
    ) -> ConditionReply {
        match req {
            ConditionRequest::CreateTrue(name) => self.add_true(&name, tracedb),
            ConditionRequest::CreateFalse(name) => self.add_false(&name, tracedb),
            ConditionRequest::CreateNot { name, dependent } => {
                self.add_not(&name, &dependent, tracedb)
            }
            ConditionRequest::CreateAnd { name, dependents } => {
                self.add_and(&name, dependents, tracedb)
            }
            ConditionRequest::CreateOr { name, dependents } => {
                self.add_or(&name, dependents, tracedb)
            }
            ConditionRequest::CreateCut {
                name,
                param_id,
                low,
                high,
            } => self.add_cut(&name, param_id, low, high, tracedb),
            ConditionRequest::CreateBand {
                name,
                x_id,
                y_id,
                points,
            } => self.add_band(&name, x_id, y_id, points, tracedb),
            ConditionRequest::CreateContour {
                name,
                x_id,
                y_id,
                points,
            } => self.add_contour(&name, x_id, y_id, points, tracedb),
            ConditionRequest::CreateMultiCut {
                name,
                ids,
                low,
                high,
            } => self.add_multicut(&name, &ids, low, high, tracedb),
            ConditionRequest::CreateMultiContour { name, ids, points } => {
                self.add_multicontour(&name, &ids, &points, tracedb)
            }
            ConditionRequest::DeleteCondition(name) => self.remove_condition(&name, tracedb),
            ConditionRequest::List(pattern) => self.list_conditions(&pattern),
        }
    }
    pub fn get_dict(&mut self) -> &mut ConditionDictionary {
        &mut self.dict
    }
}

///
/// This function reconstructs a contour in terms of the information
/// that is passed to it by the condition_messaging API.  This is needed
/// in order to construct a closure that can properly work for project_spectrum
/// when the projection is inside s contour.
///
/// ### Parameters:
///   *  props - the condition properties. Note these are consumed.
/// ### Returns:
///   Result<conditions::twod::Contour, String>  - where:
///   *  Ok encapsulates the reconstituted contour
///   *  Err encapsulates an error string (normally if props are not a
/// contour).
///
/// ### NOTE:
///   Dummy parameter numbers 0 and 1 are used for the parameter ids.
///
pub fn reconstitute_contour(props: ConditionProperties) -> Result<twod::Contour, String> {
    if props.type_name == "Contour" {
        let mut pts = Vec::<twod::Point>::new();
        for (x, y) in props.points {
            pts.push(twod::Point::new(x, y));
        }
        match twod::Contour::new(0, 1, pts) {
            Some(c) => Ok(c),
            None => Err(String::from(
                "Failed to reconstitute contour in constructor - maybe too few points?",
            )),
        }
    } else {
        Err(String::from(
            "Error reconstituting a contour - input is not a contour",
        ))
    }
}

// Tests of request message generators.

#[cfg(test)]
mod cond_msg_tests {
    use super::*;

    // let's be sure the ConditionRequest builders make valid requests:

    #[test]
    fn make_true_1() {
        let mtr = ConditionMessageClient::make_true_creation("a-condition");
        if let ConditionRequest::CreateTrue(t) = mtr {
            assert_eq!(String::from("a-condition"), t)
        } else {
            panic!("make_true_creation did not make ConditionRequest::CreateTrue");
        }
    }
    #[test]
    fn make_false_1() {
        let mfr = ConditionMessageClient::make_false_creation("false-cond");
        if let ConditionRequest::CreateFalse(n) = mfr {
            assert_eq!(String::from("false-cond"), n);
        } else {
            panic!("make_false_creation did not make a ConditionRequest::CreateFalse");
        }
    }
    #[test]
    fn make_not_1() {
        let mr = ConditionMessageClient::make_not_creation("not-cond", "dependent-cond");
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
        let mr = ConditionMessageClient::make_and_creation("test", &dependent_conds);
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
        let mr = ConditionMessageClient::make_or_creation("test", &dependent_conds);
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
        let mr = ConditionMessageClient::make_cut_creation("a-cut", 12, 100.0, 200.0);
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
        let mr = ConditionMessageClient::make_band_creation("band", 2, 5, &pts);
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
        let mr = ConditionMessageClient::make_contour_creation("cont", 2, 5, &pts);
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
        let mr = ConditionMessageClient::make_delete("junk");
        if let ConditionRequest::DeleteCondition(s) = mr {
            assert_eq!(String::from("junk"), s);
        } else {
            panic!("make_delete did not create DeleteCondition request");
        }
    }
    #[test]
    fn make_list_1() {
        let mr = ConditionMessageClient::make_list("*");
        if let ConditionRequest::List(p) = mr {
            assert_eq!(String::from("*"), p);
        } else {
            panic!("make_list did not create a List request");
        }
    }
    #[test]
    fn make_multicut_1() {
        let mc = ConditionMessageClient::make_multicut_creation("name", &[1, 2, 3], 100.0, 200.0);
        assert_eq!(
            ConditionRequest::CreateMultiCut {
                name: String::from("name"),
                ids: vec![1, 2, 3],
                low: 100.0,
                high: 200.0
            },
            mc
        )
    }
    #[test]
    fn make_multicontour_1() {
        let mc = ConditionMessageClient::make_multicontour_creation(
            "name",
            &vec![1, 2, 3],
            &vec![(100.0, 100.0), (150.0, 100.0), (125.0, 150.0)],
        );
        assert_eq!(
            ConditionRequest::CreateMultiContour {
                name: String::from("name"),
                ids: vec![1, 2, 3],
                points: vec![(100.0, 100.0), (150.0, 100.0), (125.0, 150.0)]
            },
            mc
        );
    }
}
#[cfg(test)]
mod cnd_processor_tests {
    use super::*;
    use crate::trace;
    use std::collections::HashMap;

    #[test]
    fn new_1() {
        // Construction makes a valid empty dict:

        let cp = ConditionProcessor::new();
        assert_eq!(0, cp.dict.len())
    }
    // Basic condition creation.
    #[test]
    fn make_true_1() {
        let tracedb = trace::SharedTraceStore::new();
        let mut cp = ConditionProcessor::new();
        let rep = cp.process_request(
            ConditionMessageClient::make_true_creation("true-cond"),
            &tracedb,
        );
        assert_eq!(ConditionReply::Created, rep);

        let item = cp.dict.get("true-cond");
        assert!(item.is_some());
        assert_eq!(String::from("True"), item.unwrap().borrow().gate_type());
    }
    #[test]
    fn make_false_1() {
        let tracedb = trace::SharedTraceStore::new();
        let mut cp = ConditionProcessor::new();
        let rep = cp.process_request(
            ConditionMessageClient::make_false_creation("false-cond"),
            &tracedb,
        );
        assert_eq!(ConditionReply::Created, rep);

        let item = cp.dict.get("false-cond");
        assert!(item.is_some());
        assert_eq!(String::from("False"), item.unwrap().borrow().gate_type());
    }
    #[test]
    fn make_not_1() {
        let mut cp = ConditionProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        cp.process_request(
            ConditionMessageClient::make_false_creation("false"),
            &tracedb,
        );
        let rep = cp.process_request(
            ConditionMessageClient::make_not_creation("true", "false"),
            &tracedb,
        );
        assert_eq!(ConditionReply::Created, rep);

        let item = cp.dict.get("true");
        assert!(item.is_some());
        let cond = item.unwrap();
        assert_eq!(String::from("Not"), cond.borrow().gate_type());
        let dep = cond.borrow().dependent_gates();
        assert_eq!(1, dep.len());
        assert_eq!(
            String::from("False"),
            dep[0].upgrade().unwrap().borrow().gate_type()
        );
    }
    #[test]
    fn make_and_1() {
        let mut cp = ConditionProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        cp.process_request(ConditionMessageClient::make_true_creation("true"), &tracedb);
        cp.process_request(
            ConditionMessageClient::make_false_creation("false"),
            &tracedb,
        );
        let rep = cp.process_request(
            ConditionMessageClient::make_and_creation(
                "and",
                &vec![String::from("true"), String::from("false")],
            ),
            &tracedb,
        );
        assert_eq!(ConditionReply::Created, rep);

        let cond = cp.dict.get("and").unwrap();
        assert_eq!(String::from("And"), cond.borrow().gate_type());
        let deps = cond.borrow().dependent_gates();

        assert_eq!(2, deps.len());
        assert_eq!(
            String::from("True"),
            deps[0].upgrade().unwrap().borrow().gate_type()
        );
        assert_eq!(
            String::from("False"),
            deps[1].upgrade().unwrap().borrow().gate_type()
        );
    }
    #[test]
    fn make_or_1() {
        let mut cp = ConditionProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        cp.process_request(ConditionMessageClient::make_true_creation("true"), &tracedb);
        cp.process_request(
            ConditionMessageClient::make_false_creation("false"),
            &tracedb,
        );
        let rep = cp.process_request(
            ConditionMessageClient::make_or_creation(
                "or",
                &vec![String::from("true"), String::from("false")],
            ),
            &tracedb,
        );
        assert_eq!(ConditionReply::Created, rep);

        let cond = cp.dict.get("or").unwrap();
        assert_eq!(String::from("Or"), cond.borrow().gate_type());
        let deps = cond.borrow().dependent_gates();

        assert_eq!(2, deps.len());
        assert_eq!(
            String::from("True"),
            deps[0].upgrade().unwrap().borrow().gate_type()
        );
        assert_eq!(
            String::from("False"),
            deps[1].upgrade().unwrap().borrow().gate_type()
        );
    }
    #[test]
    fn make_cut_1() {
        let mut cp = ConditionProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        let rep = cp.process_request(
            ConditionMessageClient::make_cut_creation("cut", 12, 100.0, 200.0),
            &tracedb,
        );
        assert_eq!(ConditionReply::Created, rep);

        let cond = cp.dict.get("cut").unwrap();
        assert_eq!("Cut", cond.borrow().gate_type());
        let pts = cond.borrow().gate_points();
        assert_eq!(2, pts.len());
        assert_eq!(100.0, pts[0].0);
        assert_eq!(200.0, pts[1].0);
    }
    #[test]
    fn make_band_1() {
        let mut cp = ConditionProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        let condition_pts = vec![(0.0, 100.0), (50.0, 200.0), (100.0, 50.0), (200.0, 25.0)];
        let rep = cp.process_request(
            ConditionMessageClient::make_band_creation("band", 10, 15, &condition_pts),
            &tracedb,
        );
        assert_eq!(ConditionReply::Created, rep);

        let cond = cp.dict.get("band").unwrap();
        assert_eq!(String::from("Band"), cond.borrow().gate_type());
        let pts = cond.borrow().gate_points();
        assert_eq!(condition_pts.len(), pts.len());
        for (i, p) in condition_pts.iter().enumerate() {
            assert_eq!(p.0, pts[i].0);
            assert_eq!(p.1, pts[i].1);
        }
    }
    #[test]
    fn make_contour_1() {
        let mut cp = ConditionProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        let condition_pts = vec![(0.0, 100.0), (50.0, 200.0), (100.0, 50.0), (200.0, 25.0)];
        let rep = cp.process_request(
            ConditionMessageClient::make_contour_creation("contour", 10, 15, &condition_pts),
            &tracedb,
        );
        assert_eq!(ConditionReply::Created, rep);

        let cond = cp.dict.get("contour").unwrap();
        assert_eq!(String::from("Contour"), cond.borrow().gate_type());
        let pts = cond.borrow().gate_points();
        assert_eq!(condition_pts.len(), pts.len());
        for (i, p) in condition_pts.iter().enumerate() {
            assert_eq!(p.0, pts[i].0);
            assert_eq!(p.1, pts[i].1);
        }
    }
    // Creation replacement

    #[test]
    fn make_replace_1() {
        let mut cp = ConditionProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        cp.process_request(
            ConditionMessageClient::make_true_creation("acondition"),
            &tracedb,
        );

        let cond = cp.dict.get("acondition").unwrap();
        assert_eq!("True", cond.borrow().gate_type());
        let cond = Rc::downgrade(cond); // Weak now

        // Replacing the condition should happen transparently
        // to our cond:

        let result = cp.process_request(
            ConditionMessageClient::make_false_creation("acondition"),
            &tracedb,
        );
        assert_eq!(ConditionReply::Replaced, result);

        assert_eq!("False", cond.upgrade().unwrap().borrow().gate_type());
    }

    // Other requests.

    #[test]
    fn delete_1() {
        let mut cp = ConditionProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        cp.process_request(ConditionMessageClient::make_true_creation("true"), &tracedb);
        cp.process_request(
            ConditionMessageClient::make_false_creation("false"),
            &tracedb,
        );

        // Delete the true condition

        let reply = cp.process_request(ConditionMessageClient::make_delete("true"), &tracedb);
        assert_eq!(ConditionReply::Deleted, reply); // Success.

        // It's gone:

        assert_eq!(1, cp.dict.len());
        assert!(cp.dict.get("true").is_none());
    }
    #[test]
    fn delete_2() {
        // unsuccessful delete:

        let mut cp = ConditionProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        cp.process_request(ConditionMessageClient::make_true_creation("true"), &tracedb);
        let reply = cp.process_request(ConditionMessageClient::make_delete("false"), &tracedb);
        if let ConditionReply::Error(_) = reply {
            assert!(true);
        } else {
            assert!(false);
        }
    }
    fn make_list_conditions() -> ConditionProcessor {
        let mut cp = ConditionProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        cp.process_request(ConditionMessageClient::make_true_creation("true"), &tracedb);
        cp.process_request(
            ConditionMessageClient::make_false_creation("false"),
            &tracedb,
        );
        cp.process_request(
            ConditionMessageClient::make_cut_creation("t-cut", 12, 100.0, 200.0),
            &tracedb,
        );
        cp.process_request(
            ConditionMessageClient::make_and_creation(
                "fake",
                &vec![String::from("true"), String::from("t-cut")],
            ),
            &tracedb,
        );

        cp
    }
    #[test]
    fn list_1() {
        // List all conditions:
        let mut cp = make_list_conditions();
        let tracedb = trace::SharedTraceStore::new();
        let reply = cp.process_request(ConditionMessageClient::make_list("*"), &tracedb);
        if let ConditionReply::Listing(list) = reply {
            assert_eq!(4, list.len());
            // we don't know the order in which these come back so
            // toss them into a hash indexed by name

            let mut hash = HashMap::<String, ConditionProperties>::new();
            for c in list {
                let name = c.cond_name.clone();
                hash.insert(name, c);
            }
            let t = hash.get("true").unwrap();
            assert_eq!(String::from("True"), t.type_name);

            let f = hash.get("false").unwrap();
            assert_eq!(String::from("False"), f.type_name);

            let c = hash.get("t-cut").unwrap();
            assert_eq!(String::from("Cut"), c.type_name);
            assert_eq!(2, c.points.len());
            assert_eq!(100.0, c.points[0].0);
            assert_eq!(200.0, c.points[1].0);
            assert_eq!(1, c.parameters.len());
            assert_eq!(12, c.parameters[0]);

            let a = hash.get("fake").unwrap();
            assert_eq!(String::from("And"), a.type_name);
            assert_eq!(2, a.gates.len());
            assert_eq!(String::from("true"), a.gates[0]);
            assert_eq!(String::from("t-cut"), a.gates[1]);
        } else {
            panic!("list request did not return a listing");
        }
    }
    #[test]
    fn list_2() {
        // List conditions whose names start with "f"

        let mut cp = make_list_conditions();
        let tracedb = trace::SharedTraceStore::new();
        let reply = cp.process_request(ConditionMessageClient::make_list("f*"), &tracedb);
        if let ConditionReply::Listing(list) = reply {
            assert_eq!(2, list.len());
            // we don't know the order in which these come back so
            // toss them into a hash indexed by name

            let mut hash = HashMap::<String, ConditionProperties>::new();
            for c in list {
                let name = c.cond_name.clone();
                hash.insert(name, c);
            }
            assert!(hash.get("true").is_none()); // not in f*

            let f = hash.get("false").unwrap();
            assert_eq!(String::from("False"), f.type_name);

            assert!(hash.get("t-cut").is_none()); // not in f*

            let a = hash.get("fake").unwrap();
            assert_eq!(String::from("And"), a.type_name);
            assert_eq!(2, a.gates.len());
            assert_eq!(String::from("true"), a.gates[0]);
            assert_eq!(String::from("t-cut"), a.gates[1]);
        } else {
            panic!("List operation did not return ConditionReply::Listing")
        }
    }
    #[test]
    fn list_3() {
        // Bad glob expression gives an error:

        let mut cp = make_list_conditions();
        let tracedb = trace::SharedTraceStore::new();
        let reply = cp.process_request(ConditionMessageClient::make_list("[Astuff"), &tracedb);
        if let ConditionReply::Error(_) = reply {
            assert!(true);
        } else {
            assert!(false);
        }
    }
    #[test]
    fn create_multi1_1() {
        let tracedb = trace::SharedTraceStore::new();
        let mut cp = ConditionProcessor::new();
        let rep = cp.process_request(
            ConditionMessageClient::make_multicut_creation("test", &[1, 2, 3], 100.0, 200.0),
            &tracedb,
        );
        assert_eq!(ConditionReply::Created, rep);

        let item = cp.dict.get("test");
        assert!(item.is_some());
        assert_eq!(String::from("MultiCut"), item.unwrap().borrow().gate_type());
    }
    #[test]
    fn create_multi2_1() {
        // Create a multi-contour -no error.

        let tracedb = trace::SharedTraceStore::new();
        let mut cp = ConditionProcessor::new();
        let rep = cp.process_request(
            ConditionMessageClient::make_multicontour_creation(
                "test",
                &[1, 2, 3],
                &vec![(100.0, 100.0), (150.0, 100.0), (125.0, 200.0)],
            ),
            &tracedb,
        );
        assert_eq!(ConditionReply::Created, rep);

        let item = cp.dict.get("test");
        assert!(item.is_some());
        assert_eq!(
            String::from("MultiContour"),
            item.unwrap().borrow().gate_type()
        );
    }
    #[test]
    fn create_multi2_2() {
        // Not enough pts -> error.

        let tracedb = trace::SharedTraceStore::new();
        let mut cp = ConditionProcessor::new();
        let rep = cp.process_request(
            ConditionMessageClient::make_multicontour_creation(
                "test",
                &[1, 2, 3],
                &vec![(100.0, 100.0), (150.0, 100.0)],
            ),
            &tracedb,
        );

        assert!(if let ConditionReply::Error(_) = rep {
            true
        } else {
            false
        });
    }
}
#[cfg(test)]
mod cnd_api_tests {
    use super::*;
    use std::sync::mpsc::*;
    use std::thread;

    // We need a fake server we can run.
    // It will understand all Condition requests and
    // Exit

    fn fake_server(reader: Receiver<Request>) {
        let mut processor = ConditionProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        loop {
            let request = reader.recv().unwrap();
            match request.message {
                MessageType::Condition(req) => {
                    request
                        .reply_channel
                        .send(Reply::Condition(processor.process_request(req, &tracedb)))
                        .expect("Failed to send reply message");
                }
                MessageType::Exit => {
                    request
                        .reply_channel
                        .send(Reply::Exiting)
                        .expect("Failed to send exiting message");
                    break;
                }
                _ => {
                    panic!("This fake server only understands Exit and Condition requests");
                }
            }
        }
    }
    fn start_server() -> (thread::JoinHandle<()>, Sender<Request>) {
        let (sender, receiver) = channel::<Request>();
        let handle = thread::spawn(move || fake_server(receiver));
        (handle, sender)
    }
    fn stop_server(handle: thread::JoinHandle<()>, send: Sender<Request>) {
        let (repl_send, repl_receive) = channel::<Reply>();
        let req = Request {
            reply_channel: repl_send,
            message: MessageType::Exit,
        };
        let reply = req.transaction(send, repl_receive);
        if let Reply::Exiting = reply {
            handle.join().expect("Fake server join failed");
        } else {
            panic!("Asked for an exit and did not get it");
        }
    }
    // Note that all tests will need for the list to work:

    #[test]
    fn list_1() {
        let (jh, send) = start_server();
        let api = ConditionMessageClient::new(&send);
        let repl = api.list_conditions("*");
        if let ConditionReply::Listing(l) = repl {
            assert_eq!(0, l.len());
        } else {
            panic!("List did not give a Listing reply");
        }
        stop_server(jh, send);
    }
    #[test]
    fn false_1() {
        let (jh, send) = start_server();
        let api = ConditionMessageClient::new(&send);
        let repl = api.create_false_condition("false");
        if let ConditionReply::Created = repl {
            let lrepl = api.list_conditions("*");
            if let ConditionReply::Listing(l) = lrepl {
                assert_eq!(1, l.len());
                assert_eq!(String::from("false"), l[0].cond_name);
                assert_eq!(String::from("False"), l[0].type_name);
            } else {
                panic!("Failed to list conditions.")
            }
        } else {
            panic!("Failed to create a false condition");
        }

        stop_server(jh, send);
    }
    #[test]
    fn true_1() {
        let (jh, send) = start_server();
        let api = ConditionMessageClient::new(&send);
        let repl = api.create_true_condition("true");
        if let ConditionReply::Created = repl {
            let lrepl = api.list_conditions("*");
            if let ConditionReply::Listing(l) = lrepl {
                assert_eq!(1, l.len());
                assert_eq!(String::from("true"), l[0].cond_name);
                assert_eq!(String::from("True"), l[0].type_name);
            } else {
                panic!("Failed to list conditions.")
            }
        } else {
            panic!("Failed to create a false condition");
        }
        stop_server(jh, send);
    }
    #[test]
    fn not_1() {
        let (jh, send) = start_server();
        let api = ConditionMessageClient::new(&send);
        api.create_false_condition("false");

        let repl = api.create_not_condition("true", "false");
        if let ConditionReply::Created = repl {
            let lrepl = api.list_conditions("true");
            if let ConditionReply::Listing(l) = lrepl {
                assert_eq!(1, l.len()); // due to filter pattern.
                assert_eq!(String::from("Not"), l[0].type_name);
                assert_eq!(1, l[0].gates.len());
                assert_eq!(String::from("false"), l[0].gates[0]);
            } else {
                panic!("failed to list the conditions");
            }
        } else {
            panic!("Failed to make not conditions");
        }
        stop_server(jh, send);
    }
    #[test]
    fn multi_cut_1() {
        let (jh, send) = start_server();

        let api = ConditionMessageClient::new(&send);
        let reply = api.create_multicut_condition("test", &[1, 2, 3], 100.0, 200.0);
        assert_eq!(ConditionReply::Created, reply);

        let l = api.list_conditions("test");
        assert_eq!(
            ConditionReply::Listing(vec![ConditionProperties {
                cond_name: String::from("test"),
                type_name: String::from("MultiCut"),
                points: vec![(100.0, 0.0), (200.0, 0.0)],
                gates: vec![],
                parameters: vec![1, 2, 3]
            },]),
            l
        );

        stop_server(jh, send);
    }
    #[test]
    fn multi_cont_1() {
        // Make a multi contour:

        let (jh, send) = start_server();
        let api = ConditionMessageClient::new(&send);

        let reply = api.create_multicontour_condition(
            "test",
            &vec![1, 2, 3],
            &vec![(10.0, 0.0), (20.0, 0.0), (15.0, 20.0)],
        );
        assert_eq!(ConditionReply::Created, reply);

        let l = api.list_conditions("test");
        assert_eq!(
            ConditionReply::Listing(vec![ConditionProperties {
                cond_name: String::from("test"),
                type_name: String::from("MultiContour"),
                points: vec![(10.0, 0.0), (20.0, 0.0), (15.0, 20.0)],
                gates: vec![],
                parameters: vec![1, 2, 3]
            },]),
            l
        );

        stop_server(jh, send);
    }

    fn make_some_conditions(send: &Sender<Request>) {
        let api = ConditionMessageClient::new(send);
        for i in 0..5 {
            let name = format!("condition.{}", i);
            if let ConditionReply::Created = api.create_true_condition(&name) {
            } else {
                panic!("Unable to creae condition {}", name);
            }
        }
    }
    #[test]
    fn and_1() {
        let (jh, send) = start_server();
        make_some_conditions(&send);
        let api = ConditionMessageClient::new(&send);
        let names = vec![
            String::from("condition.1"),
            String::from("condition.2"),
            String::from("condition.3"), // Dependent conditions.
            String::from("condition.4"),
        ];

        if let ConditionReply::Created = api.create_and_condition("and", &names) {
            if let ConditionReply::Listing(l) = api.list_conditions("and") {
                assert_eq!(1, l.len());
                assert_eq!(String::from("And"), l[0].type_name);
                assert_eq!(names.len(), l[0].gates.len());
                for (i, n) in names.iter().enumerate() {
                    assert_eq!(*n, l[0].gates[i]);
                }
            } else {
                panic!("Listing failed in some way");
            }
        } else {
            panic!("Could not make and condition");
        }
        stop_server(jh, send);
    }
    #[test]
    fn or_1() {
        let (jh, send) = start_server();
        make_some_conditions(&send);
        let api = ConditionMessageClient::new(&send);
        let names = vec![
            String::from("condition.1"),
            String::from("condition.2"),
            String::from("condition.3"), // Dependent conditions.
            String::from("condition.4"),
        ];

        if let ConditionReply::Created = api.create_or_condition("or", &names) {
            if let ConditionReply::Listing(l) = api.list_conditions("or") {
                assert_eq!(1, l.len());
                assert_eq!(String::from("Or"), l[0].type_name);
                assert_eq!(names.len(), l[0].gates.len());
                for (i, n) in names.iter().enumerate() {
                    assert_eq!(*n, l[0].gates[i]);
                }
            } else {
                panic!("Listing failed in some way");
            }
        } else {
            panic!("Could not make and condition");
        }
        stop_server(jh, send);
    }
    #[test]
    fn cut_1() {
        let (jh, send) = start_server();
        let api = ConditionMessageClient::new(&send);
        if let ConditionReply::Created = api.create_cut_condition("cut", 12, 100.0, 250.0) {
            if let ConditionReply::Listing(l) = api.list_conditions("*") {
                assert_eq!(1, l.len());
                assert_eq!(String::from("Cut"), l[0].type_name);
                assert_eq!(2, l[0].points.len());
                assert_eq!(100.0, l[0].points[0].0);
                assert_eq!(250.0, l[0].points[1].0);
                assert_eq!(1, l[0].parameters.len());
                assert_eq!(12, l[0].parameters[0]);
            } else {
                panic!("Failed to get Listing from list request");
            }
        } else {
            panic!("Did not get Created back from creation of cut");
        }
        stop_server(jh, send);
    }
    // just make some points for either band or contour.

    fn make_points() -> Vec<(f64, f64)> {
        vec![(0.0, 100.0), (100.0, 100.0), (200.0, 50.0), (400.0, 0.0)]
    }
    #[test]
    fn band_1() {
        let (jh, send) = start_server();
        let api = ConditionMessageClient::new(&send);
        let points = make_points();

        if let ConditionReply::Created = api.create_band_condition("band", 5, 6, &points) {
            if let ConditionReply::Listing(l) = api.list_conditions("*") {
                assert_eq!(1, l.len());
                assert_eq!(String::from("Band"), l[0].type_name);
                assert_eq!(points.len(), l[0].points.len());
                for (i, (x, y)) in points.iter().enumerate() {
                    assert_eq!(*x, l[0].points[i].0);
                    assert_eq!(*y, l[0].points[i].1);
                }
                assert_eq!(2, l[0].parameters.len());
                assert_eq!(5, l[0].parameters[0]);
                assert_eq!(6, l[0].parameters[1]);
            } else {
                panic!("Failed to get listing");
            }
        } else {
            panic!("Failed to make band condition.");
        }
        stop_server(jh, send);
    }
    #[test]
    fn contour_1() {
        let (jh, send) = start_server();
        let api = ConditionMessageClient::new(&send);
        let points = make_points();

        if let ConditionReply::Created = api.create_contour_condition("contour", 5, 6, &points) {
            if let ConditionReply::Listing(l) = api.list_conditions("*") {
                assert_eq!(1, l.len());
                assert_eq!(String::from("Contour"), l[0].type_name);
                assert_eq!(points.len(), l[0].points.len());
                for (i, (x, y)) in points.iter().enumerate() {
                    assert_eq!(*x, l[0].points[i].0);
                    assert_eq!(*y, l[0].points[i].1);
                }
                assert_eq!(2, l[0].parameters.len());
                assert_eq!(5, l[0].parameters[0]);
                assert_eq!(6, l[0].parameters[1]);
            } else {
                panic!("Failed to get listing");
            }
        } else {
            panic!("Failed to make band condition.");
        }
        stop_server(jh, send);
    }
    #[test]
    fn delete_1() {
        let (jh, send) = start_server();
        let api = ConditionMessageClient::new(&send);
        make_some_conditions(&send);
        if let ConditionReply::Deleted = api.delete_condition("condition.0") {
            if let ConditionReply::Listing(l) = api.list_conditions("condition.0") {
                assert_eq!(0, l.len());
            } else {
                panic!("failed to list conditions");
            }
        } else {
            panic!("Not Deleted answer back from delete_condition");
        }
        stop_server(jh, send);
    }
    // Check that we get a replacement status if we replace a condition:

    #[test]
    fn replace_1() {
        let (jh, send) = start_server();
        let api = ConditionMessageClient::new(&send);
        make_some_conditions(&send);

        if let ConditionReply::Replaced = api.create_false_condition("condition.1") {
            if let ConditionReply::Listing(l) = api.list_conditions("condition.1") {
                assert_eq!(1, l.len());
                assert_eq!(String::from("False"), l[0].type_name);
            }
        } else {
            panic!("Replacement did not return replaced reply.");
        }
        stop_server(jh, send);
    }
}
// Ensure that traces fire when appropriate for conditions:

#[cfg(test)]
mod condition_trace_tests {
    use super::*;
    use crate::trace;
    use std::time::Duration;

    #[test]
    fn create_1() {
        // Creating a new condition fires a condition trace:

        let tracedb = trace::SharedTraceStore::new();
        let mut cp = ConditionProcessor::new();

        let token = tracedb.new_client(Duration::from_secs(10));

        let rep = cp.process_request(
            ConditionMessageClient::make_true_creation("true-condition"),
            &tracedb,
        );
        assert_eq!(ConditionReply::Created, rep);

        // check for the trace:

        let traces = tracedb.get_traces(token).expect("Getting traces");
        assert_eq!(1, traces.len());

        assert!(
            if let trace::TraceEvent::ConditionCreated(name) = traces[0].event() {
                assert_eq!("true-condition", name);
                true
            } else {
                false
            }
        )
    }
    #[test]
    fn modify_1() {
        // Create a condition; modifying its definition results in a ConditionModified trace:

        let tracedb = trace::SharedTraceStore::new();
        let mut cp = ConditionProcessor::new();

        let rep = cp.process_request(
            ConditionMessageClient::make_true_creation("true-condition"),
            &tracedb,
        );
        assert_eq!(ConditionReply::Created, rep);

        // Registering the trace client here makes sure we don't get the crated trace:

        let token = tracedb.new_client(Duration::from_secs(10));

        let rep = cp.process_request(
            ConditionMessageClient::make_false_creation("true-condition"),
            &tracedb,
        );
        assert_eq!(ConditionReply::Replaced, rep);

        let traces = tracedb.get_traces(token).expect("Getting traces");
        assert_eq!(1, traces.len());

        assert!(
            if let trace::TraceEvent::ConditionModified(name) = traces[0].event() {
                assert_eq!("true-condition", name);
                true
            } else {
                false
            }
        )
    }
    #[test]
    fn delete_1() {
        // make sure that deleting a condition fires a trace:

        let tracedb = trace::SharedTraceStore::new();
        let mut cp = ConditionProcessor::new();

        let rep = cp.process_request(
            ConditionMessageClient::make_true_creation("true-condition"),
            &tracedb,
        );
        assert_eq!(ConditionReply::Created, rep);

        // Registering the trace client here makes sure we don't get the crated trace:

        let token = tracedb.new_client(Duration::from_secs(10));

        let rep = cp.process_request(
            ConditionMessageClient::make_delete("true-condition"),
            &tracedb,
        );
        assert_eq!(ConditionReply::Deleted, rep);

        let traces = tracedb.get_traces(token).expect("Getting traces");
        assert_eq!(1, traces.len());

        assert!(
            if let trace::TraceEvent::ConditionDeleted(name) = traces[0].event() {
                assert_eq!("true-condition", name);
                true
            } else {
                false
            }
        )
    }
}
#[cfg(test)]
mod recons_contour_tests {
    use super::*;
    use crate::messaging::condition_messages;

    #[test]
    fn err_1() {
        // Contour described is not actually a contour:

        let desc = condition_messages::ConditionProperties {
            cond_name: String::from("junk"),
            type_name: String::from("Not a contour"),
            points: vec![],
            gates: vec![],
            parameters: vec![],
        };
        assert!(reconstitute_contour(desc).is_err());
    }
    #[test]
    fn err_2() {
        // Some how too few points in a thing that claims to be a contour

        let desc = condition_messages::ConditionProperties {
            cond_name: String::from("junk"),
            type_name: String::from("Contour"),
            points: vec![(100.0, 100.0), (200.0, 100.0)],
            gates: vec![],
            parameters: vec![],
        };
        assert!(reconstitute_contour(desc).is_err());
    }
    #[test]
    fn ok_1() {
        let pts = vec![(100.0, 100.0), (200.0, 100.0), (150.0, 150.0)]; // needed for later assertion:
        let desc = condition_messages::ConditionProperties {
            cond_name: String::from("junk"),
            type_name: String::from("Contour"),
            points: pts.clone(),
            gates: vec![],
            parameters: vec![],
        };
        let result = reconstitute_contour(desc);
        assert!(result.is_ok());
        let contour = result.unwrap();

        let contour_points = contour.get_points();
        assert_eq!(pts.len(), contour_points.len());
        for (i, p) in pts.iter().enumerate() {
            assert_eq!(p.0, contour_points[i].x, "X mismatch on point {}", i);
            assert_eq!(p.1, contour_points[i].y, "Y mismatch on point {}", i);
        }
    }
}
