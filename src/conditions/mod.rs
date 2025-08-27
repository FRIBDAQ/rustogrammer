//!  Conditions are what SpecTcl called 'gates'  When a condition is
//!  *applied* to a spectrum it *gates* that spectrum in the sense
//!  That the condition can determine if a spectrum is incremented
//!  For any specific event.
//!
//!  Thus a condition can be thought of as a boolean function defined
//!  over the parameter values of an event.   What makes a struct
//!  a condition is for it to implement the Condition trait
//!  defined by this module.
//!
//!  For spectra to have a condition applied to them, they need
//!  some reference to a condition which is evaluated over each event
//!  polymorphically.  In the case of SpecTcl this is handled using
//!  C++ virtual functions and condition container pointer-like objects.
//!  For Rust, the mechanism of dynamic dispatch requires the use
//!  of *trait objects*   A trait object is a pointer like container
//!  (e.g. Rc or Box) which is defined to contain/reference an
//!  object that is known to implement a specific trait.  This trait,
//!  then defines the interface presented by objects in that container
//!  to clients via the container.
//!
//!  In our case, in order to support transparent replacement, we'll use
//!  `Rc<dyn Condition>`.  These get cloned to pass them to
//!  spectra where Rc::downgrade() turns them into Weak references.
//!  This trick will get around the SpecTcl problem of *eternal gates*
//!  Since it's too much trouble, in general, to track down all
//!  references to a conition, in SpecTcl, gates are never deleted, but
//!  turned into False  gates.  This provide known behavior but some
//!  tricks need to be employed to make those gates invisible in the
//!  gate dictionary (effectively assuming that all False gates are'
//!  actually deleted).   In Rust, a Weak referenc does not prevent
//!  The deletion of the underlying object (via dropping the
//!  remaining Rc containers),  instead, the Weak::upgrade() method is
//!  required to to actually use a Weak reference to an object and,
//!  if the underlying object has been deleted this will return
//!  None.  We can treat None both as:
//!
//!  *    A false evaluation of the condition.
//!  *    A signal that we should remove the condition from whatever it's
//! being used for.
//!
//!
//!   *  The traits needed to implement a condition (plural because
//!  there may need to be traits to get a description of the condition that
//!  can be used to create textual or graphical displays of the condition).
//!   *   A ConditionDictionary type which, when instantiated provides a
//! mechanism to lookup Conditions from names assigned to them.
//!

use crate::parameters;
use std::boxed::Box;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};
// Re-exported module:

pub mod cut;
pub use cut::Cut; // Enbables conditions::Cut to mean conditions::cut::Cut
pub use cut::MultiCut;
pub mod compound;
pub use compound::*;
pub mod twod;
pub use twod::*;

/// The Container trait defines the interface to a condition through
/// a Condition container.   This interface includes:
/// *  Support for an evaluation of the condition for a flattened
/// event
/// *  Support for caching the evaluation of the condition
///
pub trait Condition {
    // Mandatory methods

    ///
    /// Evaluate the condition for a specific event.
    /// If the implementor supports caching, this method should
    /// save the results of the evaulation and indicate its cache
    /// is valid (e.g. get_cached_value calls prior to invalidate_cache
    /// should return Some(\the_cached_value))
    ///
    fn evaluate(&mut self, event: &parameters::FlatEvent) -> bool;

    // Stuff to describe a condition:

    fn condition_type(&self) -> String; // Type of Condition.
    fn condition_points(&self) -> Vec<(f64, f64)>;
    fn dependent_conditions(&self) -> Vec<ContainerReference>;
    fn dependent_parameters(&self) -> Vec<u32>;

    /// Optional methods:
    /// Caching not implemented is the default.
    ///
    fn get_cached_value(&self) -> Option<bool> {
        None
    }
    fn invalidate_cache(&mut self) {}
    ///
    /// The method that really sould be called to check a condition:
    /// If the object has a cached value, the cached value
    /// is returned, otherwise the evaluate, required method is
    /// invoked to force condition evaluation.
    ///
    fn check(&mut self, event: &parameters::FlatEvent) -> bool {
        if let Some(b) = self.get_cached_value() {
            b
        } else {
            self.evaluate(event)
        }
    }

