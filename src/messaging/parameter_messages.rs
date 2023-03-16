//!  This module defines the request and reply
//!  messages that a request for the parameter subsystem might
//!  have.  It is assumed that a handler of these messages would
//!  include a parameter dictionary that these messages manipulate
//!  The message set is:
//!
//!  *    Create - creates a new parameter.
//!  *    List   - list the parameters and their properties
//!  that match a glob pattern.
//!  *    SetMetaData - Modifies the metadata for a parameter.
//!
//!  Note that it is a design property of parameters that, while they
//!  can be modified they *cannot* be deleted.
//!
//!  
use super::MessageType;
use super::Reply;
use super::Request;

use glob::Pattern;

use crate::parameters::{Parameter, ParameterDictionary};
use std::sync::mpsc;

/// ParameterRequest
/// Is the enum that defines requests of the parameter subsystem.
///
#[derive(Clone, Debug, PartialEq)]
pub enum ParameterRequest {
    Create(String),
    List(String),
    SetMetaData {
        name: String,
        bins: Option<u32>,
        limits: Option<(f64, f64)>,
        units: Option<String>,
        description: Option<String>,
    },
}
/// The following are possible reply mesages:
#[derive(Clone, Debug, PartialEq)]
pub enum ParameterReply {
    Error(String),
    Created,
    Listing(Vec<Parameter>),
    Modified,
}
/// Result types:

pub type ParameterResult = Result<(), String>; // /Generic result.
pub type ListResult = Result<Vec<Parameter>, String>; // Result from list request.

// Internal functions to create paramete requests:

fn make_create_request(name: &str) -> MessageType {
    let req_data = ParameterRequest::Create(String::from(name));
    MessageType::Parameter(req_data)
}
fn make_list_request(pattern: &str) -> MessageType {
    let req_data = ParameterRequest::List(String::from(pattern));
    MessageType::Parameter(req_data)
}
fn make_modify_request(
    name: &str,
    bins: Option<u32>,
    limits: Option<(f64, f64)>,
    units: Option<String>,
    description: Option<String>,
) -> MessageType {
    let req_data = ParameterRequest::SetMetaData {
        name: String::from(name),
        bins,
        limits,
        units,
        description,
    };
    MessageType::Parameter(req_data)
}

///
/// Request the creation of a new parameter.
///  -   req_chan is the channel for requests to the histogramer.
///  -   rep_send is the channel on which replies will be sent by
/// the histogrammer
///  -   rep_rcv is the channel on which replies will be recieve by us.
///  -   name is the name to be given to the new parameter.
///
/// A result is returned extracted from the message we get back.
/// The payload for Err is a human readable reason for the failure.
///
pub fn create_parameter(
    req_chan: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_rcv: mpsc::Receiver<Reply>,
    name: &str,
) -> ParameterResult {
    let create = make_create_request(name);
    let req = Request {
        reply_channel: rep_send,
        message: create,
    };
    let result = req.transaction(req_chan, rep_rcv);

    // Must be a Parameter type:

    if let Reply::Parameter(valid) = result {
        match valid {
            ParameterReply::Error(s) => Err(s),
            ParameterReply::Created => return Ok(()),
            ParameterReply::Listing(_) => Err(String::from("BUG!! Create got a Listing reply")),
            ParameterReply::Modified => Err(String::from("BUG!! Create got a Modified reply")),
        }
    } else {
        Err(String::from("BUG!!! : Invalid reply type from histogramer"))
    }
}
/// Request a list of the set of parameters that match a specified pattern.
///
///  -   req_chan is the channel for requests to the histogramer.
///  -   rep_send is the channel on which replies will be sent by
/// the histogrammer
///  -   rep_rcv is the channel on which replies will be recieve by us.
///  -   pattern is a glob pattern to match ("*" matches anything).
///
/// The result is a ListResult, which  on success is a list of
/// the parameter objects (copies) that satisfy the pattern.  This
/// can, of course, be empty.
/// On error, the payload is a human readable error string.
///
pub fn list_parameters(
    req_chan: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_rcv: mpsc::Receiver<Reply>,
    pattern: &str,
) -> ListResult {
    let list = make_list_request(pattern);
    let req = Request {
        reply_channel: rep_send,
        message: list,
    };

    let result = req.transaction(req_chan, rep_rcv);

    // must be a parameter type else that's bad.
    // Must further be a Listing else that's bad too:

    if let Reply::Parameter(valid) = result {
        if let ParameterReply::Listing(params) = valid {
            Ok(params)
        } else {
            Err(String::from(
                "Bug: Invalid histogram Parameter response to Parmeter::SetMetadata request",
            ))
        }
    } else {
        Err(String::from("BUG!!! : Invalid reply type from histogramer"))
    }
}
///
/// Modify selected metadata in a parameter.  The things that
/// can be modified (suggested limits, binning, units and description)
/// are passed as options if an option is None, no modification of that
/// metadata will be done, if Some the payload of some indicates the
/// desired modifications.
/// Parameters:
///
///  -   req_chan is the channel for requests to the histogramer.
///  -   rep_send is the channel on which replies will be sent by
/// the histogrammer
///  -   rep_rcv is the channel on which replies will be recieve by us.
///  -   name is the name of the parameter to modify.
///  -   bins - Some is a u32 number of suggested bins for he parameter.
///  -   limits - Some is a (f64, f64) with .0 the suggested low limit
/// and .1 the suggested high limt.
///  -    units - Some is a new units of measure string.
///  -    description - Some is a new description of the parameter.
///
/// The return is the generic ParameterResult
pub fn modify_parameter_metadata(
    req_chan: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_rcv: mpsc::Receiver<Reply>,
    name: &str,
    bins: Option<u32>,
    limits: Option<(f64, f64)>,
    units: Option<String>,
    description: Option<String>,
) -> ParameterResult {
    let modify = make_modify_request(name, bins, limits, units, description);
    let req = Request {
        reply_channel: rep_send,
        message: modify,
    };
    let reply = req.transaction(req_chan, rep_rcv);

    if let Reply::Parameter(valid) = reply {
        if let ParameterReply::Modified = valid {
            Ok(())
        } else {
            Err(String::from(
                "Bug: Invalid histogram Parameter response to Parmeter::Modify request",
            ))
        }
    } else {
        Err(String::from("BUG!!! : Invalid reply type from histogramer"))
    }
}

