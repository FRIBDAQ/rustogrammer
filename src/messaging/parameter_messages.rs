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

use crate::parameters::Parameter;
use std::sync::mpsc;

/// ParameterRequest
/// Is the enum that defines requests of the parameter subsystem.
///
#[derive(Clone)]
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
#[derive(Clone)]
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
        assert_eq!(String::from("No such parameter 'junk'"), reply.unwrap_err());
    }
}
