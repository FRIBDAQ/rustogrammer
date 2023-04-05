//!  This module contains handlers for the Rocket web
//!  server that implement the REST interface for
//!  that user interfaces can be built against.
//!  The interface is as compatible with SpecTcl's
//!  REST interface to the extent possible given
//!  the differences in the two programs.
//!  
//!  For example, the SpecTcl REST interface allows
//!  clients to execute arbitrary Tcl code in the
//!  SpecTcl Tcl interpreter but Rustogramer has no
//!  Tcl interpreter so therefore any attempt to use
//!  that interface by a rustogramer client results in
//!  an error return indicating there is no Tcl
//!  interpreter.
//!  
//!   The REST interface consists of a bunch of
//!   Quasi-independent domains of URLS.  Each of those
//!   domains is pulled out into a separate submodule
//!   file that is then re-exported in this top level module
//!   Which at most will define common data structures
//!   that can be used across the modules.
//!  
//!  In general the result of a request is a JSON encoded
//!  struct with two fields:
//!  
//! *  status - which on success is the value "OK" and an
//! error string if the request failed and that failure is caught
//! by the handler.
//! *  detail - whose contents vary depending on the requrest.
//!

// Re exports:

pub mod parameter;
use crate::messaging::Request;
pub use parameter as rest_parameter;
use std::sync::{mpsc, Mutex};
use std::thread;

pub struct HistogramState {
    pub state: Mutex<(thread::JoinHandle<()>, mpsc::Sender<Request>)>,
}
