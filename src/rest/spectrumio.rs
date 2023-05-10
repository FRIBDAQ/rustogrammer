//!  This module contains the client code/REST server code for spectrum I/O.
//!  We support two spectrum file formats:  SpecTcl old style format to support
//!  interchanging spectrum data with SpecTcl and Java Script Object Notation
//!  (JSON) encoded spectra.
//!
//!  Writing and (reading?) JSON encoded data is handled smoothly by
//!  serde - we can put the spectrum metadata and channel data into a nice
//!  struct that's deriving from Serialize and Deserialize then using the
//!  Rocket Json function to create the Json and serde directly to deserialize
//!  the (json::from_str e.g.).
//!
use super::*;
use crate::messaging::spectrum_messages;
use rocket::serde::{json, json::Json};
use rocket::State;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;

/// This is the structure that will contain channel data:
/// It's a bit different than the spectrum_messages::Channel struct
/// as to interchange data with SpecTcl we need to also
/// store x/y bin numbers which will get computed from the
/// raw spectrum_messages::Channel struct.
///
/// field names are chosen a bit more carefully as they will
/// appear verbatim in the JSON
///
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct SpectrumChannel {
    chan_type: spectrum_messages::ChannelType,
    x_coord: f64,
    y_coord: f64,
    x_bin: usize,
    y_bin: usize,
    value: u64,
}

/// This is, again, a bit different than the
/// spectrum_message::SpectrumProperties, however mostly in that
/// it's declared to be (de)serializable... but we also don't
/// need the gate when serializing/deserializing a spectrum.
///
/// in this case field names are chosen a bit more carefully as they
/// will appear verbatim in the JSON.
///
#[derive(Serialize, Deserialize, Clone)]
pub struct SpectrumProperties {
    name: String,
    type_string: String,
    x_parameters: Vec<String>,
    y_parameters: Vec<String>,
    x_axis: Option<(f64, f64, u32)>,
    y_axis: Option<(f64, f64, u32)>,
}

/// Spectra are their properties and a vector of their channels:

#[derive(Serialize, Deserialize, Clone)]
pub struct SpectrumFileData {
    definition: SpectrumProperties,
    channels: Vec<SpectrumChannel>,
}

//--------------------------------------------------------------------------
// swrite:

// private function turn an Option<spectrum_messages::AxisSpecification>
// into Option<(f64, f64, u32)>

fn axis_to_tuple(i: Option<spectrum_messages::AxisSpecification>) -> Option<(f64, f64, u32)> {
    match i {
        None => None,
        Some(s) => Some((s.low, s.high, s.bins)),
    }
}

// private function to get spectrum properties:

fn get_spectrum_descriptions(
    spectra: &Vec<String>,
    api: &spectrum_messages::SpectrumMessageClient,
) -> Result<Vec<SpectrumProperties>, (String, String)> {
    let mut ok_result = Vec::<SpectrumProperties>::new();
    for name in spectra {
        let info = api.list_spectra(&name);
        if let Err(i) = info {
            return Err((name.clone(), i));
        }
        let info = info.unwrap();
        if info.len() == 0 {
            return Err((name.clone(), String::from("Spectrum does not exist")));
        }
        let info = &info[0];
        ok_result.push(SpectrumProperties {
            name: info.name.clone(),
            type_string: spectrum::rg_sptype_to_spectcl(&info.type_name),
            x_parameters: info.xparams.clone(),
            y_parameters: info.yparams.clone(),
            x_axis: axis_to_tuple(info.xaxis),
            y_axis: axis_to_tuple(info.yaxis),
        });
    }

    Ok(ok_result)
}
// Simple axis -> bin transformation:
// The + 1 allows for the fact that bin 0 is underflows.

fn transform(l: f64, h: f64, b: u32, c: f64) -> usize {
    (((c - l) / (h - l)) * b as f64) as usize + 1
}

// Given coordinates  in a normal bin - convert themto (xbin, ybin):

fn bin_to_bins(x: f64, y: f64, d: &SpectrumProperties) -> (usize, usize) {
    let xbins = if let Some(xa) = d.x_axis {
        transform(xa.0, xa.1, xa.2, x)
    } else {
        0
    };
    let ybins = if let Some(ya) = d.y_axis {
        transform(ya.0, ya.1, ya.2, y)
    } else {
        0
    };

    (xbins, ybins)
}
// Compute the underflow bins for a coordinate:

fn underflow_to_bins(x: f64, y: f64, d: &SpectrumProperties) -> (usize, usize) {
    let xbin = if let Some(xa) = d.x_axis {
        if x <= xa.0 {
            0 // X is the underflow.
        } else {
            transform(xa.0, xa.1, xa.2, x) // X is a real bin.
        }
    } else {
        0 // There really isn't an x bin.
    };

    let ybin = if let Some(ya) = d.y_axis {
        if y <= ya.0 {
            0
        } else {
            transform(ya.0, ya.1, ya.2, y)
        }
    } else {
        0
    };

    (xbin, ybin)
}
// Compute bins for an overflow value:
fn overflow_to_bins(x: f64, y: f64, d: &SpectrumProperties) -> (usize, usize) {
    let xbin = if let Some(xa) = d.x_axis {
        if x >= xa.1 {
            transform(xa.0, xa.1, xa.2, xa.1) // X is the overflow
        } else {
            transform(xa.0, xa.1, xa.2, x) // X is a real bin.
        }
    } else {
        0 // There really isn't an x bin.
    };

    let ybin = if let Some(ya) = d.y_axis {
        if y >= ya.1 {
            transform(ya.0, ya.1, ya.2, ya.1)
        } else {
            transform(ya.0, ya.1, ya.2, y)
        }
    } else {
        0
    };

    (xbin, ybin)
}

