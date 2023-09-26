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
//!     is present, for each X parameter i with value xi, The channel c\[i,xi\] is
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
use ndhistogram::axis::*;
use ndhistogram::value::Sum;
use ndhistogram::*;
use std::cell::RefCell;
use std::collections::{hash_map, HashMap, HashSet};
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

pub mod integration;
pub use integration::*;

///
/// Gated spectra have this.  The condition_name just documents
/// which condition is applied to the spectrum.
/// The gate is the weakened Rc::RefCell that 'points' to the gate.
///
#[derive(Clone)]
pub struct Gate {
    condition_name: String,
    gate: ContainerReference,
}
///  Unlike SpecTcl which just makes an ungated Spectrum
/// have a 'special' True gate, we'll put one of these into the
/// spectrum and a None value for the gate field means the spetrum is
/// ungated.
#[derive(Clone)]
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
    /// Support for fold: Is the gate a fold:

    pub fn is_fold(&mut self) -> bool {
        if let Some(g) = &self.gate {
            if let Some(g) = g.gate.upgrade() {
                g.borrow().is_fold()
            } else {
                self.ungate();
                false
            }
        } else {
            false
        }
    }
    pub fn fold_1d(&mut self, e: &FlatEvent) -> HashSet<u32> {
        if let Some(g) = &self.gate {
            if let Some(g) = g.gate.upgrade() {
                g.borrow_mut().evaluate_1(e)
            } else {
                HashSet::<u32>::new()
            }
        } else {
            HashSet::<u32>::new()
        }
    }
    pub fn fold_2d(&mut self, e: &FlatEvent) -> HashSet<(u32, u32)> {
        if let Some(g) = &self.gate {
            if let Some(g) = g.gate.upgrade() {
                g.borrow_mut().evaluate_2(e)
            } else {
                HashSet::<(u32, u32)>::new()
            }
        } else {
            HashSet::<(u32, u32)>::new()
        }
    }
}

/// We have the following 1-d and 2-d spectra
/// uniform axes and sum, f64 channels:

pub type H1D = Hist1D<axis::Uniform, ndhistogram::value::Sum>;
pub type H1DContainer = Rc<RefCell<H1D>>;

pub type H2D = Hist2D<axis::Uniform, axis::Uniform, ndhistogram::value::Sum>;
pub type H2DContainer = Rc<RefCell<H2D>>;

/// In order to support dynamic dispatch, we need to define a Spectrum trait which combines the
/// Capabilities of ndhistogram objects to supply the interfaces of Axes, Fill and Histogram;
/// Along with the interfaces we need:
/// Normally clients of spectra use:
///
/// *     handle_event to process an event.  This will
///       check any applied gate before attempting to call increment
/// *     gate to gate a spectrum on a condition or replace the gate.
/// *     ungate to remove the gate condition of a spectrum, if any.
pub trait Spectrum {
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
    // Property getters:

    /// Return the spectrum name:
    ///
    fn get_name(&self) -> String;
    fn get_type(&self) -> String;
    fn get_xparams(&self) -> Vec<String>;
    fn get_yparams(&self) -> Vec<String>;
    fn get_xaxis(&self) -> Option<(f64, f64, u32)> {
        if let Some(spec) = self.get_histogram_1d() {
            let x = spec.borrow().axes().as_tuple().0.clone();
            Some((*x.low(), *x.high(), x.num_bins() as u32))
        } else if let Some(spec) = self.get_histogram_2d() {
            let x = spec.borrow().axes().as_tuple().0.clone();
            Some((*x.low(), *x.high(), x.num_bins() as u32))
        } else {
            panic!("Getting x-axis from spectrum that's neither 1 nor 2d.");
        }
    }
    fn get_yaxis(&self) -> Option<(f64, f64, u32)> {
        if let Some(spec) = self.get_histogram_2d() {
            let y = spec.borrow().axes().as_tuple().1.clone();
            Some((*y.low(), *y.high(), y.num_bins() as u32))
        } else {
            None
        }
    }
    fn get_gate(&self) -> Option<String>;

    // Methods that handle gate application:

    fn gate(&mut self, name: &str, dict: &ConditionDictionary) -> Result<(), String>;
    fn ungate(&mut self);

    // manipulate the underlying histogram:

    // Return the underlying histogram:

    fn get_histogram_1d(&self) -> Option<H1DContainer>;
    fn get_histogram_2d(&self) -> Option<H2DContainer>;
    fn is_1d(&self) -> bool {
        // Not so efficient but generic
        self.get_histogram_1d().is_some()
    }

    /// Clear the histogram counts.:

    fn clear(&mut self) {
        if let Some(spec) = self.get_histogram_1d() {
            for c in spec.borrow_mut().iter_mut() {
                *c.value = Sum::new();
            }
        } else if let Some(spec) = self.get_histogram_2d() {
            for c in spec.borrow_mut().iter_mut() {
                *c.value = Sum::new();
            }
        } else {
            panic!("Clearing spectrum that's neither 1 nor 2d");
        }
    }

    // Added to get the spectrum statistics:

