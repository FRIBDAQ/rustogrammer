//!  Handlers for the spectcl/spectrum URLs.
//!  These manipulate spectra.  A brief list of the
//!  URI's that are supported:
//!
//! *  /spectcl/spectrum/list - list spectra and their properties.
//! *  /spectcl/spectrum/delete - Deltee a spectrum.
//! *  /spectcl/spectrum/create - create a new spectrum.
//! *  /spectcl/spectrum/contents - Get the contents of a spectrum.
//! *  /spectcl/sspectrum/clear - clear
use rocket::serde::{json::Json, Serialize};
use rocket::State;

use super::*;

use crate::messaging::spectrum_messages::{
    SpectrumMessageClient, SpectrumProperties, SpectrumServerContentsResult,
    SpectrumServerEmptyResult, SpectrumServerListingResult,
};
// as with gates we need to map from Rustogramer spectrum
// types to SpecTcl spectrum types.

fn rg_sptype_to_spectcl(rg_type: &str) -> String {
    match rg_type {
        "1D" => String::from("1"),
        "Multi1D" => String::from("g1"),
        "Multi2D" => String::from("g2"),
        "PGamma" => String::from("gd"),
        "Summary" => String::from("s"),
        "2D" => String::from("2"),
        "2DSum" => String::from("m2"),
        _ => String::from("-unsupported-"),
    }
}
//------------------------------------------------------------
// Stuff we need to list spectra and their properties.

// structures that define the JSON we'll return:

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Axis {
    low: f64,
    high: f64,
    bins: u32,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SpectrumDescription {
    name: String,
    #[serde(rename = "type")]
    spectrum_type: String,
    parameters: Vec<String>,
    axes: Vec<Axis>,
    chantype: String,
    gate: Option<String>,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ListResponse {
    status: String,
    detail: Vec<SpectrumDescription>,
}

// Convert the listing from the message client to a vector
// of spectrum descriptions:

fn list_to_detail(l: Vec<SpectrumProperties>) -> Vec<SpectrumDescription> {
    let mut result = Vec::<SpectrumDescription>::new();
    for mut d in l {
        let mut def = SpectrumDescription {
            name: d.name,
            spectrum_type : rg_sptype_to_spectcl(&d.type_name),
            parameters :d.xparams,
            axes : Vec::<Axis>::new(),
            chantype: String::from("f64"),
            gate : d.gate
        };
        def.parameters.append(&mut d.yparams);
        if let Some(x) = d.xaxis {
            def.axes.push(Axis {
                low: x.low,
                high : x.high,
                bins : x.bins
            });
        }
        if let Some(y) = d.yaxis {
            def.axes.push(Axis {
                low: y.low,
                high : y.high,
                bins: y.bins
            });
        }

        result.push(def);
    }
    result
}
///
/// List the spectra.  The only query parameter is _filter_ which is an
/// optional parameter that, if provided is a glob pattern that
/// must match a spectrum name for it to be included in the 
/// set of listed spectra.  The default value for _filter_ is "*" which 
/// matches all names.
///
/// The reply consists of _status_ which, on success is _OK_ and
/// on failure is an error message string.
///
/// On failure the _detail_ field of the resonse is an empty array.
/// On success, _detail_ will be an array that describes all of the
/// spectra that match _filter_ (so this may still be empty).  Each
/// element is a JSON struct that contains:
///
/// *   name -- The name of the matching spectrum.
/// *   type -- the SpecTcl type of the matching spectrum.
//  *   parameters -- an array of paramter names.  For 2-d spectra,
/// the first parameter is the x parameter, the second, the y.
/// note that this can be ambiguous for gd and m2 which have multiple
/// x and y parameters.
/// *   axes -- an array of at least one axis definition.  Each element
/// of the array is an object with the fields:
///     - low  - low limit of the axis.
///     - high - high limit of the axis.
///     - bins - the number of bins between [low, high)
/// *   chantype -- the data type of each channel in the spectrum.
/// in rustogramer this is hardcoded to _f64_
/// *    gate if not _null_ thisi s the name of the conditions that
/// is applied as a gate to the spectrum.
#[get("/list?<filter>")]
pub fn list_spectrum(filter: OptionalString, state: &State<HistogramState>) -> Json<ListResponse> {
    let pattern = if let Some(p) = filter {
        p
    } else {
        String::from("*")
    };

    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);

    let response = match api.list_spectra(&pattern) {
        Ok(l) => ListResponse {
            status: String::from("OK"),
            detail: list_to_detail(l),
        },
        Err(s) => ListResponse {
            status: format!("Failed to list spectra: {}", s),
            detail: Vec::<SpectrumDescription>::new(),
        },
    };

    Json(response)
}