    /// Some conditions can be treated as folds on a Gamma spectrum.
    /// A fold takes an event and reduces it to the set of parameters
    /// or parameter pairs that can increment a gamma spectrum.
    /// there are 1-d and 2-d folds but they both provide the same interface,
    /// the trait below -- which, given an event, provides the indices
    /// or, for 2-d spectra, the index pairs of parameters that
    /// are allowed to increment the spectrum.
    ///
    /// What's going on:
    ///   Gamma spectra (Multi-1D - M) in general will have peaks for each
    /// gamma ray energy detected by the detectors in the parameter set.
    /// it can be that within the timing of a trigger, a cascade of gammas
    /// are emitted and detected within the trigger window resulting in
    /// contributions to several peaks in a single event.
    ///
    /// Folds allow some untangling of this.  Parameters (or pairs in the
    /// case of twod folds) which live within the fold AOI are removed
    /// from the set of parameters that can increment the spectrum.
    /// Setting a fold AOI on a peak, for example and applying that fold
    /// to the spectrum leaves peaks that are in coincidence with the
    /// peak in the AOI.
    ///
    /// Since one can only have a single dyn trait in an object,
    /// the methods below implement folding for a condition.
    /// Note that they default in a way that makes sense for conditions
    /// that cannot be used to fold.
    ///
    fn is_fold(&self) -> bool {
        false
    }

    /// Used by a fold applied to a 1-d spectrum
    ///  events go into the fold and what's
    /// returned is the set of parameter ids that can increment the
    /// spectrum
    ///
    /// ### Parameters:
    /// *  event - the event to check the fold against.
    ///
    /// ### Returns:
    /// *  HashSet&lt;u32&gt; - a vector if parameter ids that are outside the
    /// fold AOI.
    ///
    /// There are two cases: 1d and 2d AOIs (e.g. 2d AOI in another
    /// spectrum applied to this spectrum).
    ///
    /// * If a 1-d AOI is evaluated it should return the parameters
    /// that do not make the AOI true,.
    /// * If a 2-d AOI is evaluated it should return the parameters that
    /// are not in a pair that make the AOI true.
    ///
    fn evaluate_1(&mut self, _event: &parameters::FlatEvent) -> HashSet<u32> {
        HashSet::<u32>::new()
    }

    /// Used to evaluate a fold applied to a 2d spectrum.
    /// An event goes in and what comes out are the set of parameter
    /// pairs that can be allowed to increment the spectrum.
    ///
    /// ### Parameters
    /// *  event - the event to process through the fold.
    ///
    /// ### Returns:
    ///  * HashSet<(u32, u32)> - pairs of parameter ids that can increment
    /// the spectrum.
    ///
    /// The two cases above apply:
    /// *  If a 1-d AOI is evaluated then it should return all parameter
    /// pairs that do not have a parameter that made the AOI true.
    /// * IF a 2-d AOI is evaluated then it should return all paramter pairs
    /// that lie outside the AOI
    ///
    fn evaluate_2(&mut self, _event: &parameters::FlatEvent) -> HashSet<(u32, u32)> {
        HashSet::<(u32, u32)>::new()
    }
}

/// The ConditionContainer is the magic by which
/// Condition objects get dynamic dispatch to their checking
/// Condition methods
///
pub type Container = Rc<RefCell<Box<dyn Condition>>>;
pub type ContainerReference = Weak<RefCell<Box<dyn Condition>>>;

