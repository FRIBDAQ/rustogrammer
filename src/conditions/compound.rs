//!  Compound conditions are conditions that are defined on other conditions.
//!  These are called *dependent conditions*.  Dependent conditions can be
//!  Either primitive conditions, like cuts or contours, or they can be other
//!  Compund conditions.  This nesting allows one to build up arbitrarily
//!  Complex gate logic.  The compund conditions that we define are:
//!  
//!  *  Not - takes a single condition and returns its boolean negation.
//!  *  And - takes an arbitrary number of dependent conditions and
//! requires all of them to be true.
//!  *  Or - takes an arbitrary number of dependent conditions and
//! Requires at least one to be true.
//!
//!  Compound conditions make not promise that their dependent gates are
//!  Fully evaluated.  It's perfectly fair game (and is the case) that
//!  Short circuit logic can be used to reduce the number of conditions
//!  that need to be evaluated until the truth or falsity of the
//!  main condition is known. All of these gate cache as well which
//!  further reduces the number of gate evaluation needed if a
//!  compound condition is applied to more than one target.
//!
//!  And and Or conditions depend on a cache and a vector of dependent conditions,
//!  This is abstracted out as a ConditionList which has the cached value and
//!  the dependent vector of conditions.
//!
//! ### Note
//!   conditions are stored as weak references to the underlying
//!  condition.  If upgrading the condition gives a None, the underlying
//!  condition has been deleted out from underneath us and is treated
//!  as returning false.
//!
use super::*;
use crate::parameters::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;

///
/// Not conditions take a single dependent condition and
/// return the boolean inverse of that condition when checked.
/// Sinced the computational complexity of the dependent condition
/// cannot be bounded (due to nested compound conditions), this is
/// a caching condition.
///
pub struct Not {
    dependent: Weak<RefCell<dyn Condition>>,
    cache: Option<bool>,
}

impl Not {
    pub fn new(cond: &Container) -> Not {
        Not {
            dependent: Rc::downgrade(&cond.clone()),
            cache: None,
        }
    }
}
impl Condition for Not {
    fn evaluate(&mut self, event: &FlatEvent) -> bool {
        let result = if let Some(d) = self.dependent.upgrade() {
            !d.borrow_mut().check(&event)
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
        if let Some(d) = self.dependent.upgrade() {
            d.borrow_mut().invalidate_cache();
        }
    }
}
//  The ConditionList provides common structure and code for
//  maintainng an arbitrary list of dependent conditions.
//  A cache variable is also associated with the list so that
//  common caching logic can be used.
//  this struct need not be exposed to the world:
struct ConditionList {
    dependent_conditions: Vec<Weak<RefCell<dyn Condition>>>,
    cache: Option<bool>,
}
impl ConditionList {
    pub fn new() -> ConditionList {
        ConditionList {
            dependent_conditions: Vec::new(),
            cache: None,
        }
    }
    pub fn add_condition(&mut self, c: &Container) -> &mut Self {
        self.dependent_conditions.push(Rc::downgrade(&c.clone()));

        self
    }
    // Clears the dependent conditions:
    //
    pub fn clear(&mut self) -> &mut Self {
        self.dependent_conditions.clear();
        self
    }
}

/// And conditions evaluate their condition list and require
/// all dependent conditions to be true if the condition
/// is to be true.  
///
/// * This is a caching condition.
/// * The evaluation is short circuited - that is if any
/// evaluation returns false, no more dependent conditions are
/// evaluated and all are evaluated as false.
///
pub struct And {
    dependencies: ConditionList,
}

impl And {
    pub fn new() -> And {
        And {
            dependencies: ConditionList::new(),
        }
    }
    pub fn add_condition(&mut self, c: &Container) -> &mut Self {
        self.dependencies.add_condition(c);
        self
    }
    pub fn clear(&mut self) -> &mut Self {
        self.dependencies.clear();
        self
    }
}
impl Condition for And {
    fn evaluate(&mut self, event: &FlatEvent) -> bool {
        let mut result = true; // Failed gates will contradict this.

        if let Some(c) = self.dependencies.cache {
            return c;
        } else {
            for d in &self.dependencies.dependent_conditions {
                if let Some(g) = d.upgrade() {
                    if !g.borrow_mut().check(&event) {
                        result = false;
                        break;
                    }
                } else {
                    result = false;
                    break;
                }
            }
        }

        self.dependencies.cache = Some(result);
        result
    }
    fn get_cached_value(&self) -> Option<bool> {
        self.dependencies.cache
    }
    // must invalidate both our cache and the
    // caches of our dependencies:
    //
    fn invalidate_cache(&mut self) {
        self.dependencies.cache = None;
        for d in &self.dependencies.dependent_conditions {
            if let Some(r) = d.upgrade() {
                r.borrow_mut().invalidate_cache();
            }
        }
    }
}
///  Or is a compound condition that only requires that
///  one of its dependent gates is true for an event.
///
pub struct Or {
    dependencies: ConditionList,
}
impl Or {
    pub fn new() -> Or {
        Or {
            dependencies: ConditionList::new(),
        }
    }
    pub fn add_condition(&mut self, c: &Container) -> &mut Self {
        self.dependencies.add_condition(c);
        self
    }
    pub fn clear(&mut self) -> &mut Self {
        self.dependencies.clear();
        self
    }
}

impl Condition for Or {
    fn evaluate(&mut self, event: &FlatEvent) -> bool {
        let mut result = true;
        let mut falses = 0;
        if let Some(b) = self.dependencies.cache {
            return b;
        } else {
            for d in &self.dependencies.dependent_conditions {
                if let Some(c) = d.upgrade() {
                    if c.borrow_mut().check(&event) {
                        break;
                    } else {
                        falses += 1;
                    }
                }
            }
            // If all are false -- and there are dependencies:

            let l = self.dependencies.dependent_conditions.len();
            if (falses == l) && (l > 0) {
                result = false;
            }
        }
        self.dependencies.cache = Some(result);
        result
    }
    fn get_cached_value(&self) -> Option<bool> {
        self.dependencies.cache
    }
    fn invalidate_cache(&mut self) {
        self.dependencies.cache = None;
        for d in &self.dependencies.dependent_conditions {
            if let Some(r) = d.upgrade() {
                r.borrow_mut().invalidate_cache();
            }
        }
    }
}
#[cfg(test)]
mod not_tests {
    use super::*;

