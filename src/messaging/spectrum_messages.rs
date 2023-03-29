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
use ndhistogram::axis::*;
use ndhistogram::*;
use std::sync::mpsc;

use glob::Pattern;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisSpecification {
    pub low: f64,
    pub high: f64,
    pub bins: u32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChannelType {
    Underflow,
    Overflow,
    Bin,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Channel {
    pub chan_type: ChannelType,
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
    dict: spectra::SpectrumStorage,
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
        if !self.dict.exists(name) {
            match spectra::Oned::new(
                name,
                parameter,
                pdict,
                Some(axis.low),
                Some(axis.high),
                Some(axis.bins),
            ) {
                Ok(spec) => {
                    self.dict.add(Rc::new(RefCell::new(spec)));
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
        if !self.dict.exists(name) {
            match spectra::Multi1d::new(
                name,
                params.clone(),
                pdict,
                Some(axis.low),
                Some(axis.high),
                Some(axis.bins),
            ) {
                Ok(spec) => {
                    self.dict.add(Rc::new(RefCell::new(spec)));
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
        if !self.dict.exists(name) {
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
                    self.dict.add(Rc::new(RefCell::new(spec)));
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
        if !self.dict.exists(name) {
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
                    self.dict.add(Rc::new(RefCell::new(spec)));
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
        if !self.dict.exists(name) {
            match spectra::Summary::new(
                name,
                params.clone(),
                pdict,
                Some(xaxis.low),
                Some(xaxis.high),
                Some(xaxis.bins),
            ) {
                Ok(spec) => {
                    self.dict.add(Rc::new(RefCell::new(spec)));
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
        if !self.dict.exists(name) {
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
                    self.dict.add(Rc::new(RefCell::new(spec)));
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
        if !self.dict.exists(name) {
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
                    self.dict.add(Rc::new(RefCell::new(spec)));
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

    fn get_properties(spec: &spectra::SpectrumContainer) -> SpectrumProperties {
        let s = spec.borrow();
        let x = s.get_xaxis();
        let y = s.get_yaxis();
        SpectrumProperties {
            name: s.get_name(),
            type_name: s.get_type(),
            xparams: s.get_xparams(),
            yparams: s.get_yparams(),
            xaxis: if let Some(xa) = x {
                Some(AxisSpecification {
                    low: xa.0,
                    high: xa.1,
                    bins: xa.2,
                })
            } else {
                None
            },
            yaxis: if let Some(xa) = y {
                Some(AxisSpecification {
                    low: xa.0,
                    high: xa.1,
                    bins: xa.2,
                })
            } else {
                None
            },
            gate: s.get_gate(),
        }
    }

    fn list_spectra(&self, pattern: &str) -> SpectrumReply {
        let mut listing = Vec::<SpectrumProperties>::new();
        let p = Pattern::new(pattern);
        if let Err(reason) = p {
            return SpectrumReply::Error(format!("Bad glob pattern {}", reason.msg));
        }
        let p = p.unwrap();
        for (name, s) in self.dict.iter() {
            if p.matches(name) {
                listing.push(Self::get_properties(s));
            }
        }

        SpectrumReply::Listing(listing)
    }
    fn gate_spectrum(
        &self,
        sname: &str,
        gname: &str,
        cdict: &conditions::ConditionDictionary,
    ) -> SpectrumReply {
        if let Some(spec) = self.dict.get(sname) {
            if let Err(msg) = spec.borrow_mut().gate(gname, cdict) {
                return SpectrumReply::Error(msg);
            } else {
                return SpectrumReply::Gated;
            }
        } else {
            return SpectrumReply::Error(format!("Spectrum {} does not exist", sname));
        }
    }
    fn ungate_spectrum(&self, spectrum: &str) -> SpectrumReply {
        if let Some(spec) = self.dict.get(spectrum) {
            spec.borrow_mut().ungate();
            return SpectrumReply::Ungated;
        } else {
            return SpectrumReply::Error(format!("Spectrum {} does not exist", spectrum));
        }
    }
    fn clear_spectra(&self, pattern: &str) -> SpectrumReply {
        let pat = Pattern::new(pattern);
        if let Err(e) = pat {
            return SpectrumReply::Error(format!("Bad glob pattern: {}", e.msg));
        }
        let pat = pat.unwrap();
        for (name, s) in self.dict.iter() {
            if pat.matches(name) {
                s.borrow_mut().clear();
            }
        }
        SpectrumReply::Cleared
    }
    fn get_contents(
        &self,
        name: &str,
        xlow: f64,
        xhigh: f64,
        ylow: f64,
        yhigh: f64,
    ) -> SpectrumReply {
        // How we iterate depends on the type of histogram:

        let mut result = SpectrumContents::new();
        if let Some(spec) = self.dict.get(name) {
            if let Some(spectrum) = spec.borrow().get_histogram_1d() {
                for c in spectrum.borrow().iter() {
                    let v = c.value.get();
                    if v != 0.0 {
                        match c.bin {
                            BinInterval::Underflow { end } => {
                                result.push(Channel {
                                    chan_type: ChannelType::Underflow,
                                    value: v,
                                    x: 0.0,
                                    y: 0.0,
                                });
                            }
                            BinInterval::Overflow { start } => {
                                result.push(Channel {
                                    chan_type: ChannelType::Overflow,
                                    value: v,
                                    x: 0.0,
                                    y: 0.0,
                                });
                            }
                            BinInterval::Bin { start, end } => {
                                if (start >= xlow) && (start <= xhigh) {
                                    result.push(Channel {
                                        chan_type: ChannelType::Bin,
                                        x: start,
                                        y: 0.0,
                                        value: v,
                                    });
                                };
                            }
                        }
                    }
                }
            } else {
                let spectrum = spec.borrow().get_histogram_2d().unwrap();
                for c in spectrum.borrow().iter() {
                    let v = c.value.get();
                    let xbin = c.bin.0;
                    let ybin = c.bin.1;
                    let mut x = 0.0;
                    let mut y = 0.0;
                    let mut ctype = ChannelType::Bin;

                    match xbin {
                        BinInterval::Overflow { start } => {
                            ctype = ChannelType::Overflow;
                        }
                        BinInterval::Underflow { end } => {
                            ctype = ChannelType::Underflow;
                        }
                        BinInterval::Bin { start, end } => {
                            x = start;
                        }
                    };
                    match ybin {
                        BinInterval::Overflow { start } => {
                            if ctype == ChannelType::Bin {
                                ctype = ChannelType::Overflow;
                            }
                        }
                        BinInterval::Underflow { end } => {
                            if ctype == ChannelType::Bin {
                                ctype = ChannelType::Underflow;
                            }
                        }
                        BinInterval::Bin { start, end } => {
                            y = start;
                        }
                    };
                    if (v != 0.0) && (x >= xlow) && (x <= xhigh) && (y >= ylow) && (y <= yhigh) {
                        result.push(Channel {
                            chan_type: ctype,
                            x: x,
                            y: y,
                            value: v,
                        });
                    }
                }
            }
            return SpectrumReply::Contents(result);
        } else {
            return SpectrumReply::Error(format!("Spectrum {} does not exist", name));
        }
    }
    fn process_events(
        &mut self,
        events: &Vec<parameters::Event>,
        cdict: &mut conditions::ConditionDictionary,
    ) -> SpectrumReply {
        for e in events.iter() {
            conditions::invalidate_cache(cdict);
            self.dict.process_event(e);
        }
        SpectrumReply::Processed
    }

    // Public methods
    /// Construction

    pub fn new() -> SpectrumProcessor {
        SpectrumProcessor {
            dict: spectra::SpectrumStorage::new(),
        }
    }
    /// Process requests returning replies:

    pub fn process_request(
        &mut self,
        req: SpectrumRequest,
        pdict: &parameters::ParameterDictionary,
        cdict: &mut conditions::ConditionDictionary,
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
            SpectrumRequest::Events(events) => self.process_events(&events, cdict),
        }
    }
}
//----------------------------------------------------------------
// The unbound functions below provide private message formatting
// used by the client code:

fn create1d_request(
    name: &str,
    parameter: &str,
    low: f64,
    high: f64,
    bins: u32,
) -> SpectrumRequest {
    SpectrumRequest::Create1D {
        name: String::from(name),
        parameter: String::from(parameter),
        axis: AxisSpecification { low, high, bins },
    }
}

fn createmulti1d_request(
    name: &str,
    params: &Vec<String>,
    low: f64,
    high: f64,
    bins: u32,
) -> SpectrumRequest {
    SpectrumRequest::CreateMulti1D {
        name: String::from(name),
        params: params.clone(),
        axis: AxisSpecification { low, high, bins },
    }
}
fn createmulti2d_request(
    name: &str,
    params: &Vec<String>,
    xlow: f64,
    xhigh: f64,
    xbins: u32,
    ylow: f64,
    yhigh: f64,
    ybins: u32,
) -> SpectrumRequest {
    SpectrumRequest::CreateMulti2D {
        name: String::from(name),
        params: params.clone(),
        xaxis: AxisSpecification {
            low: xlow,
            high: xhigh,
            bins: xbins,
        },
        yaxis: AxisSpecification {
            low: ylow,
            high: yhigh,
            bins: ybins,
        },
    }
}
fn createpgamma_request(
    name: &str,
    xparams: &Vec<String>,
    yparams: &Vec<String>,
    xlow: f64,
    xhigh: f64,
    xbins: u32,
    ylow: f64,
    yhigh: f64,
    ybins: u32,
) -> SpectrumRequest {
    SpectrumRequest::CreatePGamma {
        name: String::from(name),
        xparams: xparams.clone(),
        yparams: yparams.clone(),
        xaxis: AxisSpecification {
            low: xlow,
            high: xhigh,
            bins: xbins,
        },
        yaxis: AxisSpecification {
            low: ylow,
            high: yhigh,
            bins: ybins,
        },
    }
}
fn createsummary_request(
    name: &str,
    params: &Vec<String>,
    low: f64,
    high: f64,
    bins: u32,
) -> SpectrumRequest {
    SpectrumRequest::CreateSummary {
        name: String::from(name),
        params: params.clone(),
        yaxis: AxisSpecification { low, high, bins },
    }
}
fn create2d_request(
    name: &str,
    xparam: &str,
    yparam: &str,
    xlow: f64,
    xhigh: f64,
    xbins: u32,
    ylow: f64,
    yhigh: f64,
    ybins: u32,
) -> SpectrumRequest {
    SpectrumRequest::Create2D {
        name: String::from(name),
        xparam: String::from(xparam),
        yparam: String::from(yparam),
        xaxis: AxisSpecification {
            low: xlow,
            high: xhigh,
            bins: xbins,
        },
        yaxis: AxisSpecification {
            low: ylow,
            high: yhigh,
            bins: ybins,
        },
    }
}
fn create2dsum_request(
    name: &str,
    xparams: &Vec<String>,
    yparams: &Vec<String>,
    xlow: f64,
    xhigh: f64,
    xbins: u32,
    ylow: f64,
    yhigh: f64,
    ybins: u32,
) -> SpectrumRequest {
    SpectrumRequest::Create2DSum {
        name: String::from(name),
        xparams: xparams.clone(),
        yparams: yparams.clone(),
        xaxis: AxisSpecification {
            low: xlow,
            high: xhigh,
            bins: xbins,
        },
        yaxis: AxisSpecification {
            low: ylow,
            high: yhigh,
            bins: ybins,
        },
    }
}
fn delete_request(name: &str) -> SpectrumRequest {
    SpectrumRequest::Delete(String::from(name))
}
fn list_request(pattern: &str) -> SpectrumRequest {
    SpectrumRequest::List(String::from(pattern))
}
fn gate_request(spectrum: &str, condition: &str) -> SpectrumRequest {
    SpectrumRequest::Gate {
        spectrum: String::from(spectrum),
        gate: String::from(condition),
    }
}
fn ungate_request(name: &str) -> SpectrumRequest {
    SpectrumRequest::Ungate(String::from(name))
}
fn clear_request(pattern: &str) -> SpectrumRequest {
    SpectrumRequest::Clear(String::from(pattern))
}
fn getcontents_request(
    name: &str,
    xlow: f64,
    xhigh: f64,
    ylow: f64,
    yhigh: f64,
) -> SpectrumRequest {
    SpectrumRequest::GetContents {
        name: String::from(name),
        xlow,
        xhigh,
        ylow,
        yhigh,
    }
}
fn events_request(events: &Vec<parameters::Event>) -> SpectrumRequest {
    SpectrumRequest::Events(events.clone())
}
//------------------- Client API methods-------------------------

/// This is a Result where the server has nothing of
/// of interest to say to the caller.
///
pub type SpectrumServerEmptyResult = Result<(), String>;

/// This is a result where the server, on success will
/// provide a list of properties of some spectra:
///
pub type SpectrumServerListingResult = Result<Vec<SpectrumProperties>, String>;
///
/// This type is a result the API will sue to return spectrum
/// contents:
pub type SpectrumServerContentsResult = Result<SpectrumContents, String>;
///
/// Perform an arbitrary transaction.  This could actually
/// be private but,  since the request and reply structs
/// are public this is too:
///
/// *  req  - The request object
/// *  req_chan - Channel to which to send the request.
/// *  reply_send - Channel to which the server shouild send the reply
/// *  reply_recv - Channel on which the client receives the reply.
///
/// *   Returns: SpectrumReply
/// *   Note:  if the reply is not a SpectrumReply, panics.
/// *   Note:  The request is consumed.
pub fn transact(
    req: SpectrumRequest,
    req_chan: mpsc::Sender<Request>,
    reply_send: mpsc::Sender<Reply>,
    reply_recv: mpsc::Receiver<Reply>,
) -> SpectrumReply {
    let request = Request {
        reply_channel: reply_send,
        message: MessageType::Spectrum(req),
    };
    let reply = request.transaction(req_chan, reply_recv);
    if let Reply::Spectrum(r) = reply {
        r
    } else {
        panic!("Expected Spectrum reply got something else");
    }
}
///
/// Create a 1d spectrum:
///
/// *  name - name of the spectrum to create.
/// *  parameter - name of the parameter to histogram
/// *  low, high, bins - axis specification for the spectrum.
/// *  req - request channely
/// *  reply_send - channel on which to send the reply.
/// *  reply_recv  - Chanel on which to recieve the reply.
///
/// Returns: SpectrumServerEmptyResult
///
pub fn create_spectrum_1d(
    name: &str,
    parameter: &str,
    low: f64,
    high: f64,
    bins: u32,
    req: mpsc::Sender<Request>,
    reply_send: mpsc::Sender<Reply>,
    reply_recv: mpsc::Receiver<Reply>,
) -> SpectrumServerEmptyResult {
    let reply = transact(
        create1d_request(name, parameter, low, high, bins),
        req,
        reply_send,
        reply_recv,
    );
    if let SpectrumReply::Error(s) = reply {
        Err(s)
    } else {
        Ok(())
    }
}
/// Create a mutiply incremented 1d spectrum (gamma 1d).
///
///
/// *   name - name of the spectrum.
/// *   params - Names of the parameters to histogram.
/// *   low, high, bins - axis specifications.
/// *  req - request channel
/// *  reply_send - channel on which to send the reply.
/// *  reply_recv  - Chanel on which to recieve the reply.
///
/// Returns: SpectrumServerEmptyResult
///
pub fn create_spectrum_multi1d(
    name: &str,
    parameters: &Vec<String>,
    low: f64,
    high: f64,
    bins: u32,
    req: mpsc::Sender<Request>,
    reply_send: mpsc::Sender<Reply>,
    reply_recv: mpsc::Receiver<Reply>,
) -> SpectrumServerEmptyResult {
    let reply = transact(
        createmulti1d_request(name, parameters, low, high, bins),
        req,
        reply_send,
        reply_recv,
    );
    if let SpectrumReply::Error(s) = reply {
        Err(s)
    } else {
        Ok(())
    }
}
/// Create a muliply incremented 2d spectrum (gamma 2)
///
/// *   name - spectrum name.
/// *   parameters - vector of  parameters (reference)
/// *   xlow, xhigh, xbins - x axis specification.
/// *   ylow, yhigh, ybins - y axis specification.
/// *  req - request channel
/// *  reply_send - channel on which to send the reply.
/// *  reply_recv  - Chanel on which to recieve the reply.
///
/// Returns: SpectrumServerEmptyResult

pub fn create_spectrum_multi2d(
    name: &str,
    parameters: &Vec<String>,
    xlow: f64,
    xhigh: f64,
    xbins: u32,
    ylow: f64,
    yhigh: f64,
    ybins: u32,
    req: mpsc::Sender<Request>,
    reply_send: mpsc::Sender<Reply>,
    reply_recv: mpsc::Receiver<Reply>,
) -> SpectrumServerEmptyResult {
    let reply = transact(
        createmulti2d_request(name, parameters, xlow, xhigh, xbins, ylow, yhigh, ybins),
        req,
        reply_send,
        reply_recv,
    );
    if let SpectrumReply::Error(s) = reply {
        Err(s)
    } else {
        Ok(())
    }
}
///  Create a particle gamma spectrum (gamma delux).
///
/// *   name -spectrum name.
/// *   xparams - xaxis parameters.
/// *   yparams - yaxis parameters.
/// *   xlow, xhigh, xbins - x axis specification.
/// *   ylow, yhigh, ybins - y axis specification.
/// *  req - request channel
/// *  reply_send - channel on which to send the reply.
/// *  reply_recv  - Chanel on which to recieve the reply.
///
/// Returns: SpectrumServerEmptyResult

pub fn create_spectrum_pgamma(
    name: &str,
    xparams: &Vec<String>,
    yparams: &Vec<String>,
    xlow: f64,
    xhigh: f64,
    xbins: u32,
    ylow: f64,
    yhigh: f64,
    ybins: u32,
    req: mpsc::Sender<Request>,
    reply_send: mpsc::Sender<Reply>,
    reply_recv: mpsc::Receiver<Reply>,
) -> SpectrumServerEmptyResult {
    let reply = transact(
        createpgamma_request(
            name, xparams, yparams, xlow, xhigh, xbins, ylow, yhigh, ybins,
        ),
        req,
        reply_send,
        reply_recv,
    );
    if let SpectrumReply::Error(s) = reply {
        Err(s)
    } else {
        Ok(())
    }
}
/// Create a summary spectrum:
///
/// *  name - name of the spectrum
/// *  params - The parameters to histogram.
/// *  low, high, bins - axis specifications (y axis).
/// *  req - request channel
/// *  reply_send - channel on which to send the reply.
/// *  reply_recv  - Chanel on which to recieve the reply.
///
/// Returns: SpectrumServerEmptyResult

pub fn create_spectrum_summary(
    name: &str,
    params: &Vec<String>,
    low: f64,
    high: f64,
    bins: u32,
    req: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_recv: mpsc::Receiver<Reply>,
) -> SpectrumServerEmptyResult {
    let reply = transact(
        createsummary_request(name, params, low, high, bins),
        req,
        rep_send,
        rep_recv,
    );
    if let SpectrumReply::Error(s) = reply {
        Err(s)
    } else {
        Ok(())
    }
}
/// Create 2d spectrum.
///
/// * name - name of the spectrum.
/// * xparam - parameter on x axis.
/// * yparam - parameter on yaxis.
/// * xlow, xhigh, xbins - X axis specification.
/// * ylow, yhigh, ybins - Y axis specification.
/// *  req - request channel
/// *  reply_send - channel on which to send the reply.
/// *  reply_recv  - Chanel on which to recieve the reply.
///
/// Returns: SpectrumServerEmptyResult

pub fn create_spectrum_2d(
    name: &str,
    xparam: &str,
    yparam: &str,
    xlow: f64,
    xhigh: f64,
    xbins: u32,
    ylow: f64,
    yhigh: f64,
    ybins: u32,
    req: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_recv: mpsc::Receiver<Reply>,
) -> SpectrumServerEmptyResult {
    let reply = transact(
        create2d_request(name, xparam, yparam, xlow, xhigh, xbins, ylow, yhigh, ybins),
        req,
        rep_send,
        rep_recv,
    );
    if let SpectrumReply::Error(s) = reply {
        Err(s)
    } else {
        Ok(())
    }
}
///  Create a 2d spectrum that is the sum of 2d spectra.
///
/// * name - name of the spectrum.
/// * xparams - Parameters on x axis.
/// * yparams - parameters on the y axis.
/// * xlow, xhigh, xbins - xaxis specification.
/// * ylow, yhigh, ybins - yaxis specification.
/// *  req - request channel
/// *  reply_send - channel on which to send the reply.
/// *  reply_recv  - Chanel on which to recieve the reply.
///
/// Returns: SpectrumServerEmptyResult
/// *  Note:  The size of xparams and yparams must be identical.
///
pub fn create_spectrum_2dsum(
    name: &str,
    xparams: &Vec<String>,
    yparams: &Vec<String>,
    xlow: f64,
    xhigh: f64,
    xbins: u32,
    ylow: f64,
    yhigh: f64,
    ybins: u32,
    req: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_recv: mpsc::Receiver<Reply>,
) -> SpectrumServerEmptyResult {
    let reply = transact(
        create2dsum_request(
            name, xparams, yparams, xlow, xhigh, xbins, ylow, yhigh, ybins,
        ),
        req,
        rep_send,
        rep_recv,
    );
    if let SpectrumReply::Error(s) = reply {
        Err(s)
    } else {
        Ok(())
    }
}

/// Delete a spectrum.
///
/// * name - name of the spectrum to delete.
/// *  req - request channel
/// *  reply_send - channel on which to send the reply.
/// *  reply_recv  - Chanel on which to recieve the reply.
///
/// Returns SpectrumServerEmptyResult
///
pub fn delete_spectrum(
    name: &str,
    req: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_recv: mpsc::Receiver<Reply>,
) -> SpectrumServerEmptyResult {
    let reply = transact(delete_request(name), req, rep_send, rep_recv);
    if let SpectrumReply::Error(s) = reply {
        Err(s)
    } else {
        Ok(())
    }
}
/// list spectra
///
/// *   pattern - Glob pattern the server will list information
/// for all spectra that match the pattern. Note that "*" will
/// match all spectgra.
/// *  req - request channel
/// *  reply_send - channel on which to send the reply.
/// *  reply_recv  - Chanel on which to recieve the reply.
///
/// Returns : SpectrumServerListingResult
///
pub fn list_spectra(
    pattern: &str,
    req: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_recv: mpsc::Receiver<Reply>,
) -> SpectrumServerListingResult {
    match transact(list_request(pattern), req, rep_send, rep_recv) {
        SpectrumReply::Error(s) => Err(s),
        SpectrumReply::Listing(l) => Ok(l),
        _ => Err(String::from("Unexpected server result for list request")),
    }
}
/// Apply a gate to a spectrum:
///
/// * spectrum -name of the spectrum.
/// * gate - name of the gate to apply.
/// *  req - request channel
/// *  reply_send - channel on which to send the reply.
/// *  reply_recv  - Chanel on which to recieve the reply.
///
/// Retuns: SpectrumServerEmptyResult.
///
pub fn gate_spectrum(
    spectrum: &str,
    gate: &str,
    req: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_recv: mpsc::Receiver<Reply>,
) -> SpectrumServerEmptyResult {
    let reply = transact(gate_request(spectrum, gate), req, rep_send, rep_recv);
    if let SpectrumReply::Error(s) = reply {
        Err(s)
    } else {
        Ok(())
    }
}
/// Ungate a spectrum.  
///
/// *  name - name of the spectrum
/// *  req - request channel
/// *  reply_send - channel on which to send the reply.
/// *  reply_recv  - Chanel on which to recieve the reply.
///
/// Retuns: SpectrumServerEmptyResult.
///
pub fn ungate_spectrum(
    name: &str,
    req: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_recv: mpsc::Receiver<Reply>,
) -> SpectrumServerEmptyResult {
    let reply = transact(ungate_request(name), req, rep_send, rep_recv);
    if let SpectrumReply::Error(s) = reply {
        Err(s)
    } else {
        Ok(())
    }
}

/// clear spectra
///
/// *  pattern - glob pattern that describes the spectra to clear.
/// e.g. "*" clears them all.
/// *  req - request channel
/// *  reply_send - channel on which to send the reply.
/// *  reply_recv  - Chanel on which to recieve the reply.
///
/// Retuns: SpectrumServerEmptyResult.
///
pub fn clear_spectrum(
    pattern: &str,
    req: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_recv: mpsc::Receiver<Reply>,
) -> SpectrumServerEmptyResult {
    let reply = transact(clear_request(pattern), req, rep_send, rep_recv);
    if let SpectrumReply::Error(s) = reply {
        Err(s)
    } else {
        Ok(())
    }
}
///
/// Get the contents of a spectrum.
///
/// * name - name of the spectrum.
/// * xlow, xhigh, ylow, yhigh - a rectangular region of interest in
/// parameter coordinate space within which the data are returned.
/// Note that only data with non-zero channel values are returned.
/// *  req - request channel
/// *  reply_send - channel on which to send the reply.
/// *  reply_recv  - Chanel on which to recieve the reply.
///
/// Returns:  SpectrumServerContentsResult
///
pub fn get_contents(
    name: &str,
    xlow: f64,
    xhigh: f64,
    ylow: f64,
    yhigh: f64,
    req: mpsc::Sender<Request>,
    rep_send: mpsc::Sender<Reply>,
    rep_recv: mpsc::Receiver<Reply>,
) -> SpectrumServerContentsResult {
    match transact(
        getcontents_request(name, xlow, xhigh, ylow, yhigh),
        req,
        rep_send,
        rep_recv,
    ) {
        SpectrumReply::Error(s) => Err(s),
        SpectrumReply::Contents(c) => Ok(c),
        _ => Err(String::from("Unexpected reply type for get_contents")),
    }
}

//--------------------------- Tests ------------------------------

#[cfg(test)]
mod spproc_tests {
    use super::*;
    use crate::conditions::*;
    use crate::parameters::*;
    use crate::spectra::*;
    use std::cmp::Ordering;

    #[test]
    fn new_1() {
        let processor = SpectrumProcessor::new();
        let mut num_spec = 0;
        for (_, _) in processor.dict.iter() {
            num_spec += 1;
        }
        assert_eq!(0, num_spec);
    }
    // for most of the tests we need, not only a SpectrumProcessor
    // but a condition dict, and a parameter dict:

    struct TestObjects {
        processor: SpectrumProcessor,
        parameters: ParameterDictionary,
        conditions: ConditionDictionary,
    }
    fn make_test_objs() -> TestObjects {
        TestObjects {
            processor: SpectrumProcessor::new(),
            parameters: ParameterDictionary::new(),
            conditions: ConditionDictionary::new(),
        }
    }
    fn make_some_params(to: &mut TestObjects) {
        for i in 0..10 {
            let name = format!("param.{}", i);
            to.parameters.add(&name).unwrap();
        }
    }
    // Spectrum creation tests:

    #[test]
    fn create1d_1() {
        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);
        assert!(to.processor.dict.exists("test"));
        let spc = to.processor.dict.get("test");
        assert!(spc.is_some());
        let spc = spc.unwrap().borrow();

        assert_eq!(String::from("test"), spc.get_name());
        assert_eq!(String::from("1D"), spc.get_type());
        assert_eq!(String::from("param.1"), spc.get_xparams()[0]);
        assert_eq!(0, spc.get_yparams().len());

        let x = spc.get_xaxis();
        assert!(x.is_some());
        let x = x.unwrap();
        assert_eq!(
            AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1026 // under/over flow bins.
            },
            AxisSpecification {
                low: x.0,
                high: x.1,
                bins: x.2
            }
        );
        assert!(spc.get_yaxis().is_none());
        assert!(spc.get_gate().is_none());
    }
    #[test]
    fn create1d_2() {
        // bad parameter:
        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.166"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        // Checking the error string is brittle so:

        if let SpectrumReply::Error(_) = reply {
            assert!(true);
        } else {
            assert!(false);
        }
    }
    #[test]
    fn create1d_3() {
        // Duplicate spectrum::

        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        if let SpectrumReply::Error(_) = reply {
            assert!(true);
        } else {
            assert!(false);
        }
        // spectrum is still in dict:

        assert!(to.processor.dict.exists("test"));
    }
    #[test]
    fn createmulti1_1() {
        // Success for multi1d:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.7"),
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti1D {
                name: String::from("test"),
                params: params.clone(),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);
        assert!(to.processor.dict.exists("test"));
        let spc = to.processor.dict.get("test");
        assert!(spc.is_some());
        let spc = spc.unwrap().borrow();

        assert_eq!(String::from("test"), spc.get_name());
        assert_eq!(String::from("Multi1d"), spc.get_type());
        assert_eq!(params, spc.get_xparams());
        assert_eq!(0, spc.get_yparams().len());

        let x = spc.get_xaxis();
        assert!(x.is_some());
        let x = x.unwrap();
        assert_eq!(
            AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1026 // under/over flow bins.
            },
            AxisSpecification {
                low: x.0,
                high: x.1,
                bins: x.2
            }
        );
        assert!(spc.get_yaxis().is_none());
        assert!(spc.get_gate().is_none());
    }
    #[test]
    fn createmulti1_2() {
        // A Parameter does not exist:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.12"),
            String::from("param.7"),
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti1D {
                name: String::from("test"),
                params: params.clone(),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        if let SpectrumReply::Error(_) = reply {
            assert!(true);
        } else {
            assert!(false);
        }
    }
    #[test]
    fn createmulti_3() {
        // Duplicate spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.7"),
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti1D {
                name: String::from("test"),
                params: params.clone(),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti1D {
                name: String::from("test"),
                params: params.clone(),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        if let SpectrumReply::Error(_) = reply {
            assert!(true);
        } else {
            assert!(false);
        }
        assert!(to.processor.dict.exists("test"));
    }
    #[test]
    fn createmult2_1() {
        // Successfully create a multi-2:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.7"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti2D {
                name: String::from("test"),
                params: params.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
                yaxis: AxisSpecification {
                    low: -512.0,
                    high: 512.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);
        assert!(to.processor.dict.exists("test"));
        let spc = to.processor.dict.get("test");
        assert!(spc.is_some());
        let spc = spc.unwrap().borrow();

        assert_eq!(String::from("test"), spc.get_name());
        assert_eq!(String::from("Multi2d"), spc.get_type());
        assert_eq!(params, spc.get_xparams());
        assert_eq!(0, spc.get_yparams().len());

        let x = spc.get_xaxis();
        assert!(x.is_some());
        let x = x.unwrap();
        assert_eq!(
            AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1026 // under/over flow bins.
            },
            AxisSpecification {
                low: x.0,
                high: x.1,
                bins: x.2
            }
        );
        let y = spc.get_yaxis();
        assert!(y.is_some());
        let y = y.unwrap();
        assert_eq!(
            AxisSpecification {
                low: -512.0,
                high: 512.0,
                bins: 1026
            },
            AxisSpecification {
                low: y.0,
                high: y.1,
                bins: y.2
            }
        );
        assert!(spc.get_gate().is_none());
    }
    #[test]
    fn creatmult2_2() {
        // invalid parametr:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.71"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti2D {
                name: String::from("test"),
                params: params.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
                yaxis: AxisSpecification {
                    low: -512.0,
                    high: 512.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        if let SpectrumReply::Error(_) = reply {
            assert!(true);
        } else {
            assert!(false);
        }
    }
    #[test]
    fn createmult2_3() {
        // duplicate spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.7"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti2D {
                name: String::from("test"),
                params: params.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
                yaxis: AxisSpecification {
                    low: -512.0,
                    high: 512.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);
        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti2D {
                name: String::from("test"),
                params: params.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
                yaxis: AxisSpecification {
                    low: -512.0,
                    high: 512.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        if let SpectrumReply::Error(_) = reply {
            assert!(true);
        } else {
            assert!(false);
        }
        assert!(to.processor.dict.exists("test"));
    }
    #[test]
    fn createpgamma_1() {
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let xparams = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.6"),
        ];
        let yparams = vec![
            String::from("param.1"),
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreatePGamma {
                name: String::from("test"),
                xparams: xparams.clone(),
                yparams: yparams.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);
        assert!(to.processor.dict.exists("test"));
        let spc = to.processor.dict.get("test");
        assert!(spc.is_some());
        let spc = spc.unwrap().borrow(); // Ref to spectrum (readonly)
        assert_eq!(String::from("test"), spc.get_name());
        assert_eq!(String::from("PGamma"), spc.get_type());
        assert_eq!(xparams, spc.get_xparams());
        assert_eq!(yparams, spc.get_yparams());
        let x = spc.get_xaxis().expect("Missing x axis");
        assert_eq!(
            AxisSpecification {
                low: 0.0,
                high: 4096.0,
                bins: 514
            },
            AxisSpecification {
                low: x.0,
                high: x.1,
                bins: x.2
            }
        );
        let y = spc.get_yaxis().expect("Missing y axis");
        assert_eq!(
            AxisSpecification {
                low: -1.0,
                high: 1.0,
                bins: 102,
            },
            AxisSpecification {
                low: y.0,
                high: y.1,
                bins: y.2
            }
        );
        assert!(spc.get_gate().is_none());
    }
    #[test]
    fn createpgamma_2() {
        // An x parameter is bad:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let xparams = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.10"), // bad.
            String::from("param.6"),
        ];
        let yparams = vec![
            String::from("param.1"),
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreatePGamma {
                name: String::from("test"),
                xparams: xparams.clone(),
                yparams: yparams.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        // maybe is more Rusty than the earlier efforts.
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn createpgamma_3() {
        let mut to = make_test_objs();
        make_some_params(&mut to);

        // bad y parameter.
        let xparams = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.6"),
        ];
        let yparams = vec![
            String::from("param.11"), // bad.
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreatePGamma {
                name: String::from("test"),
                xparams: xparams.clone(),
                yparams: yparams.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn createpgamma_4() {
        // Duplicate spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let xparams = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.6"),
        ];
        let yparams = vec![
            String::from("param.1"),
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreatePGamma {
                name: String::from("test"),
                xparams: xparams.clone(),
                yparams: yparams.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);
        let reply = to.processor.process_request(
            SpectrumRequest::CreatePGamma {
                name: String::from("test"),
                xparams: xparams.clone(),
                yparams: yparams.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn crsummary_1() {
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.8"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreateSummary {
                name: String::from("test"),
                params: params.clone(),
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);
        assert!(to.processor.dict.exists("test"));

        let spec = to
            .processor
            .dict
            .get("test")
            .expect("Missing summary spectrum")
            .borrow();
        assert_eq!(String::from("test"), spec.get_name());
        assert_eq!(String::from("Summary"), spec.get_type());
        assert_eq!(params, spec.get_xparams());
        assert_eq!(0, spec.get_yparams().len());
        assert!(spec.get_xaxis().is_none());
        let y = spec.get_yaxis().expect("Missing y axis ");
        assert_eq!(
            AxisSpecification {
                low: 0.0,
                high: 1.0,
                bins: 102,
            },
            AxisSpecification {
                low: y.0,
                high: y.1,
                bins: y.2
            }
        );
        assert!(spec.get_gate().is_none());
    }
    #[test]
    fn crsummary_2() {
        // bad parameter name:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.14"), // bad
            String::from("param.8"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreateSummary {
                name: String::from("test"),
                params: params.clone(),
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn crsummary_3() {
        // duplicate spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.8"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreateSummary {
                name: String::from("test"),
                params: params.clone(),
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);
        let reply = to.processor.process_request(
            SpectrumRequest::CreateSummary {
                name: String::from("test"),
                params: params.clone(),
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn cr2d_1() {
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.5"),
                yparam: String::from("param.7"),
                xaxis: AxisSpecification {
                    low: -10.0,
                    high: 10.0,
                    bins: 100,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 256,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);
        let spec = to
            .processor
            .dict
            .get("test")
            .expect("Missing spectru")
            .borrow();

        assert_eq!(String::from("test"), spec.get_name());
        assert_eq!(String::from("2D"), spec.get_type());
        let xp = spec.get_xparams();
        assert_eq!(1, xp.len());
        assert_eq!(String::from("param.5"), xp[0]);
        let yp = spec.get_yparams();
        assert_eq!(1, yp.len());
        assert_eq!(String::from("param.7"), yp[0]);

        let x = spec.get_xaxis().expect("Missing x axis");
        assert_eq!(
            AxisSpecification {
                low: -10.0,
                high: 10.0,
                bins: 102
            },
            AxisSpecification {
                low: x.0,
                high: x.1,
                bins: x.2
            }
        );
        let y = spec.get_yaxis().expect("Missing y axis");
        assert_eq!(
            AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 258
            },
            AxisSpecification {
                low: y.0,
                high: y.1,
                bins: y.2
            }
        );
        assert!(spec.get_gate().is_none());
    }
    #[test]
    fn cr2d_2() {
        // invalid x parameter.

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.15"),
                yparam: String::from("param.7"),
                xaxis: AxisSpecification {
                    low: -10.0,
                    high: 10.0,
                    bins: 100,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 256,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn cr2d_3() {
        // invalid y parameter;

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.5"),
                yparam: String::from("param.17"),
                xaxis: AxisSpecification {
                    low: -10.0,
                    high: 10.0,
                    bins: 100,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 256,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn cr2d_4() {
        // duplicate spectrum:
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.5"),
                yparam: String::from("param.7"),
                xaxis: AxisSpecification {
                    low: -10.0,
                    high: 10.0,
                    bins: 100,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 256,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.5"),
                yparam: String::from("param.7"),
                xaxis: AxisSpecification {
                    low: -10.0,
                    high: 10.0,
                    bins: 100,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 256,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn cr2dsum_1() {
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let xpars = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.6"),
            String::from("param.7"),
        ];
        let ypars = vec![
            String::from("param.1"),
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
            String::from("param.9"),
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Create2DSum {
                name: String::from("test"),
                xparams: xpars.clone(),
                yparams: ypars.clone(),
                xaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let spec = to
            .processor
            .dict
            .get("test")
            .expect("Could not find spectrum")
            .borrow();
        assert_eq!(String::from("test"), spec.get_name());
        assert_eq!(String::from("2DSum"), spec.get_type());
        assert_eq!(xpars, spec.get_xparams());
        assert_eq!(ypars, spec.get_yparams());
        assert!(spec.get_gate().is_none());
        let x = spec.get_xaxis().expect("Missing x axis");
        assert_eq!(
            AxisSpecification {
                low: -1.0,
                high: 1.0,
                bins: 514,
            },
            AxisSpecification {
                low: x.0,
                high: x.1,
                bins: x.2
            }
        );
        let y = spec.get_yaxis().expect("Missing y axis");
        assert_eq!(
            AxisSpecification {
                low: 0.0,
                high: 4096.0,
                bins: 514,
            },
            AxisSpecification {
                low: y.0,
                high: y.1,
                bins: y.2
            }
        );
    }
    #[test]
    fn cr2dsum_2() {
        // bad x parameter:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let xpars = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.16"),
            String::from("param.7"),
        ];
        let ypars = vec![
            String::from("param.1"),
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
            String::from("param.9"),
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Create2DSum {
                name: String::from("test"),
                xparams: xpars.clone(),
                yparams: ypars.clone(),
                xaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn cr2dsum_3() {
        // bad y parameter:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let xpars = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.6"),
            String::from("param.7"),
        ];
        let ypars = vec![
            String::from("param.11"),
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
            String::from("param.9"),
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Create2DSum {
                name: String::from("test"),
                xparams: xpars.clone(),
                yparams: ypars.clone(),
                xaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn cr2dsum_4() {
        // duplicate spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let xpars = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.6"),
            String::from("param.7"),
        ];
        let ypars = vec![
            String::from("param.1"),
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
            String::from("param.9"),
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Create2DSum {
                name: String::from("test"),
                xparams: xpars.clone(),
                yparams: ypars.clone(),
                xaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2DSum {
                name: String::from("test"),
                xparams: xpars.clone(),
                yparams: ypars.clone(),
                xaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn del_1() {
        // delete an existing spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::Delete(String::from("test")),
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Deleted, reply);
        assert!(!to.processor.dict.exists("test"));
    }
    #[test]
    fn del_2() {
        // the right one is deleted:

        // delete an existing spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("test.{}", i);
            let pname = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: pname,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }

        let reply = to.processor.process_request(
            SpectrumRequest::Delete(String::from("test.5")),
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Deleted, reply);
        assert!(!to.processor.dict.exists("test.5"));
    }
    #[test]
    fn del_3() {
        // Delete nonexisting is an error:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("test.{}", i);
            let pname = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: pname,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }
        let reply = to.processor.process_request(
            SpectrumRequest::Delete(String::from("param.1")),
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn clear_1() {
        // Put some data in a histogram then clear it

        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("test.{}", i);
            let pname = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: pname,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }
        let spec = to.processor.dict.get("test.1").expect("Missing spectrum");
        let h = spec
            .borrow()
            .get_histogram_1d()
            .expect("Not 1d but should be");
        h.borrow_mut().fill(&100.0);
        h.borrow_mut().fill(&110.0);

        // good enough for now I suspect clear them all.

        let reply = to.processor.process_request(
            SpectrumRequest::Clear(String::from("*")),
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Cleared, reply);
        let mut sum = 0.0;
        for c in h.borrow().iter() {
            sum += c.value.get();
        }
        assert_eq!(0.0, sum);
    }
    #[test]
    fn clear_2() {
        // pattern selectivity:
        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("test.{}", i);
            let pname = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: pname,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }
        let spec = to.processor.dict.get("test.1").expect("Missing spectrum");
        let h = spec
            .borrow()
            .get_histogram_1d()
            .expect("Not 1d but should be");
        h.borrow_mut().fill(&100.0);
        h.borrow_mut().fill(&110.0);

        // Clear the 'wrong' one:

        let reply = to.processor.process_request(
            SpectrumRequest::Clear(String::from("test.2")),
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Cleared, reply);
        let mut sum = 0.0;
        for c in h.borrow().iter() {
            sum += c.value.get();
        }
        assert_eq!(2.0, sum); // did not clear.

        // clear the right one:

        let reply = to.processor.process_request(
            SpectrumRequest::Clear(String::from("test.1")),
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Cleared, reply);
        let mut sum = 0.0;
        for c in h.borrow().iter() {
            sum += c.value.get();
        }
        assert_eq!(0.0, sum);
    }
    #[test]
    fn list_1() {
        // list all spectra.

        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("test.{}", i);
            let pname = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: pname,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }

        let reply = to.processor.process_request(
            SpectrumRequest::List(String::from("*")),
            &to.parameters,
            &mut to.conditions,
        );

        if let SpectrumReply::Listing(mut l) = reply {
            assert_eq!(10, l.len());

            // There's no ordering so order by name:
            l.sort_by(|a, b| {
                if a.name > b.name {
                    Ordering::Greater
                } else if a.name < b.name {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            });

            // /The listing comes in an arbitrary order so:

            for i in 0..10 {
                let name = format!("test.{}", i);
                let pname = format!("param.{}", i);

                assert_eq!(name, l[i].name);
                assert_eq!(String::from("1D"), l[i].type_name);
                assert_eq!(vec![pname], l[i].xparams);
                assert_eq!(0, l[i].yparams.len());
                assert!(l[i].yaxis.is_none());

                assert_eq!(
                    AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1026,
                    },
                    l[i].xaxis.expect("No x axis")
                );
                assert!(l[i].gate.is_none());
            }
        } else {
            panic!("listing failed");
        }
    }
    #[test]
    fn list_2() {
        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("test.{}", i);
            let pname = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: pname,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }

        let reply = to.processor.process_request(
            SpectrumRequest::List(String::from("test.9")),
            &to.parameters,
            &mut to.conditions,
        );
        if let SpectrumReply::Listing(l) = reply {
            assert_eq!(1, l.len());
            // Just check the name as we know the rest is ok from
            // list_1:

            assert_eq!(String::from("test.9"), l[0].name);
        } else {
            panic!("Listing failed");
        }
    }

    // For our gate test we need some gates:

    fn make_some_gates(cd: &mut ConditionDictionary) {
        for i in 0..10 {
            let name = format!("cond.{}", i);
            cd.insert(name, Rc::new(RefCell::new(Box::new(conditions::True {}))));
        }
    }
    #[test]
    fn gate_1() {
        let mut to = make_test_objs();
        make_some_params(&mut to);
        make_some_gates(&mut to.conditions);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::Gate {
                spectrum: String::from("test"),
                gate: String::from("cond.5"),
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Gated, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::List(String::from("*")),
            &to.parameters,
            &mut to.conditions,
        );
        if let SpectrumReply::Listing(l) = reply {
            assert_eq!(1, l.len());
            assert_eq!(
                String::from("cond.5"),
                l[0].clone().gate.expect("Missing gate")
            );
        } else {
            panic!("Listing failed");
        }
    }
    #[test]
    fn gate_2() {
        // No such gate:
        let mut to = make_test_objs();
        make_some_params(&mut to);
        make_some_gates(&mut to.conditions);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::Gate {
                spectrum: String::from("test"),
                gate: String::from("kond.5"),
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn gate_3() {
        // no such spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        make_some_gates(&mut to.conditions);
        let reply = to.processor.process_request(
            SpectrumRequest::Gate {
                spectrum: String::from("test"),
                gate: String::from("cond.5"),
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }

    #[test]
    fn ungate_1() {
        // Good ungate:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        make_some_gates(&mut to.conditions);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::Gate {
                spectrum: String::from("test"),
                gate: String::from("cond.5"),
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Gated, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::Ungate(String::from("test")),
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Ungated, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::List(String::from("test")),
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Listing(l) = reply {
            assert_eq!(1, l.len());
            assert!(l[0].gate.is_none());
            true
        } else {
            false
        });
    }
    #[test]
    fn ungate_2() {
        // no such spectrum

        let mut to = make_test_objs();
        make_some_params(&mut to);
        make_some_gates(&mut to.conditions);

        let reply = to.processor.process_request(
            SpectrumRequest::Ungate(String::from("test")),
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn events_1() {
        // Increment some spectra via an event:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("spec.{}", i);
            let par = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: par,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }
        // Make some evnts and fill some (not all) of the spectra:

        let id1 = to.parameters.lookup("param.5").unwrap().get_id();
        let id2 = to.parameters.lookup("param.7").unwrap().get_id();

        let events = vec![
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Events(events),
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Processed, reply);
        let with_counts = vec![
            (String::from("spec.5"), 512.0),
            (String::from("spec.7"), 700.0),
        ];
        let no_counts = vec![
            String::from("spec.0"),
            String::from("spec.1"),
            String::from("spec.2"),
            String::from("spec.3"),
            String::from("spec.4"),
            String::from("spec.6"),
            String::from("spec.8"),
            String::from("spec.9"),
        ];
        // These should havve counts in the indicated channels:

        for (name, chan) in with_counts {
            let spec = to.processor.dict.get(&name).unwrap().borrow();
            for ch in spec.get_histogram_1d().unwrap().borrow().iter() {
                let d = ch.value.get();
                if d != 0.0 {
                    assert_eq!(5.0, d);
                    if let BinInterval::Bin { start, end } = ch.bin {
                        assert_eq!(chan, start);
                    } else {
                        panic!("Under or overflow counts in histogram");
                    }
                }
            }
        }
        // these should have no counts.
        for name in no_counts {
            let spec = to.processor.dict.get(&name).unwrap().borrow();
            for ch in spec.get_histogram_1d().unwrap().borrow().iter() {
                assert_eq!(0.0, ch.value.get());
            }
        }
    }
    #[test]
    fn contents_1() {
        // Process some events as in events_1, get the
        // contents of the spectra...
        // should be one channel entry for each of the
        // two histograms with data and non for those with none

        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("spec.{}", i);
            let par = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: par,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }
        // Make some evnts and fill some (not all) of the spectra:

        let id1 = to.parameters.lookup("param.5").unwrap().get_id();
        let id2 = to.parameters.lookup("param.7").unwrap().get_id();

        let events = vec![
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Events(events),
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Processed, reply);
        let with_counts = vec![
            (String::from("spec.5"), 512.0),
            (String::from("spec.7"), 700.0),
        ];
        let no_counts = vec![
            String::from("spec.0"),
            String::from("spec.1"),
            String::from("spec.2"),
            String::from("spec.3"),
            String::from("spec.4"),
            String::from("spec.6"),
            String::from("spec.8"),
            String::from("spec.9"),
        ];
        // we'll ask for the entire ROI:
        for (name, chan) in with_counts {
            let reply = to.processor.process_request(
                SpectrumRequest::GetContents {
                    name: name,
                    xlow: 0.0,
                    xhigh: 1024.0,
                    ylow: 0.0,
                    yhigh: 0.0,
                },
                &to.parameters,
                &mut to.conditions,
            );
            assert!(if let SpectrumReply::Contents(sc) = reply {
                assert_eq!(1, sc.len());
                assert_eq!(ChannelType::Bin, sc[0].chan_type);
                assert_eq!(chan, sc[0].x);
                assert_eq!(5.0, sc[0].value);
                true
            } else {
                false
            });
        }
        // Nobody else should have counts:

        for name in no_counts {
            let reply = to.processor.process_request(
                SpectrumRequest::GetContents {
                    name: name,
                    xlow: 0.0,
                    xhigh: 1024.0,
                    ylow: 0.0,
                    yhigh: 0.0,
                },
                &to.parameters,
                &mut to.conditions,
            );
            assert!(if let SpectrumReply::Contents(sc) = reply {
                assert_eq!(0, sc.len());
                true
            } else {
                false
            });
        }
    }
    #[test]
    fn contents_2() {
        // Ask with ROI outside of where counts are:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("spec.{}", i);
            let par = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: par,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }
        // Make some evnts and fill some (not all) of the spectra:

        let id1 = to.parameters.lookup("param.5").unwrap().get_id();
        let id2 = to.parameters.lookup("param.7").unwrap().get_id();

        let events = vec![
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Events(events),
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Processed, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("spec.5"),
                xlow: 0.0,
                xhigh: 200.0,
                ylow: 0.0,
                yhigh: 0.0,
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Contents(sc) = reply {
            assert_eq!(0, sc.len());
            true
        } else {
            false
        });
    }
    #[test]
    fn events_2() {
        // Events for a 2-d histogram:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.5"),
                yparam: String::from("param.7"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);

        // Make and process events that will fill (512.0, 700.0):

        let id1 = to.parameters.lookup("param.5").unwrap().get_id();
        let id2 = to.parameters.lookup("param.7").unwrap().get_id();

        let events = vec![
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Events(events),
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Processed, reply);

        // Contents over the whole spectrum should only have 5
        // counts in channel 512.0, 700.0

        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 0.0,
                xhigh: 1024.0,
                ylow: 0.0,
                yhigh: 1024.0,
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Contents(l) = reply {
            assert_eq!(1, l.len());
            assert_eq!(ChannelType::Bin, l[0].chan_type);
            assert_eq!(5.0, l[0].value);
            assert_eq!(512.0, l[0].x);
            assert_eq!(700.0, l[0].y);
            true
        } else {
            false
        });
    }
    #[test]
    fn contents_3() {
        // 2d ROI checking:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.5"),
                yparam: String::from("param.7"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Created, reply);

        // Make and process events that will fill (512.0, 700.0):

        let id1 = to.parameters.lookup("param.5").unwrap().get_id();
        let id2 = to.parameters.lookup("param.7").unwrap().get_id();

        let events = vec![
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Events(events),
            &to.parameters,
            &mut to.conditions,
        );
        assert_eq!(SpectrumReply::Processed, reply);

        // Contents over the whole spectrum should only have 5
        // counts in channel 512.0, 700.0

        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 0.0,
                xhigh: 1024.0,
                ylow: 100.0,
                yhigh: 300.0, // Too small for ROI.
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Contents(l) = reply {
            assert_eq!(0, l.len());
            true
        } else {
            false
        });
        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 0.0,
                xhigh: 1024.0,
                ylow: 720.0, // Too large for ROI
                yhigh: 1024.0,
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Contents(l) = reply {
            assert_eq!(0, l.len());
            true
        } else {
            false
        });

        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 0.0,
                xhigh: 200.0, // Small for ROI
                ylow: 0.0,
                yhigh: 1024.0,
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Contents(l) = reply {
            assert_eq!(0, l.len());
            true
        } else {
            false
        });
        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 600.0, // too big for ROI.
                xhigh: 1024.0,
                ylow: 0.0,
                yhigh: 1024.0,
            },
            &to.parameters,
            &mut to.conditions,
        );
        assert!(if let SpectrumReply::Contents(l) = reply {
            assert_eq!(0, l.len());
            true
        } else {
            false
        });
    }
}
#[cfg(test)]
mod reqstruct_tests {
    // Test the request structure marshallers.
    use super::*;
    use crate::parameters::*;

    #[test]
    fn c1d_1() {
        let req = create1d_request("test", "par1", 0.0, 1024.0, 1024);
        assert_eq!(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("par1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024
                }
            },
            req
        )
    }
    #[test]
    fn cm1d_1() {
        let req = createmulti1d_request(
            "test",
            &vec![String::from("p1"), String::from("p2"), String::from("p3")],
            0.0,
            1024.0,
            1024,
        );
        assert!(
            if let SpectrumRequest::CreateMulti1D { name, params, axis } = req {
                assert_eq!(String::from("test"), name);
                assert_eq!(
                    vec![String::from("p1"), String::from("p2"), String::from("p3")],
                    params
                );
                assert_eq!(
                    AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024
                    },
                    axis
                );
                true
            } else {
                false
            }
        );
    }
    #[test]
    fn cm2d_1() {
        let p = vec![String::from("p1"), String::from("p2"), String::from("p3")];
        let req = createmulti2d_request("test", &p, 0.0, 1024.0, 1024, -1.0, 1.0, 100);
        assert!(if let SpectrumRequest::CreateMulti2D {
            name,
            params,
            xaxis,
            yaxis,
        } = req
        {
            assert_eq!(String::from("test"), name);
            assert_eq!(p, params);
            assert_eq!(
                AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024
                },
                xaxis
            );
            assert_eq!(
                AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100
                },
                yaxis
            );
            true
        } else {
            false
        });
    }
    #[test]
    fn cpgamma_1() {
        let xp = vec![String::from("x1"), String::from("x2"), String::from("x3")];
        let yp = vec![String::from("y1"), String::from("y2")];

        let req = createpgamma_request("test", &xp, &yp, 0.0, 1024.0, 1024, -1.0, 1.0, 100);
        assert!(if let SpectrumRequest::CreatePGamma {
            name,
            xparams,
            yparams,
            xaxis,
            yaxis,
        } = req
        {
            assert_eq!(String::from("test"), name);
            assert_eq!(xp, xparams);
            assert_eq!(yp, yparams);
            assert_eq!(
                AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024
                },
                xaxis
            );
            assert_eq!(
                AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100
                },
                yaxis
            );
            true
        } else {
            false
        });
    }
    #[test]
    fn c2d_1() {
        let req = create2d_request("test", "px", "py", 0.0, 1024.0, 1024, -1.0, 1.0, 100);
        assert_eq!(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("px"),
                yparam: String::from("py"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024
                },
                yaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100
                }
            },
            req
        );
    }
    #[test]
    fn c2dsum_1() {
        let xp = vec![String::from("x1"), String::from("x2"), String::from("x3")];
        let yp = vec![String::from("y1"), String::from("y2"), String::from("y3")];

        let req = create2dsum_request("test", &xp, &yp, 0.0, 1024.0, 1024, -1.0, 1.0, 100);
        assert_eq!(
            SpectrumRequest::Create2DSum {
                name: String::from("test"),
                xparams: xp.clone(),
                yparams: yp.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024
                },
                yaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100
                }
            },
            req
        );
    }
    #[test]
    fn del_1() {
        let req = delete_request("test");
        assert_eq!(SpectrumRequest::Delete(String::from("test")), req);
    }
    #[test]
    fn list_1() {
        let req = list_request("*");
        assert_eq!(SpectrumRequest::List(String::from("*")), req);
    }
    #[test]
    fn gate_1() {
        let req = gate_request("spectrum", "gate");
        assert_eq!(
            SpectrumRequest::Gate {
                spectrum: String::from("spectrum"),
                gate: String::from("gate")
            },
            req
        );
    }
    #[test]
    fn ungate_1() {
        let req = ungate_request("test");
        assert_eq!(SpectrumRequest::Ungate(String::from("test")), req)
    }
    #[test]
    fn clear_1() {
        let req = clear_request("t*");
        assert_eq!(SpectrumRequest::Clear(String::from("t*")), req);
    }
    #[test]
    fn get_1() {
        let req = getcontents_request("test", 0.0, 50.0, 100.0, 125.0);
        assert_eq!(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 0.0,
                xhigh: 50.0,
                ylow: 100.0,
                yhigh: 125.0
            },
            req
        );
    }
    fn process_1() {
        let events = vec![
            vec![EventParameter::new(1, 2.0), EventParameter::new(7, 100.)],
            vec![
                EventParameter::new(12, 1.345),
                EventParameter::new(77, 3.1416),
            ],
            vec![
                EventParameter::new(1, 2.0),
                EventParameter::new(7, 100.),
                EventParameter::new(12, 1.345),
                EventParameter::new(77, 3.1416),
            ],
        ];
        let req = events_request(&events);
        assert_eq!(SpectrumRequest::Events(events), req);
    }
}
#[cfg(test)]
mod spectrum_api_tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;

    // This is a fake server thread:
    // Creates the spectrum processor, a parameter dictionary
    // with few parameters in it and a condition dictionary
    // with a few harmless conditions.
    // Then accepts Requests until Exit.  If something other
    // than Exit or a Spectrum request arrives, panics.
    // Spectrum requests are passed to the spectrum processor
    // and the return is used to provide a spectrum reply that's
    // send back to the client.
    // All of this supports testing the spectrum section of the
    // histogram server.
    // See also:
    //   start_server - which starts the server.
    //   stop_server - which ends the server and joins with it.
    //
    // Note failing tests can leave hanging threads but
    // they are harmless as new servers are creaed for each
    // test.
    fn fake_server(reader: mpsc::Receiver<Request>) {
        let mut processor = SpectrumProcessor::new();
        let mut params = parameters::ParameterDictionary::new();
        let mut cdict = conditions::ConditionDictionary::new();

        // Make some parameters:
        // Note these wil have ids 1..10 (white box).

        for i in 0..10 {
            params
                .add(&format!("param.{}", i))
                .expect("Failed to add parameters");
        }
        // Make some conditions:

        for i in 0..10 {
            cdict.insert(
                format!("true.{}", i),
                Rc::new(RefCell::new(Box::new(conditions::True {}))),
            );
        }
        for i in 0..10 {
            cdict.insert(
                format!("false.{}", i),
                Rc::new(RefCell::new(Box::new(conditions::False {}))),
            );
        }
        // process requests:

        loop {
            let request = reader.recv().expect("Request read failed");
            match request.message {
                MessageType::Exit => {
                    request.reply_channel.send(Reply::Exiting);
                    break;
                }
                MessageType::Spectrum(sreq) => {
                    let reply = processor.process_request(sreq, &params, &mut cdict);
                    request
                        .reply_channel
                        .send(Reply::Spectrum(reply))
                        .expect("Reply to client failed");
                }
                _ => {
                    panic!("Unexpected message type in fake server");
                }
            }
        }
    }
    // Starting the server returns a join handle and the request channel.

    fn start_server() -> (thread::JoinHandle<()>, mpsc::Sender<Request>) {
        let (sender, receiver) = mpsc::channel::<Request>();
        let handle = thread::spawn(move || fake_server(receiver));
        (handle, sender)
    }
    fn stop_server(handle: thread::JoinHandle<()>, req_chan: mpsc::Sender<Request>) {
        let (repl_send, repl_receive) = mpsc::channel::<Reply>();
        let req = Request {
            reply_channel: repl_send,
            message: MessageType::Exit,
        };
        let reply = req.transaction(req_chan, repl_receive);
        if let Reply::Exiting = reply {
            handle.join().expect("Fake server join failed");
        } else {
            panic!("Requested exit from server but didn't get back Exiting reply");
        }
    }
    // Note that tests will need for list to work to probe server contents.
    // (alternative is to wrap the spectrum processor in an Arc/Mutex and make
    // it shared but we need list to work anyway so wth):
    #[test]
    fn list_1() {
        let (jh, send) = start_server();
        let (rep_send, rep_recv) = mpsc::channel::<Reply>();
        let reply = list_spectra("*", send.clone(), rep_send, rep_recv);
        assert!(if let Ok(l) = reply {
            assert_eq!(0, l.len()); // Nothing to list
            true
        } else {
            false
        });
        stop_server(jh, send);
    }
    // Now we can try to make a spectrum and see if we can get
    // it listed back:
    // Note the need to clone channels and make reply channels each
    // time since Receivers don't support cloning (that's the single receiver
    // part of these channels getting enforced)
    #[test]
    fn make1d_1() {
        let (jh, send) = start_server();
        let (rep_send, rep_recv) = mpsc::channel::<Reply>();

        // Create the spectrum:

        assert!(if let Ok(()) = create_spectrum_1d(
            "test",
            "param.1",
            0.0,
            1024.0,
            1024,
            send.clone(),
            rep_send,
            rep_recv
        ) {
            true
        } else {
            false
        });
        // See if the server knows it:

        let (rep_send, rep_recv) = mpsc::channel::<Reply>();
        assert!(
            if let Ok(listing) = list_spectra("*", send.clone(), rep_send, rep_recv) {
                assert_eq!(1, listing.len());
                assert_eq!(
                    SpectrumProperties {
                        name: String::from("test"),
                        type_name: String::from("1D"),
                        xparams: vec![String::from("param.1")],
                        yparams: vec![],
                        xaxis: Some(AxisSpecification {
                            low: 0.0,
                            high: 1024.0,
                            bins: 1026
                        }),
                        yaxis: None,
                        gate: None
                    },
                    listing[0]
                );
                true
            } else {
                false
            }
        );

        stop_server(jh, send);
    }
    #[test]
    fn make1dmulti_1() {
        let (jh, send) = start_server();
        let (rep_send, rep_recv) = mpsc::channel::<Reply>();
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.3"),
            String::from("param.4"),
            String::from("param.5"),
        ];
        assert_eq!(
            Ok(()),
            create_spectrum_multi1d(
                "test",
                &params,
                0.0,
                1024.0,
                1024,
                send.clone(),
                rep_send,
                rep_recv
            )
        );

        let (rep_send, rep_recv) = mpsc::channel::<Reply>();
        assert!(
            if let Ok(l) = list_spectra("*", send.clone(), rep_send, rep_recv) {
                assert_eq!(1, l.len());
                assert_eq!(
                    SpectrumProperties {
                        name: String::from("test"),
                        type_name: String::from("Multi1d"),
                        xparams: params,
                        yparams: vec![],
                        xaxis: Some(AxisSpecification {
                            low: 0.0, high: 1024.0, bins: 1026
                        }),
                        yaxis: None,
                        gate: None
                    },
                    l[0]
                );
                true
            } else {
                false
            }
        );

        stop_server(jh, send);
    }
}