    /// Gets the underflow and overflow statistics.
    /// The resulting tuple is
    /// (xunderflows, yunderflows, xoverflows, yoverflows)
    ///
    /// We can implement this on behalf of all spectrrum types
    /// by getting and acting on the appropriate container
    ///
    fn get_out_of_range(&self) -> (u32, u32, u32, u32) {
        let oned = self.get_histogram_1d();
        if let Some(spec) = oned {
            return self.get_out_of_range_1d(spec);
        }
        let twod = self.get_histogram_2d().unwrap();
        self.get_out_of_range_2d(twod)
    }
    // Get out of range statistics for 1d:

    fn get_out_of_range_1d(&self, spec: H1DContainer) -> (u32, u32, u32, u32) {
        let mut overs = 0;
        let mut unders = 0;
        for c in spec.borrow().iter() {
            match c.bin {
                BinInterval::Underflow { end: _ } => {
                    unders += c.value.get() as u32;
                }
                BinInterval::Overflow { start: _ } => {
                    overs += c.value.get() as u32;
                }
                _ => {}
            }
        }
        (unders, 0, overs, 0)
    }
    // Get out of range statistics for 2ds:

    fn get_out_of_range_2d(&self, spec: H2DContainer) -> (u32, u32, u32, u32) {
        let mut xunder = 0;
        let mut yunder = 0;
        let mut xover = 0;
        let mut yover = 0;

        for c in spec.borrow().iter() {
            let xbin = c.bin.0;
            let ybin = c.bin.1;

            match xbin {
                BinInterval::Overflow { start: _ } => {
                    xover += c.value.get() as u32;
                }
                BinInterval::Underflow { end: _ } => {
                    xunder += c.value.get() as u32;
                }
                _ => {}
            }
            match ybin {
                BinInterval::Overflow { start: _ } => {
                    yover += c.value.get() as u32;
                }
                BinInterval::Underflow { end: _ } => {
                    yunder += c.value.get() as u32;
                }
                _ => {}
            }
        }

        (xunder, yunder, xover, yover)
    }
    // These functions are added to support folding:

    ///  Override to return true if the spectrum can be folded.
    ///
    fn can_fold(&self) -> bool {
        false
    }
    fn fold(&mut self, _name: &str, _dict: &ConditionDictionary) -> Result<(), String> {
        Err(String::from(
            "Spectra of this type cannot have folds applied",
        ))
    }
    fn unfold(&mut self) -> Result<(), String> {
        Err(String::from("Spectra of this type can't have folds"))
    }
    fn get_fold(&self) -> Option<String> {
        None
    }
}

// We also need some sort of repository in which spectra can be stored and looked up by name.
//  A hash map does nicely:

