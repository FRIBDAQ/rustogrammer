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
/// Each event comes in as a set of id/value pairs but
///  since the incoming data may have different paramter indices than our
/// parameters with like names, we'll provide for the ability to make a mapping
/// between one set of ids and another.
///
/// Rust is not so good with global data so we'll really allow for several parameter
/// spaces, events and mapping vectors but the main might normally only
/// actually create one of these to pass to the appropriate targets.
///
use std::fmt;
use std::ops::Index;
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
pub struct ParameterDictionary {
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

///  In the histogrammer, events are collections of
/// parameter id/value pairs.

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct EventParameter {
    pub id: u32,
    pub value: f64,
}
impl EventParameter {
    pub fn new(id: u32, value: f64) -> EventParameter {
        EventParameter { id, value }
    }
}
impl fmt::Display for EventParameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "id: {} value: {}", self.id, self.value)
    }
}
/// An event is just a vector of EventParameter s.

type Event = Vec<EventParameter>;

/// ParameterIdMap provides a correspondence between
/// parameter ids in an Event and parameter ids in some dictionary.
/// It can be used to take some input event with a different Id space
/// and map it into an output event with the same parameter space as
/// the dictionary.
/// The input dictionary is used to determine the map while
/// which is an array of output ids indexed by input ids.
///
pub struct ParameterIdMap {
    dict: ParameterDictionary,
    map: Vec<Option<u32>>,
}
impl ParameterIdMap {
    fn get_mapping(&self, input_id: u32) -> Option<u32> {
        let input_id = input_id as usize;
        if input_id <= self.map.len() {
            self.map[input_id]
        } else {
            None
        }
    }

    pub fn new() -> ParameterIdMap {
        ParameterIdMap {
            dict: ParameterDictionary::new(),
            map: Vec::<Option<u32>>::new(),
        }
    }
    /// Rather than wrapping all of the dicts
    /// methods, just expose the dict itself.
    ///
    pub fn get_dict(&self) -> &ParameterDictionary {
        &self.dict
    }
    ///
    /// Expose the dict as a mut reference so that e.g.
    /// parameters can be added.
    ///
    pub fn get_dict_mut(&mut self) -> &mut ParameterDictionary {
        &mut self.dict
    }
    /// Define a mapping for an input id to a named parameter.
    ///  The Ok value is the id this is mapped to.
    /// The Err value is a stringed error message.
    /// Two such are provided:
    ///
    /// *   "No such parameter" - The named parameter is not in the dict.
    /// *    "Duplicate map" - The id is already mapped and the mapping
    /// is different from what's requested.
    /// Note - we don't attempt to detectk many to one mappings, only
    ///   one -to many.
    pub fn map(&mut self, input_id: u32, name: &str) -> Result<u32, String> {
        let input_id: usize = input_id as usize;
        if let Some(p) = self.dict.lookup(name) {
            let mapped_id = p.get_id();
            if self.map.len() < input_id {
                self.map.resize(input_id + 1, None);
            }
            if let Some(outid) = self.map[input_id] {
                if outid != mapped_id {
                    return Err(String::from("Duplicate Map"));
                }
            }
            // map[input_id] is either None or the same so this is ok:

            self.map[input_id] = Some(mapped_id);
            Ok(mapped_id)
        } else {
            Err(String::from("No Such parameter"))
        }
    }
    /// The purpose of the map:
    /// Given an input event produce an output event
    /// that uses the mapped parameters.
    /// any input ids that don't map are removed from the output event.
    ///
    pub fn map_event(&self, ine: &Event) -> Event {
        let mut result = Event::new();

        for p in ine {
            if let Some(id) = self.get_mapping(p.id) {
                result.push(EventParameter::new(id, p.value));
            }
        }

        result
    }
}