/// ParameterProcessor is a struct that encapsulates a ParmeterDictionary
/// and implements code that can process ParameterRequest objects
/// using that dictionary producing the correct ParameterReply object.
///
pub struct ParameterProcessor {
    dict: ParameterDictionary,
}
impl ParameterProcessor {
    // Private methods:

    // Create a new parameter

    fn create(&mut self, name: &str) -> ParameterReply {
        let result = self.dict.add(name);
        match result {
            Err(s) => ParameterReply::Error(s),
            Ok(_) => ParameterReply::Created,
        }
    }
    fn list(&self, pattern: &str) -> ParameterReply {
        let mut result = Vec::<Parameter>::new();
        let pat = Pattern::new(pattern);
        if pat.is_err() {
            return ParameterReply::Error(String::from(pat.unwrap_err().msg));
        }
        let pat = pat.unwrap();
        for (name, p) in self.dict.iter() {
            if pat.matches(&name) {
                result.push(p.clone());
            }
        }
        ParameterReply::Listing(result)
    }
    fn modify(
        &mut self,
        name: &str,
        bins: Option<u32>,
        limits: Option<(f64, f64)>,
        units: Option<String>,
        desc: Option<String>,
    ) -> ParameterReply {
        if let Some(p) = self.dict.lookup_mut(name) {
            if bins.is_some() {
                p.set_bins(bins.unwrap());
            }
            if limits.is_some() {
                let lims = limits.unwrap();
                p.set_limits(lims.0, lims.1);
            }
            if units.is_some() {
                p.set_units(&units.unwrap());
            }
            if desc.is_some() {
                p.set_description(&desc.unwrap());
            }
            ParameterReply::Modified
        } else {
            ParameterReply::Error(format!("Parameter {} does not exist", name))
        }
    }

    /// Create a new processor.
    pub fn new() -> ParameterProcessor {
        ParameterProcessor {
            dict: ParameterDictionary::new(),
        }
    }
    /// Process a request returning the reply.
    ///
    fn process_request(&mut self, req: ParameterRequest) -> ParameterReply {
        match req {
            ParameterRequest::Create(name) => self.create(&name),
            ParameterRequest::List(pattern) => self.list(&pattern),
            ParameterRequest::SetMetaData {
                name,
                bins,
                limits,
                units,
                description,
            } => self.modify(&name, bins, limits, units, description),
        }
    }
}

// Test for public functions note this implicitly tests the
// public functions in mod.rs
// Note that tests will, of necessity need to create threads
// that deal with a single request in a well defined way and
// then exit.
#[cfg(test)]
mod param_msg_tests {
    use super::*;
    use crate::parameters::Parameter;
    use std::sync::mpsc::channel;
    use std::thread;

