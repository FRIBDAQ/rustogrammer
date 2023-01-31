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
    use crate::conditions::*;

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
}
