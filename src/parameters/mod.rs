use std::collections::HashMap;
///
/// Parameter definitions describe parameters that can be histogramed
/// in some way.   Parameters have names, is and optional metadata:
///
///  *   low - suggested low limit for histogram axes on that parameter.
///  *   high - suggested high limit for histogram axes on that parameter.
///  *   bins - suggested number of bins for histogram axes on that parameter.
///  *   units - units of measure of the parameter.
///  *   description - new from SpecTcl a full text description of what the
///     parameter means.
///  
/// In addition to praameters and dict that can be used to look them up (std::map),
/// We need to provide the same high performance source of histograming and
/// validation/invalidation used by SpecTcl.  This implies a vector which,
/// as event are analyzed is sized to the size of the largest id containing generation
/// and value pairs. and a dope vector that contains the indices of the set parameters
/// for the current event.
///
/// Finally, since the incoming data may have different paramter indices than our
/// parameters with like names, we'll provide for the ability to make a mapping
/// between one set of ids and another.
///
/// Rust is not so good with global data so we'll really allow for several parameter
/// spaces, events and mapping vectors but the main might normally only
/// actually create one of these to pass to the appropriate targets.
///
use std::fmt;
///
/// This is what a parameter looks like:
///
#[derive(Debug, PartialEq, Clone)]
pub struct Parameter {
    name: String,
    id: u32,
    low: Option<f64>,
    high: Option<f64>,
    bins: Option<u32>,
    description: Option<String>,
}

impl Parameter {
    /// Creation only requires a name and id.  We leave to the outside world how
    /// ids are allocated -- for now.
    ///
    pub fn new(name: &str, id: u32) -> Parameter {
        Parameter {
            name: String::from(name),
            id: id,
            low: None,
            high: None,
            bins: None,
            description: None,
        }
    }
    /// Set histogram axis suggested limits:

    pub fn set_limits(&mut self, l: f64, h: f64) -> &mut Self {
        self.low = Some(l);
        self.high = Some(h);
        self
    }
    /// Set histogram suggsted bins:

    pub fn set_bins(&mut self, b: u32) -> &mut Self {
        self.bins = Some(b);
        self
    }
    /// Set a description for the parameter.
    pub fn set_description(&mut self, d: &str) -> &mut Self {
        self.description = Some(String::from(d));
        self
    }

    /// The name:
    pub fn get_name(&self) -> String {
        self.name.clone()
    }
    /// The id:
    pub fn get_id(&self) -> u32 {
        self.id
    }
    /// Get histogram axis suggested limits.
    /// In the return tuple, .0 is low, and .1 is high.
    pub fn get_limits(&self) -> (Option<f64>, Option<f64>) {
        (self.low, self.high)
    }
    /// Get histogram axis suggested binning.

    pub fn get_bins(&self) -> Option<u32> {
        self.bins
    }
    /// Get histogram description

    pub fn get_description(&self) -> Option<String> {
        match &self.description {
            Some(s) => Some(s.clone()),
            None => None,
        }
    }
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let low = if let Some(l) = self.low {
            format!("{}", l)
        } else {
            String::from("--")
        };

        let high = if let Some(h) = self.high {
            format!("{}", h)
        } else {
            String::from("--")
        };

        let bins = if let Some(b) = self.bins {
            format!("{}", b)
        } else {
            String::from("--")
        };

        let descr = if let Some(d) = &self.description {
            d.clone()
        } else {
            String::from("-None-")
        };
        write!(
            f,
            "ID: {} Name: {} low: {} high {} bins {} Description {}",
            self.id, self.name, low, high, bins, descr
        )
    }
}
///
/// ParameterInfo is the information about a parameter that's held per event:
/// In addition to the parameter definiton itself, this stuct holds:
///
/// value - the value assigned to the parameter in the last event for which there was
///         that parameter.
/// generation - The number of the event (numbered from 1) in which this event was
/// last given a value (Provides for O(1) invalidation).
///
#[derive(Clone, Debug, PartialEq)]
pub struct ParameterInfo {
    parameter: Parameter, // Simplest to clone the parameter definition here.
    value: f64,
    generation: u64,
}
impl ParameterInfo {
    /// Create a new Parameter info - the value and generation get
    /// initialized to some default thing.
    pub fn new(name: &str, id: u32) -> ParameterInfo {
        ParameterInfo {
            parameter: Parameter::new(name, id),
            value: 0.0,
            generation: 0,
        }
    }
    ///
    /// Set the value of the parameter for some generation gen
    /// the return value is the id of the
    pub fn set(&mut self, v: f64, gen: u64) -> u32 {
        self.value = v;
        self.generation = gen;
        self.parameter.get_id()
    }
    ///
    /// Getting the value depends on the generation matching:
    ///
    pub fn get(&self, gen: u64) -> Option<f64> {
        if gen == self.generation {
            Some(self.value)
        } else {
            None
        }
    }
    fn get_parameter(&self) -> &Parameter {
        &self.parameter
    }
    fn get_parameter_mut(&mut self) -> &mut Parameter {
        &mut self.parameter
    }
}