///  See FlatEvent below:
///   last_set is the generation that last set this parameter.
///   value is the value it set it to
///
#[derive(Copy, Clone)]
pub struct EventParameterInfo {
    last_set: u64,
    returned_value: Option<f64>,
}
impl EventParameterInfo {
    /// Create a new instance - e.g. when replacing a none with a some.
    ///  
    pub fn new(gen: u64, value: f64) -> EventParameterInfo {
        EventParameterInfo {
            last_set: gen,
            returned_value: None,
        }
    }
    ///
    /// update the value for a given generation
    pub fn set(&mut self, gen: u64, value: f64) {
        self.last_set = gen;
        self.returned_value = Some(value);
    }
    /// fetch the value for a given generation

    pub fn get(&self, gen: u64) -> &Option<f64> {
        if self.last_set == gen {
            &(self.returned_value)
        } else {
            &None
        }
    }
}

/// FlatEvent holds an event whose indices are parameter ids and
/// values.  While the id/value event is good for histograms that
/// are ordered by a required parameter id, there are cases when a
/// flattened version of the event is more appropriate. Specifically,
/// a 2d histogram has other parameters it'll need to validate and fetch
/// it'll need to evaluate its gate which depends on parameters stored
/// as indices and those are most quickly looked up in a flattened
/// struct.  So while the Event looks a lot like the dope vector of an
/// Event in SpecTcl, the FlattenedEvent is the SpecTcl CEvent itself.
/// We use the same generation number method to determine if an
/// entry in the vector is valid...with the added wrinkle that the
/// vector can hold Option<EventParameterInfo> structs so that entries
/// are None if they've _never_ been initialized.
///
struct FlatEvent {
    generation: u64, // Supports O(1) invalidation.
    event: Vec<EventParameterInfo>,
}

impl FlatEvent {
    fn ensure_size(&mut self, required: usize) {
        // Don't allow truncation:

        if required > self.event.len() {
            self.event.resize(required, EventParameterInfo::new(0, 0.0));
        }
    }

    pub fn new() -> FlatEvent {
        FlatEvent {
            generation: 0,
            event: Vec::<EventParameterInfo>::new(),
        }
    }
    /// Given a dope vectored event loads the flattened event
    /// from it.  Note this increments the generation number
    /// this means that you can't load several events into a single
    /// flattened event.
    ///
    pub fn load_event(&mut self, e: &Event) {
        self.generation += 1; // New event
        for p in e {
            let id = p.id as usize;
            self.ensure_size(id + 1);
            self.event[id].set(self.generation, p.value);
        }
    }
    /// Get the value of a parameter in the event for the current
    /// generation.  None if this parameter does not exist or is not set.

    pub fn get_parameter(&self, id: u32) -> &Option<f64> {
        let id = id as usize; // The better to index with:
        if id < self.event.len() {
            &(self.event[id].get(self.generation))
        } else {
            &None
        }
    }
}
/// It's reasonable to use just indexing to get the parameter:
///  This means that for a FlatEvent e; e[i] will give None
/// if parameter i has not been set for the event and 
/// Some(v) where v is the value, if it has been set.
///
impl Index<u32> for FlatEvent {
    type Output = Option<f64>;

