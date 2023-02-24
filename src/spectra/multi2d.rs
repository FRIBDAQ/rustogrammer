//! Multi2d spectra are what SpecTcl called gamma d spectra.
//! They are defined on at least two parameters.  For each
//! event, all valid parameter pairs are incremented.  
//!
//! For example, suppose a Multi2d spectrunm defined on
//! parameters with ids 1,2,3,4 and an event with parameters
//! 1,3,4,7,8 set, this Multi2d spectrum will
//! increment for parameter pairs (1,3), (1,4), (2,4).
//! Where the first parameter id is the X parameter and the
//! second the Y parameter.
//!
//! Axis defaults are handled as for 2-d spectra with the default
//! Guaranteed to be a square spectrum.  The defaults for either x/y
//! can be overidden at construction time, however.
//!  
//! As with any spectrum a condition can be applied as a gate on
//! incrementing the spectrum.  Gated spectra will only be incremented
//! for events for which the gating condition is true.

use super::*;
use ndhistogram::value::Sum;

pub struct Multi2d {
    applied_gate: SpectrumGate,
    name: String,
    histogram: Hist2D<axis::Uniform, axis::Uniform, Sum>,
    param_names: Vec<String>,
    param_ids: Vec<u32>,
}

// The spectrum trait must be implemented to support
// dynamic dispatch of gating and incrementing:

impl Spectrum for Multi2d {
    fn check_gate(&mut self, e: &FlatEvent) -> bool {
        self.applied_gate.check(e)
    }
    
    fn increment(&mut self, e: &FlatEvent) {
        for a in 0..self.param_ids.len() {
            for b in (a + 1)..self.param_ids.len() {
                let x = e[self.param_ids[a] as u32];
                let y = e[self.param_ids[b] as u32];
                if x.is_some() && y.is_some() {
                    let x = x.unwrap();
                    let y = y.unwrap();
                    self.histogram.fill(&(x, y));
                }
            }
        }
    }

