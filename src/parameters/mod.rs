use std::collections::hash_map::{Iter, IterMut};
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
/// NOte that since there isn't any user code in this histogrammer (parameters are
/// created externally), we don't need any complex validation/invalidation
/// support. Each event comes in as a set of id/value pairs but
///  since the incoming data may have different paramter indices than our
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
/// ParameterDictionary is the structure that allows
/// parmameters to be defined unique ids within the dictionary
/// and looked up by name later on.  Note that much of the stuff that's
/// needed It just consists of a hashmap of parameters indexed by their
/// names and a counter that's used to assign parameter ids to new
/// parameters as they are created.
///
/// Paramters are permanen, in the sense that once created they can
/// never be destroyed.
///
struct ParameterDictionary {
    next_id: u32,
    dictionary: HashMap<String, Parameter>,
}
impl ParameterDictionary {
    pub fn new() -> ParameterDictionary {
        ParameterDictionary {
            next_id: 1,
            dictionary: HashMap::<String, Parameter>::new(),
        }
    }
    ///
    /// Attempt to add a new named parameter to the dictioary.
    /// There are really two cases:
    ///
    /// * The parameter does not exist, it is added and Ok<&mut ref is > returned.
    /// * The parameter exists Err("Duplicate parameter") is returned
    ///
    pub fn add(&mut self, name: &str) -> Result<String, String> {
        if self.dictionary.contains_key(name) {
            Err(String::from("Duplicate parameter"))
        } else {
            self.dictionary
                .insert(String::from(name), Parameter::new(name, self.next_id));
            self.next_id += 1;
            Ok(String::from(name))
        }
    }
    ///
    /// Lookup a parameter definition in the dictionary.
    ///
    pub fn lookup(&self, name: &str) -> Option<&Parameter> {
        self.dictionary.get(name)
    }
    /// Lookup a parameter for modification:

    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Parameter> {
        self.dictionary.get_mut(name)
    }
    /// Get an iterator over the map:

    pub fn iter(&self) -> Iter<'_, String, Parameter> {
        self.dictionary.iter()
    }
    /// Get an iterator that allows modification of the parameters:

    pub fn iter_mut(&mut self) -> IterMut<'_, String, Parameter> {
        self.dictionary.iter_mut()
    }
}
///
/// Display trait:
///    We can display the dictionary as, for the most part,
///    a list of the parameters in it.
///
impl fmt::Display for ParameterDictionary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Next id: {}\n", self.next_id).unwrap();
        for (_, v) in self.iter() {
            write!(f, "{}\n", v).unwrap();
        }
        write!(f, "")
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
mod pdict_tests {
    use super::*;

    #[test]
    fn new_1() {
        let d = ParameterDictionary::new();
        assert_eq!(1, d.next_id);
        assert_eq!(0, d.dictionary.len());
    }
    #[test]
    fn add_1() {
        let mut d = ParameterDictionary::new();
        let result = d.add("parameter");
        assert!(result.is_ok());
        assert_eq!(String::from("parameter"), result.unwrap());
    }
    #[test]
    fn add_2() {
        let mut d = ParameterDictionary::new();
        d.add("parameter").unwrap(); // will work.
        assert!(d.add("parameter").is_err());
    }
    #[test]
    fn add_3() {
        let mut d = ParameterDictionary::new();
        d.add("parameter1").unwrap();
        assert!(d.add("parameter2").is_ok());
        assert!(d.dictionary.contains_key(&String::from("parameter1")));
        assert!(d.dictionary.contains_key(&String::from("parameter2")));
    }
}