    fn index(&self, index: u32) -> &Self::Output {
        self.get_parameter(index)
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
    #[test]
    fn lookup_1() {
        let d = ParameterDictionary::new();
        assert!(d.lookup("nothing").is_none());
    }
    #[test]
    fn lookup_2() {
        let mut d = ParameterDictionary::new();
        d.add("parameter").unwrap();
        let p = d.lookup("parameter");
        assert!(p.is_some());
        let p = p.unwrap();
        assert_eq!(1, p.get_id());
        assert_eq!(String::from("parameter"), p.get_name());
        assert_eq!((None, None), p.get_limits());
        assert!(p.get_bins().is_none());
        assert!(p.get_description().is_none());
    }
    #[test]
    fn lookup_3() {
        let mut d = ParameterDictionary::new();
        d.add("parameter1").unwrap();
        d.add("parameter2").unwrap();
        assert_eq!(
            String::from("parameter1"),
            d.lookup("parameter1").unwrap().get_name()
        );
        assert_eq!(
            String::from("parameter2"),
            d.lookup("parameter2").unwrap().get_name()
        );
        assert_eq!(2, d.lookup("parameter2").unwrap().get_id());
        assert_eq!(1, d.lookup("parameter1").unwrap().get_id());
    }
    #[test]
    fn lookup_mut_1() {
        let mut d = ParameterDictionary::new();
        assert!(d.lookup_mut("nosuch").is_none());
    }
    #[test]
    fn lookup_mut_2() {
        let mut d = ParameterDictionary::new();
        d.add("parameter").unwrap();
        let p = d.lookup_mut("parameter");
        assert!(p.is_some());
        let p = p.unwrap();
        p.set_limits(-1.0, 1.0)
            .set_bins(100)
            .set_description("A parameter");
        let p = d.lookup("parameter").unwrap();

        // This should be the same underlying object as the mutable
        // one so we've modified the metadata:

        assert_eq!((Some(-1.0), Some(1.0)), p.get_limits());
        assert_eq!(Some(100), p.get_bins());
        assert_eq!(Some(String::from("A parameter")), p.get_description());
    }
    #[test]
    fn iter_1() {
        // Non mutating iterator:

        let mut d = ParameterDictionary::new();
        d.add("parameter").unwrap();
        // Only one iteration so:

        for (k, p) in d.iter() {
            assert_eq!(String::from("parameter"), String::from(k));
            assert_eq!(String::from("parameter"), p.get_name());
        }
    }
    #[test]
    fn iter_2() {
        let mut d = ParameterDictionary::new();
        d.add("parameter").unwrap();
        d.add("param2").unwrap();

        for (_, p) in d.iter_mut() {
            p.set_limits(-1.0, 1.0);
        }
        // Both parameter and param 2 have limits now:

        assert_eq!(
            (Some(-1.0), Some(1.0)),
            d.lookup("parameter").unwrap().get_limits()
        );
        assert_eq!(
            (Some(-1.0), Some(1.0)),
            d.lookup("param2").unwrap().get_limits()
        )
    }
}
#[cfg(test)]
mod pevent_test {
    use super::*;

    #[test]
    fn new_1() {
        let e = EventParameter::new(1, 1.2345);
        assert_eq!(
            EventParameter {
                id: 1,
                value: 1.2345
            },
            e
        );
    }
    #[test]
    fn display_1() {
        let e = EventParameter::new(1, 1.2345);
        assert_eq!(String::from("id: 1 value: 1.2345"), format!("{}", e));
    }
}
#[cfg(test)]
mod paramap_test {
    use super::*;
    // Common code used to setup  parameters that can be mapped:
    //
    // |  id    |   Name      |
    // |--------|-------------|
    // |  1     | Parameter1  |
    // |  2     | Parameter2  |
    // |  3     | Parameter3  |
    fn stock_map(map: &mut ParameterIdMap) {
        let dict = map.get_dict_mut();
        dict.add("Parameter1").unwrap();
        dict.add("Parameter2").unwrap();
        dict.add("Parameter3").unwrap();
    }
    // make some default mappings for the parameters created by
    /// stock_map:
    //
    // | in id  | out id |
    // |--------|--------|
    // | 10     |  1     |
    // |  5     |  2     |
    // | 12     |  3     |