    #[test]
    fn new_1() {
        let cut = Cut::new(1, -100.0, 100.0);
        let c: Container = Rc::new(RefCell::new(cut));
        let not = Not::new(&c);
        assert_eq!(None, not.cache);

        let cut2 = not.dependent.upgrade();
        assert!(cut2.is_some());
        assert!(c.as_ptr() == cut2.unwrap().as_ptr()); // same slice.
    }
    #[test]
    fn check_1() {
        let t = True {};
        let c: Container = Rc::new(RefCell::new(t));
        let mut not = Not::new(&c);
        let e = FlatEvent::new();
        assert!(!not.check(&e));
    }
    #[test]
    fn check_2() {
        let f = False {};
        let c: Container = Rc::new(RefCell::new(f));
        let mut not = Not::new(&c);
        let e = FlatEvent::new();
        assert!(not.check(&e));
    }
}
#[cfg(test)]
mod and_tests {
    use super::cut::*;
    use super::*;
    #[test]
    fn new_1() {
        let a = And::new();
        assert!(a.dependencies.cache.is_none());
        assert_eq!(0, a.dependencies.dependent_conditions.len())
    }
    #[test]
    fn add_1() {
        // Add a T gate

        let t = True {};
        let c: Container = Rc::new(RefCell::new(t));
        let mut a = And::new();
        a.add_condition(&c);
        assert_eq!(1, a.dependencies.dependent_conditions.len());
    }
    #[test]
    fn add_2() {
        // add a t and an f gate:

        let t = True {};
        let f = False {};
        let ct: Container = Rc::new(RefCell::new(t));
        let cf: Container = Rc::new(RefCell::new(f));

        let mut a = And::new();
        a.add_condition(&ct);
        a.add_condition(&cf);

        assert_eq!(2, a.dependencies.dependent_conditions.len());
    }
    #[test]
    fn clear_1() {
        let t = True {};
        let f = False {};
        let ct: Container = Rc::new(RefCell::new(t));
        let cf: Container = Rc::new(RefCell::new(f));

        let mut a = And::new();
        a.add_condition(&ct);
        a.add_condition(&cf);

        assert_eq!(2, a.dependencies.dependent_conditions.len());

        // Clear will get rid of them all:

        a.clear();
        assert_eq!(0, a.dependencies.dependent_conditions.len());
    }
    #[test]
    fn check_1() {
        // And gates are true if there are no entries:

        let mut a = And::new();
        let e = FlatEvent::new();
        assert!(a.check(&e));

        // Cache is set:

        assert!(a.dependencies.cache.is_some());
        assert!(a.dependencies.cache.unwrap());
    }
    #[test]
    fn check_2() {
        // a single T gate is true:

        let mut a = And::new();
        let e = FlatEvent::new();
        let t = True {};
        let c: Container = Rc::new(RefCell::new(t));
        a.add_condition(&c);
        assert!(a.check(&e));
    }
    #[test]
    fn check_3() {
        // 2 trues is also true:

        let mut a = And::new();
        let e = FlatEvent::new();
        let t1 = True {};
        let c1: Container = Rc::new(RefCell::new(t1));
        let t2 = True {};
        let c2: Container = Rc::new(RefCell::new(t2));

        a.add_condition(&c1);
        a.add_condition(&c2);
        assert!(a.check(&e));
    }
    #[test]
    fn check_4() {
        // T and F in that order is false:

        let mut a = And::new();
        let e = FlatEvent::new();
        let t1 = True {};
        let c1: Container = Rc::new(RefCell::new(t1));
        let f2 = False {};
        let c2: Container = Rc::new(RefCell::new(f2));

        a.add_condition(&c1);
        a.add_condition(&c2);
        assert!(!a.check(&e));

        // Cache is set:

        assert!(a.dependencies.cache.is_some());
        assert!(!a.dependencies.cache.unwrap());
    }
    #[test]
    fn check_5() {
        // short circuit evaluation works:
        // We'll use a slice and its cache to check:

        let mut a = And::new();
        let e = FlatEvent::new();
        let f = False {};
        let c1: Container = Rc::new(RefCell::new(f));
        let s = Cut::new(1, 100.0, 200.0);
        let c2: Container = Rc::new(RefCell::new(s));

        a.add_condition(&c1);
        a.add_condition(&c2);

        assert!(!a.check(&e));
        assert!(c2.borrow().get_cached_value().is_none());
    }
}
#[cfg(test)]
mod or_tests {
    use super::*;

