use super::*;
use ndhistogram::value::Sum;
///
/// Summary spectra are useful in visualizing the status of
/// large detector arrays.  The best way to think of a summary
/// spetrum is as an array of one-d spectra where each  of those
/// spectra is a vertical channel strip in a 2-d spectrum.
/// That is the x coordinate of the spectrum represents an element
/// of the detector array and the y coordinate the 1-d spectrum
/// of whatever in that element.
///
/// Summary spectra, like all spectra can be gated since it
/// implements the Spectrum trait.
///
/// Creating a spectrum requires:
///
/// *   An arbitrary set of existing parameters
/// *   The range of the y axis.
/// *   The number of bins on the y axis.
///
/// Note that if these are defaulted the following selections are made:
/// *  y-min is the minimum of the default minima for the parameters.
/// *  y-max is the maximum of the default maxima for the parameters.
/// *  bins  ist the maximum of the default bins for the parameters.
///
/// In the case where any of these is not provided a default for
/// _all_ parameters, the spectrum cannot be created.
///
pub struct Summary {
    applied_gate: SpectrumGate,
    name: String,
    histogram: Hist2D<axis::Uniform, axis::Uniform, Sum>,

    // Parameter information:
    param_names: Vec<String>,
    param_ids: Vec<u32>,
}
// The trait implementation is relatively straightforward:

impl Spectrum for Summary {
    fn check_gate(&mut self, e: &FlatEvent) -> bool {
        self.applied_gate.check(e)
    }
    // Increment the param_ids index gives the x axis value
    // while its value the parameter id.
    // Increment for _all_ valid ids in the event:
    //
    fn increment(&mut self, e: &FlatEvent) {
        for (x, id) in self.param_ids.iter().enumerate() {
            if let Some(y) = e[*id] {
                self.histogram.fill(&(x as f64, y));
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
impl Summary {
    /// This local function takes the minimum of two values which
    /// may not be defined:
    /// * If both v1/v2 are None, the result is None,
    /// * If either v1/v2 are None but not both, the result is the Non_None one.
    /// * If both v1/v2 are Some, the results is Some the minimum of
    /// the two values.
    fn min<T: PartialOrd>(v1: Option<T>, v2: Option<T>) -> Option<T> {
        None
    }
    /// Same as min but uses max of v1/v2
    fn max<T: PartialOrd>(v1: Option<T>, v2: Option<T>) -> Option<T> {
        None
    }
    /// Generate the spectrum.
    /// This fails if:
    /// *    Any of the parameters is not defined.
    /// *    Any axis spec is defaulted but none of the parameters
    /// provide a default for it.  See the comments for Summmary
    /// for how these are handled.
    ///
    pub fn new(
        name: &str,
        params: Vec<String>,
        pdict: &ParameterDictionary,
        ylow: Option<f64>,
        yhigh: Option<f64>,
        bins: Option<u32>,
    ) -> Result<Summary, String> {
        let mut low = ylow;
        let mut high = yhigh;
        let mut nbins = bins;

        let mut param_ids = Vec::<u32>::new();
        let mut param_names = Vec::<String>::new();
        for name in params {
            if let Some(p) = pdict.lookup(&name) {
                param_names.push(name);
                param_ids.push(p.get_id());
                let lims = p.get_limits();
                let b = p.get_bins();
                low = Self::min(low, lims.0);
                high = Self::max(high, lims.1);
                nbins = Self::max(nbins, b);
            } else {
                return Err(format!("Parameter {} does not exist", name));
            }
        }
        // if any of the Y axis stuff are still None, that's a failure:

        if let None = low {
            return Err(String::from(
                "None of the parameters can default the axis low limit",
            ));
        }
        if let None = high {
            return Err(String::from(
                "None of the parameters can default the axis high limit",
            ));
        }
        if let None = nbins {
            return Err(String::from(
                "None of the parameters can default the bin count",
            ));
        }
        // Unwrap the axis limits.
        let low = low.unwrap();
        let high = high.unwrap();
        let nbins = nbins.unwrap();

        // create/return the spectrum:

        Ok(Summary {
            applied_gate: SpectrumGate::new(),
            name: String::from(name),
            histogram: ndhistogram!(
                axis::Uniform::new(param_names.len(), 0.0, param_names.len() as f64),
                axis::Uniform::new(nbins as usize, low,  high);
                Sum
            ),
            param_names: param_names.clone(),
            param_ids: param_ids.clone(),
        })
    }
}

#[cfg(test)]
mod summary_tests {
    use super::*;
    use std::cell::RefCell; // Needed in gating
    use std::rc::Rc; // Needed in gating.

    #[test]
    fn dummy() {}
}
