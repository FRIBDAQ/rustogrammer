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
            low: low,
            high: high,
            cache: None, // Starts with invalid cache.
        }
    }
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
    fn get_cached_value(&self) -> Option<bool> {
        self.cache
    }
    fn invalidate_cache(&mut self) {
        self.cache = None;
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
        dict.insert(k.clone(), Rc::new(RefCell::new(c)));

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
        dict.insert(k1.clone(), Rc::new(RefCell::new(c1)));

        let c2 = Cut::new(15, 50.0, 100.0);
        let k2 = String::from("cut2");
        dict.insert(k2.clone(), Rc::new(RefCell::new(c2)));

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
