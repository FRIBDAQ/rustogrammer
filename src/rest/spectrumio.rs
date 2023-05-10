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
use rocket::serde::{json::Json, json};
use serde::{Serialize, Deserialize};
use rocket::State;
use crate::messaging::spectrum_messages;
use super::*;


/// This is the structure that will contain channel data:
/// It's a bit different than the spectrum_messages::Channel struct
/// as to interchange data with SpecTcl we need to also 
/// store x/y bin numbers which will get computed from the
/// raw spectrum_messages::Channel struct.
///

#[derive(Serialize, Deserialize)]
pub struct SpectrumChannel {
    chan_type : spectrum_messages::ChannelType,
    xcoord    : f64,
    ycoord    : f64,
    xbin      : usize,
    ybin      : usize,
    value     : f64,
}

/// This is, again, a bit different than the
/// spectrum_message::SpectrumProperties, however mostly in that
/// it's declared to be (de)serializable... but we also don't
/// need the gate when serializing/deserializing a spectrum.
///
#[derive(Serialize, Deserialize)]
pub struct SpectrumProperties {
    name : String,
    type_string : String,
    x_parameters : String,
    y_parameters: String,
    x_axis : (f64, f64, u32),
    y_axis : (f64, f64, u32),
}

/// Spectra are their properties and a vector of their channels:

#[derive(Serialize, Deserialize)]
pub struct SpectrumFileData {
    definition: SpectrumProperties,
    channels : Vec<SpectrumChannel>
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
///     -  On failure, the **status** contains the top level error reason (e.g  Spectrum or spectra not found)
/// and **detail** contains a more specific message e.g. in the case above, the
/// set of spectra that could not be looked up in the histogram server.
///
#[get("/?<file>&<format>&<spectrum>")]
pub fn swrite_handler(file : String, format: String, spectrum: Vec<String>, state : &State<HistogramState>) -> Json<GenericResponse> {
    Json(GenericResponse::err("Not yet implemented", ""))
}