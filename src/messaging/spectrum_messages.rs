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

#[derive(Clone, Debug, PartialEq)]
pub struct AxisSpecification {
    pub low: f64,
    pub high: f64,
    pub bins: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub enum ChannelType {
    Underflow,
    Overflow,
    Bin,
}
#[derive(Clone, Debug, PartialEq)]
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
                                result.push(Channel {
                                    chan_type: ChannelType::Bin,
                                    x: start,
                                    y: 0.0,
                                    value: v,
                                });
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
                    result.push(Channel {
                        chan_type: ctype,
                        x: x,
                        y: y,
                        value: v,
                    });
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
        for i in 0..9 {
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
}
