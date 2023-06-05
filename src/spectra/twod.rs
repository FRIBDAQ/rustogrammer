use super::*;
use ndhistogram::value::Sum;
/// Twod is a simple two dimensional spectrum.
/// It has two parameters x and y and a SpectrumGate
/// The underlying histogram is a Hist2D<axis::Uniform, axis::Uniform, Sum>
/// which makes the channels have f64 values.
/// Member data:
///
/// *    applied_gate - The gate which can conditionalize incrementing the
/// spectrum.
/// *    name - Name of the spectrum.  The spectrum is intered in the
/// spectrum dictionary under this name.
/// *    histogram -The underlying histogram.
/// *    x_name, x_id - the name and Id of the X axis parameter.
/// *    y_name, y_id - the name and Id of the Y axis parameter.
///
pub struct Twod {
    applied_gate: SpectrumGate,
    name: String,
    histogram: H2DContainer,

    // Parameter information:
    x_name: String,
    x_id: u32,
    y_name: String,
    y_id: u32,
}
impl Spectrum for Twod {
    fn check_gate(&mut self, e: &FlatEvent) -> bool {
        self.applied_gate.check(e)
    }
    fn increment(&mut self, e: &FlatEvent) {
        let x = e[self.x_id];
        let y = e[self.y_id];

        // We need both parameters in the event:

        if x.is_some() && y.is_some() {
            self.histogram.borrow_mut().fill(&(x.unwrap(), y.unwrap()));
        }
    }
    fn required_parameter(&self) -> Option<u32> {
        Some(self.x_id)
    }
    fn get_name(&self) -> String {
        self.name.clone()
    }
    fn get_type(&self) -> String {
        String::from("2D")
    }
    fn get_xparams(&self) -> Vec<String> {
        vec![self.x_name.clone()]
    }
    fn get_yparams(&self) -> Vec<String> {
        vec![self.y_name.clone()]
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
impl Twod {
    pub fn new(
        spectrum_name: &str,
        xname: &str,
        yname: &str,
        pdict: &ParameterDictionary,
        xlow: Option<f64>,
        xhigh: Option<f64>,
        xbins: Option<u32>,
        ylow: Option<f64>,
        yhigh: Option<f64>,
        ybins: Option<u32>,
    ) -> Result<Twod, String> {
        let xpar = pdict.lookup(xname);
        let ypar = pdict.lookup(yname);

        if xpar.is_some() && ypar.is_some() {
            let xpar = xpar.unwrap(); // Get the definitions:
            let ypar = ypar.unwrap();

            let xaxis_info = axis_limits(&xpar, xlow, xhigh, xbins)?;
            let yaxis_info = axis_limits(&ypar, ylow, yhigh, ybins)?;

            Ok(Twod {
                applied_gate: SpectrumGate::new(),
                name: String::from(spectrum_name),
                histogram: Rc::new(RefCell::new(ndhistogram!(
                    axis::Uniform::new(xaxis_info.2 as usize, xaxis_info.0, xaxis_info.1),
                    axis::Uniform::new(yaxis_info.2 as usize, yaxis_info.0, yaxis_info.1)
                    ; Sum
                ))),
                x_name: String::from(xname),
                x_id: xpar.get_id(),
                y_name: String::from(yname),
                y_id: ypar.get_id(),
            })
        } else {
            Err(format!(
                "One of the parameters {}, {} are not defined",
                xname, yname
            ))
        }
    }
}
#[cfg(test)]
mod twod_tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn new_1() {
        // Everything is ok:

        // Make x/y parameters

        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();
        let xinfo = {
            let px = pdict.lookup_mut("x").unwrap();
            px.set_limits(0.0, 1023.0);
            px.set_bins(512);
            px.set_description("Just some x parameter");
            (px.get_id(), px.get_name(), px.get_limits(), px.get_bins())
        };
        let yinfo = {
            let py = pdict.lookup_mut("y").unwrap();
            py.set_limits(-1.0, 1.0);
            py.set_bins(100);
            py.set_description("Just some y parameter");
            (py.get_id(), py.get_name(), py.get_limits(), py.get_bins())
        };

        let result = Twod::new(
            "2d", "x", "y", &pdict, None, None, None, // Default xaxis.
            None, None, None, // default y axis.
        );
        assert!(result.is_ok());
        let spec = result.unwrap();
        assert!(spec.applied_gate.gate.is_none());
        assert_eq!(String::from("2d"), spec.name);
        assert_eq!(xinfo.1, spec.x_name);
        assert_eq!(xinfo.0, spec.x_id);
        assert_eq!(yinfo.1, spec.y_name);
        assert_eq!(yinfo.0, spec.y_id);

        // make sure we made a 2d histogram.borrow(). with the correct axes:

        assert_eq!(2, spec.histogram.borrow().axes().num_dim());
        let xaxis = spec.histogram.borrow().axes().as_tuple().0.clone();
        let yaxis = spec.histogram.borrow().axes().as_tuple().1.clone();

        assert_eq!(xinfo.2 .0.unwrap(), *xaxis.low());
        assert_eq!(xinfo.2 .1.unwrap(), *xaxis.high());
        assert_eq!(xinfo.3.unwrap() as usize + 2, xaxis.num_bins()); // 512 + under/overflow.
        assert_eq!(yinfo.2 .0.unwrap(), *yaxis.low());
        assert_eq!(yinfo.2 .1.unwrap(), *yaxis.high());
        assert_eq!(yinfo.3.unwrap() as usize + 2, yaxis.num_bins()); // 100 + under/overflow bins.
    }
    #[test]
    fn new_2() {
        // Can override x axis definitions:

        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();
        let xinfo = {
            let px = pdict.lookup_mut("x").unwrap();
            px.set_limits(0.0, 1023.0);
            px.set_bins(512);
            px.set_description("Just some x parameter");
            (px.get_id(), px.get_name(), px.get_limits(), px.get_bins())
        };
        let yinfo = {
            let py = pdict.lookup_mut("y").unwrap();
            py.set_limits(-1.0, 1.0);
            py.set_bins(100);
            py.set_description("Just some y parameter");
            (py.get_id(), py.get_name(), py.get_limits(), py.get_bins())
        };

        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            None,
            None,
            None, // Accept y axis defaults.
        );
        assert!(result.is_ok());
        let spec = result.unwrap();

        assert!(spec.applied_gate.gate.is_none());
        assert_eq!(String::from("2d"), spec.name);
        assert_eq!(xinfo.1, spec.x_name);
        assert_eq!(xinfo.0, spec.x_id);
        assert_eq!(yinfo.1, spec.y_name);
        assert_eq!(yinfo.0, spec.y_id);

        // make sure we made a 2d histogram.borrow(). with the correct axes:

        assert_eq!(2, spec.histogram.borrow().axes().num_dim());
        let xaxis = spec.histogram.borrow().axes().as_tuple().0.clone();
        let yaxis = spec.histogram.borrow().axes().as_tuple().1.clone();

        assert_eq!(-512.0, *xaxis.low());
        assert_eq!(512.0, *xaxis.high());
        assert_eq!(256 + 2, xaxis.num_bins()); // 512 + under/overflow.
        assert_eq!(yinfo.2 .0.unwrap(), *yaxis.low());
        assert_eq!(yinfo.2 .1.unwrap(), *yaxis.high());
        assert_eq!(yinfo.3.unwrap() as usize + 2, yaxis.num_bins()); // 100 + under/overflow bins.
    }
    #[test]
    fn new_3() {
        // Can override y axis definitions:

        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();
        let xinfo = {
            let px = pdict.lookup_mut("x").unwrap();
            px.set_limits(0.0, 1023.0);
            px.set_bins(512);
            px.set_description("Just some x parameter");
            (px.get_id(), px.get_name(), px.get_limits(), px.get_bins())
        };
        let yinfo = {
            let py = pdict.lookup_mut("y").unwrap();
            py.set_limits(-1.0, 1.0);
            py.set_bins(100);
            py.set_description("Just some y parameter");
            (py.get_id(), py.get_name(), py.get_limits(), py.get_bins())
        };

        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            Some(-2.0),
            Some(2.0),
            Some(200), // Override Y axis defaults.
        );
        assert!(result.is_ok());
        let spec = result.unwrap();

