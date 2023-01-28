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

///
/// This is what a parameter looks like:
///
#[derive(Debug, PartialEq, Clone)]
struct Parameter {
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
    pub fn set_description(&mut self, d: &str) -> &mut Self {
        self.description = Some(String::from(d));
        self
    }
}

#[cfg(test)]
mod test_parameters {
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
}
