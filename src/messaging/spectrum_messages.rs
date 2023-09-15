//!  Provides message and reply structures for the message passing
//!  interfaces to spectra in the histogrammer.
//!  Messages allow us to:
//! *   Create and delete histograms of various sorts
//! *   Apply gates to histograms.  These gates must be
//! conditions that are defined in a ConditionProcessor's dictionary.
//! *   Ungate histograms.
//! *   Clear the contents of individual or groups of histograms
//! *   Provide an event to the spectrum store for histograming.
//! *   Get descriptions of histograms.

use super::*;
use crate::conditions;
use crate::parameters;
use crate::spectra;
use crate::trace;
use ndhistogram::axis::*;
use ndhistogram::*;
use std::sync::mpsc;

use glob::Pattern;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisSpecification {
    pub low: f64,
    pub high: f64,
    pub bins: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChannelType {
    Underflow,
    Overflow,
    Bin,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Channel {
    pub chan_type: ChannelType,
    pub x: f64,
    pub y: f64,
    pub bin: usize,
    pub value: f64,
}
pub type SpectrumContents = Vec<Channel>;

#[derive(Clone, Debug, PartialEq)]
pub struct SpectrumProperties {
    pub name: String,
    pub type_name: String,
    pub xparams: Vec<String>,
    pub yparams: Vec<String>,
    pub xaxis: Option<AxisSpecification>,
    pub yaxis: Option<AxisSpecification>,
    pub gate: Option<String>,
}
/// xunder, yunder, xover, yover from get stats.
///
pub type SpectrumStatistics = (u32, u32, u32, u32);
///  Defines the requests that can be made of the spectrum
/// part of the histogram server
///
#[derive(Clone, Debug, PartialEq)]
pub enum SpectrumRequest {
    Create1D {
        name: String,
        parameter: String,
        axis: AxisSpecification,
    },
    CreateMulti1D {
        name: String,
        params: Vec<String>,
        axis: AxisSpecification,
    },
    CreateMulti2D {
        name: String,
        params: Vec<String>,
        xaxis: AxisSpecification,
        yaxis: AxisSpecification,
    },
    CreatePGamma {
        name: String,
        xparams: Vec<String>,
        yparams: Vec<String>,
        xaxis: AxisSpecification,
        yaxis: AxisSpecification,
    },
    CreateSummary {
        name: String,
        params: Vec<String>,
        yaxis: AxisSpecification,
    },
    Create2D {
        name: String,
        xparam: String,
        yparam: String,
        xaxis: AxisSpecification,
        yaxis: AxisSpecification,
    },
    Create2DSum {
        name: String,
        xparams: Vec<String>,
        yparams: Vec<String>,
        xaxis: AxisSpecification,
        yaxis: AxisSpecification,
    },
    Delete(String),
    List(String),
    Gate {
        spectrum: String,
        gate: String,
    },
    Ungate(String),
    Clear(String),
    GetContents {
        name: String,
        xlow: f64,
        xhigh: f64,
        ylow: f64,
        yhigh: f64,
    },
    Events(Vec<parameters::Event>),
    GetStats(String),
    SetContents {
        name: String,
        contents: SpectrumContents,
    },
    GetChan {
        name: String,
        xchan: i32,
        ychan: Option<i32>,
    },
    SetChan {
        name: String,
        xchan: i32,
        ychan: Option<i32>,
        value: f64,
    },
    Fold {
        spectrum_name: String,
        condition_name: String,
    },
    Unfold(String),
}

/// Defines the replies the spectrum par tof the histogram
/// server can return
#[derive(Clone, Debug, PartialEq)]
pub enum SpectrumReply {
    Error(String),
    Created,                          // Spectrum created.
    Deleted,                          // Spectrum deleted.
    Gated,                            // Condition applied.
    Ungated,                          // Spectrum ungated.
    Cleared,                          // Spectra cleared.
    Contents(SpectrumContents),       // Contents of a spectrum.
    Listing(Vec<SpectrumProperties>), // List of spectrum props.
    Processed,                        // Events processed.
    Statistics(SpectrumStatistics),   // Spectrum statistics.
    ChannelValue(f64),                // GetChan
    ChannelSet,                       // SetChan
    Folded,
    Unfolded,
}
/// Convert a coordinate to a bin:
///
/// ### Parameters:
///   *  c  - a coordinate.
///   *  a  - an axis definition.
///
/// ### Returns
///    u32 - note that if c is beyond the axis the appropriate over/underflow
/// bin is returned
///
pub fn coord_to_bin(c: f64, a: AxisSpecification) -> u32 {
    if c < a.low {
        0
    } else if c >= a.high {
        a.bins
    } else {
        // The 1.0 reflects that bin 0 is underflow.
        let result = 1.0 + (c * ((a.bins - 2) as f64) / (a.high - a.low));
        result as u32
    }
}
/// Convert a bin to a coordinate:
///
/// ### Parameters:
///    bin - bin  number along an axis.
///    a   - Axis specification.
///
/// ### Returns
///   f64 - note that there are some special cases:
///  *  bin == 0 is an underflow and translates to low-1.0
///  *  bin >= bins-1 - is an overflow and translates to high+1.0
/// All others map to the range [low, high)
///
///
pub fn bin_to_coord(bin: u32, a: AxisSpecification) -> f64 {
    // Special case handling:

    if bin == 0 {
        return a.low - 1.0; // Underflow
    }
    if bin >= (a.bins - 1) {
        return a.high + 1.0; // overflow.
    }
    // The rest maps [0, bins-2] -> [low, high]

    let b = (bin - 1) as f64;
    let bin_range = (a.bins - 2) as f64;

    b * (a.high - a.low) / bin_range // Simple linear scaling.
}
///  
///
/// SpectrumProcessor is the struct that processes
/// spectrum requests.  Some requests will need
/// a parameter and condition dictionary.  
/// Note that the implementation is divorced from the
/// actual message.  This makes testing the impl easier.
pub struct SpectrumProcessor {
    dict: spectra::SpectrumStorage,
}

impl SpectrumProcessor {
    // private methods:

    // Make a 1-d spectrum:

    fn make_1d(
        &mut self,
        name: &str,
        parameter: &str,
        axis: &AxisSpecification,
        pdict: &parameters::ParameterDictionary,
        tracedb: &trace::SharedTraceStore,
    ) -> SpectrumReply {
        if !self.dict.exists(name) {
            match spectra::Oned::new(
                name,
                parameter,
                pdict,
                Some(axis.low),
                Some(axis.high),
                Some(axis.bins),
            ) {
                Ok(spec) => {
                    self.dict.add(Rc::new(RefCell::new(spec)));
                    tracedb.add_event(trace::TraceEvent::SpectrumCreated(String::from(name)));
                    SpectrumReply::Created
                }
                Err(msg) => SpectrumReply::Error(msg),
            }
        } else {
            SpectrumReply::Error(format!("Spectrum {} already exists", name))
        }
    }
    // Make a multi incremented 1d spectrum (gamma-1d)

    fn make_multi1d(
        &mut self,
        name: &str,
        params: &[String],
        axis: &AxisSpecification,
        pdict: &parameters::ParameterDictionary,
        tracedb: &trace::SharedTraceStore,
    ) -> SpectrumReply {
        if !self.dict.exists(name) {
            match spectra::Multi1d::new(
                name,
                params.to_owned(),
                pdict,
                Some(axis.low),
                Some(axis.high),
                Some(axis.bins),
            ) {
                Ok(spec) => {
                    self.dict.add(Rc::new(RefCell::new(spec)));
                    tracedb.add_event(trace::TraceEvent::SpectrumCreated(String::from(name)));
                    SpectrumReply::Created
                }
                Err(msg) => SpectrumReply::Error(msg),
            }
        } else {
            SpectrumReply::Error(format!("Spectrum {} already exists", name))
        }
    }
    // make multi incremented 2-d (gamma2) spectrum:

    fn make_multi2d(
        &mut self,
        name: &str,
        params: &[String],
        xaxis: &AxisSpecification,
        yaxis: &AxisSpecification,
        pdict: &parameters::ParameterDictionary,
        tracedb: &trace::SharedTraceStore,
    ) -> SpectrumReply {
        if !self.dict.exists(name) {
            match spectra::Multi2d::new(
                name,
                params.to_owned(),
                pdict,
                Some(xaxis.low),
                Some(xaxis.high),
                Some(xaxis.bins),
                Some(yaxis.low),
                Some(yaxis.high),
                Some(yaxis.bins),
            ) {
                Ok(spec) => {
                    self.dict.add(Rc::new(RefCell::new(spec)));
                    tracedb.add_event(trace::TraceEvent::SpectrumCreated(String::from(name)));
                    SpectrumReply::Created
                }
                Err(msg) => SpectrumReply::Error(msg),
            }
        } else {
            SpectrumReply::Error(format!("Spectrum {} already exists", name))
        }
    }
    // make a particle gamma spectrum

    fn make_pgamma(
        &mut self,
        name: &str,
        xparams: &[String],
        yparams: &[String],
        xaxis: &AxisSpecification,
        yaxis: &AxisSpecification,
        pdict: &parameters::ParameterDictionary,
        tracedb: &trace::SharedTraceStore,
    ) -> SpectrumReply {
        if !self.dict.exists(name) {
            match spectra::PGamma::new(
                name,
                xparams,
                yparams,
                pdict,
                Some(xaxis.low),
                Some(xaxis.high),
                Some(xaxis.bins),
                Some(yaxis.low),
                Some(yaxis.high),
                Some(yaxis.bins),
            ) {
                Ok(spec) => {
                    self.dict.add(Rc::new(RefCell::new(spec)));
                    tracedb.add_event(trace::TraceEvent::SpectrumCreated(String::from(name)));
                    SpectrumReply::Created
                }
                Err(str) => SpectrumReply::Error(str),
            }
        } else {
            SpectrumReply::Error(format!("Spectrum {} already exists", name))
        }
    }
    // Make a summary spectrum

    fn make_summary(
        &mut self,
        name: &str,
        params: &[String],
        xaxis: &AxisSpecification,
        pdict: &parameters::ParameterDictionary,
        tracedb: &trace::SharedTraceStore,
    ) -> SpectrumReply {
        if !self.dict.exists(name) {
            match spectra::Summary::new(
                name,
                params.to_owned(),
                pdict,
                Some(xaxis.low),
                Some(xaxis.high),
                Some(xaxis.bins),
            ) {
                Ok(spec) => {
                    self.dict.add(Rc::new(RefCell::new(spec)));
                    tracedb.add_event(trace::TraceEvent::SpectrumCreated(String::from(name)));
                    SpectrumReply::Created
                }
                Err(msg) => SpectrumReply::Error(msg),
            }
        } else {
            SpectrumReply::Error(format!("Spectrum {} already exists", name))
        }
    }
    // Make 2-d spectrum.

    fn make_2d(
        &mut self,
        name: &str,
        xparam: &str,
        yparam: &str,
        xaxis: &AxisSpecification,
        yaxis: &AxisSpecification,
        pdict: &parameters::ParameterDictionary,
        tracedb: &trace::SharedTraceStore,
    ) -> SpectrumReply {
        if !self.dict.exists(name) {
            match spectra::Twod::new(
                name,
                xparam,
                yparam,
                pdict,
                Some(xaxis.low),
                Some(xaxis.high),
                Some(xaxis.bins),
                Some(yaxis.low),
                Some(yaxis.high),
                Some(yaxis.bins),
            ) {
                Ok(spec) => {
                    self.dict.add(Rc::new(RefCell::new(spec)));
                    tracedb.add_event(trace::TraceEvent::SpectrumCreated(String::from(name)));
                    SpectrumReply::Created
                }
                Err(msg) => SpectrumReply::Error(msg),
            }
        } else {
            SpectrumReply::Error(format!("Spectrum {} already exists", name))
        }
    }
    // Make a 2d sum spectrum.

    fn make_2dsum(
        &mut self,
        name: &str,
        xparams: &[String],
        yparams: &[String],
        xaxis: &AxisSpecification,
        yaxis: &AxisSpecification,
        pdict: &parameters::ParameterDictionary,
        tracedb: &trace::SharedTraceStore,
    ) -> SpectrumReply {
        if !self.dict.exists(name) {
            if xparams.len() != yparams.len() {
                return SpectrumReply::Error(String::from(
                    "Number of xparams must be the same as number of y params",
                ));
            }
            let mut params = spectra::XYParameters::new();
            for (i, x) in xparams.iter().enumerate() {
                let p: spectra::XYParameter = (x.to_owned(), yparams[i].to_owned());
                params.push(p);
            }
            match spectra::TwodSum::new(
                name,
                params,
                pdict,
                Some(xaxis.low),
                Some(xaxis.high),
                Some(xaxis.bins),
                Some(yaxis.low),
                Some(yaxis.high),
                Some(yaxis.bins),
            ) {
                Ok(spec) => {
                    self.dict.add(Rc::new(RefCell::new(spec)));
                    tracedb.add_event(trace::TraceEvent::SpectrumCreated(String::from(name)));
                    SpectrumReply::Created
                }
                Err(msg) => SpectrumReply::Error(msg),
            }
        } else {
            SpectrumReply::Error(format!("Spectrum {} already exists", name))
        }
    }
    // Delete an existing spectrum.

    fn delete_spectrum(&mut self, name: &str, tracedb: &trace::SharedTraceStore) -> SpectrumReply {
        if self.dict.remove(name).is_some() {
            tracedb.add_event(trace::TraceEvent::SpectrumDeleted(String::from(name)));
            SpectrumReply::Deleted
        } else {
            SpectrumReply::Error(format!("Spectrum {} does not exist", name))
        }
    }
    // List spectra and properties.

    fn get_properties(spec: &spectra::SpectrumContainer) -> SpectrumProperties {
        let s = spec.borrow();
        let x = s.get_xaxis();
        let y = s.get_yaxis();
        SpectrumProperties {
            name: s.get_name(),
            type_name: s.get_type(),
            xparams: s.get_xparams(),
            yparams: s.get_yparams(),
            xaxis: x.map(|xa| AxisSpecification {
                low: xa.0,
                high: xa.1,
                bins: xa.2,
            }),
            yaxis: y.map(|xa| AxisSpecification {
                low: xa.0,
                high: xa.1,
                bins: xa.2,
            }),
            gate: s.get_gate(),
        }
    }

    fn list_spectra(&self, pattern: &str) -> SpectrumReply {
        let mut listing = Vec::<SpectrumProperties>::new();
        let p = Pattern::new(pattern);
        if let Err(reason) = p {
            return SpectrumReply::Error(format!("Bad glob pattern {}", reason.msg));
        }
        let p = p.unwrap();
        for (name, s) in self.dict.iter() {
            if p.matches(name) {
                listing.push(Self::get_properties(s));
            }
        }

        SpectrumReply::Listing(listing)
    }
    fn gate_spectrum(
        &self,
        sname: &str,
        gname: &str,
        cdict: &conditions::ConditionDictionary,
    ) -> SpectrumReply {
        if let Some(spec) = self.dict.get(sname) {
            if let Err(msg) = spec.borrow_mut().gate(gname, cdict) {
                SpectrumReply::Error(msg)
            } else {
                SpectrumReply::Gated
            }
        } else {
            SpectrumReply::Error(format!("Spectrum {} does not exist", sname))
        }
    }
    fn ungate_spectrum(&self, spectrum: &str) -> SpectrumReply {
        if let Some(spec) = self.dict.get(spectrum) {
            spec.borrow_mut().ungate();
            SpectrumReply::Ungated
        } else {
            SpectrumReply::Error(format!("Spectrum {} does not exist", spectrum))
        }
    }
    fn clear_spectra(&self, pattern: &str) -> SpectrumReply {
        let pat = Pattern::new(pattern);
        if let Err(e) = pat {
            return SpectrumReply::Error(format!("Bad glob pattern: {}", e.msg));
        }
        let pat = pat.unwrap();
        for (name, s) in self.dict.iter() {
            if pat.matches(name) {
                s.borrow_mut().clear();
            }
        }
        SpectrumReply::Cleared
    }
    fn get_contents(
        &self,
        name: &str,
        xlow: f64,
        xhigh: f64,
        ylow: f64,
        yhigh: f64,
    ) -> SpectrumReply {
        // How we iterate depends on the type of histogram:

        let mut result = SpectrumContents::new();
        if let Some(spec) = self.dict.get(name) {
            if let Some(spectrum) = spec.borrow().get_histogram_1d() {
                for c in spectrum.borrow().iter() {
                    let v = c.value.get();
                    if v != 0.0 {
                        match c.bin {
                            BinInterval::Underflow { end } => {
                                result.push(Channel {
                                    chan_type: ChannelType::Underflow,
                                    value: v,
                                    x: end,
                                    y: 0.0,
                                    bin: c.index,
                                });
                            }
                            BinInterval::Overflow { start } => {
                                result.push(Channel {
                                    chan_type: ChannelType::Overflow,
                                    value: v,
                                    x: start,
                                    y: 0.0,
                                    bin: c.index,
                                });
                            }
                            BinInterval::Bin { start, end: _end } => {
                                if (start >= xlow) && (start <= xhigh) {
                                    result.push(Channel {
                                        chan_type: ChannelType::Bin,
                                        x: start,
                                        y: 0.0,
                                        value: v,
                                        bin: c.index,
                                    });
                                };
                            }
                        }
                    }
                }
            } else {
                let spectrum = spec.borrow().get_histogram_2d().unwrap();
                for c in spectrum.borrow().iter() {
                    let v = c.value.get();
                    let xbin = c.bin.0;
                    let ybin = c.bin.1;
                    let x;
                    let y;
                    let mut ctype = ChannelType::Bin;

                    match xbin {
                        BinInterval::Overflow { start } => {
                            ctype = ChannelType::Overflow;
                            x = start;
                        }
                        BinInterval::Underflow { end } => {
                            ctype = ChannelType::Underflow;
                            x = end;
                        }
                        BinInterval::Bin { start, end: _end } => {
                            x = start;
                        }
                    };
                    match ybin {
                        BinInterval::Overflow { start } => {
                            if ctype == ChannelType::Bin {
                                ctype = ChannelType::Overflow;
                            }
                            y = start;
                        }
                        BinInterval::Underflow { end } => {
                            if ctype == ChannelType::Bin {
                                ctype = ChannelType::Underflow;
                            }
                            y = end;
                        }
                        BinInterval::Bin { start, end: _end } => {
                            y = start;
                        }
                    };
                    if (v != 0.0) && (x >= xlow) && (x <= xhigh) && (y >= ylow) && (y <= yhigh) {
                        result.push(Channel {
                            chan_type: ctype,
                            x,
                            y,
                            value: v,
                            bin: c.index,
                        });
                    }
                }
            }
            SpectrumReply::Contents(result)
        } else {
            SpectrumReply::Error(format!("Spectrum {} does not exist", name))
        }
    }
    fn process_events(
        &mut self,
        events: &[parameters::Event],
        cdict: &mut conditions::ConditionDictionary,
    ) -> SpectrumReply {
        for e in events.iter() {
            conditions::invalidate_cache(cdict);
            self.dict.process_event(e);
        }
        SpectrumReply::Processed
    }
    // Get spectrumstatistics:
    fn get_statistics(&self, name: &str) -> SpectrumReply {
        if let Some(spec) = self.dict.get(name) {
            SpectrumReply::Statistics(spec.borrow().get_out_of_range())
        } else {
            SpectrumReply::Error(format!("Spectrum {} does not exist", name))
        }
    }
    // Set the spectrum contents
    // Notes:
    //  * The spectrum is first cleared.
    //  * Underflow and overflow are supposedly ignored by fill_with so we
    // must increment the real cooordinate locations as many times as required.
    //  * We use the real coordinates rather than the bin number
    // to set each 'channel' value provided.
    //  * The successful reply is _Processed_

    fn set_contents(&mut self, name: &str, contents: &SpectrumContents) -> SpectrumReply {
        // Find the spectrum:

        if let Some(spec) = self.dict.get(name) {
            let mut histogram = spec.borrow_mut();
            histogram.clear();
            if histogram.is_1d() {
                let spec1d = histogram.get_histogram_1d().unwrap();
                for chan in contents {
                    spec1d
                        .borrow_mut()
                        .value_mut(&chan.x)
                        .unwrap()
                        .fill_with(chan.value);
                }
            } else {
                let spec2d = histogram.get_histogram_2d().unwrap();
                for chan in contents {
                    spec2d
                        .borrow_mut()
                        .value_mut(&(chan.x, chan.y))
                        .unwrap()
                        .fill_with(chan.value);
                }
            }
            SpectrumReply::Processed
        } else {
            SpectrumReply::Error(format!("Spectrum {} does not exist", name))
        }
    }
    fn channels2d_to_index(
        spec: &spectra::H2DContainer,
        xbin: i32,
        ybin: i32,
    ) -> Result<usize, String> {
        // Offset x and y bins to allow for the overflow:

        let xbin = (xbin + 1) as usize;
        let ybin = (ybin + 1) as usize;

        // Get the x/y axes from the histogram range check them:

        let xaxis = spec.borrow().axes().as_tuple().0.clone();
        let yaxis = spec.borrow().axes().as_tuple().1.clone();

        println!("Xbin: {} bins: {}", xbin, xaxis.num_bins());

        if xbin >= xaxis.num_bins() {
            return Err(format!(
                "Xbin: {} is larger than the number of bins: {}",
                xbin,
                xaxis.num_bins()
            ));
        }
        if ybin >= yaxis.num_bins() {
            return Err(format!(
                "Ybin: {} is larger than the number of bins: {}",
                ybin,
                yaxis.num_bins()
            ));
        }
        // we have good range so:

        Ok(xbin + ybin * xaxis.num_bins())
    }
    fn get_channel_value(&self, name: &str, xchan: i32, ychan: Option<i32>) -> SpectrumReply {
        // Find the spectrum:
        // If it does not exist, then error:

        if let Some(spec) = self.dict.get(name) {
            // What we do next depends on the spectrum  dimensionality:

            if spec.borrow().is_1d() {
                let xchan = (xchan + 1) as usize;
                if let Some(f) = spec
                    .borrow()
                    .get_histogram_1d()
                    .unwrap()
                    .borrow()
                    .value_at_index(xchan)
                {
                    SpectrumReply::ChannelValue(f.get())
                } else {
                    SpectrumReply::Error(String::from("X index is out of range"))
                }
            } else {
                // Must be a y channel:

                if let Some(ybin) = ychan {
                    // Have o turn the x/y channel into an index:

                    let spec = spec.borrow().get_histogram_2d().unwrap();
                    match Self::channels2d_to_index(&spec, xchan, ybin) {
                        Ok(index) => {
                            if let Some(f) = spec.borrow().value_at_index(index) {
                                SpectrumReply::ChannelValue(f.get())
                            } else {
                                SpectrumReply::Error(String::from("Bins are out of range"))
                            }
                        }
                        Err(s) => SpectrumReply::Error(s),
                    }
                } else {
                    SpectrumReply::Error(String::from("Must have  a ybin for a 2d spectrum"))
                }
            }
        } else {
            SpectrumReply::Error(format!("No such spectrum '{}'", name))
        }
    }

    // set the value of a channel:

    fn set_channel_value(
        &mut self,
        name: &str,
        xchan: i32,
        ychan: Option<i32>,
        value: f64,
    ) -> SpectrumReply {
        // The spectru must exist:

        if let Some(spec) = self.dict.get(name) {
            // How we figure out the index etc. depends on the dimensionality:

            if spec.borrow().is_1d() {
                let xchan = (xchan + 1) as usize; // -1 is overflow so..
                if let Some(c) = spec
                    .borrow()
                    .get_histogram_1d()
                    .unwrap()
                    .borrow_mut()
                    .value_at_index_mut(xchan)
                {
                    c.fill_with(value);
                    SpectrumReply::ChannelSet
                } else {
                    SpectrumReply::Error(String::from("X index is out of range"))
                }
            } else {
                // 2d spectrum:

                if let Some(ybin) = ychan {
                    let spec = spec.borrow().get_histogram_2d().unwrap();
                    match Self::channels2d_to_index(&spec, xchan, ybin) {
                        Ok(index) => {
                            if let Some(c) = spec.borrow_mut().value_at_index_mut(index) {
                                c.fill_with(value);
                                SpectrumReply::ChannelSet
                            } else {
                                SpectrumReply::Error(String::from("Bins are out of range"))
                            }
                        }
                        Err(s) => SpectrumReply::Error(s),
                    }
                } else {
                    SpectrumReply::Error(String::from("2d spectra need a y bin"))
                }
            }
        } else {
            SpectrumReply::Error(format!("No such spectrum: {}", name))
        }
    }
    // Fold a spectrum given a condition  name and a condition name:

    fn fold_spectrum(
        &mut self,
        spectrum: &str,
        condition: &str,
        cdict: &conditions::ConditionDictionary,
    ) -> SpectrumReply {
        if let Some(s) = self.dict.get(spectrum) {
            if let Err(s) = s.borrow_mut().fold(condition, cdict) {
                SpectrumReply::Error(format!("Failed to fold {}: {}", spectrum, s))
            } else {
                SpectrumReply::Folded
            }
        } else {
            SpectrumReply::Error(format!("no such spectrum {}", spectrum))
        }
    }
    // Unfold a spectrum:

    fn unfold_spectrum(&mut self, spectrum: &str) -> SpectrumReply {
        if let Some(s) = self.dict.get(spectrum) {
            if let Err(s) = s.borrow_mut().unfold() {
                SpectrumReply::Error(format!("Failed to unfold spectrum {}: {}", spectrum, s))
            } else {
                SpectrumReply::Unfolded
            }
        } else {
            SpectrumReply::Error(format!("no such spectrum {}", spectrum))
        }
    }

    // Public methods
    /// Construction

    pub fn new() -> SpectrumProcessor {
        SpectrumProcessor {
            dict: spectra::SpectrumStorage::new(),
        }
    }
    /// Process requests returning replies:

    pub fn process_request(
        &mut self,
        req: SpectrumRequest,
        pdict: &parameters::ParameterDictionary,
        cdict: &mut conditions::ConditionDictionary,
        tracedb: &trace::SharedTraceStore,
    ) -> SpectrumReply {
        match req {
            SpectrumRequest::Create1D {
                name,
                parameter,
                axis,
            } => self.make_1d(&name, &parameter, &axis, pdict, tracedb),
            SpectrumRequest::CreateMulti1D { name, params, axis } => {
                self.make_multi1d(&name, &params, &axis, pdict, tracedb)
            }
            SpectrumRequest::CreateMulti2D {
                name,
                params,
                xaxis,
                yaxis,
            } => self.make_multi2d(&name, &params, &xaxis, &yaxis, pdict, tracedb),
            SpectrumRequest::CreatePGamma {
                name,
                xparams,
                yparams,
                xaxis,
                yaxis,
            } => self.make_pgamma(&name, &xparams, &yparams, &xaxis, &yaxis, pdict, tracedb),
            SpectrumRequest::CreateSummary {
                name,
                params,
                yaxis,
            } => self.make_summary(&name, &params, &yaxis, pdict, tracedb),
            SpectrumRequest::Create2D {
                name,
                xparam,
                yparam,
                xaxis,
                yaxis,
            } => self.make_2d(&name, &xparam, &yparam, &xaxis, &yaxis, pdict, tracedb),
            SpectrumRequest::Create2DSum {
                name,
                xparams,
                yparams,
                xaxis,
                yaxis,
            } => self.make_2dsum(&name, &xparams, &yparams, &xaxis, &yaxis, pdict, tracedb),
            SpectrumRequest::Delete(name) => self.delete_spectrum(&name, tracedb),
            SpectrumRequest::List(pattern) => self.list_spectra(&pattern),
            SpectrumRequest::Gate { spectrum, gate } => self.gate_spectrum(&spectrum, &gate, cdict),
            SpectrumRequest::Ungate(name) => self.ungate_spectrum(&name),
            SpectrumRequest::Clear(pattern) => self.clear_spectra(&pattern),
            SpectrumRequest::GetContents {
                name,
                xlow,
                xhigh,
                ylow,
                yhigh,
            } => self.get_contents(&name, xlow, xhigh, ylow, yhigh),
            SpectrumRequest::Events(events) => self.process_events(&events, cdict),
            SpectrumRequest::GetStats(name) => self.get_statistics(&name),
            SpectrumRequest::SetContents { name, contents } => self.set_contents(&name, &contents),
            SpectrumRequest::GetChan { name, xchan, ychan } => {
                self.get_channel_value(&name, xchan, ychan)
            }
            SpectrumRequest::SetChan {
                name,
                xchan,
                ychan,
                value,
            } => self.set_channel_value(&name, xchan, ychan, value),
            SpectrumRequest::Fold {
                spectrum_name,
                condition_name,
            } => self.fold_spectrum(&spectrum_name, &condition_name, cdict),
            SpectrumRequest::Unfold(spectrum) => self.unfold_spectrum(&spectrum),
        }
    }
}
//----------------------------------------------------------------
// Client code:

/// This is a Result where the server has nothing of
/// of interest to say to the caller.
///
pub type SpectrumServerEmptyResult = Result<(), String>;

/// This is a result where the server, on success will
/// provide a list of properties of some spectra:
///
pub type SpectrumServerListingResult = Result<Vec<SpectrumProperties>, String>;
///
/// This type is a result the API will sue to return spectrum
/// contents:
pub type SpectrumServerContentsResult = Result<SpectrumContents, String>;

/// Result for spectrum statistics request:

pub type SpectrumServerStatisticsResult = Result<SpectrumStatistics, String>;

/// Result from the GetChan:

pub type SpectrumChannelResult = Result<f64, String>;

///
/// This struct provides a container for the channel used to
/// make server requests.  The implementation can then be simplified
/// as each request can make the reply channel pair.
///
pub struct SpectrumMessageClient {
    req_chan: mpsc::Sender<Request>,
}

impl SpectrumMessageClient {
    fn create1d_request(
        name: &str,
        parameter: &str,
        low: f64,
        high: f64,
        bins: u32,
    ) -> SpectrumRequest {
        SpectrumRequest::Create1D {
            name: String::from(name),
            parameter: String::from(parameter),
            axis: AxisSpecification { low, high, bins },
        }
    }

    fn createmulti1d_request(
        name: &str,
        params: &[String],
        low: f64,
        high: f64,
        bins: u32,
    ) -> SpectrumRequest {
        SpectrumRequest::CreateMulti1D {
            name: String::from(name),
            params: params.to_owned(),
            axis: AxisSpecification { low, high, bins },
        }
    }
    fn createmulti2d_request(
        name: &str,
        params: &[String],
        xlow: f64,
        xhigh: f64,
        xbins: u32,
        ylow: f64,
        yhigh: f64,
        ybins: u32,
    ) -> SpectrumRequest {
        SpectrumRequest::CreateMulti2D {
            name: String::from(name),
            params: params.to_owned(),
            xaxis: AxisSpecification {
                low: xlow,
                high: xhigh,
                bins: xbins,
            },
            yaxis: AxisSpecification {
                low: ylow,
                high: yhigh,
                bins: ybins,
            },
        }
    }
    fn createpgamma_request(
        name: &str,
        xparams: &[String],
        yparams: &[String],
        xlow: f64,
        xhigh: f64,
        xbins: u32,
        ylow: f64,
        yhigh: f64,
        ybins: u32,
    ) -> SpectrumRequest {
        SpectrumRequest::CreatePGamma {
            name: String::from(name),
            xparams: xparams.to_owned(),
            yparams: yparams.to_owned(),
            xaxis: AxisSpecification {
                low: xlow,
                high: xhigh,
                bins: xbins,
            },
            yaxis: AxisSpecification {
                low: ylow,
                high: yhigh,
                bins: ybins,
            },
        }
    }
    fn createsummary_request(
        name: &str,
        params: &[String],
        low: f64,
        high: f64,
        bins: u32,
    ) -> SpectrumRequest {
        SpectrumRequest::CreateSummary {
            name: String::from(name),
            params: params.to_owned(),
            yaxis: AxisSpecification { low, high, bins },
        }
    }
    fn create2d_request(
        name: &str,
        xparam: &str,
        yparam: &str,
        xlow: f64,
        xhigh: f64,
        xbins: u32,
        ylow: f64,
        yhigh: f64,
        ybins: u32,
    ) -> SpectrumRequest {
        SpectrumRequest::Create2D {
            name: String::from(name),
            xparam: String::from(xparam),
            yparam: String::from(yparam),
            xaxis: AxisSpecification {
                low: xlow,
                high: xhigh,
                bins: xbins,
            },
            yaxis: AxisSpecification {
                low: ylow,
                high: yhigh,
                bins: ybins,
            },
        }
    }
    fn create2dsum_request(
        name: &str,
        xparams: &[String],
        yparams: &[String],
        xlow: f64,
        xhigh: f64,
        xbins: u32,
        ylow: f64,
        yhigh: f64,
        ybins: u32,
    ) -> SpectrumRequest {
        SpectrumRequest::Create2DSum {
            name: String::from(name),
            xparams: xparams.to_owned(),
            yparams: yparams.to_owned(),
            xaxis: AxisSpecification {
                low: xlow,
                high: xhigh,
                bins: xbins,
            },
            yaxis: AxisSpecification {
                low: ylow,
                high: yhigh,
                bins: ybins,
            },
        }
    }
    fn delete_request(name: &str) -> SpectrumRequest {
        SpectrumRequest::Delete(String::from(name))
    }
    fn list_request(pattern: &str) -> SpectrumRequest {
        SpectrumRequest::List(String::from(pattern))
    }
    fn gate_request(spectrum: &str, condition: &str) -> SpectrumRequest {
        SpectrumRequest::Gate {
            spectrum: String::from(spectrum),
            gate: String::from(condition),
        }
    }
    fn ungate_request(name: &str) -> SpectrumRequest {
        SpectrumRequest::Ungate(String::from(name))
    }
    fn clear_request(pattern: &str) -> SpectrumRequest {
        SpectrumRequest::Clear(String::from(pattern))
    }
    fn getcontents_request(
        name: &str,
        xlow: f64,
        xhigh: f64,
        ylow: f64,
        yhigh: f64,
    ) -> SpectrumRequest {
        SpectrumRequest::GetContents {
            name: String::from(name),
            xlow,
            xhigh,
            ylow,
            yhigh,
        }
    }
    fn events_request(events: &[parameters::Event]) -> SpectrumRequest {
        SpectrumRequest::Events(events.to_owned())
    }

    fn transact(&self, req: SpectrumRequest) -> SpectrumReply {
        let (reply_send, reply_recv) = mpsc::channel::<Reply>();
        let request = Request {
            reply_channel: reply_send,
            message: MessageType::Spectrum(req),
        };
        let reply = request.transaction(self.req_chan.clone(), reply_recv);
        if let Reply::Spectrum(r) = reply {
            r
        } else {
            panic!("Expected Spectrum reply got something else");
        }
    }

    //------------------- Client API methods-------------------------

    /// Create an instance of the api:

    pub fn new(req_chan: &mpsc::Sender<Request>) -> SpectrumMessageClient {
        SpectrumMessageClient {
            req_chan: req_chan.clone(),
        }
    }

    ///
    /// Create a 1d spectrum:
    ///
    /// *  name - name of the spectrum to create.
    /// *  parameter - name of the parameter to histogram
    /// *  low, high, bins - axis specification for the spectrum.

    ///
    /// Returns: SpectrumServerEmptyResult
    ///
    pub fn create_spectrum_1d(
        &self,
        name: &str,
        parameter: &str,
        low: f64,
        high: f64,
        bins: u32,
    ) -> SpectrumServerEmptyResult {
        let reply = self.transact(Self::create1d_request(name, parameter, low, high, bins));
        if let SpectrumReply::Error(s) = reply {
            Err(s)
        } else {
            Ok(())
        }
    }
    /// Create a mutiply incremented 1d spectrum (gamma 1d).
    ///
    ///
    /// *   name - name of the spectrum.
    /// *   params - Names of the parameters to histogram.
    /// *   low, high, bins - axis specifications.

    ///
    /// Returns: SpectrumServerEmptyResult
    ///
    pub fn create_spectrum_multi1d(
        &self,
        name: &str,
        parameters: &[String],
        low: f64,
        high: f64,
        bins: u32,
    ) -> SpectrumServerEmptyResult {
        let reply = self.transact(Self::createmulti1d_request(
            name, parameters, low, high, bins,
        ));
        if let SpectrumReply::Error(s) = reply {
            Err(s)
        } else {
            Ok(())
        }
    }
    /// Create a muliply incremented 2d spectrum (gamma 2)
    ///
    /// *   name - spectrum name.
    /// *   parameters - vector of  parameters (reference)
    /// *   xlow, xhigh, xbins - x axis specification.
    /// *   ylow, yhigh, ybins - y axis specification.
    ///
    /// Returns: SpectrumServerEmptyResult

    pub fn create_spectrum_multi2d(
        &self,
        name: &str,
        parameters: &[String],
        xlow: f64,
        xhigh: f64,
        xbins: u32,
        ylow: f64,
        yhigh: f64,
        ybins: u32,
    ) -> SpectrumServerEmptyResult {
        let reply = self.transact(Self::createmulti2d_request(
            name, parameters, xlow, xhigh, xbins, ylow, yhigh, ybins,
        ));
        if let SpectrumReply::Error(s) = reply {
            Err(s)
        } else {
            Ok(())
        }
    }
    ///  Create a particle gamma spectrum (gamma delux).
    ///
    /// *   name -spectrum name.
    /// *   xparams - xaxis parameters.
    /// *   yparams - yaxis parameters.
    /// *   xlow, xhigh, xbins - x axis specification.
    /// *   ylow, yhigh, ybins - y axis specification.

    ///
    /// Returns: SpectrumServerEmptyResult

    pub fn create_spectrum_pgamma(
        &self,
        name: &str,
        xparams: &[String],
        yparams: &[String],
        xlow: f64,
        xhigh: f64,
        xbins: u32,
        ylow: f64,
        yhigh: f64,
        ybins: u32,
    ) -> SpectrumServerEmptyResult {
        let reply = self.transact(Self::createpgamma_request(
            name, xparams, yparams, xlow, xhigh, xbins, ylow, yhigh, ybins,
        ));
        if let SpectrumReply::Error(s) = reply {
            Err(s)
        } else {
            Ok(())
        }
    }
    /// Create a summary spectrum:
    ///
    /// *  name - name of the spectrum
    /// *  params - The parameters to histogram.
    /// *  low, high, bins - axis specifications (y axis).
    ///
    /// Returns: SpectrumServerEmptyResult

    pub fn create_spectrum_summary(
        &self,
        name: &str,
        params: &[String],
        low: f64,
        high: f64,
        bins: u32,
    ) -> SpectrumServerEmptyResult {
        let reply = self.transact(Self::createsummary_request(name, params, low, high, bins));
        if let SpectrumReply::Error(s) = reply {
            Err(s)
        } else {
            Ok(())
        }
    }
    /// Create 2d spectrum.
    ///
    /// * name - name of the spectrum.
    /// * xparam - parameter on x axis.
    /// * yparam - parameter on yaxis.
    /// * xlow, xhigh, xbins - X axis specification.
    /// * ylow, yhigh, ybins - Y axis specification.
    /// *  req - request channel
    /// *  reply_send - channel on which to send the reply.
    /// *  reply_recv  - Chanel on which to recieve the reply.
    ///
    /// Returns: SpectrumServerEmptyResult

    pub fn create_spectrum_2d(
        &self,
        name: &str,
        xparam: &str,
        yparam: &str,
        xlow: f64,
        xhigh: f64,
        xbins: u32,
        ylow: f64,
        yhigh: f64,
        ybins: u32,
    ) -> SpectrumServerEmptyResult {
        let reply = self.transact(Self::create2d_request(
            name, xparam, yparam, xlow, xhigh, xbins, ylow, yhigh, ybins,
        ));
        if let SpectrumReply::Error(s) = reply {
            Err(s)
        } else {
            Ok(())
        }
    }
    ///  Create a 2d spectrum that is the sum of 2d spectra.
    ///
    /// * name - name of the spectrum.
    /// * xparams - Parameters on x axis.
    /// * yparams - parameters on the y axis.
    /// * xlow, xhigh, xbins - xaxis specification.
    /// * ylow, yhigh, ybins - yaxis specification.
    ///
    /// Returns: SpectrumServerEmptyResult
    /// *  Note:  The size of xparams and yparams must be identical.
    ///
    pub fn create_spectrum_2dsum(
        &self,
        name: &str,
        xparams: &[String],
        yparams: &[String],
        xlow: f64,
        xhigh: f64,
        xbins: u32,
        ylow: f64,
        yhigh: f64,
        ybins: u32,
    ) -> SpectrumServerEmptyResult {
        let reply = self.transact(Self::create2dsum_request(
            name, xparams, yparams, xlow, xhigh, xbins, ylow, yhigh, ybins,
        ));
        if let SpectrumReply::Error(s) = reply {
            Err(s)
        } else {
            Ok(())
        }
    }

    /// Delete a spectrum.
    ///
    /// * name - name of the spectrum to delete.
    ///
    /// Returns SpectrumServerEmptyResult
    ///
    pub fn delete_spectrum(&self, name: &str) -> SpectrumServerEmptyResult {
        let reply = self.transact(Self::delete_request(name));
        if let SpectrumReply::Error(s) = reply {
            Err(s)
        } else {
            Ok(())
        }
    }
    /// list spectra
    ///
    /// *   pattern - Glob pattern the server will list information
    /// for all spectra that match the pattern. Note that "*" will
    /// match all spectgra.
    ///
    /// Returns : SpectrumServerListingResult
    ///
    pub fn list_spectra(&self, pattern: &str) -> SpectrumServerListingResult {
        match self.transact(Self::list_request(pattern)) {
            SpectrumReply::Error(s) => Err(s),
            SpectrumReply::Listing(l) => Ok(l),
            _ => Err(String::from("Unexpected server result for list request")),
        }
    }
    /// Apply a gate to a spectrum:
    ///
    /// * spectrum -name of the spectrum.
    /// * gate - name of the gate to apply.
    ///
    /// Retuns: SpectrumServerEmptyResult.
    ///
    pub fn gate_spectrum(&self, spectrum: &str, gate: &str) -> SpectrumServerEmptyResult {
        let reply = self.transact(Self::gate_request(spectrum, gate));
        if let SpectrumReply::Error(s) = reply {
            Err(s)
        } else {
            Ok(())
        }
    }
    /// Ungate a spectrum.  
    ///
    /// *  name - name of the spectrum
    ///
    /// Retuns: SpectrumServerEmptyResult.
    ///
    pub fn ungate_spectrum(&self, name: &str) -> SpectrumServerEmptyResult {
        let reply = self.transact(Self::ungate_request(name));
        if let SpectrumReply::Error(s) = reply {
            Err(s)
        } else {
            Ok(())
        }
    }

    /// clear spectra
    ///
    /// *  pattern - glob pattern that describes the spectra to clear.
    /// e.g. "*" clears them all.
    ///
    /// Retuns: SpectrumServerEmptyResult.
    ///
    pub fn clear_spectra(&self, pattern: &str) -> SpectrumServerEmptyResult {
        let reply = self.transact(Self::clear_request(pattern));
        if let SpectrumReply::Error(s) = reply {
            Err(s)
        } else {
            Ok(())
        }
    }
    ///
    /// Get the contents of a spectrum.
    ///
    /// * name - name of the spectrum.
    /// * xlow, xhigh, ylow, yhigh - a rectangular region of interest in
    /// parameter coordinate space within which the data are returned.
    /// Note that only data with non-zero channel values are returned.
    ///
    /// Returns:  SpectrumServerContentsResult
    ///
    pub fn get_contents(
        &self,
        name: &str,
        xlow: f64,
        xhigh: f64,
        ylow: f64,
        yhigh: f64,
    ) -> SpectrumServerContentsResult {
        match self.transact(Self::getcontents_request(name, xlow, xhigh, ylow, yhigh)) {
            SpectrumReply::Error(s) => Err(s),
            SpectrumReply::Contents(c) => Ok(c),
            _ => Err(String::from("Unexpected reply type for get_contents")),
        }
    }
    ///
    /// Process events.
    ///
    /// *  events - vector of flat event.
    ///
    ///
    pub fn process_events(&self, e: &[parameters::Event]) -> SpectrumServerEmptyResult {
        match self.transact(Self::events_request(e)) {
            SpectrumReply::Processed => Ok(()),
            SpectrumReply::Error(s) => Err(s),
            _ => Err(String::from("processEvents -unexpected reply type")),
        }
    }
    /// Return the over/underflow statistics for a spectrum.
    ///
    /// ### Parameters:
    /// * name - the name of the spectrum to query.
    /// ### Returns:
    /// * SpectrumServerStatisticsResult
    ///     - Err has a string containing the error.
    ///     - Ok has a Statistics tuple.
    ///
    pub fn get_statistics(&self, name: &str) -> SpectrumServerStatisticsResult {
        match self.transact(SpectrumRequest::GetStats(String::from(name))) {
            SpectrumReply::Statistics(s) => Ok(s),
            SpectrumReply::Error(s) => Err(s),
            _ => Err(String::from("get_statistics - unexpected reply type")),
        }
    }
    /// Set the contents of a spectrum.
    ///
    /// ### Parameters:
    /// *  name - name of the spectrum to fill.
    /// *  contents - Contents to set the spectrum to.  Note that for each channel:
    ///     - chan_type is actually unimportant.
    ///     - x,y determine the fill coordinates.
    ///     - bin is ignored.
    ///     - value is the value to fill the channel selected by x/y.
    /// ### Returns:
    /// * SpectrumServerEmptyResult - on err, the string is the error message
    /// that describes the problem.
    ///
    /// ### Notes:
    ///  *   The target spectrum is cleared first.
    ///  *   **Important** If there's more than one x/y that maps to the same underlying bin,
    /// the last one determines the bin contents.  This is important if filling
    /// a spectrum with lower resolution than the one which created the
    /// initial set of x/y/value triplets.
    ///
    pub fn fill_spectrum(
        &self,
        name: &str,
        contents: SpectrumContents,
    ) -> SpectrumServerEmptyResult {
        let request = SpectrumRequest::SetContents {
            name: String::from(name),
            contents,
        };
        let reply = self.transact(request);
        match reply {
            SpectrumReply::Processed => Ok(()),
            SpectrumReply::Error(s) => Err(s),
            _ => Err(String::from("Unexpected reply type in fill_spectrum")),
        }
    }
    /// Get the value of a single channel of a spectrum.
    ///
    /// ### Parameters:
    /// *  name - name of the spectrum.
    /// *  xchan - xchannel number (always required).
    /// *  ychan - optional y channel, required for 2d spectra.
    ///
    ///  ### Returns:
    ///     SpectrumChannelResult - which, on Ok encapsulates the f64 value of
    ///  the requested channel.
    ///
    ///  ### Notes:
    ///   * -1 is the channel value for underflows and n+1 where n is the
    /// number of 'data' bins on that axis gets to the overflows.
    ///   *  If the bins are out of range the results are an error (checked in
    /// the server not the ndhistogram package as I'm not 100% sure what it does
    /// when presented with that case).
    ///
    pub fn get_channel_value(
        &self,
        name: &str,
        xchan: i32,
        ychan: Option<i32>,
    ) -> SpectrumChannelResult {
        let request = SpectrumRequest::GetChan {
            name: String::from(name),
            xchan,
            ychan,
        };
        match self.transact(request) {
            SpectrumReply::ChannelValue(value) => Ok(value),
            SpectrumReply::Error(s) => Err(s),
            _ => Err(String::from("Unexpected reply type in get_channel_value")),
        }
    }
    /// Set the value of a singl,e channel of a spectrum.
    ///
    /// ### Parameters:
    /// *  name - name of the spectrum.
    /// *  xchan - xchannel number (always required).
    /// *  ychan - optional y channel, required for 2d spectra.
    /// *  value - New value for the channel (f64).
    ///
    /// Returns: SpectrumServerEmptyResult.
    ///
    ///  ### Notes:
    ///   * -1 is the channel value for underflows and n+1 where n is the
    /// number of 'data' bins on that axis gets to the overflows.
    ///   *  If the bins are out of range the results are an error (checked in
    /// the server not the ndhistogram package as I'm not 100% sure what it does
    /// when presented with that case).
    ///
    pub fn set_channel_value(
        &self,
        name: &str,
        xchan: i32,
        ychan: Option<i32>,
        value: f64,
    ) -> SpectrumServerEmptyResult {
        let request = SpectrumRequest::SetChan {
            name: String::from(name),
            xchan,
            ychan,
            value,
        };
        match self.transact(request) {
            SpectrumReply::ChannelSet => Ok(()),
            SpectrumReply::Error(s) => Err(s),
            _ => Err(String::from("Unexpected reply type in set_channel_value")),
        }
    }
}
//--------------------------- Tests ------------------------------

#[cfg(test)]
mod spproc_tests {
    use super::*;
    use crate::conditions::*;
    use crate::parameters::*;
    use crate::trace;
    use std::cmp::Ordering;

    #[test]
    fn new_1() {
        let processor = SpectrumProcessor::new();
        let mut num_spec = 0;
        for (_, _) in processor.dict.iter() {
            num_spec += 1;
        }
        assert_eq!(0, num_spec);
    }
    // for most of the tests we need, not only a SpectrumProcessor
    // but a condition dict, and a parameter dict:

    struct TestObjects {
        processor: SpectrumProcessor,
        parameters: ParameterDictionary,
        conditions: ConditionDictionary,
        tracedb: trace::SharedTraceStore,
    }
    fn make_test_objs() -> TestObjects {
        TestObjects {
            processor: SpectrumProcessor::new(),
            parameters: ParameterDictionary::new(),
            conditions: ConditionDictionary::new(),
            tracedb: trace::SharedTraceStore::new(),
        }
    }
    fn make_some_params(to: &mut TestObjects) {
        for i in 0..10 {
            let name = format!("param.{}", i);
            to.parameters.add(&name).unwrap();
        }
    }
    // Spectrum creation tests:

    #[test]
    fn create1d_1() {
        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);
        assert!(to.processor.dict.exists("test"));
        let spc = to.processor.dict.get("test");
        assert!(spc.is_some());
        let spc = spc.unwrap().borrow();

        assert_eq!(String::from("test"), spc.get_name());
        assert_eq!(String::from("1D"), spc.get_type());
        assert_eq!(String::from("param.1"), spc.get_xparams()[0]);
        assert_eq!(0, spc.get_yparams().len());

        let x = spc.get_xaxis();
        assert!(x.is_some());
        let x = x.unwrap();
        assert_eq!(
            AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1026 // under/over flow bins.
            },
            AxisSpecification {
                low: x.0,
                high: x.1,
                bins: x.2
            }
        );
        assert!(spc.get_yaxis().is_none());
        assert!(spc.get_gate().is_none());
    }
    #[test]
    fn create1d_2() {
        // bad parameter:
        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.166"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        // Checking the error string is brittle so:

        if let SpectrumReply::Error(_) = reply {
            assert!(true);
        } else {
            assert!(false);
        }
    }
    #[test]
    fn create1d_3() {
        // Duplicate spectrum::

        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        if let SpectrumReply::Error(_) = reply {
            assert!(true);
        } else {
            assert!(false);
        }
        // spectrum is still in dict:

        assert!(to.processor.dict.exists("test"));
    }
    #[test]
    fn createmulti1_1() {
        // Success for multi1d:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.7"),
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti1D {
                name: String::from("test"),
                params: params.clone(),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);
        assert!(to.processor.dict.exists("test"));
        let spc = to.processor.dict.get("test");
        assert!(spc.is_some());
        let spc = spc.unwrap().borrow();

        assert_eq!(String::from("test"), spc.get_name());
        assert_eq!(String::from("Multi1d"), spc.get_type());
        assert_eq!(params, spc.get_xparams());
        assert_eq!(0, spc.get_yparams().len());

        let x = spc.get_xaxis();
        assert!(x.is_some());
        let x = x.unwrap();
        assert_eq!(
            AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1026 // under/over flow bins.
            },
            AxisSpecification {
                low: x.0,
                high: x.1,
                bins: x.2
            }
        );
        assert!(spc.get_yaxis().is_none());
        assert!(spc.get_gate().is_none());
    }
    #[test]
    fn createmulti1_2() {
        // A Parameter does not exist:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.12"),
            String::from("param.7"),
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti1D {
                name: String::from("test"),
                params: params.clone(),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        if let SpectrumReply::Error(_) = reply {
            assert!(true);
        } else {
            assert!(false);
        }
    }
    #[test]
    fn createmulti_3() {
        // Duplicate spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.7"),
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti1D {
                name: String::from("test"),
                params: params.clone(),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti1D {
                name: String::from("test"),
                params: params.clone(),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        if let SpectrumReply::Error(_) = reply {
            assert!(true);
        } else {
            assert!(false);
        }
        assert!(to.processor.dict.exists("test"));
    }
    #[test]
    fn createmult2_1() {
        // Successfully create a multi-2:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.7"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti2D {
                name: String::from("test"),
                params: params.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
                yaxis: AxisSpecification {
                    low: -512.0,
                    high: 512.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);
        assert!(to.processor.dict.exists("test"));
        let spc = to.processor.dict.get("test");
        assert!(spc.is_some());
        let spc = spc.unwrap().borrow();

        assert_eq!(String::from("test"), spc.get_name());
        assert_eq!(String::from("Multi2d"), spc.get_type());
        assert_eq!(params, spc.get_xparams());
        assert_eq!(0, spc.get_yparams().len());

        let x = spc.get_xaxis();
        assert!(x.is_some());
        let x = x.unwrap();
        assert_eq!(
            AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1026 // under/over flow bins.
            },
            AxisSpecification {
                low: x.0,
                high: x.1,
                bins: x.2
            }
        );
        let y = spc.get_yaxis();
        assert!(y.is_some());
        let y = y.unwrap();
        assert_eq!(
            AxisSpecification {
                low: -512.0,
                high: 512.0,
                bins: 1026
            },
            AxisSpecification {
                low: y.0,
                high: y.1,
                bins: y.2
            }
        );
        assert!(spc.get_gate().is_none());
    }
    #[test]
    fn creatmult2_2() {
        // invalid parametr:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.71"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti2D {
                name: String::from("test"),
                params: params.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
                yaxis: AxisSpecification {
                    low: -512.0,
                    high: 512.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        if let SpectrumReply::Error(_) = reply {
            assert!(true);
        } else {
            assert!(false);
        }
    }
    #[test]
    fn createmult2_3() {
        // duplicate spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.7"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti2D {
                name: String::from("test"),
                params: params.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
                yaxis: AxisSpecification {
                    low: -512.0,
                    high: 512.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);
        let reply = to.processor.process_request(
            SpectrumRequest::CreateMulti2D {
                name: String::from("test"),
                params: params.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
                yaxis: AxisSpecification {
                    low: -512.0,
                    high: 512.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        if let SpectrumReply::Error(_) = reply {
            assert!(true);
        } else {
            assert!(false);
        }
        assert!(to.processor.dict.exists("test"));
    }
    #[test]
    fn createpgamma_1() {
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let xparams = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.6"),
        ];
        let yparams = vec![
            String::from("param.1"),
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreatePGamma {
                name: String::from("test"),
                xparams: xparams.clone(),
                yparams: yparams.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);
        assert!(to.processor.dict.exists("test"));
        let spc = to.processor.dict.get("test");
        assert!(spc.is_some());
        let spc = spc.unwrap().borrow(); // Ref to spectrum (readonly)
        assert_eq!(String::from("test"), spc.get_name());
        assert_eq!(String::from("PGamma"), spc.get_type());
        assert_eq!(xparams, spc.get_xparams());
        assert_eq!(yparams, spc.get_yparams());
        let x = spc.get_xaxis().expect("Missing x axis");
        assert_eq!(
            AxisSpecification {
                low: 0.0,
                high: 4096.0,
                bins: 514
            },
            AxisSpecification {
                low: x.0,
                high: x.1,
                bins: x.2
            }
        );
        let y = spc.get_yaxis().expect("Missing y axis");
        assert_eq!(
            AxisSpecification {
                low: -1.0,
                high: 1.0,
                bins: 102,
            },
            AxisSpecification {
                low: y.0,
                high: y.1,
                bins: y.2
            }
        );
        assert!(spc.get_gate().is_none());
    }
    #[test]
    fn createpgamma_2() {
        // An x parameter is bad:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let xparams = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.10"), // bad.
            String::from("param.6"),
        ];
        let yparams = vec![
            String::from("param.1"),
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreatePGamma {
                name: String::from("test"),
                xparams: xparams.clone(),
                yparams: yparams.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        // maybe is more Rusty than the earlier efforts.
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn createpgamma_3() {
        let mut to = make_test_objs();
        make_some_params(&mut to);

        // bad y parameter.
        let xparams = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.6"),
        ];
        let yparams = vec![
            String::from("param.11"), // bad.
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreatePGamma {
                name: String::from("test"),
                xparams: xparams.clone(),
                yparams: yparams.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn createpgamma_4() {
        // Duplicate spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let xparams = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.6"),
        ];
        let yparams = vec![
            String::from("param.1"),
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreatePGamma {
                name: String::from("test"),
                xparams: xparams.clone(),
                yparams: yparams.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);
        let reply = to.processor.process_request(
            SpectrumRequest::CreatePGamma {
                name: String::from("test"),
                xparams: xparams.clone(),
                yparams: yparams.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn crsummary_1() {
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.8"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreateSummary {
                name: String::from("test"),
                params: params.clone(),
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);
        assert!(to.processor.dict.exists("test"));

        let spec = to
            .processor
            .dict
            .get("test")
            .expect("Missing summary spectrum")
            .borrow();
        assert_eq!(String::from("test"), spec.get_name());
        assert_eq!(String::from("Summary"), spec.get_type());
        assert_eq!(params, spec.get_xparams());
        assert_eq!(0, spec.get_yparams().len());
        assert!(spec.get_xaxis().is_none());
        let y = spec.get_yaxis().expect("Missing y axis ");
        assert_eq!(
            AxisSpecification {
                low: 0.0,
                high: 1.0,
                bins: 102,
            },
            AxisSpecification {
                low: y.0,
                high: y.1,
                bins: y.2
            }
        );
        assert!(spec.get_gate().is_none());
    }
    #[test]
    fn crsummary_2() {
        // bad parameter name:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.14"), // bad
            String::from("param.8"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreateSummary {
                name: String::from("test"),
                params: params.clone(),
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn crsummary_3() {
        // duplicate spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.8"),
        ];
        let reply = to.processor.process_request(
            SpectrumRequest::CreateSummary {
                name: String::from("test"),
                params: params.clone(),
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);
        let reply = to.processor.process_request(
            SpectrumRequest::CreateSummary {
                name: String::from("test"),
                params: params.clone(),
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1.0,
                    bins: 100,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn cr2d_1() {
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.5"),
                yparam: String::from("param.7"),
                xaxis: AxisSpecification {
                    low: -10.0,
                    high: 10.0,
                    bins: 100,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 256,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);
        let spec = to
            .processor
            .dict
            .get("test")
            .expect("Missing spectru")
            .borrow();

        assert_eq!(String::from("test"), spec.get_name());
        assert_eq!(String::from("2D"), spec.get_type());
        let xp = spec.get_xparams();
        assert_eq!(1, xp.len());
        assert_eq!(String::from("param.5"), xp[0]);
        let yp = spec.get_yparams();
        assert_eq!(1, yp.len());
        assert_eq!(String::from("param.7"), yp[0]);

        let x = spec.get_xaxis().expect("Missing x axis");
        assert_eq!(
            AxisSpecification {
                low: -10.0,
                high: 10.0,
                bins: 102
            },
            AxisSpecification {
                low: x.0,
                high: x.1,
                bins: x.2
            }
        );
        let y = spec.get_yaxis().expect("Missing y axis");
        assert_eq!(
            AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 258
            },
            AxisSpecification {
                low: y.0,
                high: y.1,
                bins: y.2
            }
        );
        assert!(spec.get_gate().is_none());
    }
    #[test]
    fn cr2d_2() {
        // invalid x parameter.

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.15"),
                yparam: String::from("param.7"),
                xaxis: AxisSpecification {
                    low: -10.0,
                    high: 10.0,
                    bins: 100,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 256,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn cr2d_3() {
        // invalid y parameter;

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.5"),
                yparam: String::from("param.17"),
                xaxis: AxisSpecification {
                    low: -10.0,
                    high: 10.0,
                    bins: 100,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 256,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn cr2d_4() {
        // duplicate spectrum:
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.5"),
                yparam: String::from("param.7"),
                xaxis: AxisSpecification {
                    low: -10.0,
                    high: 10.0,
                    bins: 100,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 256,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.5"),
                yparam: String::from("param.7"),
                xaxis: AxisSpecification {
                    low: -10.0,
                    high: 10.0,
                    bins: 100,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 256,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn cr2dsum_1() {
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let xpars = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.6"),
            String::from("param.7"),
        ];
        let ypars = vec![
            String::from("param.1"),
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
            String::from("param.9"),
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Create2DSum {
                name: String::from("test"),
                xparams: xpars.clone(),
                yparams: ypars.clone(),
                xaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let spec = to
            .processor
            .dict
            .get("test")
            .expect("Could not find spectrum")
            .borrow();
        assert_eq!(String::from("test"), spec.get_name());
        assert_eq!(String::from("2DSum"), spec.get_type());
        assert_eq!(xpars, spec.get_xparams());
        assert_eq!(ypars, spec.get_yparams());
        assert!(spec.get_gate().is_none());
        let x = spec.get_xaxis().expect("Missing x axis");
        assert_eq!(
            AxisSpecification {
                low: -1.0,
                high: 1.0,
                bins: 514,
            },
            AxisSpecification {
                low: x.0,
                high: x.1,
                bins: x.2
            }
        );
        let y = spec.get_yaxis().expect("Missing y axis");
        assert_eq!(
            AxisSpecification {
                low: 0.0,
                high: 4096.0,
                bins: 514,
            },
            AxisSpecification {
                low: y.0,
                high: y.1,
                bins: y.2
            }
        );
    }
    #[test]
    fn cr2dsum_2() {
        // bad x parameter:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let xpars = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.16"),
            String::from("param.7"),
        ];
        let ypars = vec![
            String::from("param.1"),
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
            String::from("param.9"),
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Create2DSum {
                name: String::from("test"),
                xparams: xpars.clone(),
                yparams: ypars.clone(),
                xaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn cr2dsum_3() {
        // bad y parameter:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let xpars = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.6"),
            String::from("param.7"),
        ];
        let ypars = vec![
            String::from("param.11"),
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
            String::from("param.9"),
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Create2DSum {
                name: String::from("test"),
                xparams: xpars.clone(),
                yparams: ypars.clone(),
                xaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn cr2dsum_4() {
        // duplicate spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let xpars = vec![
            String::from("param.0"),
            String::from("param.2"),
            String::from("param.4"),
            String::from("param.6"),
            String::from("param.7"),
        ];
        let ypars = vec![
            String::from("param.1"),
            String::from("param.3"),
            String::from("param.5"),
            String::from("param.7"),
            String::from("param.9"),
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Create2DSum {
                name: String::from("test"),
                xparams: xpars.clone(),
                yparams: ypars.clone(),
                xaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2DSum {
                name: String::from("test"),
                xparams: xpars.clone(),
                yparams: ypars.clone(),
                xaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 4096.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn del_1() {
        // delete an existing spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::Delete(String::from("test")),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Deleted, reply);
        assert!(!to.processor.dict.exists("test"));
    }
    #[test]
    fn del_2() {
        // the right one is deleted:

        // delete an existing spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("test.{}", i);
            let pname = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: pname,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }

        let reply = to.processor.process_request(
            SpectrumRequest::Delete(String::from("test.5")),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Deleted, reply);
        assert!(!to.processor.dict.exists("test.5"));
    }
    #[test]
    fn del_3() {
        // Delete nonexisting is an error:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("test.{}", i);
            let pname = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: pname,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }
        let reply = to.processor.process_request(
            SpectrumRequest::Delete(String::from("param.1")),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn clear_1() {
        // Put some data in a histogram then clear it

        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("test.{}", i);
            let pname = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: pname,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }
        let spec = to.processor.dict.get("test.1").expect("Missing spectrum");
        let h = spec
            .borrow()
            .get_histogram_1d()
            .expect("Not 1d but should be");
        h.borrow_mut().fill(&100.0);
        h.borrow_mut().fill(&110.0);

        // good enough for now I suspect clear them all.

        let reply = to.processor.process_request(
            SpectrumRequest::Clear(String::from("*")),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Cleared, reply);
        let mut sum = 0.0;
        for c in h.borrow().iter() {
            sum += c.value.get();
        }
        assert_eq!(0.0, sum);
    }
    #[test]
    fn clear_2() {
        // pattern selectivity:
        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("test.{}", i);
            let pname = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: pname,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }
        let spec = to.processor.dict.get("test.1").expect("Missing spectrum");
        let h = spec
            .borrow()
            .get_histogram_1d()
            .expect("Not 1d but should be");
        h.borrow_mut().fill(&100.0);
        h.borrow_mut().fill(&110.0);

        // Clear the 'wrong' one:

        let reply = to.processor.process_request(
            SpectrumRequest::Clear(String::from("test.2")),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Cleared, reply);
        let mut sum = 0.0;
        for c in h.borrow().iter() {
            sum += c.value.get();
        }
        assert_eq!(2.0, sum); // did not clear.

        // clear the right one:

        let reply = to.processor.process_request(
            SpectrumRequest::Clear(String::from("test.1")),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Cleared, reply);
        let mut sum = 0.0;
        for c in h.borrow().iter() {
            sum += c.value.get();
        }
        assert_eq!(0.0, sum);
    }
    #[test]
    fn list_1() {
        // list all spectra.

        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("test.{}", i);
            let pname = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: pname,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }

        let reply = to.processor.process_request(
            SpectrumRequest::List(String::from("*")),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        if let SpectrumReply::Listing(mut l) = reply {
            assert_eq!(10, l.len());

            // There's no ordering so order by name:
            l.sort_by(|a, b| {
                if a.name > b.name {
                    Ordering::Greater
                } else if a.name < b.name {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            });

            // /The listing comes in an arbitrary order so:

            for i in 0..10 {
                let name = format!("test.{}", i);
                let pname = format!("param.{}", i);

                assert_eq!(name, l[i].name);
                assert_eq!(String::from("1D"), l[i].type_name);
                assert_eq!(vec![pname], l[i].xparams);
                assert_eq!(0, l[i].yparams.len());
                assert!(l[i].yaxis.is_none());

                assert_eq!(
                    AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1026,
                    },
                    l[i].xaxis.expect("No x axis")
                );
                assert!(l[i].gate.is_none());
            }
        } else {
            panic!("listing failed");
        }
    }
    #[test]
    fn list_2() {
        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("test.{}", i);
            let pname = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: pname,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }

        let reply = to.processor.process_request(
            SpectrumRequest::List(String::from("test.9")),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        if let SpectrumReply::Listing(l) = reply {
            assert_eq!(1, l.len());
            // Just check the name as we know the rest is ok from
            // list_1:

            assert_eq!(String::from("test.9"), l[0].name);
        } else {
            panic!("Listing failed");
        }
    }

    // For our gate test we need some gates:

    fn make_some_gates(cd: &mut ConditionDictionary) {
        for i in 0..10 {
            let name = format!("cond.{}", i);
            cd.insert(name, Rc::new(RefCell::new(Box::new(conditions::True {}))));
        }
    }
    #[test]
    fn gate_1() {
        let mut to = make_test_objs();
        make_some_params(&mut to);
        make_some_gates(&mut to.conditions);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::Gate {
                spectrum: String::from("test"),
                gate: String::from("cond.5"),
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Gated, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::List(String::from("*")),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        if let SpectrumReply::Listing(l) = reply {
            assert_eq!(1, l.len());
            assert_eq!(
                String::from("cond.5"),
                l[0].clone().gate.expect("Missing gate")
            );
        } else {
            panic!("Listing failed");
        }
    }
    #[test]
    fn gate_2() {
        // No such gate:
        let mut to = make_test_objs();
        make_some_params(&mut to);
        make_some_gates(&mut to.conditions);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::Gate {
                spectrum: String::from("test"),
                gate: String::from("kond.5"),
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn gate_3() {
        // no such spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        make_some_gates(&mut to.conditions);
        let reply = to.processor.process_request(
            SpectrumRequest::Gate {
                spectrum: String::from("test"),
                gate: String::from("cond.5"),
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }

    #[test]
    fn ungate_1() {
        // Good ungate:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        make_some_gates(&mut to.conditions);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::Gate {
                spectrum: String::from("test"),
                gate: String::from("cond.5"),
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Gated, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::Ungate(String::from("test")),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Ungated, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::List(String::from("test")),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Listing(l) = reply {
            assert_eq!(1, l.len());
            assert!(l[0].gate.is_none());
            true
        } else {
            false
        });
    }
    #[test]
    fn ungate_2() {
        // no such spectrum

        let mut to = make_test_objs();
        make_some_params(&mut to);
        make_some_gates(&mut to.conditions);

        let reply = to.processor.process_request(
            SpectrumRequest::Ungate(String::from("test")),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn events_1() {
        // Increment some spectra via an event:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("spec.{}", i);
            let par = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: par,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }
        // Make some events and fill some (not all) of the spectra:

        let id1 = to.parameters.lookup("param.5").unwrap().get_id();
        let id2 = to.parameters.lookup("param.7").unwrap().get_id();

        let events = vec![
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Events(events),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Processed, reply);
        let with_counts = vec![
            (String::from("spec.5"), 512.0),
            (String::from("spec.7"), 700.0),
        ];
        let no_counts = vec![
            String::from("spec.0"),
            String::from("spec.1"),
            String::from("spec.2"),
            String::from("spec.3"),
            String::from("spec.4"),
            String::from("spec.6"),
            String::from("spec.8"),
            String::from("spec.9"),
        ];
        // These should havve counts in the indicated channels:

        for (name, chan) in with_counts {
            let spec = to.processor.dict.get(&name).unwrap().borrow();
            for ch in spec.get_histogram_1d().unwrap().borrow().iter() {
                let d = ch.value.get();
                if d != 0.0 {
                    assert_eq!(5.0, d);
                    if let BinInterval::Bin { start, end: _end } = ch.bin {
                        assert_eq!(chan, start);
                    } else {
                        panic!("Under or overflow counts in histogram");
                    }
                }
            }
        }
        // these should have no counts.
        for name in no_counts {
            let spec = to.processor.dict.get(&name).unwrap().borrow();
            for ch in spec.get_histogram_1d().unwrap().borrow().iter() {
                assert_eq!(0.0, ch.value.get());
            }
        }
    }
    #[test]
    fn contents_1() {
        // Process some events as in events_1, get the
        // contents of the spectra...
        // should be one channel entry for each of the
        // two histograms with data and non for those with none

        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("spec.{}", i);
            let par = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: par,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }
        // Make some evnts and fill some (not all) of the spectra:

        let id1 = to.parameters.lookup("param.5").unwrap().get_id();
        let id2 = to.parameters.lookup("param.7").unwrap().get_id();

        let events = vec![
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Events(events),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Processed, reply);
        let with_counts = vec![
            (String::from("spec.5"), 512.0),
            (String::from("spec.7"), 700.0),
        ];
        let no_counts = vec![
            String::from("spec.0"),
            String::from("spec.1"),
            String::from("spec.2"),
            String::from("spec.3"),
            String::from("spec.4"),
            String::from("spec.6"),
            String::from("spec.8"),
            String::from("spec.9"),
        ];
        // we'll ask for the entire ROI:
        for (name, chan) in with_counts {
            let reply = to.processor.process_request(
                SpectrumRequest::GetContents {
                    name: name,
                    xlow: 0.0,
                    xhigh: 1024.0,
                    ylow: 0.0,
                    yhigh: 0.0,
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            );
            assert!(if let SpectrumReply::Contents(sc) = reply {
                assert_eq!(1, sc.len());
                assert_eq!(ChannelType::Bin, sc[0].chan_type);
                assert_eq!(chan, sc[0].x);
                assert_eq!(5.0, sc[0].value);
                true
            } else {
                false
            });
        }
        // Nobody else should have counts:

        for name in no_counts {
            let reply = to.processor.process_request(
                SpectrumRequest::GetContents {
                    name: name,
                    xlow: 0.0,
                    xhigh: 1024.0,
                    ylow: 0.0,
                    yhigh: 0.0,
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            );
            assert!(if let SpectrumReply::Contents(sc) = reply {
                assert_eq!(0, sc.len());
                true
            } else {
                false
            });
        }
    }
    #[test]
    fn contents_2() {
        // Ask with ROI outside of where counts are:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        for i in 0..10 {
            let name = format!("spec.{}", i);
            let par = format!("param.{}", i);
            let reply = to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: name,
                    parameter: par,
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024,
                    },
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            );
            assert_eq!(SpectrumReply::Created, reply);
        }
        // Make some evnts and fill some (not all) of the spectra:

        let id1 = to.parameters.lookup("param.5").unwrap().get_id();
        let id2 = to.parameters.lookup("param.7").unwrap().get_id();

        let events = vec![
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Events(events),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Processed, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("spec.5"),
                xlow: 0.0,
                xhigh: 200.0,
                ylow: 0.0,
                yhigh: 0.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Contents(sc) = reply {
            assert_eq!(0, sc.len());
            true
        } else {
            false
        });
    }
    #[test]
    fn events_2() {
        // Events for a 2-d histogram:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.5"),
                yparam: String::from("param.7"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        // Make and process events that will fill (512.0, 700.0):

        let id1 = to.parameters.lookup("param.5").unwrap().get_id();
        let id2 = to.parameters.lookup("param.7").unwrap().get_id();

        let events = vec![
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Events(events),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Processed, reply);

        // Contents over the whole spectrum should only have 5
        // counts in channel 512.0, 700.0

        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 0.0,
                xhigh: 1024.0,
                ylow: 0.0,
                yhigh: 1024.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Contents(l) = reply {
            assert_eq!(1, l.len());
            assert_eq!(ChannelType::Bin, l[0].chan_type);
            assert_eq!(5.0, l[0].value);
            assert_eq!(512.0, l[0].x);
            assert_eq!(700.0, l[0].y);
            true
        } else {
            false
        });
    }
    #[test]
    fn contents_3() {
        // 2d ROI checking:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.5"),
                yparam: String::from("param.7"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        // Make and process events that will fill (512.0, 700.0):

        let id1 = to.parameters.lookup("param.5").unwrap().get_id();
        let id2 = to.parameters.lookup("param.7").unwrap().get_id();

        let events = vec![
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
            vec![
                EventParameter::new(id1, 512.0),
                EventParameter::new(id2, 700.0),
            ],
        ];

        let reply = to.processor.process_request(
            SpectrumRequest::Events(events),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Processed, reply);

        // Contents over the whole spectrum should only have 5
        // counts in channel 512.0, 700.0

        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 0.0,
                xhigh: 1024.0,
                ylow: 100.0,
                yhigh: 300.0, // Too small for ROI.
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Contents(l) = reply {
            assert_eq!(0, l.len());
            true
        } else {
            false
        });
        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 0.0,
                xhigh: 1024.0,
                ylow: 720.0, // Too large for ROI
                yhigh: 1024.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Contents(l) = reply {
            assert_eq!(0, l.len());
            true
        } else {
            false
        });

        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 0.0,
                xhigh: 200.0, // Small for ROI
                ylow: 0.0,
                yhigh: 1024.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Contents(l) = reply {
            assert_eq!(0, l.len());
            true
        } else {
            false
        });
        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 600.0, // too big for ROI.
                xhigh: 1024.0,
                ylow: 0.0,
                yhigh: 1024.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Contents(l) = reply {
            assert_eq!(0, l.len());
            true
        } else {
            false
        });
    }
    #[test]
    fn specstats_1() {
        // Get the statistics from a spectrum.
        // Note the statistics functions themselves are tested in
        // spectrum/mod.rs so we only check that the right
        // things get returned

        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.5"),
                yparam: String::from("param.7"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::GetStats(String::from("test")),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Statistics(s) = reply {
            assert_eq!((0, 0, 0, 0), s);
            true
        } else {
            false
        });
        // IF we use the wrong name:

        assert!(
            if let SpectrumReply::Error(_) = to.processor.process_request(
                SpectrumRequest::GetStats(String::from("none")),
                &to.parameters,
                &mut to.conditions,
                &to.tracedb
            ) {
                true
            } else {
                false
            }
        );
    }
    #[test]
    fn load_1() {
        // Load 1d spectrum contents:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        // Load the spectrum up with some data:

        let req = SpectrumRequest::SetContents {
            name: String::from("test"),
            contents: vec![
                Channel {
                    chan_type: ChannelType::Bin,
                    x: 0.0,
                    y: 0.0,
                    bin: 1,
                    value: 1.0,
                },
                Channel {
                    chan_type: ChannelType::Bin,
                    x: 10.0,
                    y: 0.0,
                    bin: 10,
                    value: 12.0,
                },
            ],
        };

        let reply =
            to.processor
                .process_request(req, &to.parameters, &mut to.conditions, &to.tracedb);
        assert_eq!(SpectrumReply::Processed, reply);

        // See if the contents match:

        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 0.0,
                xhigh: 1024.0,
                ylow: 0.0,
                yhigh: 0.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        assert!(if let SpectrumReply::Contents(c) = reply {
            assert_eq!(2, c.len());
            //There's an assumption stuff comes out in order:
            assert_eq!(0.0, c[0].x);
            assert_eq!(1.0, c[0].value);

            assert_eq!(10.0, c[1].x);
            assert_eq!(12.0, c[1].value);
            true
        } else {
            false
        });
    }
    #[test]
    fn load_2() {
        // Load a 2d spectrum.

        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.1"),
                yparam: String::from("param.2"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        // Load up some data:

        let reply = to.processor.process_request(
            SpectrumRequest::SetContents {
                name: String::from("test"),
                contents: vec![
                    Channel {
                        chan_type: ChannelType::Bin,
                        x: 10.0,
                        y: 10.0,
                        bin: 102,
                        value: 15.0,
                    },
                    Channel {
                        chan_type: ChannelType::Bin,
                        x: 20.0,
                        y: 26.0, // Note bin granularity means we only get even y.
                        bin: 502,
                        value: 172.0,
                    },
                ],
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Processed, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 0.0,
                xhigh: 1024.0,
                ylow: 0.0,
                yhigh: 1024.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        assert!(if let SpectrumReply::Contents(c) = reply {
            assert_eq!(2, c.len());

            // assume some ordering to the iteration:

            assert_eq!(10.0, c[0].x);
            assert_eq!(10.0, c[0].y);
            assert_eq!(15.0, c[0].value);

            assert_eq!(20.0, c[1].x);
            assert_eq!(26.0, c[1].y);
            assert_eq!(172.0, c[1].value);
            true
        } else {
            false
        });
    }
    #[test]
    fn load_3() {
        // Summary spectra are wonky enough they deserve their own test:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        let reply = to.processor.process_request(
            SpectrumRequest::CreateSummary {
                name: String::from("test"),
                params: vec![
                    String::from("param.1"), // x = 0.0,
                    String::from("param.2"), // x = 1.0,
                    String::from("param.3"), // x = 2.0,
                    String::from("param.4"), // x = 3.0
                ],
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::SetContents {
                name: String::from("test"),
                contents: vec![
                    Channel {
                        chan_type: ChannelType::Bin,
                        x: 0.0, // param.1
                        y: 12.0,
                        bin: 0, // ignored
                        value: 1.0,
                    },
                    Channel {
                        chan_type: ChannelType::Bin,
                        x: 1.0, // param.2
                        y: 100.0,
                        bin: 0,
                        value: 2.0,
                    },
                    Channel {
                        chan_type: ChannelType::Bin,
                        x: 2.0, // param.3
                        y: 200.0,
                        bin: 0,
                        value: 128.0,
                    },
                    Channel {
                        chan_type: ChannelType::Bin,
                        x: 3.0, // param.4
                        y: 250.0,
                        bin: 0,
                        value: 100.0,
                    },
                ],
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Processed, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 0.0,
                xhigh: 4.0,
                ylow: 0.0,
                yhigh: 1024.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Contents(c) = reply {
            assert_eq!(4, c.len());
            true
        } else {
            false
        });
    }
    #[test]
    fn getchan1_1() {
        // Get channel from 1d spectrum -  in range.
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        // Put a value in bin 512 (exclusive of 0 which is the underflow):
        // Block so that the borrow is given back:
        {
            let spc = to.processor.dict.get("test").unwrap().borrow();

            spc.get_histogram_1d()
                .unwrap()
                .borrow_mut()
                .value_at_index_mut(512)
                .unwrap()
                .fill_with(1234.0);
        }

        // Now ask for the value of bin 512:

        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: 511,
                ychan: None,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::ChannelValue(1234.0), reply);
    }
    #[test]
    fn getchan1_2() {
        // get channel from 1d spectrum - index too small.
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: -2, // -1 is underflow.
                ychan: None,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn getchan1_3() {
        // get channel from 1d spectum index too big.
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: 1026, // 1025 is overflows.
                ychan: None,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn getchan1_4() {
        // Get undeflow channel
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);
        // We do this in a block to drop the borrow at the end
        // otherwise I don't think the processor can then borrow
        // the spectrum to give us the value.
        {
            let spc = to.processor.dict.get("test").unwrap().borrow();

            spc.get_histogram_1d()
                .unwrap()
                .borrow_mut()
                .value_at_index_mut(0) // underflow channel
                .unwrap()
                .fill_with(1234.0);
        }
        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: -1, // underflow channel.
                ychan: None,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        assert_eq!(SpectrumReply::ChannelValue(1234.0), reply);
    }
    #[test]
    fn getchan1_5() {
        // Get overflow channel.
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        // We do this in a block to drop the borrow at the end
        // otherwise I don't think the processor can then borrow
        // the spectrum to give us the value.
        {
            let spc = to.processor.dict.get("test").unwrap().borrow();

            spc.get_histogram_1d()
                .unwrap()
                .borrow_mut()
                .value_at_index_mut(1025) // overflow channel
                .unwrap()
                .fill_with(1234.0);
        }
        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: 1024, // underflow channel.
                ychan: None,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        assert_eq!(SpectrumReply::ChannelValue(1234.0), reply);
    }
    #[test]
    fn getchan2_1() {
        // X/Y in range.

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.1"),
                yparam: String::from("param.2"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);
        // increment 512.0, 512.0 (bin 256, 256)
        // in a block so the borrow is released
        {
            let spc = to.processor.dict.get("test").unwrap().borrow();
            spc.get_histogram_2d()
                .unwrap()
                .borrow_mut()
                .fill(&(512.0, 512.0));
        }
        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: 256,
                ychan: Some(256),
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::ChannelValue(1.0), reply);
    }
    #[test]
    fn getchan2_2() {
        // x too small
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.1"),
                yparam: String::from("param.2"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        // Set bin 256+256*514 - to 1234.0 - that's 255,255 in external
        // bin coords:

        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: -2, // -1 is underflow.
                ychan: Some(255),
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn getchan2_3() {
        // x too big.
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.1"),
                yparam: String::from("param.2"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: 513,
                ychan: Some(0), // 512 is overflow.
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn getchan2_4() {
        // y too small.
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.1"),
                yparam: String::from("param.2"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: 512,
                ychan: Some(-2),
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn getchan2_5() {
        // y too big.
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.1"),
                yparam: String::from("param.2"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: 0,
                ychan: Some(513),
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn getchan2_6() {
        // an x underflow
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.1"),
                yparam: String::from("param.2"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        // increment coordinate (-1.0, 512.0) that's an x underflow
        // on the 256 line.

        {
            let spc = to.processor.dict.get("test").unwrap().borrow();
            spc.get_histogram_2d()
                .unwrap()
                .borrow_mut()
                .fill(&(-1.0, 512.0));
        }
        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: -1,
                ychan: Some(256),
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        assert_eq!(SpectrumReply::ChannelValue(1.0), reply);
    }
    #[test]
    fn getchan2_7() {
        // an x overflow
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.1"),
                yparam: String::from("param.2"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        // Set increment x channel at coordinate 1025.0 and y 512.0  that should
        // be an overflow an the y bin 256
        {
            let spc = to.processor.dict.get("test").unwrap().borrow();
            spc.get_histogram_2d()
                .unwrap()
                .borrow_mut()
                .fill(&(1025.0, 512.0));
        }
        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: 512,
                ychan: Some(256),
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::ChannelValue(1.0), reply);
    }
    #[test]
    fn getchan2_8() {
        // a y underflow
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.1"),
                yparam: String::from("param.2"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        // Y underflow:  set coordinate 512.0, -2.0  that's
        // bin (256, -1) in our coords for underflow

        {
            let spc = to.processor.dict.get("test").unwrap().borrow();
            spc.get_histogram_2d()
                .unwrap()
                .borrow_mut()
                .fill(&(512.0, -2.0));
        }

        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: 256,
                ychan: Some(-1),
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::ChannelValue(1.0), reply);
    }
    #[test]
    fn getchan2_9() {
        // a y overflow.
        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.1"),
                yparam: String::from("param.2"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        // Increment coordinate 512, 1025.0
        // This will be 256, 512 in bin space.

        {
            let spc = to.processor.dict.get("test").unwrap().borrow();
            spc.get_histogram_2d()
                .unwrap()
                .borrow_mut()
                .fill(&(512.0, 1025.0));
        }

        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: 256,
                ychan: Some(512),
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::ChannelValue(1.0), reply);
    }
    #[test]
    fn getchan2_10() {
        // ybin cannot be None

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let reply = to.processor.process_request(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("param.1"),
                yparam: String::from("param.2"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
                yaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 512,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert_eq!(SpectrumReply::Created, reply);

        let reply = to.processor.process_request(
            SpectrumRequest::GetChan {
                name: String::from("test"),
                xchan: 256,
                ychan: None,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    // Channel setting --
    #[test]
    fn setchan1_1() {
        // Good set of 1d channnel:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: String::from("test"),
                    parameter: String::from("param.1"),
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
        // Set channel 512 to 1234.0:

        assert_eq!(
            SpectrumReply::ChannelSet,
            to.processor.process_request(
                SpectrumRequest::SetChan {
                    name: String::from("test"),
                    xchan: 512,
                    ychan: None,
                    value: 12345.0
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );

        // Get the value --

        assert_eq!(
            SpectrumReply::ChannelValue(12345.0),
            to.processor.process_request(
                SpectrumRequest::GetChan {
                    name: String::from("test"),
                    xchan: 512,
                    ychan: None
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
    }
    #[test]
    fn setchan1_2() {
        // 1d spectrum - underflow channel:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: String::from("test"),
                    parameter: String::from("param.1"),
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
        // Set channel -1 to 1234.0:

        assert_eq!(
            SpectrumReply::ChannelSet,
            to.processor.process_request(
                SpectrumRequest::SetChan {
                    name: String::from("test"),
                    xchan: -1,
                    ychan: None,
                    value: 12345.0
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );

        // Get the value --

        assert_eq!(
            SpectrumReply::ChannelValue(12345.0),
            to.processor.process_request(
                SpectrumRequest::GetChan {
                    name: String::from("test"),
                    xchan: -1,
                    ychan: None
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
    }
    #[test]
    fn setchan1_3() {
        // Overflow channel:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: String::from("test"),
                    parameter: String::from("param.1"),
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
        // Set channel 1024 to 1234.0:

        assert_eq!(
            SpectrumReply::ChannelSet,
            to.processor.process_request(
                SpectrumRequest::SetChan {
                    name: String::from("test"),
                    xchan: 1024,
                    ychan: None,
                    value: 12345.0
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );

        // Get the value --

        assert_eq!(
            SpectrumReply::ChannelValue(12345.0),
            to.processor.process_request(
                SpectrumRequest::GetChan {
                    name: String::from("test"),
                    xchan: 1024,
                    ychan: None
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
    }
    #[test]
    fn setchan1_4() {
        // channel number is too small

        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: String::from("test"),
                    parameter: String::from("param.1"),
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
        // Set channel -2 (too small)

        let reply = to.processor.process_request(
            SpectrumRequest::SetChan {
                name: String::from("test"),
                xchan: -2,
                ychan: None,
                value: 12345.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn setchan1_5() {
        // Channel too big:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create1D {
                    name: String::from("test"),
                    parameter: String::from("param.1"),
                    axis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
        // Set channel 1025 (too small)

        let reply = to.processor.process_request(
            SpectrumRequest::SetChan {
                name: String::from("test"),
                xchan: 1025,
                ychan: None,
                value: 12345.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    // Setchan for 2-d spectra:

    #[test]
    fn setchan2_1() {
        // Middle of a 2-d spectrum:

        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create2D {
                    name: String::from("test"),
                    xparam: String::from("param.1"),
                    yparam: String::from("param.2"),
                    xaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    },
                    yaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );

        // Set channel 128,128 to 12345:

        assert_eq!(
            SpectrumReply::ChannelSet,
            to.processor.process_request(
                SpectrumRequest::SetChan {
                    name: String::from("test"),
                    xchan: 128,
                    ychan: Some(128),
                    value: 12345.0
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
        // Ensure it got set:

        assert_eq!(
            SpectrumReply::ChannelValue(12345.0),
            to.processor.process_request(
                SpectrumRequest::GetChan {
                    name: String::from("test"),
                    xchan: 128,
                    ychan: Some(128)
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
    }
    #[test]
    fn setchan2_2() {
        // X underflow.
        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create2D {
                    name: String::from("test"),
                    xparam: String::from("param.1"),
                    yparam: String::from("param.2"),
                    xaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    },
                    yaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );

        // Set channel -1,128 to 12345:

        assert_eq!(
            SpectrumReply::ChannelSet,
            to.processor.process_request(
                SpectrumRequest::SetChan {
                    name: String::from("test"),
                    xchan: -1,
                    ychan: Some(128),
                    value: 12345.0
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
        // Ensure it got set:

        assert_eq!(
            SpectrumReply::ChannelValue(12345.0),
            to.processor.process_request(
                SpectrumRequest::GetChan {
                    name: String::from("test"),
                    xchan: -1,
                    ychan: Some(128)
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
    }
    #[test]
    fn setchan2_3() {
        // x overflow

        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create2D {
                    name: String::from("test"),
                    xparam: String::from("param.1"),
                    yparam: String::from("param.2"),
                    xaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    },
                    yaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );

        // Set channel 256,128 to 12345:

        assert_eq!(
            SpectrumReply::ChannelSet,
            to.processor.process_request(
                SpectrumRequest::SetChan {
                    name: String::from("test"),
                    xchan: 256,
                    ychan: Some(128),
                    value: 12345.0
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
        // Ensure it got set:

        assert_eq!(
            SpectrumReply::ChannelValue(12345.0),
            to.processor.process_request(
                SpectrumRequest::GetChan {
                    name: String::from("test"),
                    xchan: 256,
                    ychan: Some(128)
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
    }
    #[test]
    fn setchan2_4() {
        // y underflow

        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create2D {
                    name: String::from("test"),
                    xparam: String::from("param.1"),
                    yparam: String::from("param.2"),
                    xaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    },
                    yaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );

        // Set channel 128,-1 to 12345:

        assert_eq!(
            SpectrumReply::ChannelSet,
            to.processor.process_request(
                SpectrumRequest::SetChan {
                    name: String::from("test"),
                    xchan: 128,
                    ychan: Some(-1),
                    value: 12345.0
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
        // Ensure it got set:

        assert_eq!(
            SpectrumReply::ChannelValue(12345.0),
            to.processor.process_request(
                SpectrumRequest::GetChan {
                    name: String::from("test"),
                    xchan: 128,
                    ychan: Some(-1)
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
    }
    #[test]
    fn setchan2_5() {
        // y overflow

        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create2D {
                    name: String::from("test"),
                    xparam: String::from("param.1"),
                    yparam: String::from("param.2"),
                    xaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    },
                    yaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );

        // Set channel 128,256 to 12345:

        assert_eq!(
            SpectrumReply::ChannelSet,
            to.processor.process_request(
                SpectrumRequest::SetChan {
                    name: String::from("test"),
                    xchan: 128,
                    ychan: Some(256),
                    value: 12345.0
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
        // Ensure it got set:

        assert_eq!(
            SpectrumReply::ChannelValue(12345.0),
            to.processor.process_request(
                SpectrumRequest::GetChan {
                    name: String::from("test"),
                    xchan: 128,
                    ychan: Some(256)
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );
    }
    #[test]
    fn setchan2_6() {
        // xchannel too small

        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create2D {
                    name: String::from("test"),
                    xparam: String::from("param.1"),
                    yparam: String::from("param.2"),
                    xaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    },
                    yaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );

        let reply = to.processor.process_request(
            SpectrumRequest::SetChan {
                name: String::from("test"),
                xchan: -2,
                ychan: Some(128),
                value: 12345.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn setchan2_7() {
        // xchannel too big.
        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create2D {
                    name: String::from("test"),
                    xparam: String::from("param.1"),
                    yparam: String::from("param.2"),
                    xaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    },
                    yaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );

        let reply = to.processor.process_request(
            SpectrumRequest::SetChan {
                name: String::from("test"),
                xchan: 257,
                ychan: Some(128),
                value: 12345.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn setchan2_8() {
        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create2D {
                    name: String::from("test"),
                    xparam: String::from("param.1"),
                    yparam: String::from("param.2"),
                    xaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    },
                    yaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );

        let reply = to.processor.process_request(
            SpectrumRequest::SetChan {
                name: String::from("test"),
                xchan: 129,
                ychan: Some(-2),
                value: 12345.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn setchan2_9() {
        // ychannel too big.

        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create2D {
                    name: String::from("test"),
                    xparam: String::from("param.1"),
                    yparam: String::from("param.2"),
                    xaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    },
                    yaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );

        let reply = to.processor.process_request(
            SpectrumRequest::SetChan {
                name: String::from("test"),
                xchan: 129,
                ychan: Some(257),
                value: 12345.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn setchan2_10() {
        let mut to = make_test_objs();
        make_some_params(&mut to);

        assert_eq!(
            SpectrumReply::Created,
            to.processor.process_request(
                SpectrumRequest::Create2D {
                    name: String::from("test"),
                    xparam: String::from("param.1"),
                    yparam: String::from("param.2"),
                    xaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    },
                    yaxis: AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 256
                    }
                },
                &to.parameters,
                &mut to.conditions,
                &to.tracedb,
            )
        );

        let reply = to.processor.process_request(
            SpectrumRequest::SetChan {
                name: String::from("test"),
                xchan: 129,
                ychan: None,
                value: 12345.0,
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn fold_1() {
        let mut to = make_test_objs();

        // Try to fold with no such spectrum:

        to.conditions.insert(
            String::from("true"),
            Rc::new(RefCell::new(Box::new(True {}))),
        );
        let reply = to.processor.process_request(
            SpectrumRequest::Fold {
                spectrum_name: String::from("junk"),
                condition_name: String::from("true"),
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );
        assert!(if let SpectrumReply::Error(_) = reply {
            true
        } else {
            false
        });
    }
    #[test]
    fn fold_2() {
        // Have the spectrum (and it's even ok) but condition not defined is
        // also an error:

        let mut to = make_test_objs();
        make_some_params(&mut to);
        let mut spec = spectra::Multi1d::new(
            "test",
            vec![
                String::from("param.0"),
                String::from("param.1"),
                String::from("param.2"),
            ],
            &to.parameters,
            Some(0.0),
            Some(1024.0),
            Some(1024),
        )
        .expect("Making spectrum");
    }
}
#[cfg(test)]
mod reqstruct_tests {
    // Test the request structure marshallers.
    use super::*;
    use crate::parameters::*;

    #[test]
    fn c1d_1() {
        let req = SpectrumMessageClient::create1d_request("test", "par1", 0.0, 1024.0, 1024);
        assert_eq!(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("par1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024
                }
            },
            req
        )
    }
    #[test]
    fn cm1d_1() {
        let req = SpectrumMessageClient::createmulti1d_request(
            "test",
            &vec![String::from("p1"), String::from("p2"), String::from("p3")],
            0.0,
            1024.0,
            1024,
        );
        assert!(
            if let SpectrumRequest::CreateMulti1D { name, params, axis } = req {
                assert_eq!(String::from("test"), name);
                assert_eq!(
                    vec![String::from("p1"), String::from("p2"), String::from("p3")],
                    params
                );
                assert_eq!(
                    AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1024
                    },
                    axis
                );
                true
            } else {
                false
            }
        );
    }
    #[test]
    fn cm2d_1() {
        let p = vec![String::from("p1"), String::from("p2"), String::from("p3")];
        let req = SpectrumMessageClient::createmulti2d_request(
            "test", &p, 0.0, 1024.0, 1024, -1.0, 1.0, 100,
        );
        assert!(if let SpectrumRequest::CreateMulti2D {
            name,
            params,
            xaxis,
            yaxis,
        } = req
        {
            assert_eq!(String::from("test"), name);
            assert_eq!(p, params);
            assert_eq!(
                AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024
                },
                xaxis
            );
            assert_eq!(
                AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100
                },
                yaxis
            );
            true
        } else {
            false
        });
    }
    #[test]
    fn cpgamma_1() {
        let xp = vec![String::from("x1"), String::from("x2"), String::from("x3")];
        let yp = vec![String::from("y1"), String::from("y2")];

        let req = SpectrumMessageClient::createpgamma_request(
            "test", &xp, &yp, 0.0, 1024.0, 1024, -1.0, 1.0, 100,
        );
        assert!(if let SpectrumRequest::CreatePGamma {
            name,
            xparams,
            yparams,
            xaxis,
            yaxis,
        } = req
        {
            assert_eq!(String::from("test"), name);
            assert_eq!(xp, xparams);
            assert_eq!(yp, yparams);
            assert_eq!(
                AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024
                },
                xaxis
            );
            assert_eq!(
                AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100
                },
                yaxis
            );
            true
        } else {
            false
        });
    }
    #[test]
    fn c2d_1() {
        let req = SpectrumMessageClient::create2d_request(
            "test", "px", "py", 0.0, 1024.0, 1024, -1.0, 1.0, 100,
        );
        assert_eq!(
            SpectrumRequest::Create2D {
                name: String::from("test"),
                xparam: String::from("px"),
                yparam: String::from("py"),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024
                },
                yaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100
                }
            },
            req
        );
    }
    #[test]
    fn c2dsum_1() {
        let xp = vec![String::from("x1"), String::from("x2"), String::from("x3")];
        let yp = vec![String::from("y1"), String::from("y2"), String::from("y3")];

        let req = SpectrumMessageClient::create2dsum_request(
            "test", &xp, &yp, 0.0, 1024.0, 1024, -1.0, 1.0, 100,
        );
        assert_eq!(
            SpectrumRequest::Create2DSum {
                name: String::from("test"),
                xparams: xp.clone(),
                yparams: yp.clone(),
                xaxis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024
                },
                yaxis: AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 100
                }
            },
            req
        );
    }
    #[test]
    fn del_1() {
        let req = SpectrumMessageClient::delete_request("test");
        assert_eq!(SpectrumRequest::Delete(String::from("test")), req);
    }
    #[test]
    fn list_1() {
        let req = SpectrumMessageClient::list_request("*");
        assert_eq!(SpectrumRequest::List(String::from("*")), req);
    }
    #[test]
    fn gate_1() {
        let req = SpectrumMessageClient::gate_request("spectrum", "gate");
        assert_eq!(
            SpectrumRequest::Gate {
                spectrum: String::from("spectrum"),
                gate: String::from("gate")
            },
            req
        );
    }
    #[test]
    fn ungate_1() {
        let req = SpectrumMessageClient::ungate_request("test");
        assert_eq!(SpectrumRequest::Ungate(String::from("test")), req)
    }
    #[test]
    fn clear_1() {
        let req = SpectrumMessageClient::clear_request("t*");
        assert_eq!(SpectrumRequest::Clear(String::from("t*")), req);
    }
    #[test]
    fn get_1() {
        let req = SpectrumMessageClient::getcontents_request("test", 0.0, 50.0, 100.0, 125.0);
        assert_eq!(
            SpectrumRequest::GetContents {
                name: String::from("test"),
                xlow: 0.0,
                xhigh: 50.0,
                ylow: 100.0,
                yhigh: 125.0
            },
            req
        );
    }
    #[test]
    fn process_1() {
        let events = vec![
            vec![EventParameter::new(1, 2.0), EventParameter::new(7, 100.)],
            vec![
                EventParameter::new(12, 1.345),
                EventParameter::new(77, 3.1416),
            ],
            vec![
                EventParameter::new(1, 2.0),
                EventParameter::new(7, 100.),
                EventParameter::new(12, 1.345),
                EventParameter::new(77, 3.1416),
            ],
        ];
        let req = SpectrumMessageClient::events_request(&events);
        assert_eq!(SpectrumRequest::Events(events), req);
    }
}
#[cfg(test)]
mod spectrum_api_tests {
    use super::*;
    use crate::trace;
    use std::sync::mpsc;
    use std::thread;

    // This is a fake server thread:
    // Creates the spectrum processor, a parameter dictionary
    // with few parameters in it and a condition dictionary
    // with a few harmless conditions.
    // Then accepts Requests until Exit.  If something other
    // than Exit or a Spectrum request arrives, panics.
    // Spectrum requests are passed to the spectrum processor
    // and the return is used to provide a spectrum reply that's
    // send back to the client.
    // All of this supports testing the spectrum section of the
    // histogram server.
    // See also:
    //   start_server - which starts the server.
    //   stop_server - which ends the server and joins with it.
    //
    // Note failing tests can leave hanging threads but
    // they are harmless as new servers are creaed for each
    // test.
    fn fake_server(reader: mpsc::Receiver<Request>) {
        let mut processor = SpectrumProcessor::new();
        let mut params = parameters::ParameterDictionary::new();
        let mut cdict = conditions::ConditionDictionary::new();

        // Make some parameters:
        // Note these wil have ids 1..10 (white box).

        for i in 0..10 {
            params
                .add(&format!("param.{}", i))
                .expect("Failed to add parameters");
        }
        // Make some conditions:

        for i in 0..10 {
            cdict.insert(
                format!("true.{}", i),
                Rc::new(RefCell::new(Box::new(conditions::True {}))),
            );
        }
        for i in 0..10 {
            cdict.insert(
                format!("false.{}", i),
                Rc::new(RefCell::new(Box::new(conditions::False {}))),
            );
        }
        // process requests:

        let tracedb = trace::SharedTraceStore::new();
        loop {
            let request = reader.recv().expect("Request read failed");
            match request.message {
                MessageType::Exit => {
                    request
                        .reply_channel
                        .send(Reply::Exiting)
                        .expect("Failed to send exiting reply");
                    break;
                }
                MessageType::Spectrum(sreq) => {
                    let reply = processor.process_request(sreq, &params, &mut cdict, &tracedb);
                    request
                        .reply_channel
                        .send(Reply::Spectrum(reply))
                        .expect("Reply to client failed");
                }
                _ => {
                    panic!("Unexpected message type in fake server");
                }
            }
        }
    }
    // Starting the server returns a join handle and the request channel.

    fn start_server() -> (thread::JoinHandle<()>, mpsc::Sender<Request>) {
        let (sender, receiver) = mpsc::channel::<Request>();
        let handle = thread::spawn(move || fake_server(receiver));
        (handle, sender)
    }
    fn stop_server(handle: thread::JoinHandle<()>, req_chan: mpsc::Sender<Request>) {
        let (repl_send, repl_receive) = mpsc::channel::<Reply>();
        let req = Request {
            reply_channel: repl_send,
            message: MessageType::Exit,
        };
        let reply = req.transaction(req_chan, repl_receive);
        if let Reply::Exiting = reply {
            handle.join().expect("Fake server join failed");
        } else {
            panic!("Requested exit from server but didn't get back Exiting reply");
        }
    }
    // Note that tests will need for list to work to probe server contents.
    // (alternative is to wrap the spectrum processor in an Arc/Mutex and make
    // it shared but we need list to work anyway so wth):
    #[test]
    fn list_1() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);

        let reply = api.list_spectra("*");
        assert!(if let Ok(l) = reply {
            assert_eq!(0, l.len()); // Nothing to list
            true
        } else {
            false
        });
        stop_server(jh, send);
    }
    // Now we can try to make a spectrum and see if we can get
    // it listed back:
    // Note the need to clone channels and make reply channels each
    // time since Receivers don't support cloning (that's the single receiver
    // part of these channels getting enforced)
    #[test]
    fn make1d_1() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);

        // Create the spectrum:

        assert!(
            if let Ok(()) = api.create_spectrum_1d("test", "param.1", 0.0, 1024.0, 1024,) {
                true
            } else {
                false
            }
        );
        // See if the server knows it:

        assert!(if let Ok(listing) = api.list_spectra("*",) {
            assert_eq!(1, listing.len());
            assert_eq!(
                SpectrumProperties {
                    name: String::from("test"),
                    type_name: String::from("1D"),
                    xparams: vec![String::from("param.1")],
                    yparams: vec![],
                    xaxis: Some(AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1026
                    }),
                    yaxis: None,
                    gate: None
                },
                listing[0]
            );
            true
        } else {
            false
        });

        stop_server(jh, send);
    }
    #[test]
    fn make1dmulti_1() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.3"),
            String::from("param.4"),
            String::from("param.5"),
        ];
        assert_eq!(
            Ok(()),
            api.create_spectrum_multi1d("test", &params, 0.0, 1024.0, 1024,)
        );

        assert!(if let Ok(l) = api.list_spectra("*") {
            assert_eq!(1, l.len());
            assert_eq!(
                SpectrumProperties {
                    name: String::from("test"),
                    type_name: String::from("Multi1d"),
                    xparams: params,
                    yparams: vec![],
                    xaxis: Some(AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1026
                    }),
                    yaxis: None,
                    gate: None
                },
                l[0]
            );
            true
        } else {
            false
        });

        stop_server(jh, send);
    }
    #[test]
    fn make2dmulti_1() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.3"),
            String::from("param.4"),
            String::from("param.5"),
        ];
        assert_eq!(
            Ok(()),
            api.create_spectrum_multi2d("test", &params, 0.0, 1024.0, 1024, -1.0, 1.0, 100)
        );

        assert!(if let Ok(l) = api.list_spectra("*") {
            assert_eq!(1, l.len());
            assert_eq!(
                SpectrumProperties {
                    name: String::from("test"),
                    type_name: String::from("Multi2d"),
                    xparams: params,
                    yparams: vec![],
                    xaxis: Some(AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1026
                    }),
                    yaxis: Some(AxisSpecification {
                        low: -1.0,
                        high: 1.0,
                        bins: 102
                    }),
                    gate: None
                },
                l[0]
            );
            true
        } else {
            false
        });
        stop_server(jh, send);
    }
    #[test]
    fn makepg_1() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);
        let xparams = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.3"),
            String::from("param.4"),
            String::from("param.5"),
        ];
        let yparams = vec![
            String::from("param.6"),
            String::from("param.7"),
            String::from("param.8"),
            String::from("param.9"),
        ];
        assert_eq!(
            Ok(()),
            api.create_spectrum_pgamma(
                "test", &xparams, &yparams, 0.0, 1024.0, 1024, -1.0, 1.0, 100,
            )
        );

        assert!(if let Ok(l) = api.list_spectra("*") {
            assert_eq!(1, l.len());
            assert_eq!(
                SpectrumProperties {
                    name: String::from("test"),
                    type_name: String::from("PGamma"),
                    xparams: xparams,
                    yparams: yparams,
                    xaxis: Some(AxisSpecification {
                        low: 0.0,
                        high: 1024.0,
                        bins: 1026
                    }),
                    yaxis: Some(AxisSpecification {
                        low: -1.0,
                        high: 1.0,
                        bins: 102
                    }),
                    gate: None
                },
                l[0]
            );
            true
        } else {
            false
        });

        stop_server(jh, send);
    }
    #[test]
    fn makesummary_1() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);
        let params = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.3"),
            String::from("param.4"),
            String::from("param.5"),
        ];
        assert_eq!(
            Ok(()),
            api.create_spectrum_summary("test", &params, 0.0, 1024.0, 1024,)
        );

        let l = api.list_spectra("*").expect("Failed to list spectra");
        assert_eq!(1, l.len());
        assert_eq!(
            SpectrumProperties {
                name: String::from("test"),
                type_name: String::from("Summary"),
                xparams: params,
                yparams: vec![],
                xaxis: None,
                yaxis: Some(AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1026
                }),
                gate: None
            },
            l[0]
        );

        stop_server(jh, send);
    }
    #[test]
    fn make2d_1() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);

        api.create_spectrum_2d(
            "test", "param.0", "param.1", 0.0, 1024.0, 1024, -1.0, 1.0, 100,
        )
        .expect("Failed to make 2d spectrum");

        let l = api.list_spectra("*").expect("Failed to list spectra");
        assert_eq!(1, l.len());
        assert_eq!(
            SpectrumProperties {
                name: String::from("test"),
                type_name: String::from("2D"),
                xparams: vec![String::from("param.0")],
                yparams: vec![String::from("param.1")],
                xaxis: Some(AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1026
                }),
                yaxis: Some(AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 102
                }),
                gate: None
            },
            l[0]
        );

        stop_server(jh, send);
    }
    #[test]
    fn make2dsum_1() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);
        let xparams = vec![
            String::from("param.1"),
            String::from("param.2"),
            String::from("param.3"),
            String::from("param.4"),
            String::from("param.5"),
        ];
        let yparams = vec![
            String::from("param.0"),
            String::from("param.6"),
            String::from("param.7"),
            String::from("param.8"),
            String::from("param.9"),
        ];
        api.create_spectrum_2dsum(
            "test", &xparams, &yparams, 0.0, 1024.0, 1024, -1.0, 1.0, 100,
        )
        .expect("Failed to try to make a 2dsum");

        let l = api
            .list_spectra("*")
            .expect("Failed to get  spectrum listing");
        assert_eq!(1, l.len());
        assert_eq!(
            SpectrumProperties {
                name: String::from("test"),
                type_name: String::from("2DSum"),
                xparams: xparams,
                yparams: yparams,
                xaxis: Some(AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1026
                }),
                yaxis: Some(AxisSpecification {
                    low: -1.0,
                    high: 1.0,
                    bins: 102
                }),
                gate: None
            },
            l[0]
        );

        stop_server(jh, send);
    }
    #[test]
    fn delete_1() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);
        for i in 0..10 {
            let pname = format!("param.{}", i);
            let sname = format!("test.{}", i);

            api.create_spectrum_1d(&sname, &pname, 0.0, 1024.0, 1024)
                .expect("failed to make spectrum");
        }
        // There are now 10 spectra - delete test.00:

        api.delete_spectrum("test.0")
            .expect("Delete spectrum failed");

        // Should not be able to list test.0:

        let l = api.list_spectra("test.0").expect("Failed to list spectra");
        assert_eq!(0, l.len());

        // should be 9 left:

        let l = api
            .list_spectra("test.*")
            .expect("Failed to list multiple spectra");
        assert_eq!(9, l.len());

        stop_server(jh, send);
    }
    // Test list spectra with a bad glob pattern:

    #[test]
    fn list_2() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);

        let result = api.list_spectra("test[...");
        stop_server(jh, send);

        assert!(result.is_err());
    }
    #[test]
    fn gate_1() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);
        api.create_spectrum_1d("test", "param.1", 0.0, 1024.0, 1024)
            .expect("Failed to create spectrum");
        api.gate_spectrum("test", "true.1")
            .expect("Failed to gate spectrum");

        let l = api.list_spectra("*").expect("listing failed");
        assert_eq!(1, l.len());
        assert_eq!(Some(String::from("true.1")), l[0].gate);
        stop_server(jh, send);
    }
    #[test]
    fn ungate_1() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);
        api.create_spectrum_1d("test", "param.1", 0.0, 1024.0, 1024)
            .expect("Failed to create spectrum");

        api.gate_spectrum("test", "true.1")
            .expect("Failed to gate spectrum");

        api.ungate_spectrum("test").expect("Failed to ungate");

        let l = api.list_spectra("*").expect("failed to list spectra");
        assert_eq!(None, l[0].gate);
        stop_server(jh, send);
    }
    // For clear and process, we need to have some confidence in
    // being able to get the contents.

    #[test]
    fn get_contents_1() {
        // This will give an empty value:

        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);

        api.create_spectrum_1d("test", "param.1", 0.0, 1024.0, 1024)
            .expect("Failed to make spectrum");

        let contents = api
            .get_contents("test", 0.0, 1024.0, 0.0, 0.0)
            .expect("Failed to get spectrum contents");
        assert_eq!(0, contents.len());

        stop_server(jh, send);
    }
    #[test]
    fn event_1() {
        // only way to put non-zero contents in spectra
        // is via events.
        // Note that param.0 is id 1 param1. id 2 etc...
        //
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);

        api.create_spectrum_1d("test", "param.1", 0.0, 1024.0, 1024)
            .expect("Failed to make spectrum");

        // Make events to send to the server to put some
        // counts into the spectrum:

        let events = vec![
            vec![parameters::EventParameter::new(2, 100.0)],
            vec![parameters::EventParameter::new(2, 100.0)],
            vec![parameters::EventParameter::new(2, 100.0)],
            vec![parameters::EventParameter::new(2, 100.0)],
            vec![parameters::EventParameter::new(2, 100.0)],
            vec![parameters::EventParameter::new(2, 100.0)],
        ]; // 6 bcounts at 100.0

        api.process_events(&events)
            .expect("Failed to process events");

        // Now get the contents should be one entry with 6 counts
        // at 100.0:

        let contents = api
            .get_contents("test", 0.0, 1024.0, 0.0, 0.0)
            .expect("Unable to get spectrumcontents");
        assert_eq!(1, contents.len());
        let c = contents[0];
        assert_eq!(ChannelType::Bin, c.chan_type);
        assert_eq!(100.0, c.x);
        assert_eq!(6.0, c.value);

        stop_server(jh, send);
    }
    #[test]
    fn clear_1() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);

        api.create_spectrum_1d("test", "param.1", 0.0, 1024.0, 1024)
            .expect("Failed to make spectrum");

        // Make events to send to the server to put some
        // counts into the spectrum:

        let events = vec![
            vec![parameters::EventParameter::new(2, 100.0)],
            vec![parameters::EventParameter::new(2, 100.0)],
            vec![parameters::EventParameter::new(2, 100.0)],
            vec![parameters::EventParameter::new(2, 100.0)],
            vec![parameters::EventParameter::new(2, 100.0)],
            vec![parameters::EventParameter::new(2, 100.0)],
        ]; // 6 bcounts at 100.0

        api.process_events(&events)
            .expect("Failed to process events");

        // Now get the contents should be one entry with 6 counts
        // at 100.0:

        let contents = api
            .get_contents("test", 0.0, 1024.0, 0.0, 0.0)
            .expect("Unable to get spectrumcontents");
        assert_eq!(1, contents.len());
        let c = contents[0];
        assert_eq!(ChannelType::Bin, c.chan_type);
        assert_eq!(100.0, c.x);
        assert_eq!(6.0, c.value);

        // now clear all spectra:

        api.clear_spectra("*").expect("Failed to request clear");

        let contents = api
            .get_contents("test", 0.0, 1024.0, 0.0, 0.0)
            .expect("Unable to get spectrumcontents");
        assert_eq!(0, contents.len());

        stop_server(jh, send);
    }
    #[test]
    fn getstats_1() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);

        api.create_spectrum_1d("test", "param.1", 0.0, 1024.0, 1024)
            .expect("Failed to make spectrum");

        let result = api.get_statistics("test");

        assert!(if let Ok(stats) = result {
            assert_eq!((0, 0, 0, 0), stats);
            true
        } else {
            false
        });

        stop_server(jh, send);
    }
    // test for load_spectrum method .. note that
    // the server side is already tested, so we really just need to test
    // that the messaging works rather than be exhaustive over all spectrum
    // types.
    #[test]
    fn fill_1() {
        // nonexistent spectrum gives error:

        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);
        let contents = SpectrumContents::new();
        let reply = api.fill_spectrum("test", contents);
        assert!(reply.is_err());

        stop_server(jh, send);
    }
    #[test]
    fn fill_2() {
        // fill spec;trum and get data back to match:

        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);

        api.create_spectrum_1d("test", "param.1", 0.0, 1024.0, 1024)
            .expect("Failed to make spectrum");

        let contents = vec![
            Channel {
                chan_type: ChannelType::Bin,
                x: 10.0,
                y: 0.0,
                bin: 0,
                value: 12345.0,
            },
            Channel {
                chan_type: ChannelType::Bin,
                x: 20.0,
                y: 0.0,
                bin: 0,
                value: 666.0,
            },
        ];
        let reply = api.fill_spectrum("test", contents);
        assert!(reply.is_ok());

        // Get the contents:

        let reply = api.get_contents("test", 0.0, 1024.0, 0.0, 0.0);
        assert!(if let Ok(c) = reply {
            assert_eq!(2, c.len());

            assert_eq!(10.0, c[0].x);
            assert_eq!(12345.0, c[0].value);

            assert_eq!(20.0, c[1].x);
            assert_eq!(666.0, c[1].value);
            true
        } else {
            false
        });

        stop_server(jh, send);
    }
    // set/get channel values for a spectrum:

    #[test]
    fn get_set_chan1() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);

        api.create_spectrum_1d("test", "param.1", 0.0, 1024.0, 1024)
            .expect("Failed to make spectrum");

        api.set_channel_value("test", 512, None, 12345.0)
            .expect("Setting value");
        assert_eq!(
            12345.0,
            api.get_channel_value("test", 512, None)
                .expect("Getting value")
        );

        stop_server(jh, send);
    }
    #[test]
    fn get_set_chan2() {
        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);

        api.create_spectrum_2d(
            "test", "param.1", "param.2", 0.0, 1024.0, 256, 0.0, 1024.0, 256,
        )
        .expect("Making spectrum");

        api.set_channel_value("test", 128, Some(128), 1245.0)
            .expect("Setting value");

        assert_eq!(
            1245.0,
            api.get_channel_value("test", 128, Some(128))
                .expect("Getting value")
        );

        stop_server(jh, send);
    }
    #[test]
    fn get_set_chan3() {
        // ensure that errors propagate back

        let (jh, send) = start_server();
        let api = SpectrumMessageClient::new(&send);

        api.create_spectrum_2d(
            "test", "param.1", "param.2", 0.0, 1024.0, 256, 0.0, 1024.0, 256,
        )
        .expect("Making spectrum");
        // 2d spectra need y channel value:
        assert!(api.set_channel_value("test", 128, None, 1245.0).is_err());
        assert!(api.get_channel_value("test", 128, None).is_err());

        stop_server(jh, send);
    }
}
// Tests that spectrum traces actually happen:

#[cfg(test)]
mod spectrum_trace_tests {
    use super::*;
    use crate::conditions::*;
    use crate::parameters::*;
    use crate::trace;
    //use std::cmp::Ordering;
    use std::time::Duration;

    // for most of the tests we need, not only a SpectrumProcessor
    // but a condition dict, and a parameter dict:

    struct TestObjects {
        processor: SpectrumProcessor,
        parameters: ParameterDictionary,
        conditions: ConditionDictionary,
        tracedb: trace::SharedTraceStore,
    }
    fn make_test_objs() -> TestObjects {
        TestObjects {
            processor: SpectrumProcessor::new(),
            parameters: ParameterDictionary::new(),
            conditions: ConditionDictionary::new(),
            tracedb: trace::SharedTraceStore::new(),
        }
    }
    fn make_some_params(to: &mut TestObjects) {
        for i in 0..10 {
            let name = format!("param.{}", i);
            to.parameters.add(&name).unwrap();
        }
    }
    #[test]
    fn create_1() {
        // Creating a new spectrm fires a trace event:

        let mut to = make_test_objs();
        make_some_params(&mut to); // Before registring the trace client!

        let token = to.tracedb.new_client(Duration::from_secs(100));

        to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        // A spectrum created trace should be ready for us:

        let traces = to.tracedb.get_traces(token).expect("Fetching traces.");
        assert_eq!(1, traces.len());
        assert!(
            if let trace::TraceEvent::SpectrumCreated(name) = traces[0].event() {
                assert_eq!("test", name);
                true
            } else {
                false
            }
        );
    }
    #[test]
    fn delete_1() {
        // deleting a spectrum makes a SpectrumDeleted event:

        let mut to = make_test_objs();
        make_some_params(&mut to); // Before registring the trace client!

        // Register the client after creation so we don't get a trace for it:

        to.processor.process_request(
            SpectrumRequest::Create1D {
                name: String::from("test"),
                parameter: String::from("param.1"),
                axis: AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1024,
                },
            },
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        let token = to.tracedb.new_client(Duration::from_secs(100));

        to.processor.process_request(
            SpectrumRequest::Delete(String::from("test")),
            &to.parameters,
            &mut to.conditions,
            &to.tracedb,
        );

        // A spectrum created trace should be ready for us:

        let traces = to.tracedb.get_traces(token).expect("Fetching traces.");
        assert_eq!(1, traces.len());
        assert!(
            if let trace::TraceEvent::SpectrumDeleted(name) = traces[0].event() {
                assert_eq!("test", name);
                true
            } else {
                false
            }
        );
    }
}
