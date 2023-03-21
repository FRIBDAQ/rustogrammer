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

use crate::conditions;
use crate::parameters;
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct AxisSpecification {
    pub low : f64,
    pub high : f64,
    pub bins : u32
}
#[derive(Clone, Debug, PartialEq)]
pub struct Channel {
    pub x : f64,
    pub y : f64,
    pub value : f64
}
pub type SpectrumContents = Vec<Channel>;
#[derive(Clone, Debug, PartialEq)]
pub struct SpectrumProperties {
    pub name : String,
    pub type_name : String,
    pub xparams : Vec<String>,
    pub yparams : Vec<String>,
    pub xaxis : Option<AxisSpecification>,
    pub yaxis : Option<AxisSpecification>,
    pub gate : Option<String>
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpectrumRequest {
    Create1D{
        name : String,
        parameter : String,
        axis : AxisSpecification
    },
    CreateMulti1D {
        name : String,
        params : Vec<String>,
        axis: AxisSpecification
    },
    CreateMulti2D {
        name: String,
        params : Vec<String>,
        xaxis : AxisSpecification,
        yaxis : AxisSpecification,
    },
    CreatePGamma {
        name : String,
        xparams : Vec<String>,
        yparams : Vec<String>,
        xaxis : AxisSpecification,
        yaxis: AxisSpecification,
    },
    CreateSummary {
        name : String,
        params : Vec<String>,
        yaxis : AxisSpecification
    },
    Create2D {
        name : String,
        xparam : String,
        yparam : String,
        xaxis  : AxisSpecification,
        yaxis : AxisSpecification
    },
    Create2dSum {
        name : String,
        xparams : Vec<String>,
        yparams : Vec<String>,
        xaxis : AxisSpecification,
        yaxis : AxisSpecification,
    },
    Delete(String),
    List(String),
    Gate {
        spectrum : String,
        gate : String
    },
    Ungate(String),
    Events {
        events : Vec<parameters::Event>
    },
    Clear(String),
    GetContents {
        name : String,
        xlow : f64,
        xhigh : f64,
        ylow : f64,
        yhigh: f64
    }
}