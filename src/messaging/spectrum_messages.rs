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
use crate::spectra;

use glob::Pattern;
use std::cell::RefCell;
use std::rc::Rc;

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
    Create2DSum {
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
    dict: spectra::SpectrumDictionary,
}

type ParamLookupResult = Result<u32, String>;
type ParamsLookupResult = Result<Vec<u32>, String>;
impl SpectrumProcessor {
    // private methods:

    // Make a 1-d spectrum:

    fn make_1d(
        &mut self,
        name: &str,
        parameter: &str,
        axis: &AxisSpecification,
        pdict: &parameters::ParameterDictionary,
    ) -> SpectrumReply {
        let sname = String::from(name);
        if !self.dict.contains_key(name) {
            match spectra::Oned::new(
                name,
                parameter,
                pdict,
                Some(axis.low),
                Some(axis.high),
                Some(axis.bins),
            ) {
                Ok(spec) => {
                    self.dict.insert(sname, Rc::new(RefCell::new(spec)));
                    return SpectrumReply::Created;
                }
                Err(msg) => {
                    return SpectrumReply::Error(msg);
                }
            }
        } else {
            return SpectrumReply::Error(format!("Spectrum {} already exists", name));
        }
    }
    // Make a multi incremented 1d spectrum (gamma-1d)

    fn make_multi1d(
        &mut self,
        name: &str,
        params: &Vec<String>,
        axis: &AxisSpecification,
        pdict: &parameters::ParameterDictionary,
    ) -> SpectrumReply {
        if !self.dict.contains_key(name) {
            match spectra::Multi1d::new(
                name,
                params.clone(),
                pdict,
                Some(axis.low),
                Some(axis.high),
                Some(axis.bins),
            ) {
                Ok(spec) => {
                    self.dict
                        .insert(String::from(name), Rc::new(RefCell::new(spec)));
                    return SpectrumReply::Created;
                }
                Err(msg) => {
                    return SpectrumReply::Error(msg);
                }
            }
        } else {
            return SpectrumReply::Error(format!("Spectrum {} already exists", name));
        }
    }
    // make multi incremented 2-d (gamma2) spectrum:

    fn make_multi2d(
        &mut self,
        name: &str,
        params: &Vec<String>,
        xaxis: &AxisSpecification,
        yaxis: &AxisSpecification,
        pdict: &parameters::ParameterDictionary,
    ) -> SpectrumReply {
        if !self.dict.contains_key(name) {
            match spectra::Multi2d::new(
                name,
                params.clone(),
                pdict,
                Some(xaxis.low),
                Some(xaxis.high),
                Some(xaxis.bins),
                Some(yaxis.low),
                Some(yaxis.high),
                Some(yaxis.bins),
            ) {
                Ok(spec) => {
                    self.dict
                        .insert(String::from(name), Rc::new(RefCell::new(spec)));
                    return SpectrumReply::Created;
                }
                Err(msg) => {
                    return SpectrumReply::Error(msg);
                }
            }
        } else {
            return SpectrumReply::Error(format!("Spectrum {} already exists", name));
        }
    }
    // make a particle gamma spectrum

    fn make_pgamma(
        &mut self,
        name: &str,
        xparams: &Vec<String>,
        yparams: &Vec<String>,
        xaxis: &AxisSpecification,
        yaxis: &AxisSpecification,
        pdict: &parameters::ParameterDictionary,
    ) -> SpectrumReply {
        if !self.dict.contains_key(name) {
            match spectra::PGamma::new(
                name,
                xparams,
                yparams,
                pdict,
                Some(xaxis.low),
                Some(xaxis.high),
                Some(xaxis.bins),
                Some(yaxis.low),
                Some(yaxis.high),
                Some(yaxis.bins),
            ) {
                Ok(spec) => {
                    self.dict
                        .insert(String::from(name), Rc::new(RefCell::new(spec)));
                    return SpectrumReply::Created;
                }
                Err(str) => {
                    return SpectrumReply::Error(str);
                }
            }
        } else {
            return SpectrumReply::Error(format!("Spectrum {} already exists", name));
        }
    }
    // Make a summary spectrum

    fn make_summary(
        &mut self,
        name: &str,
        params: &Vec<String>,
        xaxis: &AxisSpecification,
        pdict: &parameters::ParameterDictionary,
    ) -> SpectrumReply {
        if !self.dict.contains_key(name) {
            match spectra::Summary::new(
                name,
                params.clone(),
                pdict,
                Some(xaxis.low),
                Some(xaxis.high),
                Some(xaxis.bins),
            ) {
                Ok(spec) => {
                    self.dict
                        .insert(String::from(name), Rc::new(RefCell::new(spec)));
                    return SpectrumReply::Created;
                }
                Err(msg) => {
                    return SpectrumReply::Error(msg);
                }
            }
        } else {
            return SpectrumReply::Error(format!("Spectrum {} already exists", name));
        }
    }
    // Make 2-d spectrum.

    fn make_2d(
        &mut self,
        name: &str,
        xparam: &str,
        yparam: &str,
        xaxis: &AxisSpecification,
        yaxis: &AxisSpecification,
        pdict: &parameters::ParameterDictionary,
    ) -> SpectrumReply {
        if !self.dict.contains_key(name) {
            match spectra::Twod::new(
                name,
                xparam,
                yparam,
                pdict,
                Some(xaxis.low),
                Some(xaxis.high),
                Some(xaxis.bins),
                Some(yaxis.low),
                Some(yaxis.high),
                Some(yaxis.bins),
            ) {
                Ok(spec) => {
                    self.dict
                        .insert(String::from(name), Rc::new(RefCell::new(spec)));
                    return SpectrumReply::Created;
                }
                Err(msg) => {
                    return SpectrumReply::Error(msg);
                }
            }
        } else {
            return SpectrumReply::Error(format!("Spectrum {} already exists", name));
        }
    }
    // Make a 2d sum spectrum.

