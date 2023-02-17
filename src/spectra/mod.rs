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

///
/// Gated spectra have this.  The condition_name just documents
/// which condition is applied to the spectrum.
/// The gate is the weakened Rc::RefCell that 'points' to the gate.
///
pub struct Gate {
    condition_name: String,
    gate: ContainerReference,
}
///  Unlike SpecTcl which just makes an ungated Spectrum
/// have a 'special' True gate, we'll put one of these into the
/// spectrum and a None value for the gate field means the spetrum is
/// ungated.
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

/// In order to support dynamic dispatch, we need to define a Spectrum trait which combines the
/// Capabilities of ndhistogram objects to supply the interfaces of Axes, Fill and Histogram;
/// Along with the interfaces we need:
/// Normally clients of spectra use:
///
/// *     handle_event to process an event.  This will
///       check any applied gate before attempting to call increment
/// *     gate to gate a spectrum on a condition or replace the gate.
/// *     ungate to remove the gate condition of a spectrum, if any.
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

// Utility function to figure out the axis limits given
// a parameter definition for the axis and options for each
// of the values
// This factors out the code to determine axis limits from the
// individual spectrum new methods.
//
fn axis_limits(
    pdef: &Parameter,
    low: Option<f64>,
    high: Option<f64>,
    bins: Option<u32>,
) -> Result<(f64, f64, u32), String> {
    let default_lims = pdef.get_limits();
    let param_name = pdef.get_name();
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
        if let Some(b) = pdef.get_bins() {
            b
        } else {
            return Err(format!("No default bin count for {}", param_name));
        }
    };
    Ok((low_lim, high_lim, bin_count))
}

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
            let (low_lim, high_lim, bin_count) = axis_limits(param, low, high, bins)?;
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

/// Twod is a simple two dimensional spectrum.
/// It has two parameters x and y and a SpectrumGate
/// The underlying histogram is a Hist2D<axis::Uniform, axis::Uniform, Sum>
/// which makes the channels have f64 values.
/// Member data:
///
/// *    applied_gate - The gate which can conditionalize incrementing the
/// spectrum.
/// *    name - Name of the spectrum.  The spectrum is intered in the
/// spectrum dictionary under this name.
/// *    histogram -The underlying histogram.
/// *    x_name, x_id - the name and Id of the X axis parameter.
/// *    y_name, y_id - the name and Id of the Y axis parameter.
///
struct Twod {
    applied_gate: SpectrumGate,
    name: String,
    histogram: Hist2D<axis::Uniform, axis::Uniform, Sum>,

