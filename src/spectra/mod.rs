//! While histograms are maintained by the ndhistogram, Each spectrum
//! has a filler.  A filler can contain any object that implements the
//! ndhistogram::Fill trait - that is an object that can have its bins
//! filled.  Each filler may also have requirements on the number of
//! axes its object has e.g.
//!
//! *  1d  depends on a single parameter and, if that parameter is present
//!    and the event satisfies any applied condition (gate), the histogram is
//!    filled with that single parameter value.   The item filled must have
//!    only one axis.
//! *   2d depends on an x and y parameter.  If both parameters are present
//!     and any applied condition is satisifed, the item is filled.  The item
//!     must have two axes.
//! *   summary depends on many x parameters and its fillable item must have
//!     2 axes.  If its applied condition is satisifed, and any of the x parameters
//!     is present, for each X parameter i with value xi, The channel c[i,xi] is
//!     incremented.  If this is confusing, think of the resulting histogram as being
//!     a two dimensional histogram of vertical strips.  Each vertical strip is the
//!     1-d spectrum of one of the X parameters. Typical use ase is for a large
//!     detector array.  This summary spectrum allows one to easily see channels that
//!     are failed or, if the elements are gain matched, how well the gain matching
//!     is done aross the array.
//!  *  Multi-1d.  In SpecTcl, this was called a gamma 1d:  The histogram is a single
//!     axis histogram, any number of parameters are allowed.  If the applied condition
//!     is accepted for the event, the spectrum is incremented for each of the parameters
//!     present in the event.
//!  *  Multi-2d.  In SpecTcl, this was called a gamma 2d:  The histogram needs 2 axes
//!     and at least 2 parameters.  If the applied gate is satisfied, the spectrum
//!     is incremented for each pair of parameters present in the event.
//!  *  Twod-sum.  The histogram needs 2 axes and an arbitrary number of parameter pairs.
//!     If the spectrum's applied condition is satisfied, the spectrum is incremented
//!     Once for each pair of parameters that are both present in the event.  This makes the
//!     result look like the sum of a set of 2d speactra.
//!  *  Pgamma - The histogram requires 2 axes and an arbitrary number of x and y axis parameters.
//!     if the applied gate is satisfied, the spectrum is incremented multiply for each combination
//!     of x/y parameters present.  For example, consider a fully populated event and a Pgamma
//!     histogram with parameters 1,3 on the x axis and 5,7,8 on the y axis, the following
//!     parameter pairs will be used to increment the spectrum:
//!     (1,5), (1,7), (1,8), (3,5), (3,7), (3,8).
//!

use super::conditions::*;
use super::parameters::*;
use ndhistogram::value::Sum;
use ndhistogram::*;
use std::rc::Rc;

pub struct Gate {
    condition_name: String,
    gate: ContainerReference,
}
// None means the spectrum is ungated.
pub struct SpectrumGate {
    gate: Option<Gate>,
}
// This factors out the whole gate handling for all spectrum
// types.
impl SpectrumGate {
    pub fn new() -> SpectrumGate {
        SpectrumGate { gate: None }
    }
    /// Set a new gate:
    /// If the gate does not exist Err is returned.
    /// Otherwise self.gate is Some(name, downgraded gate container).
    /// Note that if the gate cannot be found, the prior
    /// value remains.
    ///
    pub fn set_gate(&mut self, name: &str, dict: &ConditionDictionary) -> Result<(), String> {
        if let Some(gate) = dict.get(name) {
            self.gate = Some(Gate {
                condition_name: String::from(name),
                gate: Rc::downgrade(gate),
            });
            Ok(())
        } else {
            Err(format!("No such gate {}", name))
        }
    }
    /// Remove the gate:
    pub fn ungate(&mut self) {
        self.gate = None
    }
    /// Evaluate the gate for an event  The following cases and results
    /// are considered
    /// *   self.gate.is_none() - the spectrum is ungated, true is returned.
    /// *   upgrading the gate to an RC gives None - the underlying gate
    ///     was deleted:
    ///     The gate has been deleted from the dict, we're now ungated
    ///     return true.
    /// *   Upgrading gave Some - evaluate the resulting gate.
    ///
    /// Note that if the underlying gate was deleted ungate:
    pub fn check(&mut self, e: &FlatEvent) -> bool {
        if let Some(g) = &self.gate {
            if let Some(g) = g.gate.upgrade() {
                g.borrow_mut().check(e)
            } else {
                self.ungate();
                true
            }
        } else {
            true
        }
    }
}