    fn make_map(map: &mut ParameterIdMap) {
        map.map(10, "Parameter1").unwrap();
        map.map(5, "Parameter2").unwrap();
        map.map(12, "Parameter3").unwrap();
    }
    #[test]
    fn new_1() {
        let map = ParameterIdMap::new();
        assert_eq!(0, map.dict.dictionary.len());
        assert_eq!(0, map.map.len());
    }
    #[test]
    fn get_1() {
        // Tests both get_dict and get_dict_mut
        let mut map = ParameterIdMap::new();
        let rmap = map.get_dict_mut();
        rmap.add("Parameter1").unwrap();
        rmap.add("parameter2").unwrap();
        let rmap = map.get_dict();
        assert_eq!(2, rmap.dictionary.len());

        assert_eq!(1, rmap.lookup("Parameter1").unwrap().get_id());
        assert_eq!(2, rmap.lookup("parameter2").unwrap().get_id());
        assert_eq!(0, map.map.len());
    }
    #[test]
    fn map_1() {
        let mut map = ParameterIdMap::new();
        stock_map(&mut map);

        // Make a map entry mapping id 10 -> 1:

        let r = map.map(10, "Parameter1");
        assert!(r.is_ok());
        assert_eq!(1, r.unwrap());

        assert!(map.map.len() >= 10);
        assert!(map.map[10].is_some());
        assert_eq!(1, map.map[10].unwrap());

        for (n, m) in map.map.into_iter().enumerate() {
            if n != 10 {
                assert!(m.is_none());
            }
        }
    }
    #[test]
    fn map_2() {
        // full mapping:

        let mut map = ParameterIdMap::new();
        stock_map(&mut map);

        assert!(map.map(10, "Parameter1").is_ok());
        assert!(map.map(5, "Parameter2").is_ok());
        assert!(map.map(12, "Parameter3").is_ok());

        assert_eq!(1, map.map[10].unwrap());
        assert_eq!(2, map.map[5].unwrap());
        assert_eq!(3, map.map[12].unwrap());
    }
    #[test]
    fn map_3() {
        // Duplicate map:

        let mut map = ParameterIdMap::new();
        stock_map(&mut map);

        map.map(10, "Parameter1").expect("Should have worked");
        let r = map.map(10, "Parameter2");
        assert!(r.is_err());
        assert_eq!(String::from("Duplicate Map"), r.unwrap_err());
    }
    #[test]
    fn map_4() {
        // duplicate map is ok if it's the same!

        let mut map = ParameterIdMap::new();
        stock_map(&mut map);

        map.map(10, "Parameter1").expect("Should have worked");
        let r = map.map(10, "Parameter1");
        assert!(r.is_ok());
    }
    #[test]
    fn map_5() {
        // no parameter to map to:

        let mut map = ParameterIdMap::new();
        stock_map(&mut map);

        let r = map.map(10, "Parameter10");
        assert!(r.is_err());
        assert_eq!(String::from("No Such parameter"), r.unwrap_err());
    }
    #[test]
    fn map_evt_1() {
        // Make maps for all parameters in event, check the output
        //  event is correct

        let mut map = ParameterIdMap::new();
        stock_map(&mut map);
        make_map(&mut map);

        let ine: Event = vec![EventParameter::new(10, 1.234)];
        let oute = map.map_event(&ine);

        assert_eq!(1, oute.len());
        assert_eq!(1, oute[0].id);
        assert_eq!(1.234, oute[0].value);
    }
    #[test]
    fn map_evt_2() {
        // Multiple parameters in the in enent all have maps:

        let mut map = ParameterIdMap::new();
        stock_map(&mut map);
        make_map(&mut map);

        let ine: Event = vec![
            EventParameter::new(10, 1.234),
            EventParameter::new(12, 5.5),
            EventParameter::new(5, 5.231),
        ];
        let oute = map.map_event(&ine);
        assert_eq!(3, oute.len());
        assert_eq!(EventParameter::new(1, 1.234), oute[0]);
        assert_eq!(EventParameter::new(3, 5.5), oute[1]);
        assert_eq!(EventParameter::new(2, 5.231), oute[2]);
    }
    #[test]
    fn map_evt_3() {
        // Input parameters without a map should get elided from
        // the output event:

        let mut map = ParameterIdMap::new();
        stock_map(&mut map);
        make_map(&mut map);

        let ine: Event = vec![
            EventParameter::new(10, 1.234),
            EventParameter::new(7, 3.1416), // should vanish
            EventParameter::new(12, 5.5),
            EventParameter::new(5, 5.231),
        ];
        let oute = map.map_event(&ine);
        assert_eq!(3, oute.len());
        assert_eq!(EventParameter::new(1, 1.234), oute[0]);
        assert_eq!(EventParameter::new(3, 5.5), oute[1]);
        assert_eq!(EventParameter::new(2, 5.231), oute[2]);
    }
}
#[cfg(test)]
mod parflatevt_test {
    use super::*;


}