    #[test]
    fn create_1() {
        // Ok return.
        let (req_send, req_rcv) = channel();
        let (rep_send, rep_rcv) = channel();

        let tjh = thread::spawn(move || {
            let req = Request::get_request(req_rcv);
            // success:

            let rep = Reply::Parameter(ParameterReply::Created);
            req.send_reply(rep);
        });

        let reply = create_parameter(req_send, rep_send, rep_rcv, "junk");
        tjh.join().unwrap();

        assert!(reply.is_ok()); // Was received and properly processed.
    }
    #[test]
    fn create_2() {
        // Error reply:

        let (req_send, req_rcv) = channel();
        let (rep_send, rep_rcv) = channel();
        let tjh = thread::spawn(move || {
            let req = Request::get_request(req_rcv);
            // Duplicate4 e.g

            let rep = Reply::Parameter(ParameterReply::Error(String::from(
                "Duplicate parameter 'junk'",
            )));
            req.send_reply(rep);
        });

        let reply = create_parameter(req_send, rep_send, rep_rcv, "junk");
        tjh.join().unwrap();
        assert!(reply.is_err());
        assert_eq!(
            String::from("Duplicate parameter 'junk'"),
            reply.unwrap_err()
        );
    }
    #[test]
    fn list_1() {
        // Successful list of  a parameter:

        let (req_send, req_rcv) = channel();
        let (rep_send, rep_rcv) = channel();
        let tjh = thread::spawn(move || {
            let req = Request::get_request(req_rcv);

            let mut pvec = vec![Parameter::new("a", 1), Parameter::new("b", 2)];
            pvec[0].set_limits(0.0, 4096.0);
            pvec[0].set_bins(4096);
            pvec[0].set_units("chans");
            pvec[0].set_description("Raw channel a");

            let rep = Reply::Parameter(ParameterReply::Listing(pvec));
            req.send_reply(rep);
        });
        let reply = list_parameters(req_send, rep_send, rep_rcv, "*");
        assert!(reply.is_ok());
        let pars = reply.unwrap();
        assert_eq!(2, pars.len());
        assert_eq!(String::from("a"), pars[0].get_name());
        assert_eq!(1, pars[0].get_id());

        let lims = pars[0].get_limits();
        assert!(lims.0.is_some());
        assert_eq!(0.0, lims.0.unwrap());
        assert!(lims.1.is_some());
        assert_eq!(4096.0, lims.1.unwrap());

        let bins = pars[0].get_bins();
        assert!(bins.is_some());
        assert_eq!(4096, bins.unwrap());

        let units = pars[0].get_units();
        assert!(units.is_some());
        assert_eq!(String::from("chans"), units.unwrap());

        let d = pars[0].get_description();
        assert!(d.is_some());
        assert_eq!(String::from("Raw channel a"), d.unwrap());

        assert_eq!(String::from("b"), pars[1].get_name());
        assert_eq!(2, pars[1].get_id());

        assert_eq!((None, None), pars[1].get_limits());
        assert_eq!(None, pars[1].get_bins());
        assert_eq!(None, pars[1].get_units());
        assert_eq!(None, pars[1].get_description())
    }
    #[test]
    fn mod_1() {
        // Successful modify of metadata:

        let (req_send, req_rcv) = channel();
        let (rep_send, rep_rcv) = channel();
        let tjh = thread::spawn(move || {
            let req = Request::get_request(req_rcv);
            // Duplicate4 e.g

            let rep = Reply::Parameter(ParameterReply::Modified);
            req.send_reply(rep);
        });

        let reply =
            modify_parameter_metadata(req_send, rep_send, rep_rcv, "junk", None, None, None, None);
        assert!(reply.is_ok());
    }
    #[test]
    fn mod_2() {
        // Failed modify of metadata:

        let (req_send, req_rcv) = channel();
        let (rep_send, rep_rcv) = channel();
        let tjh = thread::spawn(move || {
            let req = Request::get_request(req_rcv);
            // Duplicate4 e.g

            let rep = Reply::Parameter(ParameterReply::Error(String::from(
                "No such parameter 'junk'",
            )));
            req.send_reply(rep);
        });

        let reply =
            modify_parameter_metadata(req_send, rep_send, rep_rcv, "junk", None, None, None, None);
        tjh.join().unwrap();
        assert!(reply.is_err());
    }
}
// Tests for the ParameterProcessor implementation.
#[cfg(test)]
mod pprocessor_tests {
    use super::*;
    use std::collections::HashSet;
    fn create_req(name: &str) -> ParameterRequest {
        let result = make_create_request(name);
        if let MessageType::Parameter(req) = result {
            return req;
        } else {
            panic!("make_create_request did not make a ParameterRequest object");
        }
    }
    fn list_req(patt: &str) -> ParameterRequest {
        let result = make_list_request(patt);
        if let MessageType::Parameter(req) = result {
            return req;
        } else {
            panic!("make_list_request did not make a ParameterRequest object");
        }
    }
    // Make 10 parameters named param.0..9
    // and 10 more parameter named others.0..9
    //
    fn create_some_params() -> ParameterProcessor {
        let mut p = ParameterProcessor::new();
        for i in 0..10 {
            let name = format!("param.{}", i);
            assert_eq!(
                ParameterReply::Created,
                p.process_request(create_req(&name))
            );
            let name = format!("others.{}", i);
            assert_eq!(
                ParameterReply::Created,
                p.process_request(create_req(&name))
            );
        }

        p
    }

