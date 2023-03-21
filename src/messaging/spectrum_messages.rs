//!  Provides message and reply structures for the message passing
//!  interfaces to spectra in the histogrammer.
//!  Messages allow us to:
//! *   Create and delete histograms of various sorts
//! *   Apply gates to histograms.  These gates must be
//! conditions that are defined in a ConditionProcessor's dictionary.
//! *   Ungate histograms.
//! *   Clear the contents of individual or groups of histograms
//! *   Provide an event to the spectrum store for histograming.
//! *   Get descriptions of histograms.

use super::*;
use crate::conditions;
use crate::parameters;
use crate::spectra::*;

#[derive(Clone, Debug, PartialEq)]
pub struct AxisSpecification {
    pub low: f64,
    pub high: f64,
    pub bins: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Channel {
    pub x: f64,
    pub y: f64,
    pub value: f64,
}
pub type SpectrumContents = Vec<Channel>;
#[derive(Clone, Debug, PartialEq)]
pub struct SpectrumProperties {
    pub name: String,
    pub type_name: String,
    pub xparams: Vec<String>,
    pub yparams: Vec<String>,
    pub xaxis: Option<AxisSpecification>,
    pub yaxis: Option<AxisSpecification>,
    pub gate: Option<String>,
}
///  Defines the requests that can be made of the spectrum
/// part of the histogram server
///
#[derive(Clone, Debug, PartialEq)]
pub enum SpectrumRequest {
    Create1D {
        name: String,
        parameter: String,
        axis: AxisSpecification,
    },
    CreateMulti1D {
        name: String,
        params: Vec<String>,
        axis: AxisSpecification,
    },
    CreateMulti2D {
        name: String,
        params: Vec<String>,
        xaxis: AxisSpecification,
        yaxis: AxisSpecification,
    },
    CreatePGamma {
        name: String,
        xparams: Vec<String>,
        yparams: Vec<String>,
        xaxis: AxisSpecification,
        yaxis: AxisSpecification,
    },
    CreateSummary {
        name: String,
        params: Vec<String>,
        yaxis: AxisSpecification,
    },
    Create2D {
        name: String,
        xparam: String,
        yparam: String,
        xaxis: AxisSpecification,
        yaxis: AxisSpecification,
    },
    Create2dSum {
        name: String,
        xparams: Vec<String>,
        yparams: Vec<String>,
        xaxis: AxisSpecification,
        yaxis: AxisSpecification,
    },
    Delete(String),
    List(String),
    Gate {
        spectrum: String,
        gate: String,
    },
    Ungate(String),
    Clear(String),
    GetContents {
        name: String,
        xlow: f64,
        xhigh: f64,
        ylow: f64,
        yhigh: f64,
    },
    Events(Vec<parameters::Event>),
}

/// Defines the replies the spectrum par tof the histogram
/// server can return
#[derive(Clone, Debug, PartialEq)]
pub enum SpectrumReply {
    Error(String),
    Created,                          // Spectrum created.
    Deleted,                          // Spectrum deleted.
    Gated,                            // Condition applied.
    Ungated,                          // Spectrum ungated.
    Cleared,                          // Spectra cleared.
    Contents(SpectrumContents),       // Contents of a spectrum.
    Listing(Vec<SpectrumProperties>), // List of spectrum props.
    Processed,                        // Events processed.
}

///
/// SpectrumProcessor is the struct that processes
/// spectrum requests.  Some requests will need
/// a parameter and condition dictionary.  
/// Note that the implementation is divorced from the
/// actual message.  This makes testing the impl easier.
pub struct SpectrumProcessor {
    dict: SpectrumDictionary,
}

impl SpectrumProcessor {
    /// Construction

    pub fn new() -> SpectrumProcessor {
        SpectrumProcessor {
            dict: SpectrumDictionary::new(),
        }
    }
    /// Process requests returning replies:

    pub fn process_request(
        &mut self,
        req: SpectrumRequest,
        params: &parameters::ParameterDictionary,
        conditions: &conditions::ConditionDictionary,
    ) -> SpectrumReply {
        SpectrumReply::Error(String::from("Placeholder method"))
    }
}