#[cfg(test)]
mod parameters_test {
    use super::*;

    #[test]
    fn new_1() {
        let p = Parameter::new("test", 1);
        assert_eq!(
            Parameter {
                name: String::from("test"),
                id: 1,
                low: None,
                high: None,
                bins: None,
                description: None
            },
            p
        );
    }
    #[test]
    fn set_1() {
        let mut p = Parameter::new("test", 1);
        p.set_limits(-1.0, 1.0);
        assert_eq!(
            Parameter {
                name: String::from("test"),
                id: 1,
                low: Some(-1.0),
                high: Some(1.0),
                bins: None,
                description: None
            },
            p
        );
    }
    #[test]
    fn set_2() {
        let mut p = Parameter::new("test", 1);
        p.set_bins(128);
        assert_eq!(
            Parameter {
                name: String::from("test"),
                id: 1,
                low: None,
                high: None,
                bins: Some(128),
                description: None
            },
            p
        );
    }
    #[test]
    fn set_3() {
        let mut p = Parameter::new("test", 1);
        p.set_description("Test parameter");
        assert_eq!(
            Parameter {
                name: String::from("test"),
                id: 1,
                low: None,
                high: None,
                bins: None,
                description: Some(String::from("Test parameter"))
            },
            p
        );
    }
    #[test]
    fn set_4() {
        let mut p = Parameter::new("test", 1);
        p.set_limits(-1.0, 1.0)
            .set_bins(128)
            .set_description("Test parameter");

        assert_eq!(
            Parameter {
                name: String::from("test"),
                id: 1,
                low: Some(-1.0),
                high: Some(1.0),
                bins: Some(128),
                description: Some(String::from("Test parameter"))
            },
            p
        );
    }
    #[test]
    fn get_1() {
        let mut p = Parameter::new("test", 1);
        let r1 = p.get_limits();
        assert_eq!((None, None), r1);
        p.set_limits(-1.0, 1.0)
            .set_bins(128)
            .set_description("Test parameter");
        let r1 = p.get_limits();
        assert_eq!((Some(-1.0), Some(1.0)), r1);
    }
    #[test]
    fn get_2() {
        let mut p = Parameter::new("test", 1);
        let r1 = p.get_bins();
        assert_eq!(None, r1);
        p.set_limits(-1.0, 1.0)
            .set_bins(128)
            .set_description("Test parameter");
        let r1 = p.get_bins();
        assert_eq!(Some(128), r1);
    }
    #[test]
    fn get_3() {
        let mut p = Parameter::new("test", 1);
        let r1 = p.get_description();
        assert_eq!(None, r1);
        p.set_limits(-1.0, 1.0)
            .set_bins(128)
            .set_description("Test parameter");
        let r1 = p.get_description();
        assert_eq!(Some(String::from("Test parameter")), r1);
    }
    #[test]
    fn get_4() {
        let p = Parameter::new("test", 1);
        assert_eq!(String::from("test"), p.get_name());
    }
    #[test]
    fn get_5() {
        let p = Parameter::new("test", 1);
        assert_eq!(1, p.get_id());
    }
}
#[cfg(test)]
mod pinfo_tests {
    use super::*;

    #[test]
    fn new_1() {
        let pi = ParameterInfo::new("test", 1);
        assert_eq!(
            ParameterInfo {
                parameter: Parameter {
                    name: String::from("test"),
                    id: 1,
                    low: None,
                    high: None,
                    bins: None,
                    description: None,
                },
                value: 0.0,
                generation: 0
            },
            pi
        );
    }
    #[test]
    fn set_1() {
        let mut pi = ParameterInfo::new("test", 1);
        pi.set(1.234, 1);
        assert_eq!(1.234, pi.value);
        assert_eq!(1, pi.generation);
    }
    #[test]
    fn get_1() {
        let mut pi = ParameterInfo::new("test", 1);
        pi.set(1.2345, 1);
        let v = pi.get(1); // should be ok:
        assert!(v.is_some());
        assert_eq!(1.2345, v.unwrap());
    }
    #[test]
    fn get_2() {
        let mut pi = ParameterInfo::new("test", 1);
        pi.set(1.2345, 1);
        assert!(pi.get(2).is_none()); // mismatch generations.
    }
    #[test]
    fn getp_1() {
        let pi = ParameterInfo::new("test", 1);
        assert_eq!(String::from("test"), pi.get_parameter().get_name());
        assert_eq!(1, pi.get_parameter().get_id());
    }
    #[test]
    fn getp_2() {
        let mut pi = ParameterInfo::new("test", 1);
        pi.get_parameter_mut().set_limits(-1.0, 1.0);
        assert_eq!((Some(-1.0), Some(1.0)), pi.get_parameter().get_limits());
    }
}