    #[test]
    fn new_1() {
        let pp = ParameterProcessor::new();
        assert_eq!(0, pp.dict.len())
    }
    #[test]
    fn add_1() {
        // Create a single parameter in an empty processor:
        // Should return Created reply:

        let mut pp = ParameterProcessor::new();

        let result = pp.process_request(create_req("Test"));
        assert_eq!(ParameterReply::Created, result);

        // Make sure the parameter is in the dict and properly formed:

        assert_eq!(1, pp.dict.len());
        let p = pp.dict.lookup("Test").expect("Failed parameter lookup");
        assert_eq!(String::from("Test"), p.get_name());
        assert_eq!(1, p.get_id()); // Ids start at 1 I think.
        assert_eq!((None, None), p.get_limits());
        assert!(p.get_bins().is_none());
        assert!(p.get_units().is_none());
        assert!(p.get_description().is_none());
    }
    #[test]
    fn add_2() {
        // Adding a duplicate fails with that reply.

        let mut pp = ParameterProcessor::new();
        pp.dict.add("test").expect("Failed to add to empty dict");
        let result = pp.process_request(create_req("test"));
        if let ParameterReply::Error(_) = result {
            assert!(true); // Correct result.
        } else {
            assert!(false); // shouild have been an error.
        }
    }
    #[test]
    fn add_3() {
        // add several parameters:

        let mut pp = ParameterProcessor::new();

        if let ParameterReply::Error(s) = pp.process_request(create_req("test.1")) {
            panic!("{}", s);
        }
        if let ParameterReply::Error(s) = pp.process_request(create_req("test.2")) {
            panic!("{}", s);
        }
        if let ParameterReply::Error(s) = pp.process_request(create_req("test.3")) {
            panic!("{}", s);
        }

        assert_eq!(3, pp.dict.len());
        assert!(pp.dict.lookup("test.1").is_some());
        assert!(pp.dict.lookup("test.2").is_some());
        assert!(pp.dict.lookup("test.3").is_some());
    }
    #[test]
    fn list_1() {
        // all inclusive list:

        let mut pp = create_some_params();

        if let ParameterReply::Listing(v) = pp.process_request(list_req("*")) {
            assert_eq!(20, v.len());
            // We're not gauranteed about the order of parameter names
            // so make a set with all of them.  Meanwhile we expect:

            let expected_names = vec![
                "param.1", "others.1", "param.2", "others.2", "param.3", "others.3", "param.4",
                "others.4", "param.5", "others.5", "param.6", "others.6", "param.7", "others.7",
                "param.8", "others.8", "param.9", "others.9",
            ];
            let mut got = HashSet::new();
            for p in v.iter() {
                got.insert(p.get_name());
            }
            // make sure all expected names are in the list:

            for name in expected_names {
                assert!(got.contains(name), "Missing {}", name);
            }
        } else {
            panic!("process_request for list returned the wrong reply type");
        }
    }
    #[test]
    fn list_2() {
        // list with pattern:

        let mut pp = create_some_params();
        if let ParameterReply::Listing(v) = pp.process_request(list_req("param.*")) {
            assert_eq!(10, v.len());
            let expected_names = vec![
                "param.1",
                "param.2",
                "param.3",
                "param.4",
                "param.5",
                "param.6",
                "param.7",
                "param.8",
                "param.9",
            ];
            let mut got = HashSet::new();
            for p in v.iter() {
                got.insert(p.get_name());
            }
            for name in expected_names {
                assert!(got.contains(name), "Missing {}", name);
            }
        } else {
            panic!("process_request for list returned the wrong reply type");
        }
    }
    #[test]
    fn list_3() {
        // Pattern with no matches - ok but emtpy list
    }
    #[test]
    fn list_4() {
        // Glob pattern syntax errors ->Error return.
    }
}
