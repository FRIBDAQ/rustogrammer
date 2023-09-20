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
use std::collections::HashSet;

///
/// *  applied_gate - is the gate that can conditionalize increments.
/// *  name         - is the name of the spectrum.
/// *  histogram    - is the actual histogram object.
/// *  param_names  - are the names of the parameters we're defined on.
/// *  param_ids    - Are the corresponding parameter ids (indices into FlatEvent).
///
pub struct Multi1d {
    applied_gate: SpectrumGate,
    applied_fold: SpectrumGate,
    name: String,
    histogram: H1DContainer,
    param_names: Vec<String>,
    param_ids: Vec<u32>,
    param_id_hash: HashSet<u32>,
}
//
impl Spectrum for Multi1d {
    fn check_gate(&mut self, e: &FlatEvent) -> bool {
        self.applied_gate.check(e)
    }
    fn increment(&mut self, e: &FlatEvent) {
        let ids = self.get_param_ids(e); // Raw or from fold.
        let mut histogram = self.histogram.borrow_mut();
        for id in ids {
            if let Some(x) = e[id] {
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
    // Implement support for setting folds:

    fn can_fold(&self) -> bool {
        true
    }
    fn fold(&mut self, name: &str, dict: &ConditionDictionary) -> Result<(), String> {
        // We need to lookup the gate and determine if it is a fold:

        if let Some(cond) = dict.get(name) {
            if cond.borrow().is_fold() {
                self.applied_fold.set_gate(name, dict)
            } else {
                Err(format!("{} cannot be used as a fold", name))
            }
        } else {
            Err(format!("There is no condition named {}", name))
        }
    }
    fn unfold(&mut self) -> Result<(), String> {
        self.applied_fold.ungate();
        Ok(())
    }
    fn get_fold(&self) -> Option<String> {
        if let Some(g) = self.applied_fold.gate.clone() {
            Some(g.condition_name)
        } else {
            None
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
    /// ###
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

        if xlow.is_none() {
            return Err(String::from("X axis low limit cannot be defaulted"));
        }
        if xmax.is_none() {
            return Err(String::from("X axis high limit cannot be defaulted"));
        }
        if xbins.is_none() {
            return Err(String::from("X axis binning cannot be defaulted"));
        }
        let hash = param_ids.clone().into_iter().collect::<HashSet<u32>>();
        Ok(Multi1d {
            applied_gate: SpectrumGate::new(),
            applied_fold: SpectrumGate::new(),
            name: String::from(name),
            histogram: Rc::new(RefCell::new(ndhistogram!(
                axis::Uniform::new(xbins.unwrap() as usize, xlow.unwrap(), xmax.unwrap());
                Sum
            ))),
            param_names,
            param_ids,
            param_id_hash:  hash
        })
    }
    /// Determine the ids to be used for incrementing the spectra.
    /// If there is no applied fold, this is just the raw ids.
    /// otherwise it's the intersection between the raw ids and the ids
    /// that come out of the fold:
    /// Returning a box avoids copying the param_ids vec.
    fn get_param_ids(&mut self, e: &FlatEvent) -> Vec<u32> {
        if self.applied_fold.is_fold() {
            // This branch can probably use some optimization
            let fold_ids = self.applied_fold.fold_1d(e);
            let fold_set = fold_ids.into_iter().collect::<HashSet<u32>>();
            let mut result = vec![];
            for i in fold_set.intersection(&self.param_id_hash) {
                result.push(*i);
            }
            result
        } else {
            self.param_ids.clone()
        }
    }
}
#[cfg(test)]
mod test_support {
    use super::*;
    pub fn make_default_parameters(pdict: &mut ParameterDictionary) -> Vec<String> {
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
        names
    }
}
#[cfg(test)]
mod multi1d_tests {
    use super::*;
    // use ndhistogram::axis::*;
    use super::test_support::make_default_parameters;
    use std::cell::RefCell; // Needed in gating
    use std::rc::Rc; // Needed in gating.

    #[test]
    fn new_1() {
        // Success with default x axis and uniform parameter defs:

        let mut pdict = ParameterDictionary::new();
        let names = make_default_parameters(&mut pdict);
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
        let names = make_default_parameters(&mut pdict);
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
        let names = make_default_parameters(&mut pdict);

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
        let mut names = make_default_parameters(&mut pdict);

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
        let names = make_default_parameters(&mut pdict);

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
        let names = make_default_parameters(&mut pdict);
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
        let names = make_default_parameters(&mut pdict);
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
        let names = make_default_parameters(&mut pdict);
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
// Test folds of Multi1d.

#[cfg(test)]
mod fold_tests {
    use super::test_support::make_default_parameters;
    use super::*;
    use crate::conditions::{cut, twod, ConditionDictionary};
    use crate::parameters::{EventParameter, FlatEvent};
    use std::cell::RefCell; // Needed in gating
    use std::rc::Rc; // Needed in gate/folds

    #[test]
    fn fold_1() {
        // Can fold with a multicut.

        let mut pdict = ParameterDictionary::new();
        let pnames = make_default_parameters(&mut pdict);
        let mut spec = Multi1d::new("Testing", pnames, &pdict, None, None, None).unwrap();

        // Make a Condition Dict and put a multicut into it.

        let mut gdict = ConditionDictionary::new();
        let mcut = cut::MultiCut::new(&vec![0, 1, 2, 3], 100.0, 200.0);
        gdict.insert(String::from("gs"), Rc::new(RefCell::new(Box::new(mcut))));

        spec.fold("gs", &gdict).expect("Appling fold to spectrum");
    }
    #[test]
    fn fold_2() {
        // Normal cut cannot be used as a fold:

        let mut pdict = ParameterDictionary::new();
        let pnames = make_default_parameters(&mut pdict);
        let mut spec = Multi1d::new("Testing", pnames, &pdict, None, None, None).unwrap();

        // Make a Condition Dict and put a multicut into it.

        let mut gdict = ConditionDictionary::new();
        let cut = cut::Cut::new(1, 100.0, 200.0);
        gdict.insert(String::from("cut"), Rc::new(RefCell::new(Box::new(cut))));
        assert!(spec.fold("cut", &gdict).is_err());
    }
    #[test]
    fn fold_3() {
        // Attempting to fold nonexistent condition also fails:

        let mut pdict = ParameterDictionary::new();
        let pnames = make_default_parameters(&mut pdict);
        let mut spec = Multi1d::new("Testing", pnames, &pdict, None, None, None).unwrap();

        let gdict = ConditionDictionary::new();

        assert!(spec.fold("nosuch", &gdict).is_err());
    }
    #[test]
    fn fold_4() {
        // a multicontour can be fold too:

        let mut pdict = ParameterDictionary::new();
        let pnames = make_default_parameters(&mut pdict);
        let mut spec = Multi1d::new("Testing", pnames, &pdict, None, None, None).unwrap();

        // Make a Condition Dict and put a multicut into it.

        let mut gdict = ConditionDictionary::new();
        let fold = twod::MultiContour::new(
            &vec![0, 1, 2, 3],
            vec![
                Point::new(2.0, 5.0),
                Point::new(5.0, 5.0),
                Point::new(10.0, 0.0),
            ],
        )
        .expect("Making multicontour");
        gdict.insert(String::from("gc"), Rc::new(RefCell::new(Box::new(fold))));
        assert!(spec.fold("gc", &gdict).is_ok());
    }
    #[test]
    fn unfold_1() {
        // Can unfold a folded spectrum:

        let mut pdict = ParameterDictionary::new();
        let pnames = make_default_parameters(&mut pdict);
        let mut spec = Multi1d::new("Testing", pnames, &pdict, None, None, None).unwrap();

        // Make a Condition Dict and put a multicut into it.

        let mut gdict = ConditionDictionary::new();
        let mcut = cut::MultiCut::new(&vec![0, 1, 2, 3], 100.0, 200.0);
        gdict.insert(String::from("gs"), Rc::new(RefCell::new(Box::new(mcut))));

        spec.fold("gs", &gdict).expect("Appling fold to spectrum");

        spec.unfold().expect("Unfolding");
        assert!(!spec.applied_fold.is_fold());
    }
    #[test]
    fn lsfold_1() {
        let mut pdict = ParameterDictionary::new();
        let pnames = make_default_parameters(&mut pdict);
        let mut spec = Multi1d::new("Testing", pnames, &pdict, None, None, None).unwrap();

        // Make a Condition Dict and put a multicut into it.

        let mut gdict = ConditionDictionary::new();
        let mcut = cut::MultiCut::new(&vec![0, 1, 2, 3], 100.0, 200.0);
        gdict.insert(String::from("gs"), Rc::new(RefCell::new(Box::new(mcut))));

        spec.fold("gs", &gdict).expect("Appling fold to spectrum");

        let fold = spec.get_fold();
        assert!(fold.is_some());
        assert_eq!("gs", fold.unwrap());
    }
    #[test]
    fn lsfold_2() {
        let mut pdict = ParameterDictionary::new();
        let pnames = make_default_parameters(&mut pdict);
        let spec = Multi1d::new("Testing", pnames, &pdict, None, None, None).unwrap();

        assert!(spec.get_fold().is_none());
    }
    #[test]
    fn pid_1() {
        // If not folded, parameter ids will be just the underlying set.

        let mut pdict = ParameterDictionary::new();
        let pnames = make_default_parameters(&mut pdict);
        let mut spec = Multi1d::new("Testing", pnames, &pdict, None, None, None).unwrap();

        let ev = FlatEvent::new();
        let ps = spec.get_param_ids(&ev);
        assert_eq!(spec.param_ids, ps);
    }
    #[test]
    fn pid_2() {
        // If folded but nothing is in the gate again, we get all parameters
        // in the fold back (due to the intersection).

        let mut pdict = ParameterDictionary::new();
        let pnames = make_default_parameters(&mut pdict);
        let mut spec = Multi1d::new("Testing", pnames, &pdict, None, None, None).unwrap();
        let mut gdict = ConditionDictionary::new();
        let mcut = cut::MultiCut::new(&vec![0, 1, 2, 3], 100.0, 200.0);
        gdict.insert(String::from("gs"), Rc::new(RefCell::new(Box::new(mcut))));

        spec.fold("gs", &gdict).expect("Appling fold to spectrum");

        let mut ev = FlatEvent::new();
        let event = vec![
            EventParameter::new(1, 5.0),
            EventParameter::new(2, 20.0),
            EventParameter::new(3, 202.0),
        ];
        ev.load_event(&event);

        let mut ps = spec.get_param_ids(&ev);
        ps.sort();
        assert_eq!(vec![1, 2, 3], ps);
    }
    #[test]
    fn pid_3() {
        // If there are parameters in the gate, they are omited from get_param_ids

        // If folded but nothing is in the gate again, we get all parameters
        // in the fold back (due to the intersection).

        let mut pdict = ParameterDictionary::new();
        let pnames = make_default_parameters(&mut pdict);
        let mut spec = Multi1d::new("Testing", pnames, &pdict, None, None, None).unwrap();
        let mut gdict = ConditionDictionary::new();
        let mcut = cut::MultiCut::new(&vec![0, 1, 2, 3], 100.0, 200.0);
        gdict.insert(String::from("gs"), Rc::new(RefCell::new(Box::new(mcut))));

        spec.fold("gs", &gdict).expect("Appling fold to spectrum");

        let mut ev = FlatEvent::new();
        let event = vec![
            EventParameter::new(1, 5.0),
            EventParameter::new(2, 150.0),
            EventParameter::new(3, 202.0),
        ];
        ev.load_event(&event);

        let mut ps = spec.get_param_ids(&ev);
        ps.sort();
        assert_eq!(vec![1, 3], ps);
    }
}
