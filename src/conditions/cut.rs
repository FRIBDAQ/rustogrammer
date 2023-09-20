//!  A Cut condition is defined by a low and high limit
//!  in a single parameter.  Cut conditions evaluate True
//!  If and only if all of the following are true:
//!  
//!  *  The parameter is present in the event.
//!  *  The parameter value is in the range [low, high] for that
//! event.
//! Cut conditions are defined to support caching.  That is
//! Having evaluated the condition for the gate, get_cached_value
//! Will return Some containing the value of the last evaluation
//! until the cache is explicitly invalidated.
//!

use super::*;
use crate::parameters;
use std::collections::HashSet;

/// Cut
///  This struct implements the condition:
///
#[derive(PartialEq, Debug)]
pub struct Cut {
    parameter_id: u32,
    low: f64,
    high: f64,
    cache: Option<bool>,
}
impl Cut {
    pub fn new(id: u32, low: f64, high: f64) -> Cut {
        Cut {
            parameter_id: id,
            low,
            high,
            cache: None, // Starts with invalid cache.
        }
    }
    #[allow(dead_code)]
    pub fn replace_limits(&mut self, low: f64, high: f64) -> &Cut {
        self.low = low;
        self.high = high;
        self.cache = None; // New limits invalidates.
        self
    }
}

impl Condition for Cut {
    fn evaluate(&mut self, event: &parameters::FlatEvent) -> bool {
        let result = if let Some(p) = event[self.parameter_id] {
            (p >= self.low) && (p <= self.high)
        } else {
            false
        };
        self.cache = Some(result);
        result
    }
    fn gate_type(&self) -> String {
        String::from("Cut")
    }
    fn gate_points(&self) -> Vec<(f64, f64)> {
        vec![(self.low, 0.0), (self.high, 0.0)]
    }
    fn dependent_gates(&self) -> Vec<ContainerReference> {
        Vec::<ContainerReference>::new()
    }
    fn get_cached_value(&self) -> Option<bool> {
        self.cache
    }
    fn dependent_parameters(&self) -> Vec<u32> {
        vec![self.parameter_id]
    }
    fn invalidate_cache(&mut self) {
        self.cache = None;
    }
}
/// MultiCut
///     This is a cut that is set on several parameters.
///  The truth or falsity of the cut is the same as it would
///  be for an OR of cuts on the individual parameters.
///
/// Another use of Multicut is that it _does_ implement the
/// Fold trait so it can be used to fold gamma spectra.
///
#[derive(PartialEq, Debug)]
pub struct MultiCut {
    parameters: Vec<u32>,
    low: f64,
    high: f64,
    cache: Option<bool>,
}
impl MultiCut {
    /// Create a new MultiCut condition.
    /// We need a set of parameter ids, a low and a high value:
    ///
    pub fn new(params: &[u32], low: f64, high: f64) -> MultiCut {
        MultiCut {
            parameters: params.to_owned(),
            low,
            high,
            cache: None,
        }
    }
    /// Given a coordinate value, returns true if it lies
    /// inside the low/high limits.
    pub fn inside(&self, value: f64) -> bool {
        (value >= self.low) && (value < self.high)
    }
}
impl Condition for MultiCut {
    fn evaluate(&mut self, event: &parameters::FlatEvent) -> bool {
        for p in self.parameters.iter() {
            if let Some(value) = event[*p] {
                if self.inside(value) {
                    self.cache = Some(true);
                    return true;
                }
            }
        }
        // Failed the gate:
        self.cache = Some(false);
        false
    }
    fn gate_type(&self) -> String {
        String::from("MultiCut")
    }
    fn gate_points(&self) -> Vec<(f64, f64)> {
        vec![(self.low, 0.0), (self.high, 0.0)]
    }
    fn dependent_gates(&self) -> Vec<ContainerReference> {
        vec![]
    }
    fn dependent_parameters(&self) -> Vec<u32> {
        self.parameters.clone()
    }
    fn get_cached_value(&self) -> Option<bool> {
        self.cache
    }
    fn invalidate_cache(&mut self) {
        self.cache = None;
    }

    // fold:

    fn is_fold(&self) -> bool {
        true
    }