pub type SpectrumContainer = Rc<RefCell<dyn Spectrum>>;
pub type SpectrumContainerReference = Weak<RefCell<dyn Spectrum>>;
pub type SpectrumReferences = Vec<SpectrumContainerReference>;
pub type SpectrumDictionary = HashMap<String, SpectrumContainer>;

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
    // Utility methods (Private):

    // Increment the spectra in the specified SpectrumReferences using
    // e the flattened event.  the return value is the set of spectra
    // weak referencds that failed to upgrade to a strong reference.
    //
    fn increment_spectra(spectra: &SpectrumReferences, e: &FlatEvent) -> Vec<usize> {
        let mut result = Vec::<usize>::new();

        for (i, s_container) in spectra.iter().enumerate() {
            if let Some(spectrum) = s_container.upgrade() {
                spectrum.borrow_mut().handle_event(e);
            } else {
                result.push(i); // Spectrum removed from dictionary.
            }
        }

        result
    }
    // It can happen that when running through a spectrum increment list,
    // we come across a spectrum that was removed from the dict.
    // in that case, the promotion from a weak reference to a strong
    // reference will fail.  increment_spectra, makes a list of the
    // indices of spectra for which this has happened (usually empty).
    // This method will run through those indices in reverse order
    // removing those spectra from the list
    //
    fn prune_spectra(spectrum_list: &mut SpectrumReferences, drop_list: &[usize]) {
        for i in drop_list.iter().rev() {
            spectrum_list.remove(*i);
        }
    }

    /// Create a new SpectrumStorage object:
    ///
    pub fn new() -> SpectrumStorage {
        SpectrumStorage {
            dict: SpectrumDictionary::new(),
            spectra_by_parameter: Vec::<Option<SpectrumReferences>>::new(),
            other_spectra: SpectrumReferences::new(),
        }
    }
    /// Iterate over the dict:
    ///
    pub fn iter(&self) -> hash_map::Iter<'_, String, SpectrumContainer> {
        self.dict.iter()
    }

    /// Add a spectrum encapslated in a SpectrumContainer to the
    /// spectrum storage:
    /// -    We clone the input spectrum twice, once for the
    ///      dictionary and once for the increment list.
    /// -    The dictionary clone is inserted directly in the dictionary.
    /// -    The increment clone is asked to give us the required parameter
    /// and then demoted to a weak reference.
    ///     *   If the required parameter is None, the spectrum is inserted
    /// in the other_spectra list.
    ///     *   if the required parameter is Some, the parameter number
    /// is extracted and inserted in the correct slot of spectra_by_parameter,
    /// expanding that vector if needed and changing it's None to a Some --
    /// if needed.
    ///
    /// The result of the dictionary insertion is what's returned so that
    /// we are aware of a duplicate spectrum name overriding the existing
    /// spectrum name.
    ///
    pub fn add(&mut self, spectrum: SpectrumContainer) -> Option<SpectrumContainer> {
        let inc_ref = Rc::clone(&spectrum);
        let result = self
            .dict
            .insert(inc_ref.borrow().get_name(), Rc::clone(&spectrum));

        let param = inc_ref.borrow().required_parameter();
        let inc_ref = Rc::downgrade(&inc_ref);

        if let Some(pno) = param {
            let pno = pno as usize;
            if self.spectra_by_parameter.len() <= pno {
                self.spectra_by_parameter.resize(pno + 1, None);
            }
            // The array is big enough but the element might be None

            if self.spectra_by_parameter[pno].is_none() {
                self.spectra_by_parameter[pno] = Some(SpectrumReferences::new());
            }
            // Now we can insert the new spectrum in the vector:

            let list = self.spectra_by_parameter[pno].as_mut().unwrap();
            list.push(inc_ref);
        } else {
            self.other_spectra.push(inc_ref);
        }
        result
    }
    /// Does the specified spectrum exist?
    ///
    pub fn exists(&self, name: &str) -> bool {
        self.dict.contains_key(name)
    }

    /// get the spectrum with a given name.  The result is an Option:
    /// -    None if there is no matching spectrum.
    /// -    Some(&SpectrumContainer) if there is.
    /// If the caller is going to hold on to that reference for
    /// some time, they should clone the container.
    ///
    pub fn get(&self, name: &str) -> Option<&SpectrumContainer> {
        self.dict.get(name)
    }
    /// Clear all the spectra
    ///
    #[allow(dead_code)]
    pub fn clear_all(&self) {
        for (_, spec) in self.dict.iter() {
            spec.borrow_mut().clear();
        }
    }
    /// Process an event
    /// We get a raw event:
    /// *    Populate a flat event from it.
    /// *    For each parameter in the event, if there's Some in its
    /// spectra_by_parameter list, iterate over the list promoting the
    /// the reference and asking the spectrum to process the flattened parameter
    /// *    Finally do the same for the other_spectra list.
    /// *    Keep lists of spectra that have been deleted (upgrade gave None)
    /// when all this is done, remove those spectra from the associated arrays.
    ///
    pub fn process_event(&mut self, e: &Event) {
        let mut fe = FlatEvent::new();
        fe.load_event(e);

        for p in e.iter() {
            let id = p.id as usize;
            if id < self.spectra_by_parameter.len() {
                if let Some(spectra) = self.spectra_by_parameter[id].as_mut() {
                    let dropped_list = Self::increment_spectra(spectra, &fe);
                    Self::prune_spectra(spectra, &dropped_list);
                }
            }
        }
        // Now do the other spectra:

        let dropped_list = Self::increment_spectra(&self.other_spectra, &fe);
        Self::prune_spectra(&mut self.other_spectra, &dropped_list);
    }
    /// Delete a spectrum.
    /// Given how we handle spectra in process_event, we only need to remove
    /// the item from the dict.  When the next event that would
    /// attempt to increment the spectrum is is processed it will be pruned from
    /// the appropriate spectrum list.
    /// What is returned is an option
    /// *  None - The item was not in the dict.
    /// *  Some - the payload is a SpectrumContainer for the spectrum
    /// which the caller can do with as they please (including dropping).
    ///
    pub fn remove(&mut self, name: &str) -> Option<SpectrumContainer> {
        self.dict.remove(name)
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
    let low_lim = if let Some(low) = low {
        low
    } else if let Some(l) = default_lims.0 {
        l
    } else {
        return Err(format!("No default low limit defined for {}", param_name));
    };
    let high_lim = if let Some(h) = high {
        h
    } else if let Some(h) = default_lims.1 {
        h
    } else {
        return Err(format!("No default high limit defined for {}", param_name));
    };

    let bin_count = if let Some(b) = bins {
        b
    } else if let Some(b) = pdef.get_bins() {
        b
    } else {
        return Err(format!("No default bin count for {}", param_name));
    };

    Ok((low_lim, high_lim, bin_count))
}

// Useful utility methods (private):