// Convert one channel toa SpectrumChannel:

fn convert_channel(c: &spectrum_messages::Channel, d: &SpectrumProperties) -> SpectrumChannel {
    let mut result = SpectrumChannel {
        chan_type: c.chan_type,
        x_coord: c.x,
        y_coord: c.y,
        x_bin: 0,
        y_bin: 0, //tentative values:
        value: c.value as u64,
    };
    // Figure out the x/y bin numbers
    let coords = match c.chan_type {
        spectrum_messages::ChannelType::Underflow => underflow_to_bins(c.x, c.y, d),
        spectrum_messages::ChannelType::Overflow => overflow_to_bins(c.x, c.y, d),
        spectrum_messages::ChannelType::Bin => bin_to_bins(c.x, c.y, d),
    };

    result.x_bin = coords.0;
    result.y_bin = coords.1;

    result
}

// Convert a histogrammer channel to vector to our vector of SpectrumChannels
// doing this requires the spectrum definition so, if necessary , we can
// make the x/y bin numbers.
//
fn convert_channels(
    channels: &Vec<spectrum_messages::Channel>,
    d: &SpectrumProperties,
) -> Vec<SpectrumChannel> {
    let mut result = Vec::<SpectrumChannel>::new();
    for c in channels.iter() {
        result.push(convert_channel(c, d));
    }

    result
}

/// This is the handler for the Spectrum write method.
///
/// ### Parameters
/// *  file - path to the file to create. Must not exist.
/// *  format - Format - legal values are "ascii", and "json"  these
/// are matched case insensitively (e.g. "ASCII" and "Json" are legal and do
/// what you think they might do).
/// * spectrum - Can appear multiple times and are the names of the
/// spectra that should be written to file.
/// * state - The REST state object that contains what we need to form an
/// API object to talk to the histogram thread.
///
/// ### Returns:
/// * JSON encoded GenericResponse object.  
///     -  On success only **status** is non-empty and contains _OK_
///     -  On failure, the **status** contains the top level error reason
///  (e.g  Spectrum or spectra not found)
/// and **detail** contains a more specific message e.g. in the case above, the
/// set of spectra that could not be looked up in the histogram server.
///
#[get("/?<file>&<format>&<spectrum>")]
pub fn swrite_handler(
    file: String,
    format: String,
    spectrum: Vec<String>,
    state: &State<HistogramState>,
) -> Json<GenericResponse> {
    let api =
        spectrum_messages::SpectrumMessageClient::new(&(state.inner().state.lock().unwrap().1));

    // Get the spectrum properties for the spectra:

    let descriptions = get_spectrum_descriptions(&spectrum, &api);
    if let Err(e) = descriptions {
        return Json(GenericResponse::err(
            &format!("Spectrum could not be found: {}", e.0),
            &e.1,
        ));
    }
    let descriptions = descriptions.unwrap();
    // For each description, get the contents and build a vector of Spectrum
    // file data from them.  Note it's possible to fail to get contents
    // if another process has killed off a spectrum whlie we're running.
    // In that case, we just drop that spectrum from the output file:

    let mut spectra = Vec::<SpectrumFileData>::new();
    for d in descriptions.iter() {
        let (xlow, xhigh) = if let Some(x) = d.x_axis {
            (x.0, x.1)
        } else {
            (-1.0, 1.0)
        };
        let (ylow, yhigh) = if let Some(y) = d.y_axis {
            (y.0, y.1)
        } else {
            (-1.0, 1.0)
        };
        let contents = api.get_contents(&d.name, xlow, xhigh, ylow, yhigh);
        if let Ok(c) = contents {
            spectra.push(SpectrumFileData {
                definition: d.clone(),
                channels: convert_channels(&c, d),
            });
        }
    }

    // Try to create the file

    let fd = File::create(&file);
    if let Err(e) = fd {
        return Json(GenericResponse::err(
            &format!("Unable to create file: {}", file),
            &e.to_string(),
        ));
    }
    let mut fd = fd.unwrap();

    // make the format lower case for string blind compare:

    let mut fmt = format.clone();
    fmt.make_ascii_lowercase();

    let response = match fmt.as_str() {
        "json" => {
            if let Err(e) = fd.write_all(json::to_string(&spectra).expect("Failed conversion to JSON").as_bytes()) {
                GenericResponse::err("Failed to write spectra to file", &e.to_string())
            } else {
                GenericResponse::ok("")
            }
        }
        "ascii" => GenericResponse::err("Unimplemented", "ASCII (SpecTcl format)"),
        _ => GenericResponse::err("Invalid format type specification:", &format!("{}", format)),
    };

    Json(response)
}
