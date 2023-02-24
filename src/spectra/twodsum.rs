//!  A two d sum spectrum is exactly that. A spectrum that would
//!  be the sum of several two d spectra with the same gate applied.
//!
//!  The spectrum is defined over an arbitrary set of x/y parameter
//!  pairs.  If the applied gate is satisfied, the spectrum is incremented
//!  for each of those pairs which have a value in the event.
//!
//!  Suppose, for example, the spectrum is defined on the following (x,y)
//!  pairs: (1,2), (3,4), (5,6).  And an event has contents:
//!   1=100, 2=100, 3=500, 5=600, 6=700.  The channels for:
//!  (100,200), and (600,700) will be incremented (4 is not present
//!  so no increment for the pair (3,4) will be done).
//!
//!  As with all spectra a gate can be applied to the spectrum.
//!  If one is, increments only occur if the evaluation of that
//!  gate returns true for the event.
//!
//!  Note that spectrum axis defaults are handled the same way as for
//!  2d spectrum, however the min/maxes are done over all x parameters for
//!  the x axis and all y parameters for the y axis.
//!
use super::*;
use ndhistogram::value::Sum;

// 2d sum spectra are defined on x/y parameter pairs.
// here's a convenient container for one used internally:

#[derive(Clone)]
struct ParameterPair {
    x_name: String,
    x_id: u32,

    y_name: String,
    y_id: u32,
}
/// This type is used to define xy parameter pairs to the TwodSum
/// creational:

pub type XYParameter = (String, String);
pub type XYParameters = Vec<XYParameter>;

///
/// This is the struct that defines a TwodSum spectrum.
/// It should be created for each 2-d sum spectrum desired.
/// See the implementation and TwodSum::new for a creational operation.
///
pub struct TwodSum {
    applied_gate: SpectrumGate,
    name: String,
    histogram: Hist2D<axis::Uniform, axis::Uniform, Sum>,
    parameters: Vec<ParameterPair>,
}
impl Spectrum for TwodSum {
    fn check_gate(&mut self, e: &FlatEvent) -> bool {
        self.applied_gate.check(e)
    }
    fn increment(&mut self, e: &FlatEvent) {
        for pair in self.parameters.iter() {
            let xid = pair.x_id;
            let yid = pair.y_id;
            let x = e[xid];
            let y = e[yid];
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
impl TwodSum {
    pub fn new(
        name: &str,
        parameters: XYParameters,
        pdict: &ParameterDictionary,
        xlow: Option<f64>,
        xhigh: Option<f64>,
        xbins: Option<u32>,
        ylow: Option<f64>,
        yhigh: Option<f64>,
        ybins: Option<u32>,
    ) -> Result<TwodSum, String> {
        let mut x_low = None;
        let mut x_high = None;
        let mut x_bins = None;
        let mut y_low = None;
        let mut y_high = None;
        let mut y_bins = None;

        let mut params = Vec::<ParameterPair>::new();

        // Ensure all parameters are defined an figure out
        // axis defaults:

        for param_pair in parameters {
            let px = pdict.lookup(&param_pair.0);
            if px.is_none() {
                return Err(format!("X parameter {} does not exist", param_pair.0));
            }
            let py = pdict.lookup(&param_pair.1);
            if py.is_none() {
                return Err(format!("Y parameter {} does not exist", param_pair.1));
            }
            let px = px.unwrap();
            let py = py.unwrap();
            // Save the pair for the spectrum:

            params.push(ParameterPair {
                x_name: param_pair.0,
                x_id: px.get_id(),
                y_name: param_pair.1,
                y_id: py.get_id(),
            });
            // Update default axis defs:
            let xlims = px.get_limits();
            let ylims = py.get_limits();

            x_low = optmin(x_low, xlims.0);
            x_high = optmax(x_high, xlims.1);
            x_bins = optmax(x_bins, px.get_bins());

            y_low = optmin(y_low, ylims.0);
            y_high = optmax(y_high, ylims.1);
            y_bins = optmax(y_bins, py.get_bins());
        }
        // fold in any overrides for axis definitions passed in by caller:

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

        // No axis definition element can be None - that would mean
        // no default could be established and none provided by
        // the user:

        if x_low.is_none() {
            return Err(String::from(
                "There is no default value for the X axis low limit",
            ));
        }
        if x_high.is_none() {
            return Err(String::from(
                "There is no default value for the X axis high limit",
            ));
        }
        if x_bins.is_none() {
            return Err(String::from(
                "There is no default value for the X axis binning",
            ));
        }
        if y_low.is_none() {
            return Err(String::from(
                "There is no default value for the Y axis low limit",
            ));
        }
        if y_high.is_none() {
            return Err(String::from(
                "There is no default value for the Y axis high limit",
            ));
        }
        if y_bins.is_none() {
            return Err(String::from(
                "There is no default value for the Y axis binning",
            ));
        }
        // We know enough to build the struct:
        Ok(TwodSum {
            applied_gate: SpectrumGate::new(),
            name: String::from(name),
            histogram: ndhistogram!(
                axis::Uniform::new(x_bins.unwrap() as usize, x_low.unwrap(), x_high.unwrap()),
                axis::Uniform::new(y_bins.unwrap() as usize, y_low.unwrap(), y_high.unwrap());
                Sum
            ),
            parameters: params,
        })
    }
}
#[cfg(test)]
mod twodsum_tests {
    use super::*;
    use ndhistogram::axis::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    #[test]
    fn new_1() {
        // Simple success:

        // First make some parameters -- we'll make 5 x and 5 y params
        // named xparam.n and yparam.n all with 0-1024/512 axis specs:

        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname);
            pd.add(&yname);

            let px = pd.lookup_mut(&xname).expect("Failed to find xname");
            px.set_limits(0.0, 1024.0);
            px.set_bins(512);

            let py = pd.lookup_mut(&yname).expect("Failed to find yname");
            py.set_limits(0.0, 1024.0);
            py.set_bins(512);
        }

        // try to make the spectrum.
        let result = TwodSum::new("test", params, &pd, None, None, None, None, None, None);
        assert!(result.is_ok());
        let spec = result.unwrap();
        assert!(spec.applied_gate.gate.is_none());
        assert_eq!("test", spec.name);

        assert_eq!(2, spec.histogram.axes().num_dim());
        let x = spec.histogram.axes().as_tuple().0.clone();
        let y = spec.histogram.axes().as_tuple().1.clone();

        assert_eq!(0.0, *x.low());
        assert_eq!(1024.0, *x.high());
        assert_eq!(512 + 2, x.num_bins());

        assert_eq!(0.0, *y.low());
        assert_eq!(1024.0, *y.high());
        assert_eq!(512 + 2, y.num_bins());

        assert_eq!(5, spec.parameters.len());
        for (i, p) in spec.parameters.iter().enumerate() {
            let xname = format!("xparam.{}", i); 
            let yname = format!("yparam.{}", i);
            assert_eq!(xname, *p.x_name);
            assert_eq!(yname, *p.y_name);

            let px = pd.lookup(&xname).expect("Unable to lookup x");
            assert_eq!(px.get_id(), p.x_id);
            let py = pd.lookup(&yname).expect("Unable to lookup y");
            assert_eq!(py.get_id(), p.y_id);
        }

        


    }
}