fn optmin<T: PartialOrd>(v1: Option<T>, v2: Option<T>) -> Option<T> {
    if v1.is_none() && v2.is_none() {
        None
    } else if v1.is_none() || v2.is_none() {
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
/// Same as min but uses max of v1/v2
fn optmax<T: PartialOrd>(v1: Option<T>, v2: Option<T>) -> Option<T> {
    if v1.is_none() && v2.is_none() {
        None
    } else if v1.is_none() || v2.is_none() {
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
        dict.insert(
            String::from("true"),
            Rc::new(RefCell::new(Box::new(test_gate))),
        );

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
        dict.insert(
            String::from("true"),
            Rc::new(RefCell::new(Box::new(test_gate))),
        );

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
        dict.insert(
            String::from("true"),
            Rc::new(RefCell::new(Box::new(test_gate))),
        );

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
        dict.insert(
            String::from("false"),
            Rc::new(RefCell::new(Box::new(test_gate))),
        );

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
        dict.insert(
            String::from("false"),
            Rc::new(RefCell::new(Box::new(test_gate))),
        );

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
mod spec_storage_tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    // Utility method to create the parameters:

    fn make_params() -> ParameterDictionary {
        let mut p = ParameterDictionary::new();

        for i in 0..16 {
            let name = format!("param.{}", i);
            p.add(&name).expect("Failed to make a parameter");
            let param = p.lookup_mut(&name).expect("failed to lookup param");
            param.set_limits(0.0, 4096.0);
            param.set_bins(4096);
        }
        p
    }

    #[test]
    fn new_1() {
        // New creates what it says it will.

        let ss = SpectrumStorage::new();
        assert_eq!(0, ss.dict.len());
        assert_eq!(0, ss.spectra_by_parameter.len());
        assert_eq!(0, ss.other_spectra.len());
    }
    #[test]
    fn add_1() {
        // Add a 1-d spectrum - should show up in the dict
        // and appropriate index of the by parameter list:

        let pdict = make_params();
        let spec = Oned::new("test", "param.5", &pdict, None, None, None).unwrap();
        let spec_container: SpectrumContainer = Rc::new(RefCell::new(spec));

        let mut store = SpectrumStorage::new();
        store.add(Rc::clone(&spec_container)); // Clone so I keep mine.

        assert_eq!(1, store.dict.len());
        let dict_spec = store.dict.get("test");
        assert!(dict_spec.is_some());
        let dict_spec = dict_spec.as_ref().unwrap();
        assert_eq!(
            spec_container.borrow().get_name(),
            dict_spec.borrow().get_name()
        );

        // Figure out which element of spectra_by_parameter it should be in:

        let param = pdict.lookup("param.5").expect("Failed parameter lookup");
        let pid = param.get_id() as usize;

        assert!(store.spectra_by_parameter.len() >= pid);
        assert!(store.spectra_by_parameter[pid].is_some());
        assert_eq!(1, store.spectra_by_parameter[pid].as_ref().unwrap().len());
        let inc_container = store.spectra_by_parameter[pid].as_ref().unwrap()[0].upgrade();
        assert!(inc_container.is_some());
        let inc_container = inc_container.unwrap();
        assert_eq!(
            spec_container.borrow().get_name(),
            inc_container.borrow().get_name()
        );
    }
    #[test]
    fn add_2() {
        // 2d the same but the index in the by parameter list is the
        // x parameter:

        let pdict = make_params();
        let spec = Twod::new(
            "test", "param.2", "param.3", &pdict, None, None, None, None, None, None,
        )
        .unwrap();
        let spec_container: SpectrumContainer = Rc::new(RefCell::new(spec));

        let mut store = SpectrumStorage::new();
        store.add(Rc::clone(&spec_container)); // Clone so I keep mine.

        assert_eq!(1, store.dict.len());
        let dict_spec = store.dict.get("test");
        assert!(dict_spec.is_some());
        let dict_spec = dict_spec.as_ref().unwrap();
        assert_eq!(
            spec_container.borrow().get_name(),
            dict_spec.borrow().get_name()
        );

        // Figure out which element of spectra_by_parameter it should be in
        // the id of the x parameter:

        let param = pdict.lookup("param.2").expect("Failed parameter lookup");
        let pid = param.get_id() as usize;

        assert!(store.spectra_by_parameter.len() >= pid);
        assert!(store.spectra_by_parameter[pid].is_some());
        assert_eq!(1, store.spectra_by_parameter[pid].as_ref().unwrap().len());
        let inc_container = store.spectra_by_parameter[pid].as_ref().unwrap()[0].upgrade();
        assert!(inc_container.is_some());
        let inc_container = inc_container.unwrap();
        assert_eq!(
            spec_container.borrow().get_name(),
            inc_container.borrow().get_name()
        );
    }
    #[test]
    fn add_3() {
        // A multi1d has no required param so it should land in
        // other_spectra.

        let pdict = make_params();
        let spec = Multi1d::new(
            "test",
            vec![
                String::from("param.1"),
                String::from("param.2"),
                String::from("param.3"),
                String::from("param.4"),
            ],
            &pdict,
            None,
            None,
            None,
        )
        .expect("Failed to make spectrum");

        let spec_container: SpectrumContainer = Rc::new(RefCell::new(spec));

        let mut store = SpectrumStorage::new();
        store.add(Rc::clone(&spec_container)); // Clone so I keep mine.

        assert_eq!(1, store.dict.len());
        let dict_spec = store.dict.get("test");
        assert!(dict_spec.is_some());
        let dict_spec = dict_spec.as_ref().unwrap();
        assert_eq!(
            spec_container.borrow().get_name(),
            dict_spec.borrow().get_name()
        );
        // The spectrum should be in other_spectra:

        assert_eq!(1, store.other_spectra.len());
        let inc_spec = store.other_spectra[0]
            .upgrade()
            .expect("Could not make Ref from spectrum weak ptr");
        assert_eq!(
            spec_container.borrow().get_name(),
            inc_spec.borrow().get_name()
        );
    }
    #[test]
    fn add_4() {
        // multi2d adds correctly.
        let pdict = make_params();
        let pars = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.3"),
            String::from("param.4"),
            String::from("param.5"),
            String::from("param.6"),
        ];

        let spec = Multi2d::new("test", pars, &pdict, None, None, None, None, None, None)
            .expect("could not create spectrum");
        let spec_container: SpectrumContainer = Rc::new(RefCell::new(spec));

        let mut store = SpectrumStorage::new();
        store.add(Rc::clone(&spec_container)); // Clone so I keep mine.

        assert_eq!(1, store.dict.len());
        let dict_spec = store.dict.get("test");
        assert!(dict_spec.is_some());
        let dict_spec = dict_spec.as_ref().unwrap();
        assert_eq!(
            spec_container.borrow().get_name(),
            dict_spec.borrow().get_name()
        );
        // The spectrum should be in other_spectra:

        assert_eq!(1, store.other_spectra.len());
        let inc_spec = store.other_spectra[0]
            .upgrade()
            .expect("Could not make Ref from spectrum weak ptr");
        assert_eq!(
            spec_container.borrow().get_name(),
            inc_spec.borrow().get_name()
        );
    }
    #[test]
    fn add_5() {
        // PGamma adds correctly:

        let pdict = make_params();
        let xpars = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.3"),
            String::from("param.4"),
            String::from("param.5"),
            String::from("param.6"),
        ];
        let ypars = vec![
            String::from("param.7"),
            String::from("param.8"),
            String::from("param.9"),
            String::from("param.10"),
            String::from("param.11"),
            String::from("param.12"),
        ];

        let spec = PGamma::new(
            "test", &xpars, &ypars, &pdict, None, None, None, None, None, None,
        )
        .expect("Unable to make spectrum");

        let spec_container: SpectrumContainer = Rc::new(RefCell::new(spec));

        let mut store = SpectrumStorage::new();
        store.add(Rc::clone(&spec_container)); // Clone so I keep mine.

        assert_eq!(1, store.dict.len());
        let dict_spec = store.dict.get("test");
        assert!(dict_spec.is_some());
        let dict_spec = dict_spec.as_ref().unwrap();
        assert_eq!(
            spec_container.borrow().get_name(),
            dict_spec.borrow().get_name()
        );
        // The spectrum should be in other_spectra:

        assert_eq!(1, store.other_spectra.len());
        let inc_spec = store.other_spectra[0]
            .upgrade()
            .expect("Could not make Ref from spectrum weak ptr");
        assert_eq!(
            spec_container.borrow().get_name(),
            inc_spec.borrow().get_name()
        );
    }
    #[test]
    fn add_6() {
        // Summary spectrum:

        let pdict = make_params();
        let pars = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.3"),
            String::from("param.4"),
            String::from("param.5"),
            String::from("param.6"),
        ];

        let spec =
            Summary::new("test", pars, &pdict, None, None, None).expect("Unable to make spectrum");

        let spec_container: SpectrumContainer = Rc::new(RefCell::new(spec));

        let mut store = SpectrumStorage::new();
        store.add(Rc::clone(&spec_container)); // Clone so I keep mine.

        assert_eq!(1, store.dict.len());
        let dict_spec = store.dict.get("test");
        assert!(dict_spec.is_some());
        let dict_spec = dict_spec.as_ref().unwrap();
        assert_eq!(
            spec_container.borrow().get_name(),
            dict_spec.borrow().get_name()
        );
        // The spectrum should be in other_spectra:

        assert_eq!(1, store.other_spectra.len());
        let inc_spec = store.other_spectra[0]
            .upgrade()
            .expect("Could not make Ref from spectrum weak ptr");
        assert_eq!(
            spec_container.borrow().get_name(),
            inc_spec.borrow().get_name()
        );
    }
    #[test]
    fn add_7() {
        // 2d sum spectrum adds correctly

        let pdict = make_params();
        let pars = vec![
            (String::from("param.1"), String::from("param.2")),
            (String::from("param.3"), String::from("param.4")),
            (String::from("param.5"), String::from("param.6")),
            (String::from("param.7"), String::from("param.7")),
            (String::from("param.8"), String::from("param.8")),
        ];
        let spec = TwodSum::new("test", pars, &pdict, None, None, None, None, None, None)
            .expect("Failed to make spectrum");
        let spec_container: SpectrumContainer = Rc::new(RefCell::new(spec));

        let mut store = SpectrumStorage::new();
        store.add(Rc::clone(&spec_container)); // Clone so I keep mine.

        assert_eq!(1, store.dict.len());
        let dict_spec = store.dict.get("test");
        assert!(dict_spec.is_some());
        let dict_spec = dict_spec.as_ref().unwrap();
        assert_eq!(
            spec_container.borrow().get_name(),
            dict_spec.borrow().get_name()
        );
        // The spectrum should be in other_spectra:

        assert_eq!(1, store.other_spectra.len());
        let inc_spec = store.other_spectra[0]
            .upgrade()
            .expect("Could not make Ref from spectrum weak ptr");
        assert_eq!(
            spec_container.borrow().get_name(),
            inc_spec.borrow().get_name()
        );
    }
    #[test]
    fn get_1() {
        // Having put a spectrum into the storage we can look it up by
        // name:

        let pdict = make_params();
        let spec1 =
            Oned::new("spec1", "param.1", &pdict, None, None, None).expect("failed to make spec1");
        let spec2 =
            Oned::new("spec2", "param.2", &pdict, None, None, None).expect("failed to make spec2");

        let spec1_container: SpectrumContainer = Rc::new(RefCell::new(spec1));
        let mut store = SpectrumStorage::new();
        assert!(store.add(spec1_container).is_none());
        let spec2_container: SpectrumContainer = Rc::new(RefCell::new(spec2));
        assert!(store.add(spec2_container).is_none());

        let result = store.get("spec1");
        assert!(result.is_some());
        let spec = result.unwrap();
        assert_eq!(String::from("spec1"), spec.borrow().get_name());
    }
    #[test]
    fn get_2() {
        // Looking up a nonexistent sepctrum gives none:

        let pdict = make_params();
        let spec1 =
            Oned::new("spec1", "param.1", &pdict, None, None, None).expect("failed to make spec1");
        let spec2 =
            Oned::new("spec2", "param.2", &pdict, None, None, None).expect("failed to make spec2");

        let spec1_container: SpectrumContainer = Rc::new(RefCell::new(spec1));
        let mut store = SpectrumStorage::new();
        assert!(store.add(spec1_container).is_none());
        let spec2_container: SpectrumContainer = Rc::new(RefCell::new(spec2));
        assert!(store.add(spec2_container).is_none());

        assert!(store.get("no-such").is_none());
    }
    #[test]
    fn clear_all_1() {
        // Test the ability to clear all spectra...

        let pdict = make_params();
        let mut spec1 = Oned::new("spec1", "param.1", &pdict, None, None, None)
            .expect("Failed to make spectrum 1");
        let mut spec2 = Twod::new(
            "spec2",
            "param.2",
            "param.3",
            &pdict,
            Some(0.0),
            Some(1024.0),
            Some(256),
            Some(0.),
            Some(1024.0),
            Some(256),
        )
        .expect("Failed to make spec2");

        // Get parameter ids for param.1, param.2, param.3:

        let p1 = pdict.lookup("param.1").expect("param.1 should be created");
        let p2 = pdict.lookup("param.2").expect("param.2 should be created");
        let p3 = pdict.lookup("param.3").expect("param3. should be created");

        let idvec = [p1.get_id(), p2.get_id(), p3.get_id()];

        // Easiest to do the increments prior to insertion for this test:

        let mut counter = 0.0;
        for _ in 0..100 {
            let mut event = Event::new();
            let mut fe = FlatEvent::new();
            for j in &idvec {
                event.push(EventParameter::new(*j, counter));
                counter += 1.0;
            }
            fe.load_event(&event);
            spec1.handle_event(&fe);
            spec2.handle_event(&fe);
        }
        // Each spectrum has 100 events scattered through it

        let mut store = SpectrumStorage::new();
        let c1: SpectrumContainer = Rc::new(RefCell::new(spec1));
        let c2: SpectrumContainer = Rc::new(RefCell::new(spec2));
        assert!(store.add(c1).is_none());
        assert!(store.add(c2).is_none());

        // After clearing all all spectra shoulid be empty:

        store.clear_all();

        let c1 = store
            .get("spec1")
            .expect("spec1 should have been in the container");
        for c in c1.borrow().get_histogram_1d().unwrap().borrow().iter() {
            assert_eq!(0.0, c.value.get());
        }
        let c2 = store
            .get("spec2")
            .expect("spec2 should have been in the container");
        for c in c2.borrow().get_histogram_2d().unwrap().borrow().iter() {
            assert_eq!(0.0, c.value.get());
        }
    }
    // process an event should visit all contained spectra and invoke their
    // handle_event resulting in increments when appropriate.

    #[test]
    fn prcevent_1() {
        let pdict = make_params();
        let spec1 = Oned::new("spec1", "param.1", &pdict, None, None, None)
            .expect("Failed to make spectrum 1");
        let spec2 = Twod::new(
            "spec2",
            "param.2",
            "param.3",
            &pdict,
            Some(0.0),
            Some(1024.0),
            Some(256),
            Some(0.),
            Some(1024.0),
            Some(256),
        )
        .expect("Failed to make spec2");

        // Add the spectra to the container:

        let mut store = SpectrumStorage::new();
        store.add(Rc::new(RefCell::new(spec1)));
        store.add(Rc::new(RefCell::new(spec2)));

        // Get parameter ids for param.1, param.2, param.3:

        let p1 = pdict.lookup("param.1").expect("param.1 should be created");
        let p2 = pdict.lookup("param.2").expect("param.2 should be created");
        let p3 = pdict.lookup("param.3").expect("param3. should be created");

        let idvec = [p1.get_id(), p2.get_id(), p3.get_id()];

        // Make and process some events:

        let mut counter = 0.0;
        for _ in 0..100 {
            let mut event = Event::new();
            for j in &idvec {
                event.push(EventParameter::new(*j, counter));
                counter += 1.0;
            }

            store.process_event(&event);
        }

        // We're just going to be sure each spectrum has 100 counts:

        let s1 = store
            .get("spec1")
            .expect("Failed to fetch spec1 from store");
        let h1 = s1
            .borrow()
            .get_histogram_1d()
            .expect("Failed to get 1d histogram");

        let mut sum1 = 0.0;
        for c in h1.borrow().iter() {
            sum1 += c.value.get();
        }
        assert_eq!(100.0, sum1);

        let s2 = store
            .get("spec2")
            .expect("Failed to fetch spec2 from store");
        let h2 = s2
            .borrow()
            .get_histogram_2d()
            .expect("Failed to get 2d histogram");
        let mut sum2 = 0.0;
        for c in h2.borrow().iter() {
            sum2 += c.value.get();
        }
        assert_eq!(100.0, sum2);
    }
    // Now s1 will be gated on True and s2 on False.

    #[test]
    fn prcevent_2() {
        let pdict = make_params();
        let spec1 = Oned::new("spec1", "param.1", &pdict, None, None, None)
            .expect("Failed to make spectrum 1");
        let spec2 = Twod::new(
            "spec2",
            "param.2",
            "param.3",
            &pdict,
            Some(0.0),
            Some(1024.0),
            Some(256),
            Some(0.),
            Some(1024.0),
            Some(256),
        )
        .expect("Failed to make spec2");

        // Add the spectra to the container:

        let mut store = SpectrumStorage::new();
        store.add(Rc::new(RefCell::new(spec1)));
        store.add(Rc::new(RefCell::new(spec2)));

        // Make true/false conditions.

        let mut cd = ConditionDictionary::new();
        assert!(cd
            .insert(
                String::from("true"),
                Rc::new(RefCell::new(Box::new(True {})))
            )
            .is_none());
        assert!(cd
            .insert(
                String::from("false"),
                Rc::new(RefCell::new(Box::new(False {})))
            )
            .is_none());

        // Gate "spec1" on "true" and "spec2" on "false"

        store
            .get("spec1")
            .expect("spec1 was missing")
            .borrow_mut()
            .gate("true", &cd)
            .expect("true gate not found when gating spec1.");
        store
            .get("spec2")
            .expect("spec2 was missing")
            .borrow_mut()
            .gate("false", &cd)
            .expect("false gate not found when gating spec2");

        // Get parameter ids for param.1, param.2, param.3:

        let p1 = pdict.lookup("param.1").expect("param.1 should be created");
        let p2 = pdict.lookup("param.2").expect("param.2 should be created");
        let p3 = pdict.lookup("param.3").expect("param3. should be created");

        let idvec = [p1.get_id(), p2.get_id(), p3.get_id()];

        // Make and process some events:

        let mut counter = 0.0;
        for _ in 0..100 {
            let mut event = Event::new();
            for j in &idvec {
                event.push(EventParameter::new(*j, counter));
                counter += 1.0;
            }

            store.process_event(&event);
        }

        // We're just going to be sure each spectrum has 100 counts:

        let s1 = store
            .get("spec1")
            .expect("Failed to fetch spec1 from store");
        let h1 = s1
            .borrow()
            .get_histogram_1d()
            .expect("Failed to get 1d histogram");

        let mut sum1 = 0.0;
        for c in h1.borrow().iter() {
            sum1 += c.value.get();
        }
        assert_eq!(100.0, sum1);

        let s2 = store
            .get("spec2")
            .expect("Failed to fetch spec2 from store");
        let h2 = s2
            .borrow()
            .get_histogram_2d()
            .expect("Failed to get 2d histogram");
        let mut sum2 = 0.0;
        for c in h2.borrow().iter() {
            sum2 += c.value.get();
        }
        assert_eq!(0.0, sum2);
    }
    #[test]
    fn remove_1() {
        // Remove nonexistent spectrum returns None

        let mut store = SpectrumStorage::new();
        assert!(store.remove("nosuch").is_none());
    }
    #[test]
    fn remove_2() {
        // remove existing spectrum
        // 1. returns Some with the spectrum container as the payload.
        // 2. the spectrum is no longer in the store:

        let pdict = make_params();
        let spec1 = Oned::new("spec1", "param.1", &pdict, None, None, None)
            .expect("Failed to make spectrum 1");
        let spec2 = Twod::new(
            "spec2",
            "param.2",
            "param.3",
            &pdict,
            Some(0.0),
            Some(1024.0),
            Some(256),
            Some(0.),
            Some(1024.0),
            Some(256),
        )
        .expect("Failed to make spec2");

        // Add the spectra to the container:

        let mut store = SpectrumStorage::new();
        store.add(Rc::new(RefCell::new(spec1)));
        store.add(Rc::new(RefCell::new(spec2)));

        // REmove "spec1"

        let s1 = store.remove("spec1");
        assert!(s1.is_some());
        assert_eq!(String::from("spec1"), s1.unwrap().borrow().get_name());
        assert!(store.remove("spec1").is_none());
    }
}
// tests for the trait function to get statistics.
// Note that this can be tested here using simple 1-d and 2-d histograms
// because the default implementation of the trait works by understanding
// the underlying data is just an ndhistogram either 1d or 2d.