    fn gate(&mut self, name: &str, dict: &ConditionDictionary) -> Result<(), String> {
        self.applied_gate.set_gate(name, dict)
    }
    fn ungate(&mut self) {
        self.applied_gate.ungate()
    }
    fn clear(&mut self) {
        for c in self.histogram.iter_mut() {
            *c.value = Sum::new();
        }
    }
}
impl Multi2d {
    /// Create a multi2d spectrum.
    /// *   name - name of the spectrum.
    /// *   params - parameters vector of parameter names for the spectrum parameters.
    /// *   pdict - parameter dictionary in which the parameter properties
    /// can be looked up
    /// *   xlow   - Override for default X axis low limit.
    /// *   xhigh  - Override for default X axis high limit.
    /// *   xbins  - Override for default X axis binning.
    /// *   ylow   - Override for default Y axis low limit.
    /// *   yhigh  - Override for default Y axis high limit.
    /// *   ybins  - Override for default Y axis binning.
    ///
    pub fn new(
        name: &str,
        params: Vec<String>,
        pdict: &ParameterDictionary,
        xlow: Option<f64>,
        xhigh: Option<f64>,
        xbins: Option<u32>,
        ylow: Option<f64>,
        yhigh: Option<f64>,
        ybins: Option<u32>,
    ) -> Result<Multi2d, String> {
        // maintain the defaults here:

        // Note we can set square defaults after the loop over the
        // parameters:

        let mut x_low = None;
        let mut x_high = None;
        let mut x_bins = None;

        let mut pnames = Vec::<String>::new();
        let mut pids = Vec::<u32>::new();

        for pname in params {
            if let Some(p) = pdict.lookup(&pname) {
                let lim = p.get_limits();
                x_low = optmin(x_low, lim.0);
                x_high = optmax(x_high, lim.1);
                x_bins = optmax(x_bins, p.get_bins());

                pnames.push(p.get_name());
                pids.push(p.get_id());
            } else {
                return Err(format!("Parameter {} cannot be found", pname));
            }
        }
        // Make the defaults square and override as desired:

        let mut y_low = x_low;
        let mut y_high = x_high;
        let mut y_bins = x_bins;

        // Override the defaults and make surea all axis definitions
        // have, in fact, been made:

        if let Some(xl) = xlow {
            x_low = Some(xl);
        }
        if let Some(xh) = xhigh {
            x_high = Some(xh);
        }
        if let Some(xb) = xbins {
            x_bins = Some(xb);
        }

        if let Some(yl) = ylow {
            y_low = Some(yl);
        }
        if let Some(yh) = yhigh {
            y_high = Some(yh);
        }
        if let Some(yb) = ybins {
            y_bins = Some(yb);
        }

        if x_low.is_none() {
            return Err(String::from("X axis low limit cannot be defaulted"));
        }
        if x_high.is_none() {
            return Err(String::from("X axis high limit cannot be defaulted"));
        }
        if x_bins.is_none() {
            return Err(String::from("X axis binning cannot be defaulted"));
        }
        if y_low.is_none() {
            return Err(String::from("Y axis low limit cannot be defaulted"));
        }
        if y_high.is_none() {
            return Err(String::from("Y axis high limit cannot be defaulted"));
        }
        if y_bins.is_none() {
            return Err(String::from("Y axis binning cannot be defaulted"));
        }
        Ok(Multi2d {
            applied_gate: SpectrumGate::new(),
            name: String::from(name),
            histogram: ndhistogram!(
                axis::Uniform::new(
                    x_bins.unwrap() as usize, x_low.unwrap(), x_high.unwrap()
                ),
                axis::Uniform::new(
                    y_bins.unwrap() as usize, y_low.unwrap(), y_high.unwrap()
                );
                Sum
            ),
            param_names: pnames,
            param_ids: pids,
        })
    }
}
#[cfg(test)]
mod multi2d_tests {
    use super::*;
    use ndhistogram::axis::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn new_1() {
        let mut pdict = ParameterDictionary::new();
        let mut pnames = Vec::<String>::new();
        for i in 0..10 {
            let pname = format!("param.{}", i);
            pdict.add(&pname).unwrap();
            let p = pdict.lookup_mut(&pname).unwrap();

            p.set_limits(0.0, 1024.0);
            p.set_bins(1024);
            pnames.push(pname);
        }
        let result = Multi2d::new("test", pnames, &pdict, None, None, None, None, None, None);
        assert!(result.is_ok());
        let spec = result.unwrap();

        assert!(spec.applied_gate.gate.is_none());
        assert_eq!(String::from("test"), spec.name);

        assert_eq!(2, spec.histogram.axes().num_dim());
        let x = spec.histogram.axes().as_tuple().0.clone();
        let y = spec.histogram.axes().as_tuple().1.clone();
        assert_eq!(0.0, *x.low());
        assert_eq!(1024.0, *x.high());
        assert_eq!(1024 + 2, x.num_bins());

        assert_eq!(0.0, *y.low());
        assert_eq!(1024.0, *y.high());
        assert_eq!(1024 + 2, y.num_bins());

        assert_eq!(10, spec.param_names.len());
        assert_eq!(10, spec.param_ids.len());

        for (i, name) in spec.param_names.iter().enumerate() {
            let sbname = format!("param.{}", i);
            assert_eq!(sbname, *name);
            let p = pdict.lookup(name).unwrap();
            assert_eq!(p.get_id(), spec.param_ids[i]);
        }
    }
    #[test]
    fn new_2() {
        // Override X axis definitions:

        let mut pdict = ParameterDictionary::new();
        let mut pnames = Vec::<String>::new();
        for i in 0..10 {
            let pname = format!("param.{}", i);
            pdict.add(&pname).unwrap();
            let p = pdict.lookup_mut(&pname).unwrap();

            p.set_limits(0.0, 1024.0);
            p.set_bins(1024);
            pnames.push(pname);
        }
        let result = Multi2d::new(
            "test",
            pnames,
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(2048),
            None,
            None,
            None,
        );
        assert!(result.is_ok());
        let spec = result.unwrap();

        assert!(spec.applied_gate.gate.is_none());
        assert_eq!(String::from("test"), spec.name);

        assert_eq!(2, spec.histogram.axes().num_dim());
        let x = spec.histogram.axes().as_tuple().0.clone();
        let y = spec.histogram.axes().as_tuple().1.clone();
        assert_eq!(-512.0, *x.low());
        assert_eq!(512.0, *x.high());
        assert_eq!(2048 + 2, x.num_bins());

        assert_eq!(0.0, *y.low());
        assert_eq!(1024.0, *y.high());
        assert_eq!(1024 + 2, y.num_bins());

        assert_eq!(10, spec.param_names.len());
        assert_eq!(10, spec.param_ids.len());

        for (i, name) in spec.param_names.iter().enumerate() {
            let sbname = format!("param.{}", i);
            assert_eq!(sbname, *name);
            let p = pdict.lookup(name).unwrap();
            assert_eq!(p.get_id(), spec.param_ids[i]);
        }
    }
    #[test]
    fn new_3() {
        // Override y axis defs:

        let mut pdict = ParameterDictionary::new();
        let mut pnames = Vec::<String>::new();
        for i in 0..10 {
            let pname = format!("param.{}", i);
            pdict.add(&pname).unwrap();
            let p = pdict.lookup_mut(&pname).unwrap();

            p.set_limits(0.0, 1024.0);
            p.set_bins(1024);
            pnames.push(pname);
        }
        let result = Multi2d::new(
            "test",
            pnames,
            &pdict,
            None,
            None,
            None,
            Some(-512.0),
            Some(512.0),
            Some(2048),
        );
        assert!(result.is_ok());
        let spec = result.unwrap();

        assert!(spec.applied_gate.gate.is_none());
        assert_eq!(String::from("test"), spec.name);

        assert_eq!(2, spec.histogram.axes().num_dim());
        let x = spec.histogram.axes().as_tuple().0.clone();
        let y = spec.histogram.axes().as_tuple().1.clone();
        assert_eq!(0.0, *x.low());
        assert_eq!(1024.0, *x.high());
        assert_eq!(1024 + 2, x.num_bins());

        assert_eq!(-512.0, *y.low());
        assert_eq!(512.0, *y.high());
        assert_eq!(2048 + 2, y.num_bins());

        assert_eq!(10, spec.param_names.len());
        assert_eq!(10, spec.param_ids.len());

        for (i, name) in spec.param_names.iter().enumerate() {
            let sbname = format!("param.{}", i);
            assert_eq!(sbname, *name);
            let p = pdict.lookup(name).unwrap();
            assert_eq!(p.get_id(), spec.param_ids[i]);
        }
    }
    // These new tests test various failure cases.

