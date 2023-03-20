use super::*;
use ndhistogram::value::Sum;

/// This is a simple 1-d histogram with f64 valued channels.
/// *   applied_gate - conditionalizes the increment of the histogram.
/// *   name is the spectrum name (under which it will be entered into
///     the spectrum dictionary).
/// *   histogram is the underlying ndhistogram that maintains the counts.
/// *   parameter_name is the name of the parameter used to increment the
///     spectrum and
/// *   parameter_id is its id in the flattened event.
///
pub struct Oned {
    applied_gate: SpectrumGate,
    name: String,
    histogram: H1DContainer,
    parameter_name: String,
    parameter_id: u32,
}
impl Spectrum for Oned {
    fn check_gate(&mut self, e: &FlatEvent) -> bool {
        self.applied_gate.check(e)
    }
    fn increment(&mut self, e: &FlatEvent) {
        if let Some(p) = e[self.parameter_id] {
            self.histogram.borrow_mut().fill(&p);
        }
    }
    fn required_parameter(&self) -> Option<u32> {
        Some(self.parameter_id)
    }
    fn get_name(&self) -> String {
        self.name.clone()
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

impl Oned {
    ///
    /// Create a new 1d spectrum.   The spectrum is initially ungated.
    /// the parameters of creation are:
    ///  *   spectrum name.
    ///  *   param_name - name of the parameter on the X axis.
    ///  *   pdict      - reference the parameter dictionary to use
    ///         for lookup.
    ///  *   low - axis low limit if overriding default
    ///  *   high - axis high limit....
    ///  *   bins  - bins on the axis.
    /// Return value is: Result<Oned, String>  Where on error
    /// the string is an error message that is human readable:
    ///
    pub fn new(
        spectrum_name: &str,
        param_name: &str,
        pdict: &ParameterDictionary,
        low: Option<f64>,
        high: Option<f64>,
        bins: Option<u32>,
    ) -> Result<Oned, String> {
        if let Some(param) = pdict.lookup(param_name) {
            let (low_lim, high_lim, bin_count) = axis_limits(param, low, high, bins)?;
            // make result as an ok:

            Ok(Oned {
                applied_gate: SpectrumGate::new(),
                name: String::from(spectrum_name),
                histogram: Rc::new(RefCell::new(ndhistogram!(
                    axis::Uniform::new(bin_count as usize, low_lim, high_lim);
                    Sum
                ))),
                parameter_name: String::from(param_name),
                parameter_id: param.get_id(),
            })
        } else {
            Err(format!("No such parameter: {}", param_name))
        }
    }
}
#[cfg(test)]
mod oned_tests {
    use super::*;
    use ndhistogram::axis::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn new_1() {
        // No such parameter so it's a failure.

        let dict = ParameterDictionary::new();
        let result = Oned::new("test", "test", &dict, None, None, None);
        assert!(result.is_err());
        assert_eq!(
            String::from("No such parameter: test"),
            result.err().unwrap()
        );
    }
    #[test]
    fn new_2() {
        // Parameter exists - use default specifications.

        let mut d = ParameterDictionary::new();
        d.add("test").unwrap();
        let id = {
            let p = d.lookup_mut("test").unwrap();
            p.set_limits(0.0, 1023.0);
            p.set_bins(1024);
            p.set_description("This is a test parameter");
            p.get_id()
        };

        let result = Oned::new("test_spec", "test", &d, None, None, None);
        assert!(result.is_ok());
        let one = result.unwrap();
        assert!(one.applied_gate.gate.is_none());
        assert_eq!(String::from("test_spec"), one.name);
        assert_eq!(String::from("test"), one.parameter_name);
        assert_eq!(id, one.parameter_id);

        // Spectrum axis specs:

        assert_eq!(1, one.histogram.borrow().axes().num_dim());
        let x = one.histogram.borrow().axes().as_tuple().0.clone();
        assert_eq!(0.0, *x.low());
        assert_eq!(1023.0, *x.high());
        assert_eq!(1026, x.num_bins());
    }
    #[test]
    fn new_3() {
        // Override low:

        let mut d = ParameterDictionary::new();
        d.add("test").unwrap();
        let id = {
            let p = d.lookup_mut("test").unwrap();
            p.set_limits(0.0, 1023.0);
            p.set_bins(1024);
            p.set_description("This is a test parameter");
            p.get_id()
        };

        let result = Oned::new("test_spec", "test", &d, Some(-1023.0), None, None);
        assert!(result.is_ok());
        let one = result.unwrap();
        assert!(one.applied_gate.gate.is_none());
        assert_eq!(String::from("test_spec"), one.name);
        assert_eq!(String::from("test"), one.parameter_name);
        assert_eq!(id, one.parameter_id);

        // Spectrum axis specs:

        assert_eq!(1, one.histogram.borrow().axes().num_dim());
        let x = one.histogram.borrow().axes().as_tuple().0.clone();
        assert_eq!(-1023.0, *x.low());
        assert_eq!(1023.0, *x.high());
        assert_eq!(1026, x.num_bins());
    }
    #[test]
    fn new_4() {
        // override high

        let mut d = ParameterDictionary::new();
        d.add("test").unwrap();
        let id = {
            let p = d.lookup_mut("test").unwrap();
            p.set_limits(0.0, 1023.0);
            p.set_bins(1024);
            p.set_description("This is a test parameter");
            p.get_id()
        };

        let result = Oned::new("test_spec", "test", &d, Some(-1023.0), Some(0.0), None);
        assert!(result.is_ok());
        let one = result.unwrap();
        assert!(one.applied_gate.gate.is_none());
        assert_eq!(String::from("test_spec"), one.name);
        assert_eq!(String::from("test"), one.parameter_name);
        assert_eq!(id, one.parameter_id);

        // Spectrum axis specs:

        assert_eq!(1, one.histogram.borrow().axes().num_dim());
        let x = one.histogram.borrow().axes().as_tuple().0.clone();
        assert_eq!(-1023.0, *x.low());
        assert_eq!(0.0, *x.high());
        assert_eq!(1026, x.num_bins());
    }
    #[test]
    fn new_5() {
        // override bins

        let mut d = ParameterDictionary::new();
        d.add("test").unwrap();
        let id = {
            let p = d.lookup_mut("test").unwrap();
            p.set_limits(0.0, 1023.0);
            p.set_bins(1024);
            p.set_description("This is a test parameter");
            p.get_id()
        };

        let result = Oned::new("test_spec", "test", &d, Some(-1023.0), Some(0.0), Some(512));
        assert!(result.is_ok());
        let one = result.unwrap();
        assert!(one.applied_gate.gate.is_none());
        assert_eq!(String::from("test_spec"), one.name);
        assert_eq!(String::from("test"), one.parameter_name);
        assert_eq!(id, one.parameter_id);

        // Spectrum axis specs:

        assert_eq!(1, one.histogram.borrow().axes().num_dim());
        let x = one.histogram.borrow().axes().as_tuple().0.clone();
        assert_eq!(-1023.0, *x.low());
        assert_eq!(0.0, *x.high());
        assert_eq!(514, x.num_bins());
    }
    // Fail to create because we try to default charcterisics
    // that don't default:

    #[test]
    fn new_6() {
        let mut d = ParameterDictionary::new();
        d.add("test").unwrap();

        let result = Oned::new("test_spec", "test", &d, Some(-1023.0), Some(0.0), None);
        assert!(result.is_err());
        assert_eq!(
            String::from("No default bin count for test"),
            result.err().unwrap()
        );
    }
    #[test]
    fn new_7() {
        let mut d = ParameterDictionary::new();
        d.add("test").unwrap();

        let result = Oned::new("test_spec", "test", &d, Some(-1023.0), None, Some(512));
        assert!(result.is_err());
        assert_eq!(
            String::from("No default high limit defined for test"),
            result.err().unwrap()
        );
    }
    #[test]
    fn new_8() {
        let mut d = ParameterDictionary::new();
        d.add("test").unwrap();

        let result = Oned::new("test_spec", "test", &d, None, Some(0.0), Some(512));
        assert!(result.is_err());
        assert_eq!(
            String::from("No default low limit defined for test"),
            result.err().unwrap()
        );
    }
    // There are many tests we need to see if a
    // Spectrum is incremented properly as well.
    // Under the assumption that a bin number is valid this
    // should get the value at the bin of the 1-d Note that
    // bin 0 and n are under/overflow counts:

    fn bin_value(b: usize, h: &Oned) -> f64 {
        h.histogram.borrow().value_at_index(b).unwrap().get()
    }
    fn make_1d() -> Oned {
        // Create a one d histogram we'll use in our tests:
        // 1k channels from [0-1023)

        let mut d = ParameterDictionary::new();
        d.add("test").unwrap();

        Oned::new("test_spec", "test", &d, Some(0.0), Some(1023.0), Some(1024)).unwrap()
    }
    // all of our tests below will increment dead center.
    // Only gates applied will affect the increment.

    #[test]
    fn incr_1() {
        // ungated:

        let mut s = make_1d();
        let pid = s.parameter_id; // so we know how to fill in flat event:

        let mut fe = FlatEvent::new();
        let mut e = Event::new();
        e.push(EventParameter::new(pid, 511.0));
        fe.load_event(&e);

        s.handle_event(&fe);
        let v = bin_value(512, &s);
        assert_eq!(1.0, v);
    }
    #[test]
    fn incr_2() {
        // Spectrum with a true gate:

        let mut gdict = ConditionDictionary::new();
        gdict.insert(String::from("true"), Rc::new(RefCell::new(Box::new(True {}))));
        let mut s = make_1d();
        let pid = s.parameter_id;
        s.gate("true", &gdict).unwrap();

        //Now make the event -- should increment with a True gate:

        let mut fe = FlatEvent::new();
        let mut e = Event::new();
        e.push(EventParameter::new(pid, 511.0));
        fe.load_event(&e);

        s.handle_event(&fe);
        let v = bin_value(512, &s);
        assert_eq!(1.0, v);
    }
    #[test]
    fn incr_3() {
        // Spectrum with false gate won't increment:

        let mut gdict = ConditionDictionary::new();
        gdict.insert(String::from("false"), Rc::new(RefCell::new(Box::new(False {}))));
        let mut s = make_1d();
        let pid = s.parameter_id;
        s.gate("false", &gdict).unwrap();

        //Now make the event -- should increment with a True gate:

        let mut fe = FlatEvent::new();
        let mut e = Event::new();
        e.push(EventParameter::new(pid, 511.0));
        fe.load_event(&e);

        s.handle_event(&fe);
        let v = bin_value(512, &s);
        assert_eq!(0.0, v);
    }

    // The tests below ensure that events without our parameter
    // won't increment

    #[test]
    fn incr_4() {
        // Event without our parameter won't increment:

        let mut s = make_1d();
        let pid = s.parameter_id; // so we know how to fill in flat event:

        let mut fe = FlatEvent::new();
        let mut e = Event::new();
        e.push(EventParameter::new(pid + 1, 511.0)); // not ours...
        fe.load_event(&e);

        s.handle_event(&fe);

        // no bins set anywhere:

        for i in s.histogram.borrow().iter() {
            assert_eq!(0.0, i.value.get());
        }
    }

    // Tests below check over/underflow values.

    #[test]
    fn incr_5() {
        // overflow value:

        let mut s = make_1d();
        let pid = s.parameter_id; // so we know how to fill in flat event:

        let mut fe = FlatEvent::new();
        let mut e = Event::new();
        e.push(EventParameter::new(pid, 1023.0)); // just overflows I think:
        fe.load_event(&e);

        s.handle_event(&fe);

        assert_eq!(1.0, bin_value(1025, &s));
    }
    #[test]
    fn incr_6() {
        // underflow value:

        let mut s = make_1d();
        let pid = s.parameter_id; // so we know how to fill in flat event:

        let mut fe = FlatEvent::new();
        let mut e = Event::new();
        e.push(EventParameter::new(pid, -0.001)); // just underflows I think.
        fe.load_event(&e);

        s.handle_event(&fe);

        assert_eq!(1.0, bin_value(0, &s));
    }
    // Lastly make sure that fills sum:
    #[test]
    fn incr_7() {
        // underflow value:

        let mut s = make_1d();
        let pid = s.parameter_id; // so we know how to fill in flat event:

        let mut fe = FlatEvent::new();
        let mut e = Event::new();
        e.push(EventParameter::new(pid, 511.0)); //Back to middle.
        fe.load_event(&e);

        for _ in 0..100 {
            s.handle_event(&fe); // 100 counts in middle bin:
        }
        assert_eq!(100.0, bin_value(512, &s));
    }
    #[test]
    fn clear_1() {
        let mut s = make_1d();
        let pid = s.parameter_id; // so we know how to fill in flat event:

        let mut fe = FlatEvent::new();
        let mut e = Event::new();
        e.push(EventParameter::new(pid, 511.0)); //Back to middle.
        fe.load_event(&e);

        for _ in 0..100 {
            s.handle_event(&fe); // 100 counts in middle bin:
        }
        assert_eq!(100.0, bin_value(512, &s));

        s.clear();
        assert_eq!(0.0, bin_value(512, &s));
    }
}