#[cfg(test)]
mod stats_tests {
    use super::*;
    #[test]
    fn onedstats_1() {
        let mut p = ParameterDictionary::new();
        p.add("someparam").expect("Failed to add 'someparam'");
        let spec = Oned::new("test", "someparam", &p, Some(0.0), Some(10.0), Some(100))
            .expect("Unable to create 1d spectrum");

        let hist = spec
            .get_histogram_1d()
            .expect("Failed to unwrap 1d histogram");
        hist.borrow_mut().fill(&-1.0); // Underflow in x.

        assert_eq!((1, 0, 0, 0), spec.get_out_of_range());
    }
    #[test]
    fn onedstats_2() {
        let mut p = ParameterDictionary::new();
        p.add("someparam").expect("Failed to add 'someparam'");
        let spec = Oned::new("test", "someparam", &p, Some(0.0), Some(10.0), Some(100))
            .expect("Unable to create 1d spectrum");

        let hist = spec
            .get_histogram_1d()
            .expect("Failed to unwrap 1d histogram");
        hist.borrow_mut().fill(&10.5); // Underflow in x.

        assert_eq!((0, 0, 1, 0), spec.get_out_of_range());
    }

    #[test]
    fn twodstats_1() {
        // Underflow in x only:

        let mut p = ParameterDictionary::new();
        p.add("x").expect("Unable to add x parameter");
        p.add("y").expect("Unable to add y parameter");

        let spec = Twod::new(
            "test",
            "x",
            "y",
            &p,
            Some(0.0),
            Some(1.0),
            Some(100),
            Some(-1.0),
            Some(1.0),
            Some(100),
        )
        .expect("Unable to create 2d spectrum");

        let hist = spec
            .get_histogram_2d()
            .expect("Failed to unwrap 2d histogram");

        hist.borrow_mut().fill(&(-0.5, 0.0));
        assert_eq!((1, 0, 0, 0), spec.get_out_of_range());
    }
    #[test]
    fn twodstats_2() {
        // Undeflow in y only:

        let mut p = ParameterDictionary::new();
        p.add("x").expect("Unable to add x parameter");
        p.add("y").expect("Unable to add y parameter");

        let spec = Twod::new(
            "test",
            "x",
            "y",
            &p,
            Some(0.0),
            Some(1.0),
            Some(100),
            Some(-1.0),
            Some(1.0),
            Some(100),
        )
        .expect("Unable to create 2d spectrum");

        let hist = spec
            .get_histogram_2d()
            .expect("Failed to unwrap 2d histogram");

        hist.borrow_mut().fill(&(0.0, -1.5));

        assert_eq!((0, 1, 0, 0), spec.get_out_of_range());
    }
    #[test]
    fn twodstats_3() {
        // underflow in both x/y:

        let mut p = ParameterDictionary::new();
        p.add("x").expect("Unable to add x parameter");
        p.add("y").expect("Unable to add y parameter");

        let spec = Twod::new(
            "test",
            "x",
            "y",
            &p,
            Some(0.0),
            Some(1.0),
            Some(100),
            Some(-1.0),
            Some(1.0),
            Some(100),
        )
        .expect("Unable to create 2d spectrum");

        let hist = spec
            .get_histogram_2d()
            .expect("Failed to unwrap 2d histogram");

        hist.borrow_mut().fill(&(-0.5, -1.5));
        assert_eq!((1, 1, 0, 0), spec.get_out_of_range());
    }
    #[test]
    fn twodstats_4() {
        // Overflow in x:

        let mut p = ParameterDictionary::new();
        p.add("x").expect("Unable to add x parameter");
        p.add("y").expect("Unable to add y parameter");

        let spec = Twod::new(
            "test",
            "x",
            "y",
            &p,
            Some(0.0),
            Some(1.0),
            Some(100),
            Some(-1.0),
            Some(1.0),
            Some(100),
        )
        .expect("Unable to create 2d spectrum");

        let hist = spec
            .get_histogram_2d()
            .expect("Failed to unwrap 2d histogram");

        hist.borrow_mut().fill(&(1.2, 0.0));
        assert_eq!((0, 0, 1, 0), spec.get_out_of_range());
    }
    #[test]
    fn twodstats_5() {
        // Overflow in y only:

        let mut p = ParameterDictionary::new();
        p.add("x").expect("Unable to add x parameter");
        p.add("y").expect("Unable to add y parameter");

        let spec = Twod::new(
            "test",
            "x",
            "y",
            &p,
            Some(0.0),
            Some(1.0),
            Some(100),
            Some(-1.0),
            Some(1.0),
            Some(100),
        )
        .expect("Unable to create 2d spectrum");

        let hist = spec
            .get_histogram_2d()
            .expect("Failed to unwrap 2d histogram");

        hist.borrow_mut().fill(&(0.0, 1.1));
        assert_eq!((0, 0, 0, 1), spec.get_out_of_range());
    }
    #[test]
    fn twodstats_6() {
        // overflows both x/y:

        let mut p = ParameterDictionary::new();
        p.add("x").expect("Unable to add x parameter");
        p.add("y").expect("Unable to add y parameter");

        let spec = Twod::new(
            "test",
            "x",
            "y",
            &p,
            Some(0.0),
            Some(1.0),
            Some(100),
            Some(-1.0),
            Some(1.0),
            Some(100),
        )
        .expect("Unable to create 2d spectrum");

        let hist = spec
            .get_histogram_2d()
            .expect("Failed to unwrap 2d histogram");

        hist.borrow_mut().fill(&(1.1, 1.1));
        assert_eq!((0, 0, 1, 1), spec.get_out_of_range());
    }
}