    #[test]
    fn new_4() {
        // A nonexistent parameter is in the parameter array:

        let mut pdict = ParameterDictionary::new();
        let mut pnames = Vec::<String>::new();
        for i in 0..10 {
            let pname = format!("param.{}", i);
            pdict.add(&pname).unwrap();
            let p = pdict.lookup_mut(&pname).unwrap();

            p.set_limits(0.0, 1024.0);
            p.set_bins(1024);
            pnames.push(pname);
        }
        pnames.push(String::from("param.11"));
        let result = Multi2d::new("test", pnames, &pdict, None, None, None, None, None, None);
        assert!(result.is_err());
    }
    #[test]
    fn new_5() {
        // Can't default various bits of axis definitions:
        // Remember parameters supply both x/y defaults:

        let mut pdict = ParameterDictionary::new();
        let mut pnames = Vec::<String>::new();
        for i in 0..10 {
            let pname = format!("param.{}", i);
            pdict.add(&pname).unwrap();
            pnames.push(pname);
        }

        let result = Multi2d::new(
            "test",
            pnames.clone(),
            &pdict,
            None,
            Some(1024.0),
            Some(1024),
            Some(0.0),
            Some(1024.0),
            Some(1024),
        );
        assert!(result.is_err());
        let result = Multi2d::new(
            "test",
            pnames,
            &pdict,
            Some(0.0),
            Some(1024.0),
            Some(1024),
            None,
            Some(1024.0),
            Some(1024),
        );
        assert!(result.is_err());
    }
    #[test]
    fn new_6() {
        let mut pdict = ParameterDictionary::new();
        let mut pnames = Vec::<String>::new();
        for i in 0..10 {
            let pname = format!("param.{}", i);
            pdict.add(&pname).unwrap();
            pnames.push(pname);
        }

        let result = Multi2d::new(
            "test",
            pnames.clone(),
            &pdict,
            Some(0.0),
            None,
            Some(1024),
            Some(0.0),
            Some(1024.0),
            Some(1024),
        );
        assert!(result.is_err());
        let result = Multi2d::new(
            "test",
            pnames,
            &pdict,
            Some(0.0),
            Some(1024.0),
            Some(1024),
            Some(0.0),
            None,
            Some(1024),
        );
        assert!(result.is_err());
    }
    #[test]
    fn new_7() {
        let mut pdict = ParameterDictionary::new();
        let mut pnames = Vec::<String>::new();
        for i in 0..10 {
            let pname = format!("param.{}", i);
            pdict.add(&pname).unwrap();
            pnames.push(pname);
        }

        let result = Multi2d::new(
            "test",
            pnames.clone(),
            &pdict,
            Some(0.0),
            Some(1024.0),
            None,
            Some(0.0),
            Some(1024.0),
            Some(1024),
        );
        assert!(result.is_err());
        let result = Multi2d::new(
            "test",
            pnames,
            &pdict,
            Some(0.0),
            Some(1024.0),
            Some(1024),
            Some(0.0),
            Some(1024.0),
            None,
        );
        assert!(result.is_err());
    }
    // Next set of test ensure the spectrum is properly incremented.

    #[test]
    fn incr_1() {
        let mut pdict = ParameterDictionary::new();
        let mut pnames = Vec::<String>::new();
        for i in 0..10 {
            let pname = format!("param.{}", i);
            pdict.add(&pname).unwrap();
            let p = pdict.lookup_mut(&pname).unwrap();

            p.set_limits(0.0, 1024.0);
            p.set_bins(1024);
            pnames.push(pname);
        }
        let mut spec =
            Multi2d::new("test", pnames, &pdict, None, None, None, None, None, None).unwrap();

        let mut fe = FlatEvent::new();
        let mut e = Event::new();

        for (i, pid) in spec.param_ids.iter().enumerate() {
            e.push(EventParameter::new(*pid, i as f64 * 10.0));
        }
        fe.load_event(&e);

        // Without an applied gate:

        spec.handle_event(&fe);

        // Let's look at all parameter pairs should have an increment for them
        // We're going to do this the clumsy way since otherwise
        // it's a circular test to see if the iterator we use in increment
        // actually works:

        for i in 0..spec.param_ids.len() {
            for j in (i + 1)..spec.param_ids.len() {
                let px = spec.param_ids[i];
                let py = spec.param_ids[j];
                let x = fe[px as u32].unwrap();
                let y = fe[py as u32].unwrap();
                let v = spec.histogram.value(&(x, y));
                assert!(v.is_some());
                assert_eq!(1.0, v.unwrap().get());
            }
        }
    }
}