/// ConditionDictionary contains a correspondence between textual
/// names and conditions held in Containers.
/// This provides storage and lookup for conditions that are created
/// in the rustogrammer program through e.g. commands or applying
/// a condition to an spectrum.
///
pub type ConditionDictionary = HashMap<String, Container>;
///
/// Given a condition container which we _think_ lives in a condition dictionary,
///  return the name of the gate.  This is not particluarly fast,
///  we iterate over all key value pairs and if we find one where the
///  RefCell's in the container point to the same underlying object, we
///  return its name.
pub fn condition_name(dict: &ConditionDictionary, condition: &Container) -> Option<String> {
    for (k, v) in dict.iter() {
        if condition.as_ptr() == v.as_ptr() {
            // Same underlying conditions.
            return Some(k.clone());
        }
    }
    None
}
pub fn condition_name_from_ref(
    dict: &ConditionDictionary,
    condition: &ContainerReference,
) -> Option<String> {
    if let Some(s) = condition.upgrade() {
        condition_name(dict, &s)
    } else {
        None
    }
}

///
/// Given a condition dictionary, this free fuction will
/// invalidate the cached values of any conditions that support
/// caching.

pub fn invalidate_cache(d: &mut ConditionDictionary) {
    for (_, v) in d.iter_mut() {
        v.borrow_mut().invalidate_cache();
    }
}

/// The True condition is implemented in this module and returns True
/// no matter what the event contains.  It serves as a trival example
/// of how conditions can be implemented.  No caching is required
/// for the True condition:
pub struct True {}
impl Condition for True {
    fn evaluate(&mut self, _event: &parameters::FlatEvent) -> bool {
        true
    }
    fn condition_type(&self) -> String {
        String::from("True")
    }
    fn condition_points(&self) -> Vec<(f64, f64)> {
        Vec::<(f64, f64)>::new()
    }
    fn dependent_conditions(&self) -> Vec<ContainerReference> {
        Vec::<ContainerReference>::new()
    }
    fn dependent_parameters(&self) -> Vec<u32> {
        Vec::<u32>::new()
    }
}

/// The false gate is implemented in this module and returns
/// False no matter what the event contains.  It servers as a trivial
/// example of a condition implementation
///
pub struct False {}
impl Condition for False {
    fn evaluate(&mut self, _event: &parameters::FlatEvent) -> bool {
        false
    }
    fn condition_type(&self) -> String {
        String::from("False")
    }
    fn condition_points(&self) -> Vec<(f64, f64)> {
        Vec::<(f64, f64)>::new()
    }
    fn dependent_conditions(&self) -> Vec<ContainerReference> {
        Vec::<ContainerReference>::new()
    }
    fn dependent_parameters(&self) -> Vec<u32> {
        Vec::<u32>::new()
    }
}

#[cfg(test)]
mod condition_tests {
    // we can test the polymorphic  evaluation of the
    // conditions in a condition dictionarly
    use super::*;
    use crate::parameters::*;
    #[test]
    fn true_1() {
        let mut dict = ConditionDictionary::new();
        let t: True = True {};
        let k = String::from("true");
        dict.insert(k.clone(), Rc::new(RefCell::new(Box::new(t))));

        let lookedup = dict.get(&k);
        assert!(lookedup.is_some());
        let lookedup = lookedup.unwrap();
        let e = FlatEvent::new();
        assert!(lookedup.borrow_mut().check(&e));
    }
    #[test]
    fn false_1() {
        let mut dict = ConditionDictionary::new();
        let t: False = False {};
        let k = String::from("false");
        dict.insert(k.clone(), Rc::new(RefCell::new(Box::new(t))));

        let lookedup = dict.get(&k);
        assert!(lookedup.is_some());
        let lookedup = lookedup.unwrap();
        let e = FlatEvent::new();
        assert!(!lookedup.borrow_mut().check(&e));
    }
    #[test]
    fn mixed_1() {
        let mut dict = ConditionDictionary::new();
        let t = True {};
        let f = False {};
        let k1 = String::from("true");
        let k2 = String::from("false");

        dict.insert(k1.clone(), Rc::new(RefCell::new(Box::new(t))));
        dict.insert(k2.clone(), Rc::new(RefCell::new(Box::new(f))));
        let e = FlatEvent::new();

        assert!(dict.get(&k1).unwrap().borrow_mut().check(&e));
        assert!(!(dict.get(&k2).unwrap().borrow_mut().check(&e)));
    }
}
