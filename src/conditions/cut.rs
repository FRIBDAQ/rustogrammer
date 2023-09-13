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
struct MultiCut {
    parameters: Vec<u32>,
    low: f64,
    high: f64,
    cache: Option<bool>,
}
impl MultiCut {
    /// Create a new MultiCut condition.
    /// We need a set of parameter ids, a low and a high value:
    ///
    pub fn new(params: &Vec<u32>, low: f64, high: f64) -> MultiCut {
        MultiCut {
            parameters: params.clone(),
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

    fn invalidate_cache(&mut self) {
        self.cache = None;
    }
}
impl Fold for MultiCut {
    fn evaluate_1(&mut self, event: &parameters::FlatEvent) -> Vec<u32> {
        let mut result = Vec::<u32>::new();

        for p in self.parameters.iter() {
            if let Some(value) = event[*p] {
                if !self.inside(value) {
                    result.push(*p);
                }
            }
        }
        result
    }
    fn evaluate_2(&mut self, event: &parameters::FlatEvent) -> Vec<(u32, u32)> {
        let mut result = Vec::<(u32, u32)>::new();

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
                            result.push((*p1, *p2));
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
    use crate::parameters;

    #[test]
    fn new_1() {
        // Create a new multicut:

        let mcut = MultiCut::new(&vec![1, 2, 3], 100.0, 200.0);
        assert_eq!(
            MultiCut {
                parameters: vec![1, 2, 3],
                low: 100.0,
                high: 200.0,
                cache: None
            },
            mcut
        );
    }
    #[test]
    fn inside_1() {
        // Is inside:

        let mcut = MultiCut::new(&vec![1,2,3], 100.0, 200.0);
        assert!(mcut.inside(150.0));
    }
    #[test]
    fn inside_2() {
        // is not inside (low)

        let mcut = MultiCut::new(&vec![1,2,3], 100.0, 200.0);
        assert!(!mcut.inside(90.0));
    }
    #[test]
    fn inside_3() {
        // is not inside (high).
        
        let mcut = MultiCut::new(&vec![1,2,3], 100.0, 200.0);
        assert!(!mcut.inside(201.0));
    }
}