// In order to support dynamic dispatch, we need to define a Spectrum trait which combines the
// Capabilities of ndhistogram objects to supply the interfaces of Axes, Fill and Histogram;
// Along with the interfaces we need:
// Default implementation assume
//   - Spectra have a field 'applied_gate' which is Option<Gate>

trait Spectrum {
    // Method that handle incrementing/gating
    fn check_gate(&mut self, e: &FlatEvent) -> bool;
    fn increment(&mut self, e: &FlatEvent);

    fn handle_event(&mut self, e: &FlatEvent) {
        if self.check_gate(e) {
            self.increment(e);
        }
    }
    // Methods that handle gate application:

    fn gate(&mut self, name: &str, dict: &ConditionDictionary) -> Result<(), String>;
    fn ungate(&mut self);
}

// 1-d histogram:

pub struct Oned {
    applied_gate: SpectrumGate,
    name: String,
    histogram: Hist1D<axis::Uniform, Sum>,
    parameter_name: String,
    parameter_id: u32,
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
            let default_lims = param.get_limits();
            let low_lim = if low.is_some() {
                low.unwrap()
            } else {
                if let Some(l) = default_lims.0 {
                    l
                } else {
                    return Err(format!("No default low limit defined for {}", param_name));
                }
            };
            let high_lim = if high.is_some() {
                high.unwrap()
            } else {
                if let Some(h) = default_lims.1 {
                    h
                } else {
                    return Err(format!("No default high limit defined for {}", param_name));
                }
            };
            let bin_count = if bins.is_some() {
                bins.unwrap()
            } else {
                if let Some(b) = param.get_bins() {
                    b
                } else {
                    return Err(format!("No default bin count for {}", param_name));
                }
            };
            // make result as an ok:

            Ok(Oned {
                applied_gate: SpectrumGate::new(),
                name: String::from(spectrum_name),
                histogram: ndhistogram!(
                    axis::Uniform::new(bin_count as usize, low_lim, high_lim);
                    Sum
                ),
                parameter_name: String::from(param_name),
                parameter_id: param.get_id(),
            })
        } else {
            Err(format!("No such parameter: {}", param_name))
        }
    }
}

impl Spectrum for Oned {
    fn check_gate(&mut self, e: &FlatEvent) -> bool {
        self.applied_gate.check(e)
    }
    fn increment(&mut self, e: &FlatEvent) {
        if let Some(p) = e[self.parameter_id] {
            self.histogram.fill(&p);
        }
    }
    fn gate(&mut self, name: &str, dict: &ConditionDictionary) -> Result<(), String> {
        self.applied_gate.set_gate(name, dict)
    }
    fn ungate(&mut self) {
        self.applied_gate.ungate()
    }
}

