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
use ndhistogram::axis::*;
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
    histogram: H2DContainer,
    parameters: Vec<ParameterPair>,
}
impl Spectrum for TwodSum {
    fn check_gate(&mut self, e: &FlatEvent) -> bool {
        self.applied_gate.check(e)
    }
    fn increment(&mut self, e: &FlatEvent) {
        let mut histogram = self.histogram.borrow_mut();
        for pair in self.parameters.iter() {
            let xid = pair.x_id;
            let yid = pair.y_id;
            let x = e[xid];
            let y = e[yid];
            if x.is_some() && y.is_some() {
                let x = x.unwrap();
                let y = y.unwrap();
                histogram.fill(&(x, y));
            }
        }
    }
    fn get_name(&self) -> String {
        self.name.clone()
    }
    fn get_type(&self) -> String {
        String::from("2DSum")
    }
    fn get_xparams(&self) -> Vec<String> {
        let mut result = Vec::<String>::new();
        for n in self.parameters.iter() {
            result.push(n.x_name.clone());
        }
        result
    }
    fn get_yparams(&self) -> Vec<String> {
        let mut result = Vec::<String>::new();
        for n in self.parameters.iter() {
            result.push(n.y_name.clone());
        }
        result
    }
    fn get_xaxis(&self) -> Option<(f64, f64, u32)> {
        let x = self.histogram.borrow().axes().as_tuple().0.clone();
        Some((*x.low(), *x.high(), x.num_bins() as u32))
    }
    fn get_yaxis(&self) -> Option<(f64, f64, u32)> {
        let y = self.histogram.borrow().axes().as_tuple().1.clone();
        Some((*y.low(), *y.high(), y.num_bins() as u32))
    }
    fn get_gate(&self) -> Option<String> {
        if let Some(g) = self.applied_gate.gate.clone() {
            Some(g.condition_name)
        } else {
            None
        }
    }
    fn gate(&mut self, name: &str, dict: &ConditionDictionary) -> Result<(), String> {
        self.applied_gate.set_gate(name, dict)
    }
    fn ungate(&mut self) {
        self.applied_gate.ungate()
    }
    fn get_histogram_1d(&self) -> Option<H1DContainer> {
        None
    }
    fn get_histogram_2d(&self) -> Option<H2DContainer> {
        Some(Rc::clone(&self.histogram))
    }
    fn clear(&mut self) {
        for c in self.histogram.borrow_mut().iter_mut() {
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
            histogram: Rc::new(RefCell::new(ndhistogram!(
                axis::Uniform::new(x_bins.unwrap() as usize, x_low.unwrap(), x_high.unwrap()),
                axis::Uniform::new(y_bins.unwrap() as usize, y_low.unwrap(), y_high.unwrap());
                Sum
            ))),
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
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameter");

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

        assert_eq!(2, spec.histogram.borrow().axes().num_dim());
        let x = spec.histogram.borrow().axes().as_tuple().0.clone();
        let y = spec.histogram.borrow().axes().as_tuple().1.clone();

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
    #[test]
    fn new_2() {
        // we should be able to override the x/y axis definitions:

        // First make some parameters -- we'll make 5 x and 5 y params
        // named xparam.n and yparam.n all with 0-1024/512 axis specs:

        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameter");

            let px = pd.lookup_mut(&xname).expect("Failed to find xname");
            px.set_limits(0.0, 1024.0);
            px.set_bins(512);

            let py = pd.lookup_mut(&yname).expect("Failed to find yname");
            py.set_limits(0.0, 1024.0);
            py.set_bins(512);
        }

        // try to make the spectrum.
        let result = TwodSum::new(
            "test",
            params,
            &pd,
            Some(-1024.0),
            Some(256.0),
            Some(1024),
            Some(-512.0),
            Some(128.0),
            Some(256),
        );
        assert!(result.is_ok());
        let spec = result.unwrap();

        assert_eq!(2, spec.histogram.borrow().axes().num_dim());
        let x = spec.histogram.borrow().axes().as_tuple().0.clone();
        let y = spec.histogram.borrow().axes().as_tuple().1.clone();

        assert_eq!(-1024.0, *x.low());
        assert_eq!(256.0, *x.high());
        assert_eq!(1024 + 2, x.num_bins());

        assert_eq!(-512.0, *y.low());
        assert_eq!(128.0, *y.high());
        assert_eq!(256 + 2, y.num_bins());
    }
    #[test]
    fn new_3() {
        // Ok to make a spectrum when there are no default axis
        // defs but we provide them:

        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameters");
        }

        // try to make the spectrum.
        let result = TwodSum::new(
            "test",
            params,
            &pd,
            Some(-1024.0),
            Some(256.0),
            Some(1024),
            Some(-512.0),
            Some(128.0),
            Some(256),
        );
        assert!(result.is_ok());
        let spec = result.unwrap();

        assert_eq!(2, spec.histogram.borrow().axes().num_dim());
        let x = spec.histogram.borrow().axes().as_tuple().0.clone();
        let y = spec.histogram.borrow().axes().as_tuple().1.clone();

        assert_eq!(-1024.0, *x.low());
        assert_eq!(256.0, *x.high());
        assert_eq!(1024 + 2, x.num_bins());

        assert_eq!(-512.0, *y.low());
        assert_eq!(128.0, *y.high());
        assert_eq!(256 + 2, y.num_bins());
    }
    // Tests where new fails:

    #[test]
    fn new_4() {
        // Parameter not found.
        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameters");
        }
        params.push((String::from("x"), String::from("y")));

        let result = TwodSum::new(
            "test",
            params,
            &pd,
            Some(-1024.0),
            Some(256.0),
            Some(1024),
            Some(-512.0),
            Some(128.0),
            Some(256),
        );
        assert!(result.is_err());
    }
    #[test]
    fn new_5() {
        // Can't default x low:

        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameters");
        }

        let result = TwodSum::new(
            "test",
            params,
            &pd,
            None,
            Some(256.0),
            Some(1024),
            Some(-512.0),
            Some(128.0),
            Some(256),
        );
        assert!(result.is_err());
    }
    #[test]
    fn new_6() {
        // Can't default xhigh

        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameters");
        }

        let result = TwodSum::new(
            "test",
            params,
            &pd,
            Some(0.0),
            None,
            Some(1024),
            Some(-512.0),
            Some(128.0),
            Some(256),
        );
        assert!(result.is_err());
    }
    #[test]
    fn new_7() {
        // Can't default xbins:

        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameters");
        }

        let result = TwodSum::new(
            "test",
            params,
            &pd,
            Some(0.0),
            Some(1024.0),
            None,
            Some(-512.0),
            Some(128.0),
            Some(256),
        );
        assert!(result.is_err());
    }
    #[test]
    fn new_8() {
        // Can't default y low:

        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameters");
        }

        let result = TwodSum::new(
            "test",
            params,
            &pd,
            Some(0.0),
            Some(1024.0),
            Some(512),
            None,
            Some(128.0),
            Some(256),
        );
        assert!(result.is_err());
    }
    #[test]
    fn new_9() {
        // Can't default yhigh

        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameters");
        }

        let result = TwodSum::new(
            "test",
            params,
            &pd,
            Some(0.0),
            Some(1024.0),
            Some(512),
            Some(0.0),
            None,
            Some(256),
        );
        assert!(result.is_err());
    }
    #[test]
    fn new_10() {
        // can'd default ybins

        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameters");
        }

        let result = TwodSum::new(
            "test",
            params,
            &pd,
            Some(0.0),
            Some(1024.0),
            Some(512),
            Some(0.0),
            Some(1024.0),
            None,
        );
        assert!(result.is_err());
    }
    // Subsequent tests check increments.
    // We don't check overflows because we assume ndhistogram
    // works.  We check:
    //   -  All increments on ungated spectra work.
    //   -  All increments on spectrum gated with True work.
    //   -  All increments on spectrum gated with False don't happen.
    //   -  Events with only some parameters are correctly handled.

    #[test]
    fn incr_1() {
        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameters");
        }

        let mut spec = TwodSum::new(
            "test",
            params,
            &pd,
            Some(0.0),
            Some(1024.0),
            Some(512),
            Some(0.0),
            Some(1024.0),
            Some(512),
        )
        .expect("Failed to create the spectrum");

        // Make an event that has sutff for all x and y parameters:
        // Increments will like on an x=y line:

        let mut fe = FlatEvent::new();
        let mut e = Event::new();

        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);

            let px = pd.lookup(&xname).unwrap().get_id();
            let py = pd.lookup(&yname).unwrap().get_id();

            e.push(EventParameter::new(px, i as f64 * 10.0));
            e.push(EventParameter::new(py, i as f64 * 10.0));
        }
        fe.load_event(&e);

        spec.handle_event(&fe);

        // should be increments on 0,0, 10,0, 20,20, 30,30, 40,40

        for i in 0..5 {
            let xy = i as f64 * 10.0;
            let v = spec
                .histogram
                .borrow()
                .value(&(xy, xy))
                .expect("Value should exist")
                .clone();

            assert_eq!(1.0, v.get());
        }
    }
    #[test]
    fn incr_2() {
        // increment with True gate applied:

        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameters");
        }

        let mut spec = TwodSum::new(
            "test",
            params,
            &pd,
            Some(0.0),
            Some(1024.0),
            Some(512),
            Some(0.0),
            Some(1024.0),
            Some(512),
        )
        .expect("Failed to create the spectrum");

        // Gate the spectrum:

        let mut cd = ConditionDictionary::new();
        cd.insert(
            String::from("true"),
            Rc::new(RefCell::new(Box::new(True {}))),
        );

        spec.gate("true", &cd).expect("Unable to gate spectrum");

        // Make an event that has sutff for all x and y parameters:
        // Increments will like on an x=y line:

        let mut fe = FlatEvent::new();
        let mut e = Event::new();

        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);

            let px = pd.lookup(&xname).unwrap().get_id();
            let py = pd.lookup(&yname).unwrap().get_id();

            e.push(EventParameter::new(px, i as f64 * 10.0));
            e.push(EventParameter::new(py, i as f64 * 10.0));
        }
        fe.load_event(&e);

        spec.handle_event(&fe);

        // should be increments on 0,0, 10,0, 20,20, 30,30, 40,40

        for i in 0..5 {
            let xy = i as f64 * 10.0;
            let v = spec
                .histogram
                .borrow()
                .value(&(xy, xy))
                .expect("Value should exist")
                .clone();

            assert_eq!(1.0, v.get());
        }
    }
    #[test]
    fn incr_3() {
        // Increment with false gate applied does not happen:

        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameters");
        }

        let mut spec = TwodSum::new(
            "test",
            params,
            &pd,
            Some(0.0),
            Some(1024.0),
            Some(512),
            Some(0.0),
            Some(1024.0),
            Some(512),
        )
        .expect("Failed to create the spectrum");

        // Gate the spectrum:

        let mut cd = ConditionDictionary::new();
        cd.insert(
            String::from("false"),
            Rc::new(RefCell::new(Box::new(False {}))),
        );

        spec.gate("false", &cd).expect("Unable to gate spectrum");

        // Make an event that has sutff for all x and y parameters:
        // Increments will like on an x=y line:

        let mut fe = FlatEvent::new();
        let mut e = Event::new();

        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);

            let px = pd.lookup(&xname).unwrap().get_id();
            let py = pd.lookup(&yname).unwrap().get_id();

            e.push(EventParameter::new(px, i as f64 * 10.0));
            e.push(EventParameter::new(py, i as f64 * 10.0));
        }
        fe.load_event(&e);

        spec.handle_event(&fe);

        // the entire histogram.borrow(). should still be clear:

        for chan in spec.histogram.borrow_mut().iter() {
            assert_eq!(0.0, chan.value.get());
        }
    }
    // Now only set some of the parameter pairs in the event:

    #[test]
    fn incr_4() {
        // Some parameter pairs are completely missing:

        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameters");
        }

        let mut spec = TwodSum::new(
            "test",
            params,
            &pd,
            Some(0.0),
            Some(1024.0),
            Some(512),
            Some(0.0),
            Some(1024.0),
            Some(512),
        )
        .expect("Failed to create the spectrum");

        // Make an event that has sutff for all x and y parameters:
        // Increments will like on an x=y line:

        let mut fe = FlatEvent::new();
        let mut e = Event::new();

        for i in 0..5 {
            if i % 2 == 0 {
                // only even parameters are set:
                let xname = format!("xparam.{}", i);
                let yname = format!("yparam.{}", i);

                let px = pd.lookup(&xname).unwrap().get_id();
                let py = pd.lookup(&yname).unwrap().get_id();

                e.push(EventParameter::new(px, i as f64 * 10.0));
                e.push(EventParameter::new(py, i as f64 * 10.0));
            }
        }
        fe.load_event(&e);

        spec.handle_event(&fe);

        for i in 0..5 {
            let xy = i as f64 * 10.0;
            let v = spec
                .histogram
                .borrow()
                .value(&(xy, xy))
                .expect("Value should exist")
                .clone();
            let expected_value = if i % 2 == 0 { 1.0 } else { 0.0 };

            assert_eq!(expected_value, v.get());
        }
    }
    #[test]
    fn incr_5() {
        // Some pairs have the x parameter set but not the y:

        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameters");
        }

        let mut spec = TwodSum::new(
            "test",
            params,
            &pd,
            Some(0.0),
            Some(1024.0),
            Some(512),
            Some(0.0),
            Some(1024.0),
            Some(512),
        )
        .expect("Failed to create the spectrum");

        // Make an event that has sutff for all x and y parameters:
        // Increments will like on an x=y line:

        let mut fe = FlatEvent::new();
        let mut e = Event::new();

        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);

            let px = pd.lookup(&xname).unwrap().get_id();
            let py = pd.lookup(&yname).unwrap().get_id();

            e.push(EventParameter::new(px, i as f64 * 10.0));
            // Only even ones have the y parameter:
            if i % 2 == 0 {
                e.push(EventParameter::new(py, i as f64 * 10.0));
            }
        }
        fe.load_event(&e);

        spec.handle_event(&fe);

        for i in 0..5 {
            let xy = i as f64 * 10.0;
            let v = spec
                .histogram
                .borrow()
                .value(&(xy, xy))
                .expect("Value should exist")
                .clone();
            let expected_value = if i % 2 == 0 { 1.0 } else { 0.0 };

            assert_eq!(expected_value, v.get());
        }
        // there are only 3 non zeros in the histogram 0,0,

        let mut sum = 0;
        for chan in spec.histogram.borrow().iter() {
            if chan.value.get() != 0.0 {
                sum += 1;
            }
        }
        assert_eq!(3, sum);
    }
    #[test]
    fn incr_6() {
        // Same as above but only the y parameter is present for some:

        let mut pd = ParameterDictionary::new();
        let mut params = XYParameters::new();
        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);
            params.push((xname.clone(), yname.clone()));
            pd.add(&xname).expect("Could not add x parameter");
            pd.add(&yname).expect("Could not add y parameters");
        }

        let mut spec = TwodSum::new(
            "test",
            params,
            &pd,
            Some(0.0),
            Some(1024.0),
            Some(512),
            Some(0.0),
            Some(1024.0),
            Some(512),
        )
        .expect("Failed to create the spectrum");

        // Make an event that has sutff for all x and y parameters:
        // Increments will like on an x=y line:

        let mut fe = FlatEvent::new();
        let mut e = Event::new();

        for i in 0..5 {
            let xname = format!("xparam.{}", i);
            let yname = format!("yparam.{}", i);

            let px = pd.lookup(&xname).unwrap().get_id();
            let py = pd.lookup(&yname).unwrap().get_id();

            e.push(EventParameter::new(py, i as f64 * 10.0));
            // Only even ones have the y parameter:
            if i % 2 == 0 {
                e.push(EventParameter::new(px, i as f64 * 10.0));
            }
        }
        fe.load_event(&e);

        spec.handle_event(&fe);

        for i in 0..5 {
            let xy = i as f64 * 10.0;
            let v = spec
                .histogram
                .borrow()
                .value(&(xy, xy))
                .expect("Value should exist")
                .clone();
            let expected_value = if i % 2 == 0 { 1.0 } else { 0.0 };

            assert_eq!(expected_value, v.get());
        }
        // there are only 3 non zeros in the histogram 0,0,

        let mut sum = 0;
        for chan in spec.histogram.borrow().iter() {
            if chan.value.get() != 0.0 {
                sum += 1;
            }
        }
        assert_eq!(3, sum);
    }
}
