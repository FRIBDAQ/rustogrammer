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
//!     result look like the sum of a set of 2d spectra.
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
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

// Re-exports

pub mod oned;
pub use oned::*;

pub mod twod;
pub use twod::*;

pub mod summary;
pub use summary::*;

pub mod multi1d;
pub use multi1d::*;

pub mod multi2d;
pub use multi2d::*;

pub mod twodsum;
pub use twodsum::*;

pub mod pgamma;
pub use pgamma::*;

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
    // informational methods:

    /// This should return a parameter id if there is a parameter
    /// id that is required to increment the spectrum.
    /// It is used by the SpectrumStorage app to put the spectrum
    /// in the correct list of spectra to increment.
    fn required_parameter(&self) -> Option<u32> {
        None
    }
    /// Return the spectrum name:
    ///
    fn get_name(&self) -> String;
    
    // Methods that handle gate application:

    fn gate(&mut self, name: &str, dict: &ConditionDictionary) -> Result<(), String>;
    fn ungate(&mut self);

    // manipulate the underlying histogram:

    /// Clear the histogram counts.:

    fn clear(&mut self);
}

// We also need some sort of repository in which spectra can be stored and looked up by name.
//  A hash map does nicely:

type SpectrumContainer = Rc<RefCell<dyn Spectrum>>;
type SpectrumContainerReference = Weak<RefCell<dyn Spectrum>>;
type SpectrumReferences = Vec<SpectrumContainerReference>;
type SpectrumDictionary = HashMap<String, SpectrumContainer>;

/// The SpectrumStorage type supports several things:
/// -   Spectrum storage by name through a contained SpectrumDictionary.
/// -   Rapid spectrum increment by holding a set of spectra that are
///     indexed by a required parameter as well as a set of spectra for which
///     We cannot make that statement.
/// -   Handling an event by incrementing appropriate spectra.  This is done by
///     First incrementing all spectra which have a required parameter in the
///     event then incrementing any spectra for which we can't say there's a
///     required parameter.
/// Note that the name dictionary retains a strong reference while the increment lists
/// retain weak references under the assumption that promition of a weak reference costs little.
/// and that spectra are not rapidly deleted/changed.

pub struct SpectrumStorage {
    dict: SpectrumDictionary,
    spectra_by_parameter: Vec<Option<SpectrumReferences>>,
    other_spectra: SpectrumReferences,
}

impl SpectrumStorage {
    pub fn new() -> SpectrumStorage {
        SpectrumStorage {
            dict: SpectrumDictionary::new(),
            spectra_by_parameter: Vec::<Option<SpectrumReferences>>::new(),
            other_spectra: SpectrumReferences::new(),
        }
    }
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

// Useful utility methods (private):

fn optmin<T: PartialOrd>(v1: Option<T>, v2: Option<T>) -> Option<T> {
    if v1.is_none() && v2.is_none() {
        None
    } else {
        if v1.is_none() || v2.is_none() {
            if let Some(v1) = v1 {
                Some(v1)
            } else {
                Some(v2.unwrap())
            }
        } else {
            // neither are none:

            let v1 = v1.unwrap();
            let v2 = v2.unwrap();
            if v1 < v2 {
                Some(v1)
            } else {
                Some(v2)
            }
        }
    }
}
/// Same as min but uses max of v1/v2
fn optmax<T: PartialOrd>(v1: Option<T>, v2: Option<T>) -> Option<T> {
    if v1.is_none() && v2.is_none() {
        None
    } else {
        if v1.is_none() || v2.is_none() {
            if let Some(v1) = v1 {
                Some(v1)
            } else {
                Some(v2.unwrap())
            }
        } else {
            // neither are none:

            let v1 = v1.unwrap();
            let v2 = v2.unwrap();
            if v1 > v2 {
                Some(v1)
            } else {
                Some(v2)
            }
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
