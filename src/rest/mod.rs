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

pub mod apply;
pub mod channel;
pub mod data_processing;
pub mod evbunpack;
pub mod exit;
pub mod filter;
pub mod fit;
pub mod fold;
pub mod gates;
pub mod getstats;
pub mod integrate;
pub mod parameter;
pub mod ringversion;
pub mod sbind;
pub mod shm;
pub mod spectrum;
pub mod spectrumio;
pub mod unbind;
pub mod unimplemented;
pub mod version;

pub use parameter as rest_parameter;

use crate::messaging::parameter_messages::ParameterMessageClient;
use crate::messaging::Request;
use crate::processing;
use crate::sharedmem::binder;
use portman_client;
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use std::sync::{mpsc, Mutex};

pub struct HistogramState {
    pub histogramer: Mutex<mpsc::Sender<Request>>,
    pub binder: Mutex<mpsc::Sender<binder::Request>>,
    pub processing: Mutex<processing::ProcessingApi>,
    pub portman_client: Option<portman_client::Client>,
}

pub type OptionalStringVec = Option<Vec<String>>;
pub type OptionalString = Option<String>;
pub type OptionalF64Vec = Option<Vec<f64>>;
pub type OptionalFlag = Option<bool>;

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct GenericResponse {
    status: String,
    detail: String,
}
impl GenericResponse {
    pub fn ok(detail: &str) -> GenericResponse {
        GenericResponse {
            status: String::from("OK"),
            detail: String::from(detail),
        }
    }
    pub fn err(status: &str, detail: &str) -> GenericResponse {
        GenericResponse {
            status: String::from(status),
            detail: String::from(detail),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct StringArrayResponse {
    status: String,
    pub detail: Vec<String>,
}

impl StringArrayResponse {
    pub fn new(status: &str) -> StringArrayResponse {
        StringArrayResponse {
            status: String::from(status),
            detail: vec![],
        }
    }
    #[allow(dead_code)]
    pub fn push(&mut self, s: &str) {
        self.detail.push(String::from(s));
    }
}

// Utility method to return the name of a parameter given its id

fn find_parameter_by_id(id: u32, state: &State<HistogramState>) -> Option<String> {
    let api = ParameterMessageClient::new(&state.inner().histogramer.lock().unwrap());
    if let Ok(l) = api.list_parameters("*") {
        for p in l {
            if p.get_id() == id {
                return Some(p.get_name());
            }
        }
        None
    } else {
        None // Error is non for now.
    }
}
// utility to find a parameter given it's name:

fn find_parameter_by_name(name: &str, state: &State<HistogramState>) -> Option<u32> {
    let api = ParameterMessageClient::new(&state.inner().histogramer.lock().unwrap());
    if let Ok(l) = api.list_parameters(name) {
        if l.len() > 0 {
            Some(l[0].get_id())
        } else {
            None
        }
    } else {
        None
    }
}