    fn evaluate_1(&mut self, event: &parameters::FlatEvent) -> HashSet<u32> {
        let mut result = HashSet::<u32>::new();

        for p in self.parameters.iter() {
            if let Some(value) = event[*p] {
                if !self.inside(value) {
                    result.insert(*p);
                }
            }
        }
        result
    }
    fn evaluate_2(&mut self, event: &parameters::FlatEvent) -> HashSet<(u32, u32)> {
        let mut result = HashSet::<(u32, u32)>::new();

        // iterate over pairs:
        // outer loop goes from [0 - last) index i
        for (i, p1) in self.parameters[0..self.parameters.len() - 1]
            .iter()
            .enumerate()
        {
            // Inner loop goes from [i+1 last]
            for p2 in self.parameters.iter().skip(i + 1) {
                if let Some(val1) = event[*p1] {
                    if let Some(val2) = event[*p2] {
                        if !self.inside(val1) && !self.inside(val2) {
                            result.insert((*p1, *p2));
                        }
                    }
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod cut_tests {
    use super::*;
    use crate::parameters::*;
    #[test]
    fn new_1() {
        let c = Cut::new(12, 100.0, 200.0);
        assert_eq!(
            Cut {
                parameter_id: 12,
                low: 100.0,
                high: 200.0,
                cache: None
            },
            c
        );
    }
    #[test]
    fn replace_1() {
        let mut c = Cut::new(12, 100.0, 200.0);
        c.cache = Some(true); // must get invalidated.
        c.replace_limits(10.0, 20.0);
        assert_eq!(
            Cut {
                parameter_id: 12,
                low: 10.0,
                high: 20.0,
                cache: None
            },
            c
        );
    }
    #[test]
    fn check_1() {
        let mut c = Cut::new(12, 100.0, 200.0);
        let e = FlatEvent::new();

        // My parameter is not present so the gate is false:

        assert!(!c.check(&e));
        assert!(c.get_cached_value().is_some());
        assert!(!c.get_cached_value().unwrap());
        assert!(!c.evaluate(&e));
        c.invalidate_cache();
        assert!(c.get_cached_value().is_none());
    }
    #[test]
    fn check_2() {
        // Event has our parameter in range:

        let mut c = Cut::new(12, 100.0, 200.0);
        let mut e = FlatEvent::new();
        let ev = vec![EventParameter::new(12, 125.0)];
        e.load_event(&ev);

        assert!(c.evaluate(&e));
        assert!(c.check(&e)); // From cache.
        assert!(c.get_cached_value().is_some());
        assert!(c.get_cached_value().unwrap());
    }
    #[test]
    fn check_3() {
        // Event has parameter but not in range:

        let mut c = Cut::new(12, 100.0, 200.0);
        let mut e = FlatEvent::new();
        let ev = vec![EventParameter::new(12, 5.0)];
        e.load_event(&ev);

        assert!(!c.evaluate(&e));
        assert!(!c.check(&e)); // From cache.
        assert!(c.get_cached_value().is_some());
        assert!(!c.get_cached_value().unwrap());
    }
    #[test]
    fn foldable() {
        let c = Cut::new(12, 100.0, 200.0);
        assert!(!c.is_fold());
    }
    // The next tests test cuts in dictionaries.

    #[test]
    fn indict_1() {
        let c = Cut::new(12, 100.0, 200.0);
        let mut dict = ConditionDictionary::new();
        let k = String::from("acut");
        dict.insert(k.clone(), Rc::new(RefCell::new(Box::new(c))));

        let mut e = FlatEvent::new();

        assert!(!dict.get(&k).unwrap().borrow_mut().check(&e));
        invalidate_cache(&mut dict);

        let ev = vec![EventParameter::new(12, 125.0)];
        e.load_event(&ev);

        assert!(dict.get(&k).unwrap().borrow_mut().check(&e));
        invalidate_cache(&mut dict);

        let ev = vec![EventParameter::new(12, 5.0)];
        e.load_event(&ev);
        assert!(!dict.get(&k).unwrap().borrow_mut().check(&e));
    }
    #[test]
    fn indict_2() {
        let mut dict = ConditionDictionary::new();
        let c1 = Cut::new(12, 100.0, 200.0);
        let k1 = String::from("cut1");
        dict.insert(k1.clone(), Rc::new(RefCell::new(Box::new(c1))));

        let c2 = Cut::new(15, 50.0, 100.0);
        let k2 = String::from("cut2");
        dict.insert(k2.clone(), Rc::new(RefCell::new(Box::new(c2))));

        let mut e = FlatEvent::new();
        assert!(!dict.get(&k1).unwrap().borrow_mut().check(&e));
        assert!(!dict.get(&k2).unwrap().borrow_mut().check(&e));
        invalidate_cache(&mut dict);

        let ev = vec![EventParameter::new(12, 125.0)];
        e.load_event(&ev);
        assert!(dict.get(&k1).unwrap().borrow_mut().check(&e));
        assert!(!dict.get(&k2).unwrap().borrow_mut().check(&e));
        invalidate_cache(&mut dict);

        let ev = vec![EventParameter::new(15, 75.0)];
        e.load_event(&ev);
        assert!(!dict.get(&k1).unwrap().borrow_mut().check(&e));
        assert!(dict.get(&k2).unwrap().borrow_mut().check(&e));
        invalidate_cache(&mut dict);

        let ev = vec![
            EventParameter::new(15, 75.0),
            EventParameter::new(12, 125.0),
        ];
        e.load_event(&ev);
        assert!(dict.get(&k1).unwrap().borrow_mut().check(&e));
        assert!(dict.get(&k2).unwrap().borrow_mut().check(&e));
        invalidate_cache(&mut dict);
    }
}
#[cfg(test)]
mod multicut_tests {
    use super::*;
    use crate::parameters::{Event, EventParameter, FlatEvent};

    #[test]
    fn new_1() {
        // Create a new multicut:

        let mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        assert_eq!(
            MultiCut {
                parameters: vec![1, 2, 3],
                low: 100.0,
                high: 200.0,
                cache: None
            },
            mcut
        );
        assert!(mcut.is_fold());
    }
    #[test]
    fn inside_1() {
        // Is inside:

        let mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        assert!(mcut.inside(150.0));
    }
    #[test]
    fn inside_2() {
        // is not inside (low)

        let mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        assert!(!mcut.inside(90.0));
    }
    #[test]
    fn inside_3() {
        // is not inside (high).

        let mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        assert!(!mcut.inside(201.0));
    }
    // Test implementation of Condition trait for MultiCut

    #[test]
    fn eval_1() {
        // Evaluate an event that is inside the gate b/c one of the parameters
        // is -- all are in the event:

        let mut mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        let event: Event = vec![
            EventParameter::new(1, 50.0),
            EventParameter::new(2, 150.0),
            EventParameter::new(3, 210.0),
        ];
        let mut fevent = FlatEvent::new();
        fevent.load_event(&event);
        assert!(mcut.evaluate(&fevent));
        assert_eq!(Some(true), mcut.cache);
    }
    #[test]
    fn eval_2() {
        // Inside gate but not all params are present:

        let mut mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        let event: Event = vec![EventParameter::new(2, 150.0)];
        let mut fevent = FlatEvent::new();
        fevent.load_event(&event);
        assert!(mcut.evaluate(&fevent));
        assert_eq!(Some(true), mcut.cache);
    }
    #[test]
    fn eval_3() {
        // Outside cut all parameters present

        let mut mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        let event: Event = vec![
            EventParameter::new(1, 20.0),
            EventParameter::new(2, 50.0),
            EventParameter::new(3, 250.0),
        ];
        let mut fevent = FlatEvent::new();
        fevent.load_event(&event);

        assert!(!mcut.evaluate(&fevent));
        assert_eq!(Some(false), mcut.cache);
    }
    #[test]
    fn eval_4() {
        // outside cut only some parameters

        let mut mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        let event: Event = vec![EventParameter::new(1, 20.0), EventParameter::new(3, 250.0)];
        let mut fevent = FlatEvent::new();
        fevent.load_event(&event);

        assert!(!mcut.evaluate(&fevent));
        assert_eq!(Some(false), mcut.cache);
    }
    #[test]
    fn eval_5() {
        // outside cut since no parameters are present.

        let mut mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        let event: Event = vec![];
        let mut fevent = FlatEvent::new();
        fevent.load_event(&event);

        assert!(!mcut.evaluate(&fevent));
        assert_eq!(Some(false), mcut.cache);
    }
    #[test]
    fn type_1() {
        // Gate type should be "MultiCut"

        let mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        assert_eq!("MultiCut", mcut.gate_type());
    }
    #[test]
    fn points_1() {
        // Test gate_points:

        let mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        assert_eq!(vec![(100.0, 0.0), (200.0, 0.0)], mcut.gate_points());
    }
    // Dependent gatews and parameters are empty:

    #[test]
    fn dependencies_1() {
        let mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        assert!(mcut.dependent_gates().is_empty());
        assert_eq!(vec![1, 2, 3], mcut.dependent_parameters());
    }
    #[test]
    fn cache_1() {
        // Ensure that caching works properly:

        let mut mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        assert_eq!(None, mcut.get_cached_value());

        let event: Event = vec![];
        let mut fevent = FlatEvent::new();
        fevent.load_event(&event);
        mcut.evaluate(&fevent);
        assert_eq!(Some(false), mcut.get_cached_value());

        let event: Event = vec![EventParameter::new(2, 150.0)];
        fevent.load_event(&event);
        mcut.evaluate(&fevent);
        assert_eq!(Some(true), mcut.get_cached_value());
    }
    #[test]
    fn invalidate_1() {
        // Test invalidate cache:

        let mut mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);

        let event: Event = vec![];
        let mut fevent = FlatEvent::new();
        fevent.load_event(&event);
        mcut.evaluate(&fevent);
        assert_eq!(Some(false), mcut.get_cached_value());

        mcut.invalidate_cache();
        assert_eq!(None, mcut.get_cached_value());
    }
    // Test implementation of Fold trait for Multicut.

    #[test]
    fn fold1_1() {
        // All parameters are in the cut - none come back from evaluate_1:

        let mut mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);

        let event = vec![
            EventParameter::new(1, 110.0),
            EventParameter::new(2, 120.0),
            EventParameter::new(3, 180.0),
        ];
        let mut fevent = FlatEvent::new();
        fevent.load_event(&event);
        assert!(mcut.evaluate_1(&fevent).is_empty());
    }
    #[test]
    fn fold1_2() {
        // All parameters are out of the cut, all come back.

        let mut mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        let event = vec![
            EventParameter::new(1, 10.0),
            EventParameter::new(2, 20.0),
            EventParameter::new(3, 80.0),
        ];
        let mut fevent = FlatEvent::new();
        fevent.load_event(&event);

        assert_eq!(HashSet::from_iter([1, 2, 3].iter().cloned()), mcut.evaluate_1(&fevent));
    }
    #[test]
    fn fold1_3() {
        // Some are in some are out - the ones that are out come back.
        let mut mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        let event = vec![
            EventParameter::new(1, 10.0),
            EventParameter::new(2, 120.0),
            EventParameter::new(3, 80.0),
        ];
        let mut fevent = FlatEvent::new();
        fevent.load_event(&event);

        assert_eq!(HashSet::from_iter([1, 3].iter().cloned()), mcut.evaluate_1(&fevent));
    }
    #[test]
    fn fold2_1() {
        // All pairs have an item in the cut - no pairs returned:

        let mut mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        let event = vec![
            EventParameter::new(1, 110.0),
            EventParameter::new(2, 20.0),
            EventParameter::new(3, 180.0),
        ];
        let mut fevent = FlatEvent::new();
        fevent.load_event(&event);

        assert!(mcut.evaluate_2(&fevent).is_empty());
    }
    #[test]
    fn fold2_2() {
        // There's a pair not in the slice:

        let mut mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        let event = vec![
            EventParameter::new(1, 110.0),
            EventParameter::new(2, 20.0),
            EventParameter::new(3, 80.0),
        ];
        let mut fevent = FlatEvent::new();
        fevent.load_event(&event);

        assert_eq!(HashSet::from_iter([(2, 3)].iter().cloned()), mcut.evaluate_2(&fevent));
    }
    #[test]
    fn fold2_3() {
        // all pairs are in the slice:

        let mut mcut = MultiCut::new(&[1, 2, 3], 100.0, 200.0);
        let event = vec![
            EventParameter::new(1, 10.0),
            EventParameter::new(2, 20.0),
            EventParameter::new(3, 80.0),
        ];
        let mut fevent = FlatEvent::new();
        fevent.load_event(&event);

        assert_eq!(HashSet::from_iter([(1, 2), (1, 3), (2, 3)].iter().cloned()), mcut.evaluate_2(&fevent));
    }
}
