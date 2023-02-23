//! Multi2d spectra are what SpecTcl called gamma d spectra.
//! They are defined on at least two parameters.  For each
//! event, all valid parameter pairs are incremented.  
//!
//! For example, suppose a Multi2d spectrunm defined on
//! parameters with ids 1,2,3,4 and an event with parameters
//! 1,3,4,7,8 set, this Multi2d spectrum will
//! increment for parameter pairs (1,3), (1,4), (2,4).
//! Where the first parameter id is the X parameter and the
//! second the Y parameter.
//!
//! Axis defaults are handled as for 2-d spectra with the default
//! Guaranteed to be a square spectrum.  The defaults for either x/y
//! can be overidden at construction time, however.
//!  
//! As with any spectrum a condition can be applied as a gate on
//! incrementing the spectrum.  Gated spectra will only be incremented
//! for events for which the gating condition is true.

use super::*;
use ndhistogram::value::Sum;

pub struct Multi2d {
    applied_gate: SpectrumGate,
    name: String,
    histogram: Hist2D<axis::Uniform, axis::Uniform, Sum>,
    param_names: Vec<String>,
    param_ids: Vec<u32>,
}

// The spectrum trait must be implemented to support
// dynamic dispatch of gating and incrementing:

impl Spectrum for Multi2d {
    fn check_gate(&mut self, e: &FlatEvent) -> bool {
        self.applied_gate.check(e)
    }
    // The method of iterating over the parameter ids to get the ordered pairs
    // out for computation comes from:
    // https://stackoverflow.com/questions/66386013/how-to-iterate-over-two-elements-in-a-collection-stepping-by-one-using-iterator
    // with the inpect dropped out in favor of a loop to make things a
    // bit clearer(?)
    // The key is that .zip makes an iterator over the outer iterator
    // which is from 0..n while zip iterates over the 1..n and that outer
    // iterator.
    fn increment(&mut self, e: &FlatEvent) {
        for (a, b) in self.param_ids.iter().zip(self.param_ids.iter().skip(1)) {
            let x = e[*a];
            let y = e[*b];
            if x.is_some() && y.is_some() {
                let x = x.unwrap();
                let y = y.unwrap();
                self.histogram.fill(&(x, y));
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
impl Multi2d {
    /// Create a multi2d spectrum.
    /// *   name - name of the spectrum.
    /// *   params - parameters vector of parameter names for the spectrum parameters.
    /// *   pdict - parameter dictionary in which the parameter properties
    /// can be looked up
    /// *   xlow   - Override for default X axis low limit.
    /// *   xhigh  - Override for default X axis high limit.
    /// *   xbins  - Override for default X axis binning.
    /// *   ylow   - Override for default Y axis low limit.
    /// *   yhigh  - Override for default Y axis high limit.
    /// *   ybins  - Override for default Y axis binning.
    ///
    pub fn new(
        name: &str,
        params: Vec<String>,
        pdict: &ParameterDictionary,
        xlow: Option<f64>,
        xhigh: Option<f64>,
        xbins: Option<u32>,
        ylow: Option<f64>,
        yhigh: Option<f64>,
        ybins: Option<u32>,
    ) -> Result<Multi2d, String> {
        // maintain the defaults here:

        // Note we can set square defaults after the loop over the
        // parameters:

        let mut x_low = None;
        let mut x_high = None;
        let mut x_bins = None;

        let mut pnames = Vec::<String>::new();
        let mut pids = Vec::<u32>::new();

        for pname in params {
            if let Some(p) = pdict.lookup(&pname) {
                let lim = p.get_limits();
                x_low = optmin(x_low, lim.0);
                x_high = optmax(x_high, lim.1);
                x_bins = optmax(x_bins, p.get_bins());

                pnames.push(p.get_name());
                pids.push(p.get_id());
            } else {
                return Err(format!("Parameter {} cannot be found", pname));
            }
        }
        // Make the defaults square and override as desired:

        let mut y_low = x_low;
        let mut y_high = x_high;
        let mut y_bins = x_bins;

        // Override the defaults and make surea all axis definitions
        // have, in fact, been made:

        if let Some(xl) = xlow {
            x_low = Some(xl);
        }
        if let Some(xh) = xhigh {
            x_high = Some(xh);
        }
        if let Some(xb) = xbins {
            x_bins = Some(xb);
        }

        if let Some(yl) = ylow {
            y_low = Some(yl);
        }
        if let Some(yh) = yhigh {
            y_high = Some(yh);
        }
        if let Some(yb) = ybins {
            y_bins = Some(yb);
        }

        if x_low.is_none() {
            return Err(String::from("X axis low limit cannot be defaulted"));
        }
        if x_high.is_none() {
            return Err(String::from("X axis high limit cannot be defaulted"));
        }
        if x_bins.is_none() {
            return Err(String::from("X axis binning cannot be defaulted"));
        }
        if y_low.is_none() {
            return Err(String::from("Y axis low limit cannot be defaulted"));
        }
        if y_high.is_none() {
            return Err(String::from("Y axis high limit cannot be defaulted"));
        }
        if y_bins.is_none() {
            return Err(String::from("Y axis binning cannot be defaulted"));
        }
        Ok(Multi2d {
            applied_gate: SpectrumGate::new(),
            name: String::from(name),
            histogram: ndhistogram!(
                axis::Uniform::new(
                    x_bins.unwrap() as usize, x_low.unwrap(), x_high.unwrap()
                ),
                axis::Uniform::new(
                    y_bins.unwrap() as usize, y_low.unwrap(), y_high.unwrap()
                );
                Sum
            ),
            param_names: pnames,
            param_ids: pids,
        })
    }
}