    // Parameter information:
    x_name: String,
    x_id: u32,
    y_name: String,
    y_id: u32,
}
impl Spectrum for Twod {
    fn check_gate(&mut self, e: &FlatEvent) -> bool {
        self.applied_gate.check(e)
    }
    fn increment(&mut self, e: &FlatEvent) {
        let x = e[self.x_id];
        let y = e[self.y_id];

        // We need both parameters in the event:

        if x.is_some() && y.is_some() {
            self.histogram.fill(&(x.unwrap(), y.unwrap()));
        }
    }
    fn gate(&mut self, name: &str, dict: &ConditionDictionary) -> Result<(), String> {
        self.applied_gate.set_gate(name, dict)
    }
    fn ungate(&mut self) {
        self.applied_gate.ungate()
    }
}
impl Twod {
    pub fn new(
        spectrum_name: &str,
        xname: &str,
        yname: &str,
        pdict: &ParameterDictionary,
        xlow: Option<f64>,
        xhigh: Option<f64>,
        xbins: Option<u32>,
        ylow: Option<f64>,
        yhigh: Option<f64>,
        ybins: Option<u32>,
    ) -> Result<Twod, String> {
        let xpar = pdict.lookup(xname);
        let ypar = pdict.lookup(yname);

        if xpar.is_some() && ypar.is_some() {
            let xpar = xpar.unwrap(); // Get the definitions:
            let ypar = ypar.unwrap();

            let xaxis_info = axis_limits(&xpar, xlow, xhigh, xbins)?;
            let yaxis_info = axis_limits(&ypar, ylow, yhigh, ybins)?;

            Ok(Twod {
                applied_gate: SpectrumGate::new(),
                name: String::from(spectrum_name),
                histogram: ndhistogram!(
                    axis::Uniform::new(xaxis_info.2 as usize, xaxis_info.0, xaxis_info.1),
                    axis::Uniform::new(yaxis_info.2 as usize, yaxis_info.0, yaxis_info.1)
                    ; Sum
                ),
                x_name: String::from(xname),
                x_id: xpar.get_id(),
                y_name: String::from(yname),
                y_id: ypar.get_id(),
            })
        } else {
            Err(format!(
                "One of the parameters {}, {} are not defined",
                xname, yname
            ))
        }
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

        for i in s.histogram.iter() {
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
}

#[cfg(test)]
mod twod_tests {
    use super::*;
    use ndhistogram::axis::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn new_1() {
        // Everything is ok:

        // Make x/y parameters

        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();
        let xinfo = {
            let px = pdict.lookup_mut("x").unwrap();
            px.set_limits(0.0, 1023.0);
            px.set_bins(512);
            px.set_description("Just some x parameter");
            (px.get_id(), px.get_name(), px.get_limits(), px.get_bins())
        };
        let yinfo = {
            let py = pdict.lookup_mut("y").unwrap();
            py.set_limits(-1.0, 1.0);
            py.set_bins(100);
            py.set_description("Just some y parameter");
            (py.get_id(), py.get_name(), py.get_limits(), py.get_bins())
        };

        let result = Twod::new(
            "2d", "x", "y", &pdict, None, None, None, // Default xaxis.
            None, None, None, // default y axis.
        );
        assert!(result.is_ok());
        let spec = result.unwrap();
        assert!(spec.applied_gate.gate.is_none());
        assert_eq!(String::from("2d"), spec.name);
        assert_eq!(xinfo.1, spec.x_name);
        assert_eq!(xinfo.0, spec.x_id);
        assert_eq!(yinfo.1, spec.y_name);
        assert_eq!(yinfo.0, spec.y_id);

        // make sure we made a 2d histogram with the correct axes:

        assert_eq!(2, spec.histogram.axes().num_dim());
        let xaxis = spec.histogram.axes().as_tuple().0.clone();
        let yaxis = spec.histogram.axes().as_tuple().1.clone();

        assert_eq!(xinfo.2 .0.unwrap(), *xaxis.low());
        assert_eq!(xinfo.2 .1.unwrap(), *xaxis.high());
        assert_eq!(xinfo.3.unwrap() as usize + 2, xaxis.num_bins()); // 512 + under/overflow.
        assert_eq!(yinfo.2 .0.unwrap(), *yaxis.low());
        assert_eq!(yinfo.2 .1.unwrap(), *yaxis.high());
        assert_eq!(yinfo.3.unwrap() as usize + 2, yaxis.num_bins()); // 100 + under/overflow bins.
    }
    #[test]
    fn new_2() {
        // Can override x axis definitions:

        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();
        let xinfo = {
            let px = pdict.lookup_mut("x").unwrap();
            px.set_limits(0.0, 1023.0);
            px.set_bins(512);
            px.set_description("Just some x parameter");
            (px.get_id(), px.get_name(), px.get_limits(), px.get_bins())
        };
        let yinfo = {
            let py = pdict.lookup_mut("y").unwrap();
            py.set_limits(-1.0, 1.0);
            py.set_bins(100);
            py.set_description("Just some y parameter");
            (py.get_id(), py.get_name(), py.get_limits(), py.get_bins())
        };

        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            None,
            None,
            None, // Accept y axis defaults.
        );
        assert!(result.is_ok());
        let spec = result.unwrap();

        assert!(spec.applied_gate.gate.is_none());
        assert_eq!(String::from("2d"), spec.name);
        assert_eq!(xinfo.1, spec.x_name);
        assert_eq!(xinfo.0, spec.x_id);
        assert_eq!(yinfo.1, spec.y_name);
        assert_eq!(yinfo.0, spec.y_id);

        // make sure we made a 2d histogram with the correct axes:

        assert_eq!(2, spec.histogram.axes().num_dim());
        let xaxis = spec.histogram.axes().as_tuple().0.clone();
        let yaxis = spec.histogram.axes().as_tuple().1.clone();

        assert_eq!(-512.0, *xaxis.low());
        assert_eq!(512.0, *xaxis.high());
        assert_eq!(256 + 2, xaxis.num_bins()); // 512 + under/overflow.
        assert_eq!(yinfo.2 .0.unwrap(), *yaxis.low());
        assert_eq!(yinfo.2 .1.unwrap(), *yaxis.high());
        assert_eq!(yinfo.3.unwrap() as usize + 2, yaxis.num_bins()); // 100 + under/overflow bins.
    }
    #[test]
    fn new_3() {
        // Can override y axis definitions:

        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();
        let xinfo = {
            let px = pdict.lookup_mut("x").unwrap();
            px.set_limits(0.0, 1023.0);
            px.set_bins(512);
            px.set_description("Just some x parameter");
            (px.get_id(), px.get_name(), px.get_limits(), px.get_bins())
        };
        let yinfo = {
            let py = pdict.lookup_mut("y").unwrap();
            py.set_limits(-1.0, 1.0);
            py.set_bins(100);
            py.set_description("Just some y parameter");
            (py.get_id(), py.get_name(), py.get_limits(), py.get_bins())
        };

        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            Some(-2.0),
            Some(2.0),
            Some(200), // Override Y axis defaults.
        );
        assert!(result.is_ok());
        let spec = result.unwrap();

        assert!(spec.applied_gate.gate.is_none());
        assert_eq!(String::from("2d"), spec.name);
        assert_eq!(xinfo.1, spec.x_name);
        assert_eq!(xinfo.0, spec.x_id);
        assert_eq!(yinfo.1, spec.y_name);
        assert_eq!(yinfo.0, spec.y_id);

        // make sure we made a 2d histogram with the correct axes:

        assert_eq!(2, spec.histogram.axes().num_dim());
        let xaxis = spec.histogram.axes().as_tuple().0.clone();
        let yaxis = spec.histogram.axes().as_tuple().1.clone();

        assert_eq!(-512.0, *xaxis.low());
        assert_eq!(512.0, *xaxis.high());
        assert_eq!(256 + 2, xaxis.num_bins()); // 512 + under/overflow.
        assert_eq!(-2.0, *yaxis.low());
        assert_eq!(2.0, *yaxis.high());
        assert_eq!(200 as usize + 2, yaxis.num_bins()); // 100 + under/overflow bins.
    }
    #[test]
    fn new_4() {
        // Must provide x axis information (no defaults) but don't failure:

        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();

        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            None,
            Some(1024.0),
            Some(512),
            Some(-1.0),
            Some(-1.0),
            Some(100),
        );
        assert!(result.is_err());

        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(0.0),
            None,
            Some(512),
            Some(-1.0),
            Some(-1.0),
            Some(100),
        );
        assert!(result.is_err());
        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(0.0),
            Some(1024.0),
            None,
            Some(-1.0),
            Some(-1.0),
            Some(100),
        );
        assert!(result.is_err());
    }
    #[test]
    fn new_5() {
        // Can't default y axis but attempts to:

        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();
        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            None,
            Some(2.0),
            Some(200), // Override Y axis defaults.
        );
        assert!(result.is_err());
        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            Some(-2.0),
            None,
            Some(200), // Override Y axis defaults.
        );
        assert!(result.is_err());
        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            Some(-2.0),
            Some(2.0),
            None, // Override Y axis defaults.
        );
        assert!(result.is_err());

        // Fully specified _is_ ok though:

        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            Some(-2.0),
            Some(2.0),
            Some(200), // Override Y axis defaults.
        );
        assert!(result.is_ok());
    }
    #[test]
    fn new_6() {
        // No such x parameter:

        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();
        let result = Twod::new(
            "2d",
            "xx",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            Some(-2.0),
            Some(2.0),
            Some(200), // Override Y axis defaults.
        );
        assert!(result.is_err());
    }
    #[test]
    fn new_7() {
        // NO such y parameter:

        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();
        let result = Twod::new(
            "2d",
            "x",
            "yy",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            Some(-2.0),
            Some(2.0),
            Some(200), // Override Y axis defaults.
        );
        assert!(result.is_err());
    }
    // The remaining tests test increments.
    // To support them utility function below creates and returns
    // a standard 2d spectrum.  Note that the fact that
    // the parameter dict goes out of scope is unimportant as
    // id's can still be used to pull data from FlatEvent references:

    fn make_test_2d() -> Twod {
        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();
        Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            Some(-2.0),
            Some(2.0),
            Some(200), // Override Y axis defaults.
        )
        .unwrap()
    }
    #[test]
    fn incr_1() {
        // Increment dead center - ungated.

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 0.0),
            EventParameter::new(spec.y_id, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        spec.handle_event(&e);

        let v = spec.histogram.value(&(0.0, 0.0));
        assert!(v.is_some());
        assert_eq!(1.0, v.unwrap().get());
    }
    #[test]
    fn incr_2() {
        // Increment dead center - Gated with T.

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 0.0),
            EventParameter::new(spec.y_id, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        let mut gd = ConditionDictionary::new();
        gd.insert(String::from("true"), Rc::new(RefCell::new(True {})));
        spec.gate("true", &gd).unwrap();

        spec.handle_event(&e);

        let v = spec.histogram.value(&(0.0, 0.0));
        assert!(v.is_some());
        assert_eq!(1.0, v.unwrap().get());
    }
    #[test]
    fn incr_3() {
        // Incr dead center with gate false -- that'll not increment:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 0.0),
            EventParameter::new(spec.y_id, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        let mut gd = ConditionDictionary::new();
        gd.insert(String::from("false"), Rc::new(RefCell::new(False {})));
        spec.gate("false", &gd).unwrap();

        spec.handle_event(&e);

        let v = spec.histogram.value(&(0.0, 0.0));
        assert!(v.is_some());
        assert_eq!(0.0, v.unwrap().get());
    }
    #[test]
    fn incr_4() {
        // X parameter not in event:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id + 100, 0.0),
            EventParameter::new(spec.y_id, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        spec.handle_event(&e);

        for c in spec.histogram.iter() {
            assert_eq!(0.0, c.value.get());
        }
    }
    #[test]
    fn incr_5() {
        // Y parameter not present:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 0.0),
            EventParameter::new(spec.y_id + 100, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        spec.handle_event(&e);

        for c in spec.histogram.iter() {
            assert_eq!(0.0, c.value.get());
        }
    }
    #[test]
    fn incr_6() {
        // Underflow in x:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, -600.0),
            EventParameter::new(spec.y_id, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        spec.handle_event(&e);

        // Just try to get the undeflow x channel:

        let v = spec.histogram.value(&(-512.01, 0.0));
        assert!(v.is_some());
        assert_eq!(1.0, v.unwrap().get());
    }
    #[test]
    fn incr_7() {
        // overflow in x:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 600.0),
            EventParameter::new(spec.y_id, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        spec.handle_event(&e);

        // Just try to get the undeflow x channel:

        let v = spec.histogram.value(&(512.0, 0.0));
        assert!(v.is_some());
        assert_eq!(1.0, v.unwrap().get());
    }
    #[test]
    fn incr_8() {
        // underflow in y:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 0.0),
            EventParameter::new(spec.y_id, -3.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        spec.handle_event(&e);

        // Just try to get the undeflow x channel:

        let v = spec.histogram.value(&(0.0, -2.01));
        assert!(v.is_some());
        assert_eq!(1.0, v.unwrap().get());
    }
    #[test]
    fn incr_9() {
        // Overflow in y:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 0.0),
            EventParameter::new(spec.y_id, 2.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        spec.handle_event(&e);

        // Just try to get the undeflow x channel:

        let v = spec.histogram.value(&(0.0, 2.01));
        assert!(v.is_some());
        assert_eq!(1.0, v.unwrap().get());
    }
}