#[cfg(test)]
mod gate_tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    #[test]
    fn spgate_new() {
        let g = SpectrumGate::new();
        assert!(g.gate.is_none());
    }
    #[test]
    fn spgate_set1() {
        // Error to set a gate that's not in the dictionary:

        let dict = ConditionDictionary::new();
        let mut g = SpectrumGate::new();
        let result = g.set_gate("no-such", &dict);
        assert!(result.is_err());
        assert_eq!(String::from("No such gate no-such"), result.unwrap_err());
    }
    #[test]
    fn spgate_set2() {
        // Can set a gate in the dict:

        let mut dict = ConditionDictionary::new();
        let mut g = SpectrumGate::new();

        // Put a true condition in the dict:

        let test_gate = True {};
        dict.insert(String::from("true"), Rc::new(RefCell::new(test_gate)));

        let result = g.set_gate("true", &dict);
        assert!(result.is_ok());

        assert!(g.gate.is_some());
        assert_eq!(
            String::from("true"),
            g.gate.as_ref().unwrap().condition_name
        );
        assert!(g.gate.as_ref().unwrap().gate.upgrade().is_some());
    }
    #[test]
    fn spgate_ungate1() {
        // can ungate an ugate - still none:

        let mut g = SpectrumGate::new();
        g.ungate();
        assert!(g.gate.is_none());
    }
    #[test]
    fn spgate_ungate_2() {
        let mut dict = ConditionDictionary::new();
        let mut g = SpectrumGate::new();

        // Put a true condition in the dict:

        let test_gate = True {};
        dict.insert(String::from("true"), Rc::new(RefCell::new(test_gate)));

        let result = g.set_gate("true", &dict);
        assert!(result.is_ok());

        // now ungate:

        g.ungate();
        assert!(g.gate.is_none());
    }
    // Test for checking the gate
    // - Ungated is always true:
    // - Gated gives the result of the gate.
    //   *  True gate.
    //   *  False gate.
    // - Gated but the gate was deleted is always true...and ungates us.
    //
    #[test]
    fn spgate_check1() {
        let mut g = SpectrumGate::new();
        let e = FlatEvent::new();
        assert!(g.check(&e));
    }
    #[test]
    fn spgate_check2() {
        let mut dict = ConditionDictionary::new();
        let mut g = SpectrumGate::new();

        // Put a true condition in the dict:

        let test_gate = True {};
        dict.insert(String::from("true"), Rc::new(RefCell::new(test_gate)));

        g.set_gate("true", &dict).expect("Couldn't find gate");

        let e = FlatEvent::new();
        assert!(g.check(&e));
    }
    #[test]
    fn spgate_check3() {
        let mut dict = ConditionDictionary::new();
        let mut g = SpectrumGate::new();

        // Put a true condition in the dict:

        let test_gate = False {};
        dict.insert(String::from("false"), Rc::new(RefCell::new(test_gate)));

        g.set_gate("false", &dict).expect("Couldn't find gate");

        let e = FlatEvent::new();
        assert!(!g.check(&e));
    }
    #[test]
    fn spgate_check4() {
        let mut dict = ConditionDictionary::new();
        let mut g = SpectrumGate::new();

        // Put a true condition in the dict:

        let test_gate = False {};
        dict.insert(String::from("false"), Rc::new(RefCell::new(test_gate)));

        g.set_gate("false", &dict).expect("Couldn't find gate");

        let e = FlatEvent::new();
        assert!(!g.check(&e));

        // Now kill off the gate from the dict:
        // The {} ensures the container is dropped.
        {
            dict.remove(&String::from("false"))
                .expect("Not found to remove");
        }
        assert!(g.check(&e));
        assert!(g.gate.is_none());
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

        assert_eq!(1, one.histogram.axes().num_dim());
        let x = one.histogram.axes().as_tuple().0.clone();
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

        assert_eq!(1, one.histogram.axes().num_dim());
        let x = one.histogram.axes().as_tuple().0.clone();
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

        assert_eq!(1, one.histogram.axes().num_dim());
        let x = one.histogram.axes().as_tuple().0.clone();
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

        assert_eq!(1, one.histogram.axes().num_dim());
        let x = one.histogram.axes().as_tuple().0.clone();
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
        h.histogram.value_at_index(b).unwrap().get()
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
        gdict.insert(String::from("true"), Rc::new(RefCell::new(True {})));
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
        gdict.insert(String::from("false"), Rc::new(RefCell::new(False {})));
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

    // Tests below check over/underflow values.
}
