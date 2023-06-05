//!  Multi1d spectra are spectra that are defined on several parameters.
//!  Each parameter in the event that has been given a value increments
//!  the histogram on a common X axis.  In SpecTcl, these spectra were
//!  called Gamma-1d spectra because they were used primarily to accumulate
//!  summed spectra of the detectors in a gamma ray spectrometer array.
//!
//!  Multi1d spectra can have a gate as well which can conditionalize when
//!  the spectrum is incremented.  
//!
//!  Axis defaults are determined in a manner identical to the way
//!  axis defaults for the y axis of a summary spectrum are determined.
//!  
//!  
use super::*;
use ndhistogram::value::Sum;

///
/// *  applied_gate - is the gate that can conditionalize increments.
/// *  name         - is the name of the spectrum.
/// *  histogram    - is the actual histogram object.
/// *  param_names  - are the names of the parameters we're defined on.
/// *  param_ids    - Are the corresponding parameter ids (indices into FlatEvent).
///
pub struct Multi1d {
    applied_gate: SpectrumGate,
    name: String,
    histogram: H1DContainer,
    param_names: Vec<String>,
    param_ids: Vec<u32>,
}
//
impl Spectrum for Multi1d {
    fn check_gate(&mut self, e: &FlatEvent) -> bool {
        self.applied_gate.check(e)
    }
    fn increment(&mut self, e: &FlatEvent) {
        let mut histogram = self.histogram.borrow_mut();
        for id in self.param_ids.iter() {
            if let Some(x) = e[*id] {
                histogram.fill(&x);
            }
        }
    }
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_type(&self) -> String {
        String::from("Multi1d")
    }
    fn get_xparams(&self) -> Vec<String> {
        self.param_names.clone()
    }
    fn get_yparams(&self) -> Vec<String> {
        vec![]
    }
    fn get_xaxis(&self) -> Option<(f64, f64, u32)> {
        let x = self.histogram.borrow().axes().as_tuple().0.clone();
        Some((*x.low(), *x.high(), x.num_bins() as u32))
    }
    fn get_yaxis(&self) -> Option<(f64, f64, u32)> {
        None
    }
    fn get_gate(&self) -> Option<String> {
        if let Some(g) = self.applied_gate.gate.clone() {
            Some(g.condition_name)
        } else {
            None
        }
    }

    fn gate(&mut self, name: &str, dict: &ConditionDictionary) -> Result<(), String> {
        self.applied_gate.set_gate(name, dict)
    }
    fn ungate(&mut self) {
        self.applied_gate.ungate()
    }
    fn get_histogram_1d(&self) -> Option<H1DContainer> {
        Some(Rc::clone(&self.histogram))
    }
    fn get_histogram_2d(&self) -> Option<H2DContainer> {
        None
    }

    fn clear(&mut self) {
        for c in self.histogram.borrow_mut().iter_mut() {
            *c.value = Sum::new();
        }
    }
}

impl Multi1d {
    /// Create a spectrum of type Multi1d:
    /// *    name - the name of the new spectrm.
    /// *    params - Vector of the names of parameters we're defined on.
    /// *    pdict  - References the parameter dictionary the parameters are defined in.
    /// *    low   - X axis low limit (or None for default).
    /// *    high  - X axis high limit (or None for default).
    /// *    bins  - X axis bins (or None for default).
    ///
    /// Default low is the smallest of the paramter default lows.
    /// Default high is the largest of the paramete default highs.
    /// Default bins is the largest of the parameter default bins.
    ///
    pub fn new(
        name: &str,
        params: Vec<String>,
        pdict: &ParameterDictionary,
        low: Option<f64>,
        high: Option<f64>,
        bins: Option<u32>,
    ) -> Result<Multi1d, String> {
        let mut xlow = None;
        let mut xmax = None;
        let mut xbins = None;

        let mut param_names = Vec::<String>::new();
        let mut param_ids = Vec::<u32>::new();

        for name in params {
            if let Some(p) = pdict.lookup(&name) {
                param_names.push(name);
                param_ids.push(p.get_id());
                let lims = p.get_limits();
                let b = p.get_bins();
                xlow = optmin(xlow, lims.0);
                xmax = optmax(xmax, lims.1);
                xbins = optmax(xbins, b);
            } else {
                return Err(format!("Parameter {} does not exist", name));
            }
        }
        // override defaults for axes etc:
        if let Some(l) = low {
            xlow = Some(l);
        }
        if let Some(h) = high {
            xmax = Some(h);
        }
        if let Some(b) = bins {
            xbins = Some(b);
        }

        if let None = xlow {
            return Err(String::from("X axis low limit cannot be defaulted"));
        }
        if let None = xmax {
            return Err(String::from("X axis high limit cannot be defaulted"));
        }
        if let None = xbins {
            return Err(String::from("X axis binning cannot be defaulted"));
        }
        Ok(Multi1d {
            applied_gate: SpectrumGate::new(),
            name: String::from(name),
            histogram: Rc::new(RefCell::new(ndhistogram!(
                axis::Uniform::new(xbins.unwrap() as usize, xlow.unwrap(), xmax.unwrap());
                Sum
            ))),
            param_names: param_names,
            param_ids: param_ids,
        })
    }
}