        assert!(spec.applied_gate.gate.is_none());
        assert_eq!(String::from("2d"), spec.name);
        assert_eq!(xinfo.1, spec.x_name);
        assert_eq!(xinfo.0, spec.x_id);
        assert_eq!(yinfo.1, spec.y_name);
        assert_eq!(yinfo.0, spec.y_id);

        // make sure we made a 2d histogram with the correct axes:

        assert_eq!(2, spec.histogram.borrow().axes().num_dim());
        let xaxis = spec.histogram.borrow().axes().as_tuple().0.clone();
        let yaxis = spec.histogram.borrow().axes().as_tuple().1.clone();

        assert_eq!(-512.0, *xaxis.low());
        assert_eq!(512.0, *xaxis.high());
        assert_eq!(256 + 2, xaxis.num_bins()); // 512 + under/overflow.
        assert_eq!(-2.0, *yaxis.low());
        assert_eq!(2.0, *yaxis.high());
        assert_eq!(200 as usize + 2, yaxis.num_bins()); // 100 + under/overflow bins.
    }
    #[test]
    fn new_4() {
        // Must provide x axis information (no defaults) but don't failure:

        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();

        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            None,
            Some(1024.0),
            Some(512),
            Some(-1.0),
            Some(-1.0),
            Some(100),
        );
        assert!(result.is_err());

        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(0.0),
            None,
            Some(512),
            Some(-1.0),
            Some(-1.0),
            Some(100),
        );
        assert!(result.is_err());
        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(0.0),
            Some(1024.0),
            None,
            Some(-1.0),
            Some(-1.0),
            Some(100),
        );
        assert!(result.is_err());
    }
    #[test]
    fn new_5() {
        // Can't default y axis but attempts to:

        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();
        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            None,
            Some(2.0),
            Some(200), // Override Y axis defaults.
        );
        assert!(result.is_err());
        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            Some(-2.0),
            None,
            Some(200), // Override Y axis defaults.
        );
        assert!(result.is_err());
        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            Some(-2.0),
            Some(2.0),
            None, // Override Y axis defaults.
        );
        assert!(result.is_err());

        // Fully specified _is_ ok though:

        let result = Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            Some(-2.0),
            Some(2.0),
            Some(200), // Override Y axis defaults.
        );
        assert!(result.is_ok());
    }
    #[test]
    fn new_6() {
        // No such x parameter:

        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();
        let result = Twod::new(
            "2d",
            "xx",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            Some(-2.0),
            Some(2.0),
            Some(200), // Override Y axis defaults.
        );
        assert!(result.is_err());
    }
    #[test]
    fn new_7() {
        // NO such y parameter:

        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();
        let result = Twod::new(
            "2d",
            "x",
            "yy",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            Some(-2.0),
            Some(2.0),
            Some(200), // Override Y axis defaults.
        );
        assert!(result.is_err());
    }
    // The remaining tests test increments.
    // To support them utility function below creates and returns
    // a standard 2d spectrum.  Note that the fact that
    // the parameter dict goes out of scope is unimportant as
    // id's can still be used to pull data from FlatEvent references:

    fn make_test_2d() -> Twod {
        let mut pdict = ParameterDictionary::new();
        pdict.add("x").unwrap();
        pdict.add("y").unwrap();
        Twod::new(
            "2d",
            "x",
            "y",
            &pdict,
            Some(-512.0),
            Some(512.0),
            Some(256), // Overrride X axis defaults
            Some(-2.0),
            Some(2.0),
            Some(200), // Override Y axis defaults.
        )
        .unwrap()
    }
    #[test]
    fn incr_1() {
        // Increment dead center - ungated.

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 0.0),
            EventParameter::new(spec.y_id, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        spec.handle_event(&e);

        let v = spec
            .histogram
            .borrow()
            .value(&(0.0, 0.0))
            .expect("Value not defined")
            .clone();

        assert_eq!(1.0, v.get());
    }
    #[test]
    fn incr_2() {
        // Increment dead center - Gated with T.

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 0.0),
            EventParameter::new(spec.y_id, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        let mut gd = ConditionDictionary::new();
        gd.insert(
            String::from("true"),
            Rc::new(RefCell::new(Box::new(True {}))),
        );
        spec.gate("true", &gd).unwrap();

        spec.handle_event(&e);

        let v = spec
            .histogram
            .borrow()
            .value(&(0.0, 0.0))
            .expect("Value should exist")
            .clone();

        assert_eq!(1.0, v.get());
    }
    #[test]
    fn incr_3() {
        // Incr dead center with gate false -- that'll not increment:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 0.0),
            EventParameter::new(spec.y_id, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        let mut gd = ConditionDictionary::new();
        gd.insert(
            String::from("false"),
            Rc::new(RefCell::new(Box::new(False {}))),
        );
        spec.gate("false", &gd).unwrap();

        spec.handle_event(&e);

        let v = spec
            .histogram
            .borrow()
            .value(&(0.0, 0.0))
            .expect("Value should exist")
            .clone();

        assert_eq!(0.0, v.get());
    }
    #[test]
    fn incr_4() {
        // X parameter not in event:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id + 100, 0.0),
            EventParameter::new(spec.y_id, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        spec.handle_event(&e);

        for c in spec.histogram.borrow().iter() {
            assert_eq!(0.0, c.value.get());
        }
    }
    #[test]
    fn incr_5() {
        // Y parameter not present:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 0.0),
            EventParameter::new(spec.y_id + 100, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        spec.handle_event(&e);

        for c in spec.histogram.borrow().iter() {
            assert_eq!(0.0, c.value.get());
        }
    }
    #[test]
    fn incr_6() {
        // Underflow in x:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, -600.0),
            EventParameter::new(spec.y_id, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        spec.handle_event(&e);

        // Just try to get the undeflow x channel:

        let v = spec
            .histogram
            .borrow()
            .value(&(-512.01, 0.0))
            .expect("Value should exist")
            .clone();

        assert_eq!(1.0, v.get());
    }
    #[test]
    fn incr_7() {
        // overflow in x:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 600.0),
            EventParameter::new(spec.y_id, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        spec.handle_event(&e);

        // Just try to get the undeflow x channel:

        let v = spec
            .histogram
            .borrow()
            .value(&(512.0, 0.0))
            .expect("Value should exist")
            .clone();

        assert_eq!(1.0, v.get());
    }
    #[test]
    fn incr_8() {
        // underflow in y:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 0.0),
            EventParameter::new(spec.y_id, -3.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        spec.handle_event(&e);

        // Just try to get the undeflow x channel:

        let v = spec
            .histogram
            .borrow()
            .value(&(0.0, -2.01))
            .expect("Value should exist")
            .clone();

        assert_eq!(1.0, v.get());
    }
    #[test]
    fn incr_9() {
        // Overflow in y:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 0.0),
            EventParameter::new(spec.y_id, 2.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        spec.handle_event(&e);
    }
    #[test]
    fn incr_10() {
        // Increment 0.0,0.0 100 times and ensure it's got 100 counts:

        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 0.0),
            EventParameter::new(spec.y_id, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        for _ in 0..100 {
            spec.handle_event(&e);
        }

        let v = spec
            .histogram
            .borrow()
            .value(&(0.0, 0.0))
            .expect("Value should exist")
            .clone();

        assert_eq!(100.0, v.get());
    }
    #[test]
    fn clear_1() {
        let mut spec = make_test_2d();
        let event = vec![
            EventParameter::new(spec.x_id, 0.0),
            EventParameter::new(spec.y_id, 0.0),
        ];
        let mut e = FlatEvent::new();
        e.load_event(&event);

        for _ in 0..100 {
            spec.handle_event(&e);
        }

        let v = spec
            .histogram
            .borrow()
            .value(&(0.0, 0.0))
            .expect("Value should exist")
            .clone();

        assert_eq!(100.0, v.get());

        spec.clear();
        let v = spec
            .histogram
            .borrow()
            .value(&(0.0, 0.0))
            .expect("Value should exist")
            .clone();

        assert_eq!(0.0, v.get());
    }
}
