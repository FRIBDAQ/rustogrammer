//!  Multi1d spectra are spectra that are defined on several parameters.
//!  Each parameter in the event that has been given a value increments
//!  the histogram on a common X axis.  In SpecTcl, these spectra were
//!  called Gamma-1d spectra because they were used primarily to accumulate
//!  summed spectra of the detectors in a gamma ray spectrometer array.
//!
//!  Multi1d spectra can have a gate as well which can conditionalize when
//!  the spectrum is incremented.  
//!
//!  Axis defaults are determined in a manner identical to the way
//!  axis defaults for the y axis of a summary spectrum are determined.
//!  
//!  
use super::*;
use ndhistogram::value::Sum;

///
/// *  applied_gate - is the gate that can conditionalize increments.
/// *  name         - is the name of the spectrum.
/// *  histogram    - is the actual histogram object.
/// *  param_names  - are the names of the parameters we're defined on.
/// *  param_ids    - Are the corresponding parameter ids (indices into FlatEvent).
///
pub struct Multi1d {
    applied_gate: SpectrumGate,
    name: String,
    histogram: Hist1D<axis::Uniform, Sum>,
    param_names: Vec<String>,
    param_ids: Vec<u32>,
}
//
impl Spectrum for Multi1d {
    fn check_gate(&mut self, e: &FlatEvent) -> bool {
        self.applied_gate.check(e)
    }
    fn increment(&mut self, e: &FlatEvent) {
        for id in self.param_ids.iter() {
            if let Some(x) = e[*id] {
                self.histogram.fill(&x);
            }
        }
    }
    fn gate(&mut self, name: &str, dict: &ConditionDictionary) -> Result<(), String> {
        self.applied_gate.set_gate(name, dict)
    }
    fn ungate(&mut self) {
        self.applied_gate.ungate()
    }
    fn clear(&mut self) {
        for c in self.histogram.iter_mut() {
            *c.value = Sum::new();
        }
    }
}

impl Multi1d {
    /// Create a spectrum of type Multi1d:
    /// *    name - the name of the new spectrm.
    /// *    params - Vector of the names of parameters we're defined on.
    /// *    pdict  - References the parameter dictionary the parameters are defined in.
    /// *    low   - X axis low limit (or None for default).
    /// *    high  - X axis high limit (or None for default).
    /// *    bins  - X axis bins (or None for default).
    ///
    /// Default low is the smallest of the paramter default lows.
    /// Default high is the largest of the paramete default highs.
    /// Default bins is the largest of the parameter default bins.
    ///
    fn new(
        name: &str,
        params: Vec<String>,
        pdict: &ParameterDictionary,
        low: Option<f64>,
        high: Option<f64>,
        bins: Option<u32>,
    ) -> Result<Multi1d, String> {
        let mut xlow = None;
        let mut xmax = None;
        let mut xbins = None;

        let mut param_names = Vec::<String>::new();
        let mut param_ids = Vec::<u32>::new();

        for name in params {
            if let Some(p) = pdict.lookup(&name) {
                param_names.push(name);
                param_ids.push(p.get_id());
                let lims = p.get_limits();
                let b = p.get_bins();
                xlow = optmin(xlow, lims.0);
                xmax = optmax(xmax, lims.1);
                xbins = optmax(xbins, b);
            } else {
                return Err(format!("Parameter {} does not exist", name));
            }
        }
        // override defaults for axes etc:
        if let Some(l) = low { xlow = Some(l);}
        if let Some(h) = high {xmax = Some(h);}
        if let Some(b) = bins {xbins = Some(b);}

        if let None = xlow {
            return Err(String::from("X axis low limit cannot be defaulted"));
        }
        if let None = xmax {
            return Err(String::from("X axis high limit cannot be defaulted"));
        }
        if let None = xbins {
            return Err(String::from("X axis binning cannot be defaulted"));
        }
        Ok(Multi1d {
            applied_gate : SpectrumGate::new(),
            name : String::from(name), 
            histogram : ndhistogram!(
                axis::Uniform::new(xbins.unwrap() as usize, xlow.unwrap(), xmax.unwrap());
                Sum
            ),
            param_names : param_names,
            param_ids : param_ids
        })

    }
}