#[cfg(test)]
mod multi1d_tests {
    use super::*;
    // use ndhistogram::axis::*;
    use std::cell::RefCell; // Needed in gating
    use std::rc::Rc; // Needed in gating.
    #[test]
    fn new_1() {
        // Success with default x axis and uniform parameter defs:

        let mut pdict = ParameterDictionary::new();
        let mut names = Vec::<String>::new();
        for i in 0..10 {
            let name = format!("param.{}", i);
            names.push(name.clone());
            pdict.add(&name).expect("Could not add parameter");
            let p = pdict.lookup_mut(&name).unwrap();
            p.set_limits(0.0, 1023.0);
            p.set_bins(1024);
            p.set_description("Some things in arb units");
        }
        let result = Multi1d::new("Testing", names, &pdict, None, None, None);
        assert!(result.is_ok());
        let s = result.unwrap();
        assert!(s.applied_gate.gate.is_none());
        assert_eq!(String::from("Testing"), s.name);

        assert_eq!(1, s.histogram.borrow().axes().num_dim());
        let x = s.histogram.borrow().axes().as_tuple().0.clone();
        assert_eq!(0.0, *x.low());
        assert_eq!(1023.0, *x.high());
        assert_eq!(1024 + 2, x.num_bins());

        assert_eq!(10, s.param_names.len());
        assert_eq!(10, s.param_ids.len());
        for i in 0..10 {
            let name = format!("param.{}", i);
            assert_eq!(name, s.param_names[i]);
        }
    }
    #[test]
    fn new_2() {
        // non uniform parameter defs:

        let mut pdict = ParameterDictionary::new();
        let mut names = Vec::<String>::new();
        for i in 0..10 {
            let name = format!("param.{}", i);
            names.push(name.clone());
            pdict.add(&name).expect("Could not add parameter");
            let p = pdict.lookup_mut(&name).unwrap();
            p.set_limits(0.0, 1023.0);
            p.set_bins(1024);
            p.set_description("Some things in arb units");
        }
        {
            let p = pdict.lookup_mut("param.5").unwrap();
            p.set_limits(-2048.0, 2048.0);
            p.set_bins(4096);
        }
        let result = Multi1d::new("Testing", names, &pdict, None, None, None);
        assert!(result.is_ok());
        let s = result.unwrap();
        assert!(s.applied_gate.gate.is_none());
        assert_eq!(String::from("Testing"), s.name);

        assert_eq!(1, s.histogram.borrow().axes().num_dim());
        let x = s.histogram.borrow().axes().as_tuple().0.clone();
        assert_eq!(-2048.0, *x.low());
        assert_eq!(2048.0, *x.high());
        assert_eq!(4096 + 2, x.num_bins());

        assert_eq!(10, s.param_names.len());
        assert_eq!(10, s.param_ids.len());
        for i in 0..10 {
            let name = format!("param.{}", i);
            assert_eq!(name, s.param_names[i]);
        }
    }
    #[test]
    fn new_3() {
        // Override the histogram axis defaults:

        // Success with default x axis and uniform parameter defs:

        let mut pdict = ParameterDictionary::new();
        let mut names = Vec::<String>::new();
        for i in 0..10 {
            let name = format!("param.{}", i);
            names.push(name.clone());
            pdict.add(&name).expect("Could not add parameter");
            let p = pdict.lookup_mut(&name).unwrap();
            p.set_limits(0.0, 1023.0);
            p.set_bins(1024);
            p.set_description("Some things in arb units");
        }
        let result = Multi1d::new(
            "Testing",
            names,
            &pdict,
            Some(-2048.0),
            Some(2048.0),
            Some(4096),
        );
        assert!(result.is_ok());
        let s = result.unwrap();
        assert!(s.applied_gate.gate.is_none());
        assert_eq!(String::from("Testing"), s.name);

        assert_eq!(1, s.histogram.borrow().axes().num_dim());
        let x = s.histogram.borrow().axes().as_tuple().0.clone();
        assert_eq!(-2048.0, *x.low());
        assert_eq!(2048.0, *x.high());
        assert_eq!(4096 + 2, x.num_bins());

        assert_eq!(10, s.param_names.len());
        assert_eq!(10, s.param_ids.len());
        for i in 0..10 {
            let name = format!("param.{}", i);
            assert_eq!(name, s.param_names[i]);
        }
    }
    #[test]
    fn new_4() {
        // invalid parameter in the list:

        let mut pdict = ParameterDictionary::new();
        let mut names = Vec::<String>::new();
        for i in 0..10 {
            let name = format!("param.{}", i);
            names.push(name.clone());
            pdict.add(&name).expect("Could not add parameter");
            let p = pdict.lookup_mut(&name).unwrap();
            p.set_limits(0.0, 1023.0);
            p.set_bins(1024);
            p.set_description("Some things in arb units");
        }
        names.push(String::from("No-such"));
        let result = Multi1d::new(
            "Testing",
            names,
            &pdict,
            Some(-2048.0),
            Some(2048.0),
            Some(4096),
        );
        assert!(result.is_err());
    }
    #[test]
    fn new_5() {
        // Cannot default axis specs:

        let mut pdict = ParameterDictionary::new();
        let mut names = Vec::<String>::new();
        for i in 0..10 {
            let name = format!("param.{}", i);
            names.push(name.clone());
            pdict.add(&name).expect("Could not add parameter");
        }
        let result = Multi1d::new(
            "Testing",
            names.clone(),
            &pdict,
            None,
            Some(2048.0),
            Some(4096),
        );
        assert!(result.is_err());
        let result = Multi1d::new(
            "Testing",
            names.clone(),
            &pdict,
            Some(-2048.0),
            None,
            Some(4096),
        );
        assert!(result.is_err());
        let result = Multi1d::new(
            "Testing",
            names.clone(),
            &pdict,
            Some(0.0),
            Some(2048.0),
            None,
        );
        assert!(result.is_err());
    }
    // next tests that ensure the spectrum is properly  incremented.
    #[test]
    fn incr_1() {
        // Increment in all parameters ungated.

        let mut pdict = ParameterDictionary::new();
        let mut names = Vec::<String>::new();
        for i in 0..10 {
            let name = format!("param.{}", i);
            names.push(name.clone());
            pdict.add(&name).expect("Could not add parameter");
            let p = pdict.lookup_mut(&name).unwrap();
            p.set_limits(0.0, 1023.0);
            p.set_bins(1024);
            p.set_description("Some things in arb units");
        }
        let mut spec = Multi1d::new("Testing", names, &pdict, None, None, None).unwrap();

        let mut fe = FlatEvent::new();
        let mut e = Event::new();
        for i in 0..10 {
            let name = format!("param.{}", i);
            let pid = pdict.lookup_mut(&name).unwrap().get_id();
            e.push(EventParameter::new(pid, i as f64 * 10.0));
        }
        fe.load_event(&e);

        spec.handle_event(&fe);

        for i in 0..10 {
            let vo = spec
                .histogram
                .borrow()
                .value(&(i as f64 * 10.0))
                .expect("Value should exist")
                .clone();

            assert_eq!(1.0, vo.get());
        }
    }
    #[test]
    fn incr_2() {
        // Increment gated on T in all parameters.

        let mut pdict = ParameterDictionary::new();
        let mut names = Vec::<String>::new();
        for i in 0..10 {
            let name = format!("param.{}", i);
            names.push(name.clone());
            pdict.add(&name).expect("Could not add parameter");
            let p = pdict.lookup_mut(&name).unwrap();
            p.set_limits(0.0, 1023.0);
            p.set_bins(1024);
            p.set_description("Some things in arb units");
        }
        let mut spec = Multi1d::new("Testing", names, &pdict, None, None, None).unwrap();

        let mut fe = FlatEvent::new();
        let mut e = Event::new();
        for i in 0..10 {
            let name = format!("param.{}", i);
            let pid = pdict.lookup_mut(&name).unwrap().get_id();
            e.push(EventParameter::new(pid, i as f64 * 10.0));
        }
        fe.load_event(&e);

        let mut cd = ConditionDictionary::new();
        cd.insert(
            String::from("true"),
            Rc::new(RefCell::new(Box::new(True {}))),
        );
        spec.gate("true", &cd).expect("Can't gate");

        fe.load_event(&e);
        spec.handle_event(&fe);

        for i in 0..10 {
            let vo = spec
                .histogram
                .borrow()
                .value(&(i as f64 * 10.0))
                .expect("Value should exist")
                .clone();

            assert_eq!(1.0, vo.get());
        }
    }
    #[test]
    fn incr_3() {
        // Gated on F no increments happen:

        let mut pdict = ParameterDictionary::new();
        let mut names = Vec::<String>::new();
        for i in 0..10 {
            let name = format!("param.{}", i);
            names.push(name.clone());
            pdict.add(&name).expect("Could not add parameter");
            let p = pdict.lookup_mut(&name).unwrap();
            p.set_limits(0.0, 1023.0);
            p.set_bins(1024);
            p.set_description("Some things in arb units");
        }
        let mut spec = Multi1d::new("Testing", names, &pdict, None, None, None).unwrap();

        let mut fe = FlatEvent::new();
        let mut e = Event::new();
        for i in 0..10 {
            let name = format!("param.{}", i);
            let pid = pdict.lookup_mut(&name).unwrap().get_id();
            e.push(EventParameter::new(pid, i as f64 * 10.0));
        }
        fe.load_event(&e);

        let mut cd = ConditionDictionary::new();
        cd.insert(
            String::from("false"),
            Rc::new(RefCell::new(Box::new(False {}))),
        );
        spec.gate("false", &cd).expect("Can't gate");

        fe.load_event(&e);
        spec.handle_event(&fe);

        for i in 0..10 {
            let vo = spec
                .histogram
                .borrow()
                .value(&(i as f64 * 10.0))
                .expect("Value should exist")
                .clone();

            assert_eq!(0.0, vo.get());
        }
    }
    #[test]
    fn incr_4() {
        // Over/underflow - 1/2 will under, 1/2 will over:

        let mut pdict = ParameterDictionary::new();
        let mut names = Vec::<String>::new();
        for i in 0..10 {
            let name = format!("param.{}", i);
            names.push(name.clone());
            pdict.add(&name).expect("Could not add parameter");
            let p = pdict.lookup_mut(&name).unwrap();
            p.set_limits(0.0, 1023.0);
            p.set_bins(1024);
            p.set_description("Some things in arb units");
        }
        let mut spec = Multi1d::new("Testing", names, &pdict, None, None, None).unwrap();

        let mut fe = FlatEvent::new();
        let mut e = Event::new();
        for i in 0..10 {
            let v = if i % 2 == 0 { -1.0 } else { 1024.0 };
            let name = format!("param.{}", i);
            let pid = pdict.lookup_mut(&name).unwrap().get_id();
            e.push(EventParameter::new(pid, v));
        }
        fe.load_event(&e);
        spec.handle_event(&fe);
        assert_eq!(5.0, spec.histogram.borrow().value(&-1.0).unwrap().get());
        assert_eq!(5.0, spec.histogram.borrow().value(&1025.0).unwrap().get());
    }
}
