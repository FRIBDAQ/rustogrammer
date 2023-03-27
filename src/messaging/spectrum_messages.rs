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
                    if (v != 0.0) &&(x >= xlow) && (x <= xhigh) && (y >= ylow) && (y <= yhigh) {
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
}
