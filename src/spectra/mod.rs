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
    histogram: Hist1D<axis::Uniform>,
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
                histogram: ndhistogram!(axis::Uniform::new(bin_count as usize, low_lim, high_lim)),
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