    fn make_2dsum(
        &mut self,
        name: &str,
        xparams: &Vec<String>,
        yparams: &Vec<String>,
        xaxis: &AxisSpecification,
        yaxis: &AxisSpecification,
        pdict: &parameters::ParameterDictionary,
    ) -> SpectrumReply {
        if !self.dict.contains_key(name) {
            if xparams.len() != yparams.len() {
                return SpectrumReply::Error(String::from(
                    "Number of xparams must be the same as number of y params",
                ));
            }
            let mut params = spectra::XYParameters::new();
            for (i, x) in xparams.iter().enumerate() {
                let p: spectra::XYParameter = (x.clone(), yparams[i].clone());
                params.push(p);
            }
            match spectra::TwodSum::new(
                name,
                params,
                pdict,
                Some(xaxis.low),
                Some(xaxis.high),
                Some(xaxis.bins),
                Some(yaxis.low),
                Some(yaxis.high),
                Some(yaxis.bins),
            ) {
                Ok(spec) => {
                    self.dict
                        .insert(String::from(name), Rc::new(RefCell::new(spec)));
                    return SpectrumReply::Created;
                }
                Err(msg) => {
                    return SpectrumReply::Error(msg);
                }
            }
        } else {
            return SpectrumReply::Error(format!("Spectrum {} already exists", name));
        }
    }
    // Delete an existing spectrum.

    fn delete_spectrum(&mut self, name: &str) -> SpectrumReply {
        if let Some(_) = self.dict.remove(name) {
            SpectrumReply::Deleted
        } else {
            SpectrumReply::Error(format!("Spectrum {} does not exist", name))
        }
    }
    // List spectra and properties.

    fn list_spectra(&self, pattern: &str) -> SpectrumReply {
        SpectrumReply::Error(String::from("Unimplemnented operation"))
    }
    fn gate_spectrum(
        &self,
        spectrum: &str,
        gate: &str,
        cdict: &conditions::ConditionDictionary,
    ) -> SpectrumReply {
        SpectrumReply::Error(String::from("Unimplemnented operation"))
    }
    fn ungate_spectrum(&self, spectrum: &str) -> SpectrumReply {
        SpectrumReply::Error(String::from("Unimplemnented operation"))
    }
    fn clear_spectra(&self, pattern: &str) -> SpectrumReply {
        SpectrumReply::Error(String::from("Unimplemnented operation"))
    }
    fn get_contents(
        &self,
        name: &str,
        xlow: f64,
        xhigh: f64,
        ylow: f64,
        yhigh: f64,
    ) -> SpectrumReply {
        SpectrumReply::Error(String::from("Unimplemnented operation"))
    }
    fn process_events(&mut self, events: &Vec<parameters::Event>) -> SpectrumReply {
        SpectrumReply::Error(String::from("Unimplemnented operation"))
    }

    // Public methods
    /// Construction

    pub fn new() -> SpectrumProcessor {
        SpectrumProcessor {
            dict: spectra::SpectrumDictionary::new(),
        }
    }
    /// Process requests returning replies:

    pub fn process_request(
        &mut self,
        req: SpectrumRequest,
        pdict: &parameters::ParameterDictionary,
        cdict: &conditions::ConditionDictionary,
    ) -> SpectrumReply {
        match req {
            SpectrumRequest::Create1D {
                name,
                parameter,
                axis,
            } => self.make_1d(&name, &parameter, &axis, &pdict),
            SpectrumRequest::CreateMulti1D { name, params, axis } => {
                self.make_multi1d(&name, &params, &axis, &pdict)
            }
            SpectrumRequest::CreateMulti2D {
                name,
                params,
                xaxis,
                yaxis,
            } => self.make_multi2d(&name, &params, &xaxis, &yaxis, &pdict),
            SpectrumRequest::CreatePGamma {
                name,
                xparams,
                yparams,
                xaxis,
                yaxis,
            } => self.make_pgamma(&name, &xparams, &yparams, &xaxis, &yaxis, &pdict),
            SpectrumRequest::CreateSummary {
                name,
                params,
                yaxis,
            } => self.make_summary(&name, &params, &yaxis, &pdict),
            SpectrumRequest::Create2D {
                name,
                xparam,
                yparam,
                xaxis,
                yaxis,
            } => self.make_2d(&name, &xparam, &yparam, &xaxis, &yaxis, &pdict),
            SpectrumRequest::Create2DSum {
                name,
                xparams,
                yparams,
                xaxis,
                yaxis,
            } => self.make_2dsum(&name, &xparams, &yparams, &xaxis, &yaxis, &pdict),
            SpectrumRequest::Delete(name) => self.delete_spectrum(&name),
            SpectrumRequest::List(pattern) => self.list_spectra(&pattern),
            SpectrumRequest::Gate { spectrum, gate } => {
                self.gate_spectrum(&spectrum, &gate, &cdict)
            }
            SpectrumRequest::Ungate(name) => self.ungate_spectrum(&name),
            SpectrumRequest::Clear(pattern) => self.clear_spectra(&pattern),
            SpectrumRequest::GetContents {
                name,
                xlow,
                xhigh,
                ylow,
                yhigh,
            } => self.get_contents(&name, xlow, xhigh, ylow, yhigh),
            SpectrumRequest::Events(events) => self.process_events(&events),
        }
    }
}
