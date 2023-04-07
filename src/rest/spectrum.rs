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
    SpectrumMessageClient, SpectrumServerContentsResult, SpectrumServerEmptyResult,
    SpectrumServerListingResult,
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
    params: Vec<String>,
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

#[get("/list?<filter>")]
pub fn list_spectrum(filter: OptionalString, state: &State<HistogramState>) -> Json<ListResponse> {
    Json(ListResponse {
        status: String::from("OK"),
        detail: Vec::<SpectrumDescription>::new(),
    })
}