    #[test]
    fn new_1() {
        let o = Or::new();
        assert!(o.dependencies.cache.is_none());
        assert_eq!(0, o.dependencies.dependent_conditions.len());
    }
    #[test]
    fn add_1() {
        let mut o = Or::new();
        let t = True {};
        let c: Container = Rc::new(RefCell::new(t));

        o.add_condition(&c);

        assert_eq!(1, o.dependencies.dependent_conditions.len());
    }
    #[test]
    fn add_2() {
        let mut o = Or::new();
        let t = True {};
        let ct: Container = Rc::new(RefCell::new(t));
        let f = False {};
        let cf: Container = Rc::new(RefCell::new(f));

        o.add_condition(&ct);
        o.add_condition(&cf);

        assert_eq!(2, o.dependencies.dependent_conditions.len());
    }
    #[test]
    fn clear_1() {
        let mut o = Or::new();
        let t = True {};
        let ct: Container = Rc::new(RefCell::new(t));
        let f = False {};
        let cf: Container = Rc::new(RefCell::new(f));

        o.add_condition(&ct);
        o.add_condition(&cf);

        assert_eq!(2, o.dependencies.dependent_conditions.len());

        o.clear();
        assert_eq!(0, o.dependencies.dependent_conditions.len());
    }
    #[test]
    fn check_1() {
        // empty gate is true:

        let mut o = Or::new();
        let e = FlatEvent::new();
        assert!(o.check(&e));
    }
    #[test]
    fn check_2() {
        //Single true gate gives true:

        let mut o = Or::new();
        let e = FlatEvent::new();
        let t = True {};
        let c: Container = Rc::new(RefCell::new(t));
        o.add_condition(&c);

        assert!(o.check(&e));
    }
    #[test]
    fn check_3() {
        // t,f is true

        let mut o = Or::new();
        let e = FlatEvent::new();
        let t = True {};
        let f = False {};
        let ct: Container = Rc::new(RefCell::new(t));
        let cf: Container = Rc::new(RefCell::new(f));

        o.add_condition(&ct);
        o.add_condition(&cf);

        assert!(o.check(&e));
    }
    #[test]
    fn check_4() {
        // f,t is true:

        let mut o = Or::new();
        let e = FlatEvent::new();
        let t = True {};
        let f = False {};
        let ct: Container = Rc::new(RefCell::new(t));
        let cf: Container = Rc::new(RefCell::new(f));

        o.add_condition(&cf);
        o.add_condition(&ct);

        assert!(o.check(&e));
    }
    #[test]
    fn check_5() {
        // f is false:
        let mut o = Or::new();
        let e = FlatEvent::new();
        let f = False {};
        let cf: Container = Rc::new(RefCell::new(f));

        o.add_condition(&cf);

        assert!(!o.check(&e));
    }
    #[test]
    fn check_6() {
        // ff is false:

        let mut o = Or::new();
        let e = FlatEvent::new();
        let f1 = False {};
        let cf1: Container = Rc::new(RefCell::new(f1));
        let f2 = False {};
        let cf2: Container = Rc::new(RefCell::new(f2));

        o.add_condition(&cf1);
        o.add_condition(&cf2);

        assert!(!o.check(&e));
    }
}
