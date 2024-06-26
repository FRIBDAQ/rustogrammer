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
use crate::messaging::condition_messages;
use crate::messaging::parameter_messages;
use crate::messaging::spectrum_messages;
use crate::sharedmem::binder;
use crate::spectclio;
use rocket::serde::{json, json::Json};
use rocket::State;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
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
#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct SpectrumChannel {
    pub chan_type: spectrum_messages::ChannelType,
    pub x_coord: f64,
    pub y_coord: f64,
    pub x_bin: usize,
    pub y_bin: usize,
    pub value: u64,
}

/// This is, again, a bit different than the
/// spectrum_message::SpectrumProperties, however mostly in that
/// it's declared to be (de)serializable... but we also don't
/// need the gate when serializing/deserializing a spectrum.
///
/// in this case field names are chosen a bit more carefully as they
/// will appear verbatim in the JSON.
///
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpectrumProperties {
    pub name: String,
    pub type_string: String,
    pub x_parameters: Vec<String>,
    pub y_parameters: Vec<String>,
    pub x_axis: Option<(f64, f64, u32)>,
    pub y_axis: Option<(f64, f64, u32)>,
}

/// Spectra are their properties and a vector of their channels:

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpectrumFileData {
    pub definition: SpectrumProperties,
    pub channels: Vec<SpectrumChannel>,
}

//--------------------------------------------------------------------------
// swrite:

// private function turn an Option<spectrum_messages::AxisSpecification>
// into Option<(f64, f64, u32)>

fn axis_to_tuple(i: Option<spectrum_messages::AxisSpecification>) -> Option<(f64, f64, u32)> {
    i.map(|s| (s.low, s.high, s.bins))
}

// private function to get spectrum properties:

fn get_spectrum_descriptions(
    spectra: &[String],
    api: &spectrum_messages::SpectrumMessageClient,
) -> Result<Vec<SpectrumProperties>, (String, String)> {
    let mut ok_result = Vec::<SpectrumProperties>::new();
    for name in spectra {
        let info = api.list_spectra(name);
        if let Err(i) = info {
            return Err((name.clone(), i));
        }
        let info = info.unwrap();
        if info.is_empty() {
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
    (((c - l) / (h - l)) * (b - 2) as f64) as usize + 1
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
    channels: &[spectrum_messages::Channel],
    d: &SpectrumProperties,
) -> Vec<SpectrumChannel> {
    let mut result = Vec::<SpectrumChannel>::new();

    // Summary type has axis defs that are 'special'.
    // specifically, the x axis specification has to be generated from
    // the x parameter list size.

    let desc = if d.type_string == "s" {
        let mut summary_d = d.clone();
        summary_d.x_axis = Some((
            0.0,
            d.x_parameters.len() as f64,
            (d.x_parameters.len() + 2) as u32,
        ));
        summary_d
    } else {
        d.clone()
    };

    for c in channels.iter() {
        result.push(convert_channel(c, &desc));
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
    state: &State<SharedHistogramChannel>,
) -> Json<GenericResponse> {
    let api = spectrum_messages::SpectrumMessageClient::new(&(state.inner().lock().unwrap()));

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
            (0.0, d.x_parameters.len() as f64) // summary spectrum correction.
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
            if let Err(e) = fd.write_all(
                json::to_string(&spectra)
                    .expect("Failed conversion to JSON")
                    .as_bytes(),
            ) {
                GenericResponse::err("Failed to write spectra to file", &e.to_string())
            } else {
                // Add code for spectrum write. In SpecTcl format.
                GenericResponse::ok("")
            }
        }
        "ascii" => {
            if let Err(s) = spectclio::write_spectrum(&mut fd, &spectra) {
                GenericResponse::err("Unable to write ASCII spectra", &s)
            } else {
                GenericResponse::ok("")
            }
        }
        _ => GenericResponse::err("Invalid format type specification:", &format),
    };

    Json(response)
}
//--------------------------------------------------------------------
// Stuff needed for sread.
//

// read Json - deserialize a vector of spectra from a stream given
// something that supports the Read trait:

fn read_json<T>(fd: &mut T) -> Result<Vec<SpectrumFileData>, String>
where
    T: Read,
{
    let mut src = String::new();
    let io = fd.read_to_string(&mut src);

    if let Err(s) = io {
        return Err(format!("{}", s));
    }

    let result = json::from_str::<Vec<SpectrumFileData>>(&src);
    if let Err(e) = result {
        return Err(format!("{}", e));
    }
    Ok(fix_json_bins(result.unwrap()))
}
// Create a hash set of the existing parameter names.

fn make_parameter_set(
    api: &parameter_messages::ParameterMessageClient,
) -> Result<HashSet<String>, String> {
    let params = api.list_parameters("*")?;

    let mut result = HashSet::<String>::new();
    for p in params {
        result.insert(p.get_name());
    }
    Ok(result)
}
// Given a vector of parameter names, makes new parameters for all that are not
// in the existing hash -- updating the hash.

fn make_missing_params(
    params: &[String],
    existing: &mut HashSet<String>,
    api: &parameter_messages::ParameterMessageClient,
) -> Result<(), String> {
    for p in params.iter() {
        if !existing.contains(p) {
            api.create_parameter(p)?;
            existing.insert(p.clone());
        }
    }
    Ok(())
}
// Given a spectrum definition make new parameters for all parameters
// not known to the histogramer the local hash of existing parameters
// is updated with the paramters made.

fn make_parameters(
    def: &SpectrumProperties,
    existing: &mut HashSet<String>,
    api: &parameter_messages::ParameterMessageClient,
) -> Result<(), String> {
    make_missing_params(&def.x_parameters, existing, api)?;
    make_missing_params(&def.y_parameters, existing, api)
}

// If a spectrum with 'name' exists it is deleted:

fn delete_existing(
    name: &str,
    api: &spectrum_messages::SpectrumMessageClient,
) -> Result<(), String> {
    // See if name exists:

    let listing = api.list_spectra(name)?;
    if !listing.is_empty() {
        api.delete_spectrum(name)?;
    }
    Ok(())
}
// Create a unique name:

fn make_unique_name(
    base: &str,
    api: &spectrum_messages::SpectrumMessageClient,
) -> Result<String, String> {
    let mut candidate_name = String::from(base);
    let mut counter = 0;
    loop {
        let list = api.list_spectra(&candidate_name)?;
        if list.is_empty() {
            break;
        }
        // Make next candidate name:

        candidate_name = format!("{}_{}", base, counter);
        counter += 1;
    }
    Ok(candidate_name)
}
// Make a spectrum -- when we know that
//  - all parameters have been defined.
// - We won't be replacing an existing spectrum:
//
fn make_spectrum(
    name: &str,
    def: &SpectrumProperties,
    api: &spectrum_messages::SpectrumMessageClient,
) -> Result<String, String> {
    match def.type_string.as_str() {
        "1" => {
            let axis = def.x_axis.unwrap();
            api.create_spectrum_1d(name, &def.x_parameters[0], axis.0, axis.1, axis.2)?;
        }
        "g1" => {
            let axis = def.x_axis.unwrap();
            api.create_spectrum_multi1d(name, &def.x_parameters, axis.0, axis.1, axis.2)?;
        }
        "g2" => {
            let xaxis = def.x_axis.unwrap();
            let yaxis = def.y_axis.unwrap();
            api.create_spectrum_multi2d(
                name,
                &def.x_parameters,
                xaxis.0,
                xaxis.1,
                xaxis.2,
                yaxis.0,
                yaxis.1,
                yaxis.2,
            )?;
        }
        "gd" => {
            let xaxis = def.x_axis.unwrap();
            let yaxis = def.y_axis.unwrap();
            api.create_spectrum_pgamma(
                name,
                &def.x_parameters,
                &def.y_parameters,
                xaxis.0,
                xaxis.1,
                xaxis.2,
                yaxis.0,
                yaxis.1,
                yaxis.2,
            )?;
        }
        "s" => {
            let axis = if let Some(y) = def.y_axis {
                y
            } else {
                // ASCII format special case - ascii reader thinks the
                // axis is an x axis.
                def.x_axis.unwrap()
            };
            api.create_spectrum_summary(name, &def.x_parameters, axis.0, axis.1, axis.2)?;
        }
        "2" => {
            let xaxis = def.x_axis.unwrap();
            let yaxis = def.y_axis.unwrap();
            api.create_spectrum_2d(
                name,
                &def.x_parameters[0],
                &def.y_parameters[0],
                xaxis.0,
                xaxis.1,
                xaxis.2,
                yaxis.0,
                yaxis.1,
                yaxis.2,
            )?;
        }
        "m2" => {
            let xaxis = def.x_axis.unwrap();
            let yaxis = def.y_axis.unwrap();
            api.create_spectrum_2dsum(
                name,
                &def.x_parameters,
                &def.y_parameters,
                xaxis.0,
                xaxis.1,
                xaxis.2,
                yaxis.0,
                yaxis.1,
                yaxis.2,
            )?;
        }
        _ => {
            return Err(format!("Unsupported spectrum type {}", def.type_string));
        }
    };

    Ok(String::from(name))
}

// Called if replace is turned off..
// in
// Enter one spectrum in the histogramer.  If replace is on,
// we delete the existing spectrum and enter the new one.
// If not we create a new unique name for the spectrum.

fn enter_spectrum(
    def: &SpectrumProperties,
    can_replace: bool,
    api: &spectrum_messages::SpectrumMessageClient,
) -> Result<String, String> {
    let actual_name = if can_replace {
        delete_existing(&def.name, api)?; // Delete any pev. spectrum.
        def.name.clone()
    } else {
        make_unique_name(&def.name, api)? // Generate a unique name.
    };
    make_spectrum(&actual_name, def, api)
}
// Given a spectrum we know now exists, fill it:

fn fill_spectrum(
    name: &str,
    c: &[SpectrumChannel],
    api: &spectrum_messages::SpectrumMessageClient,
) -> Result<(), String> {
    // Need to map our channels -> contents:

    let mut contents = spectrum_messages::SpectrumContents::new();
    for chan in c.iter() {
        contents.push(spectrum_messages::Channel {
            chan_type: spectrum_messages::ChannelType::Bin,
            x: chan.x_coord,
            y: chan.y_coord,
            bin: 0,
            value: chan.value as f64,
        });
    }
    if let Err(s) = api.fill_spectrum(name, contents) {
        Err(s)
    } else {
        Ok(())
    }
}

// Given deserialized spectra - enter them in the histogram thread:

fn enter_spectra(
    spectra: &Vec<SpectrumFileData>,
    as_snapshot: bool,
    replace: bool,
    to_shm: bool,
    hg_chan: &State<SharedHistogramChannel>,
    state: &State<SharedBinderChannel>,
) -> Result<(), String> {
    // We need the API:

    let spectrum_api =
        spectrum_messages::SpectrumMessageClient::new(&hg_chan.inner().lock().unwrap());
    let parameter_api =
        parameter_messages::ParameterMessageClient::new(&hg_chan.inner().lock().unwrap());
    let mut parameters = make_parameter_set(&parameter_api)?;
    // snapshots require a _snapshot_condition_ gate.  This is a False
    // condition.  No harm to make it again so just unconditionally make it:
    if as_snapshot {
        let condition_api =
            condition_messages::ConditionMessageClient::new(&hg_chan.inner().lock().unwrap());
        condition_api.create_false_condition("_snapshot_condition_");
    }
    for s in spectra {
        // We need to create parameters for each missing parameter each spectrum
        // needs:

        make_parameters(&s.definition, &mut parameters, &parameter_api)?;

        // Create the spectrum and, if necessary gate it on our False condition.

        let actual_name = enter_spectrum(&s.definition, replace, &spectrum_api)?;
        if as_snapshot {
            spectrum_api.gate_spectrum(&actual_name, "_snapshot_condition_")?
        }

        // Now fill the spectrum from the data we got from the file
        // Note that doing it in this order ensures that snapshots don't have
        // stray counts that can accumulate between spectrum creation and
        // gating the spectrum .

        fill_spectrum(&actual_name, &s.channels, &spectrum_api)?;

        // Bind the spectrum if it's supposed to be in shared memory.

        if to_shm {
            let bind_api = binder::BindingApi::new(&state.inner().lock().unwrap());
            bind_api.bind(&actual_name)?;
        }
    }
    Ok(())
}
// JSON Spectra bin count includes the overflows so 2 must be
// deducted from each one

fn fix_json_bins(input: Vec<SpectrumFileData>) -> Vec<SpectrumFileData> {
    let mut result = Vec::<SpectrumFileData>::new();

    for item in input.iter().map(|x| {
        let mut x = x.clone();
        if x.definition.x_axis.is_some() {
            x.definition.x_axis = Some((
                x.definition.x_axis.unwrap().0,
                x.definition.x_axis.unwrap().1,
                x.definition.x_axis.unwrap().2 - 2,
            ));
        }
        if x.definition.y_axis.is_some() {
            x.definition.y_axis = Some((
                x.definition.y_axis.unwrap().0,
                x.definition.y_axis.unwrap().1,
                x.definition.y_axis.unwrap().2 - 2,
            ));
        }
        x
    }) {
        result.push(item);
    }

    result
}

///
/// sread_handler
///
/// Handle REST requests to read a spectrum.
/// This has a pair of mandatory and a bunch of
/// optionals:
///
/// ### Parameters:
/// *  filename - (mandatory) path to the file to read.
/// *  format - (mandatory) spectrum format.  json and ascii are supported in
/// a case blind way.
/// *  snapshot - (optional) if true (default is yes), a _False_ condition is
/// set on the spectrum that's read in.  If necessary a _False_ condition named
/// _snapshot_condition_ is created.  If snapshot is false, then the spectrum
/// will increment if new data is processed.
/// *  replace - (optional) if true (default is no), it is deleted and
/// a new spectrum created to hold the data with the same name and the
/// characteristics of the spectrum in file.  The default is not, in which case a
/// _similar_ spectrum name is constructedm created and used.
/// *  bind - (optional) if true (defalt is yes),  the final spectrum is
// bound to the Xamine shared memory.
/// * state (mandatory) the state of the server (contains what's needed to
/// access various APIs).
///
/// ### Returns:
///
/// ### Notes:
///   *   It is possible that this will require the creation of new parameters.
///   *   Several spectra can be in one file.
///   *   If replace is true, it is possible that the replaced spectrum
/// will have a completely different definition than the original.
///   * The file is processed serially, that is if there is a failure (e.g.
/// the file format has an error), any spectra correctly read in are fully
/// processed.
#[get("/?<filename>&<format>&<snapshot>&<replace>&<bind>")]
pub fn sread_handler(
    filename: String,
    format: String,
    snapshot: OptionalFlag,
    replace: OptionalFlag,
    bind: OptionalFlag,
    hg_chan: &State<SharedHistogramChannel>,
    state: &State<SharedBinderChannel>,
) -> Json<GenericResponse> {
    // Figure out the flag states:

    let snap = if let Some(s) = snapshot { s } else { true };

    let repl = if let Some(r) = replace { r } else { false };

    let toshm = if let Some(b) = bind { b } else { true };
    //See if we can open the file:  If not that's an error:

    let fd = File::open(&filename);
    if let Err(why) = fd {
        return Json(GenericResponse::err(
            &format!("Failed to open input file: {}", filename),
            &format!("{}", why),
        ));
    }
    let mut fd = fd.unwrap();

    // how we read the spectra depends on the format:

    let mut fmt = format.clone();
    fmt.make_ascii_lowercase();

    let spectra = match fmt.as_str() {
        "json" => read_json(&mut fd),
        "ascii" => spectclio::read_spectra(&mut fd),
        _ => {
            return Json(GenericResponse::err("Unsupported format", &format));
        }
    };

    if spectra.is_err() {
        let msg = spectra.as_ref().err().unwrap();
        return Json(GenericResponse::err("Unable to deserialize from file", msg));
    }
    let spectra = spectra.as_ref().unwrap();

    let response = if let Err(e) = enter_spectra(spectra, snap, repl, toshm, hg_chan, state) {
        GenericResponse::err("Unable to enter spectra in histogram thread: ", &e)
    } else {
        GenericResponse::ok("")
    };
    Json(response)
}
#[cfg(test)]
mod read_tests {
    // reads are easier to sort of test since wwe have the 'test.json' and 'junk.asc' files we can use.

    use super::*;
    use crate::messaging;
    use crate::messaging::{condition_messages, parameter_messages, spectrum_messages}; // to interrogate.
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;

    fn setup() -> Rocket<Build> {
        rest_common::setup().mount("/", routes![sread_handler])
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
    ) {
        rest_common::get_state(r)
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        rest_common::teardown(c, p, b);
    }
    // This is a bit of a long test but then it establishes
    // that pretty much everything, other than the
    // mode options work.  Once this one works we
    // know that we only need to flip switches and look for
    // differences.

    #[test]
    fn json_1() {
        // All thedefaults on test.json make 1 and 2
        // 1 is a 1-d spectrum 2 a 2-d spectrum.  The
        // required parameters are also created.
        // These are snapshot, no replace, and bound to shared memory.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/?filename=test.json&format=json");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status, "Detail: {}", reply.detail);

        // we now should have parameters parameters.{05,06}:

        let param_api = parameter_messages::ParameterMessageClient::new(&chan);
        let p = param_api
            .list_parameters("parameters.05")
            .expect("Getting parameters.05");
        assert_eq!(1, p.len());
        let p = param_api
            .list_parameters("parameters.06")
            .expect("getting parameters.06");
        assert_eq!(1, p.len());

        // There should be a condition named "_snapshot_condition_"
        // and it's a False condition:

        let cond_api = condition_messages::ConditionMessageClient::new(&chan);
        let c = cond_api.list_conditions("_snapshot_condition_");
        assert!(if let condition_messages::ConditionReply::Listing(l) = c {
            assert_eq!(1, l.len());
            assert_eq!("False", l[0].type_name);
            true
        } else {
            false
        });
        // Spectrum '1' exists:
        //  -   Native type is Oned
        //  -   Xparameters is "parameters.05"
        //  -   x_axis = (0,1024,1026)
        //  -   Bin 500 should have 163500 counts.
        //  -   Is bound into shared memory.
        let spec_api = spectrum_messages::SpectrumMessageClient::new(&chan);
        let s = spec_api.list_spectra("1").expect("Listing '1' spectrum");
        assert_eq!(1, s.len());
        let sp = &s[0];
        assert_eq!("1D", sp.type_name);
        assert_eq!(1, sp.xparams.len());
        assert_eq!("parameters.05", sp.xparams[0]);
        let x = sp.xaxis.expect("Unwraping 1's x axis");
        assert_eq!(0.0, x.low);
        assert_eq!(1024.0, x.high);
        assert_eq!(1026, x.bins);
        assert!(sp.yaxis.is_none());
        assert!(sp.gate.is_some());
        assert_eq!("_snapshot_condition_", sp.gate.clone().unwrap());

        let counts = spec_api
            .get_contents("1", 0.0, 1024.0, 0.0, 0.0)
            .expect("getting contents");
        assert_eq!(1, counts.len());
        let ch = &counts[0];
        assert_eq!(500.0, ch.x);
        assert_eq!(501, ch.bin);
        assert_eq!(spectrum_messages::ChannelType::Bin, ch.chan_type);
        assert_eq!(163500.0, ch.value);

        let bindings = bind_api.list_bindings("1").expect("listing bindings");
        assert_eq!(1, bindings.len());
        assert_eq!("1", bindings[0].1);

        // Spectrum '2' exists:
        // - Native type is Twod
        // - xparameters is 'parameters.05"
        // - yparameters is "parameters.06"
        // - xaxis  (0, 1024, 1026)
        // - yaxis  (0, 1024, 1026),
        // - (500, 600) has 163500 counts.
        // - Is bound into shared memory.

        let s = spec_api.list_spectra("2").expect("listing '2' spectrum");
        assert_eq!(1, s.len());
        let sp = &s[0];
        assert_eq!("2D", sp.type_name);
        assert_eq!(1, sp.xparams.len());
        assert_eq!("parameters.05", sp.xparams[0]);
        assert_eq!(1, sp.yparams.len());
        assert_eq!("parameters.06", sp.yparams[0]);
        let x = sp.xaxis.expect("Unwrapping x axis definition of 2");
        assert_eq!(0.0, x.low);
        assert_eq!(1024.0, x.high);
        assert_eq!(1026, x.bins);
        let y = sp.yaxis.expect("UNwrapgin 2's y axis");
        assert_eq!(0.0, y.low);
        assert_eq!(1024.0, y.high);
        assert_eq!(1026, y.bins);
        assert!(sp.gate.is_some());
        assert_eq!("_snapshot_condition_", sp.gate.clone().unwrap());

        let counts = spec_api
            .get_contents("2", 0.0, 1024.0, 0.0, 1024.0)
            .expect("Getting contents of 2");
        assert_eq!(1, counts.len());
        let ch = &counts[0];
        assert_eq!(500.0, ch.x);
        assert_eq!(600.0, ch.y);
        assert_eq!(501 + (601 * 1026), ch.bin);
        assert_eq!(spectrum_messages::ChannelType::Bin, ch.chan_type);
        assert_eq!(163500.0, ch.value);

        let bindings = bind_api.list_bindings("2").expect("Listing bindings");
        assert_eq!(1, bindings.len());
        assert_eq!("2", bindings[0].1);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn json_2() {
        // Turn off snapshot mode and the created spectra won't be
        // gated:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/?filename=test.json&format=json&snapshot=false");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status, "Detail: {}", reply.detail);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let listing = sapi.list_spectra("[12]").expect("Getting spectrum list");
        assert_eq!(2, listing.len());
        for s in listing {
            assert!(s.gate.is_none(), "There's a gate for {}", s.name);
        }

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn json_3() {
        // bind = false does not bind the spectrum:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/?filename=test.json&format=json&bind=false");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status, "Detail: {}", reply.detail);

        let bindings = bind_api.list_bindings("[12]").expect("Getting bindings");
        assert_eq!(0, bindings.len());

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn json_4() {
        // no replace - makes new spectra.  The simplest way to
        // test this is to read twice.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/?filename=test.json&format=json&bind=false");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", reply.status);

        let req = client.get("/?filename=test.json&format=json&bind=false");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", reply.status);

        // should have 2 spectra with names matching 1_* and
        // 2 matching 2_*
        //

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let list = sapi.list_spectra("1*").expect("listing 1*");
        assert_eq!(2, list.len());

        let list = sapi.list_spectra("2*").expect("listing 2*");
        assert_eq!(2, list.len());

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn json_5() {
        // IF replace is allowed double reads don't add spectra:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/?filename=test.json&format=json&bind=false");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", reply.status);

        let req = client.get("/?filename=test.json&format=json&replace=true&bind=false");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", reply.status);

        // should have 2 spectra with names matching 1_* and
        // 2 matching 2_*
        //

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let list = sapi.list_spectra("1*").expect("listing 1*");
        assert_eq!(1, list.len());

        let list = sapi.list_spectra("2*").expect("listing 2*");
        assert_eq!(1, list.len());

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn json_6() {
        // no such file:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/?filename=/no/such/test.json&format=json&bind=false");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!(
            "Failed to open input file: /no/such/test.json",
            reply.status
        );

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn json_7() {
        // Bad file format:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/?filename=Cargo.toml&format=json");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Unable to deserialize from file", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    // Test ASCII reads note that all the option handling is common
    // code as is the unable to open file thing.
    // We will test the default case and bad format case.
    ///
    #[test]
    fn ascii_1() {
        // Read ASCII file with default options.  This is the same
        // stuff as test.json so the same tests can be done:

        // All thedefaults on test.json make 1 and 2
        // 1 is a 1-d spectrum 2 a 2-d spectrum.  The
        // required parameters are also created.
        // These are snapshot, no replace, and bound to shared memory.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/?filename=junk.asc&format=ascii");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status, "Detail: {}", reply.detail);

        // we now should have parameters parameters.{05,06}:

        let param_api = parameter_messages::ParameterMessageClient::new(&chan);
        let p = param_api
            .list_parameters("parameters.05")
            .expect("Getting parameters.05");
        assert_eq!(1, p.len());
        let p = param_api
            .list_parameters("parameters.06")
            .expect("getting parameters.06");
        assert_eq!(1, p.len());

        // There should be a condition named "_snapshot_condition_"
        // and it's a False condition:

        let cond_api = condition_messages::ConditionMessageClient::new(&chan);
        let c = cond_api.list_conditions("_snapshot_condition_");
        assert!(if let condition_messages::ConditionReply::Listing(l) = c {
            assert_eq!(1, l.len());
            assert_eq!("False", l[0].type_name);
            true
        } else {
            false
        });
        // Spectrum '1' exists:
        //  -   Native type is Oned
        //  -   Xparameters is "parameters.05"
        //  -   x_axis = (0,1024,1026)
        //  -   Bin 500 should have 163500 counts.
        //  -   Is bound into shared memory.
        let spec_api = spectrum_messages::SpectrumMessageClient::new(&chan);
        let s = spec_api.list_spectra("1").expect("Listing '1' spectrum");
        assert_eq!(1, s.len());
        let sp = &s[0];
        assert_eq!("1D", sp.type_name);
        assert_eq!(1, sp.xparams.len());
        assert_eq!("parameters.05", sp.xparams[0]);
        let x = sp.xaxis.expect("Unwraping 1's x axis");
        assert_eq!(0.0, x.low);
        assert_eq!(1024.0, x.high);
        assert_eq!(1026, x.bins);
        assert!(sp.yaxis.is_none());
        assert!(sp.gate.is_some());
        assert_eq!("_snapshot_condition_", sp.gate.clone().unwrap());

        let counts = spec_api
            .get_contents("1", 0.0, 1024.0, 0.0, 0.0)
            .expect("getting contents");
        assert_eq!(1, counts.len());
        let ch = &counts[0];
        assert_eq!(500.0, ch.x);
        assert_eq!(501, ch.bin);
        assert_eq!(spectrum_messages::ChannelType::Bin, ch.chan_type);
        assert_eq!(163500.0, ch.value);

        let bindings = bind_api.list_bindings("1").expect("listing bindings");
        assert_eq!(1, bindings.len());
        assert_eq!("1", bindings[0].1);

        // Spectrum '2' exists:
        // - Native type is Twod
        // - xparameters is 'parameters.05"
        // - yparameters is "parameters.06"
        // - xaxis  (0, 1024, 1026)
        // - yaxis  (0, 1024, 1026),
        // - (500, 600) has 163500 counts.
        // - Is bound into shared memory.

        let s = spec_api.list_spectra("2").expect("listing '2' spectrum");
        assert_eq!(1, s.len());
        let sp = &s[0];
        assert_eq!("2D", sp.type_name);
        assert_eq!(1, sp.xparams.len());
        assert_eq!("parameters.05", sp.xparams[0]);
        assert_eq!(1, sp.yparams.len());
        assert_eq!("parameters.06", sp.yparams[0]);
        let x = sp.xaxis.expect("Unwrapping x axis definition of 2");
        assert_eq!(0.0, x.low);
        assert_eq!(1024.0, x.high);
        assert_eq!(1026, x.bins);
        let y = sp.yaxis.expect("UNwrapgin 2's y axis");
        assert_eq!(0.0, y.low);
        assert_eq!(1024.0, y.high);
        assert_eq!(1026, y.bins);
        assert!(sp.gate.is_some());
        assert_eq!("_snapshot_condition_", sp.gate.clone().unwrap());

        let counts = spec_api
            .get_contents("2", 0.0, 1024.0, 0.0, 1024.0)
            .expect("Getting contents of 2");
        assert_eq!(1, counts.len());
        let ch = &counts[0];
        assert_eq!(500.0, ch.x);
        assert_eq!(600.0, ch.y);
        assert_eq!(501 + (601 * 1026), ch.bin);
        assert_eq!(spectrum_messages::ChannelType::Bin, ch.chan_type);
        assert_eq!(163500.0, ch.value);

        let bindings = bind_api.list_bindings("2").expect("Listing bindings");
        assert_eq!(1, bindings.len());
        assert_eq!("2", bindings[0].1);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn ascii_2() {
        // Bad file format ASCII works but produces nothing:
        // There is an issue that this should be changed to produce
        // errors (Issue #88).

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/?filename=Cargo.toml&format=ascii");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Unable to deserialize from file", reply.status);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let l = sapi.list_spectra("*").expect("listing spectra");
        assert_eq!(0, l.len());

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn bad_format() {
        // Format specification in get is bad:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/?filename=test.json&format=no-such-format");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Unsupported format", reply.status);

        teardown(chan, &papi, &bind_api);
    }
}
// Testing swrite is a bit harder.
// I think what I'll do is populate spectra,
// Do swrites and then sreads to check that the spectra read in
// match the ones written out.
// this relies on a well tested sread (see the module above).
// we'll read with bind=false so we don't need a big shared memory region.
//
#[cfg(test)]
mod swrite_tests {
    use super::*;
    use crate::messaging;
    use crate::messaging::{parameter_messages, spectrum_messages}; // to interrogate.
    use crate::parameters;
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;

    use names;

    fn setup() -> Rocket<Build> {
        let rocket = rest_common::setup()
            .mount("/swrite", routes![swrite_handler])
            .mount("/sread", routes![sread_handler]);

        // Make a parameter and spectrum API so that we can
        // call make_test_spectra:

        let papi = parameter_messages::ParameterMessageClient::new(
            &rocket
                .state::<SharedHistogramChannel>()
                .expect("Getting state")
                .lock()
                .unwrap()
                .clone(),
        );
        let hapi = spectrum_messages::SpectrumMessageClient::new(
            &rocket
                .state::<SharedHistogramChannel>()
                .expect("Getting state")
                .lock()
                .unwrap()
                .clone(),
        );
        make_test_spectra(&papi, &hapi);

        //

        rocket
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
    ) {
        rest_common::get_state(r)
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        rest_common::teardown(c, p, b);
    }

    //Creating spectra is divorced from filling it so we
    //can write/read empty spectra if we want:

    fn make_test_spectra(
        papi: &parameter_messages::ParameterMessageClient,
        sapi: &spectrum_messages::SpectrumMessageClient,
    ) {
        // Make parameters p.0 -> p.9:
        // ids 1-10.
        //
        for i in 0..10 {
            let name = format!("p.{}", i);
            papi.create_parameter(&name)
                .expect("Failed to make parameter");
        }

        // make 1 of each kind of spectrum:
        // we're going to keep them unbound so we don't need to

        sapi.create_spectrum_1d("oned", "p.0", 0.0, 1024.0, 1024)
            .expect("Create 'oned'");
        sapi.create_spectrum_multi1d(
            "gamma1",
            &[
                String::from("p.0"),
                String::from("p.1"),
                String::from("p.2"),
                String::from("p.3"),
                String::from("p.4"),
                String::from("p.5"),
                String::from("p.6"),
                String::from("p.7"),
                String::from("p.8"),
                String::from("p.9"),
            ],
            0.0,
            1024.0,
            1024,
        )
        .expect("Making multi-1d spectrum");
        sapi.create_spectrum_multi2d(
            "gamma2",
            &[
                String::from("p.0"),
                String::from("p.1"),
                String::from("p.2"),
                String::from("p.3"),
                String::from("p.4"),
                String::from("p.5"),
                String::from("p.6"),
                String::from("p.7"),
                String::from("p.8"),
                String::from("p.9"),
            ],
            0.0,
            512.0,
            512,
            0.0,
            512.0,
            512,
        )
        .expect("Multi 2d spectrum");
        sapi.create_spectrum_pgamma(
            "particle-gamma",
            &[
                String::from("p.0"),
                String::from("p.1"),
                String::from("p.2"),
                String::from("p.3"),
                String::from("p.4"),
            ],
            &[
                String::from("p.5"),
                String::from("p.6"),
                String::from("p.7"),
                String::from("p.8"),
                String::from("p.9"),
            ],
            0.0,
            256.0,
            256,
            0.0,
            256.0,
            256,
        )
        .expect("particle-gamma spectrum");
        sapi.create_spectrum_summary(
            "summary",
            &[
                String::from("p.0"),
                String::from("p.1"),
                String::from("p.2"),
                String::from("p.3"),
                String::from("p.4"),
                String::from("p.5"),
                String::from("p.6"),
                String::from("p.7"),
                String::from("p.8"),
                String::from("p.9"),
            ],
            0.0,
            1024.0,
            1024,
        )
        .expect("summary spectrum");
        sapi.create_spectrum_2d("twod", "p.0", "p.1", 0.0, 256.0, 256, 0.0, 256.0, 256)
            .expect("Making twod");
        sapi.create_spectrum_2dsum(
            "2d-sum",
            &[
                String::from("p.0"),
                String::from("p.1"),
                String::from("p.2"),
                String::from("p.3"),
                String::from("p.4"),
            ],
            &[
                String::from("p.5"),
                String::from("p.6"),
                String::from("p.7"),
                String::from("p.8"),
                String::from("p.9"),
            ],
            0.0,
            256.0,
            256,
            0.0,
            256.0,
            256,
        )
        .expect("Making 2d sum spectrum");
    }
    fn fill_test_spectra(api: &spectrum_messages::SpectrumMessageClient) {
        // we'll make rolling values.

        let num_events = 100;
        let mut events = vec![];
        for evt in 0..num_events {
            let mut event = vec![];
            for i in 1..11 {
                event.push(parameters::EventParameter::new(i, (i * 10 + evt) as f64));
            }
            events.push(event);
        }
        api.process_events(&events).expect("FIlling spectra");
    }

    #[test]
    fn setup_and_teardown() {
        // JUst make sure we can get throur the initialization/shutdown

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn json1d_1() {
        // Write the empty 1d spectrum as json. see if it reads back:

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?file={}&format=json&spectrum=oned", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing write JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?filename={}&format=json&bind=false", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing read JSON");
        assert_eq!("OK", read_response.status);

        // The base spectrum is 'oned'.  The one read in should be
        // 'oned_0'

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let original = sapi.list_spectra("oned").expect("Listing oned");
        let copy = sapi.list_spectra("oned_0").expect("Listing oned_0");

        // Spectrum descriptions must match execpt for the gate
        // which is the snapshot condition since we did not turn that off.

        assert_eq!(1, original.len());
        let o = &original[0];
        assert_eq!(1, copy.len());
        let c = &copy[0];
        assert_eq!(o.type_name, c.type_name);
        assert_eq!(o.xparams, c.xparams);
        assert_eq!(o.yparams, c.yparams);
        assert_eq!(o.xaxis, c.xaxis);
        assert_eq!(o.yaxis, c.yaxis);
        assert_eq!(Some(String::from("_snapshot_condition_")), c.gate);

        // Should not have counts:

        let contents = sapi
            .get_contents(&c.name, 0.0, 1024.0, 0.0, 1024.0)
            .expect("Getting read spectrum contents");
        assert_eq!(0, contents.len());

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn json1d_2() {
        // Put counts in the two spectra, they should match:

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?file={}&format=json&spectrum=oned", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing write JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?filename={}&format=json&bind=false", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing read JSON");
        assert_eq!("OK", read_response.status);

        // get contents of both "oned" and "oned_0":

        let original_contents = sapi
            .get_contents("oned", 0.0, 1024.0, 0.0, 1024.0)
            .expect("original contents");
        let copy_contents = sapi
            .get_contents("oned_0", 0.0, 1024.0, 0.0, 1024.0)
            .expect("copy contents");

        assert_eq!(original_contents, copy_contents);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn ascii1d_1() {
        // Write the empty 1d spectrum as ascii. see if it reads back:

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?file={}&format=ascii&spectrum=oned", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing write JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?filename={}&format=ascii&bind=false", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing read JSON");
        assert_eq!("OK", read_response.status);

        // The base spectrum is 'oned'.  The one read in should be
        // 'oned_0'

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let original = sapi.list_spectra("oned").expect("Listing oned");
        let copy = sapi.list_spectra("oned_0").expect("Listing oned_0");

        // Spectrum descriptions must match except for the gate
        // which is the snapshot condition since we did not turn that off.

        assert_eq!(1, original.len());
        let o = &original[0];
        assert_eq!(1, copy.len());
        let c = &copy[0];
        assert_eq!(o.type_name, c.type_name);
        assert_eq!(o.xparams, c.xparams);
        assert_eq!(o.yparams, c.yparams);
        assert_eq!(o.xaxis, c.xaxis);
        assert_eq!(o.yaxis, c.yaxis);
        assert_eq!(Some(String::from("_snapshot_condition_")), c.gate);

        // Should not have counts:

        let contents = sapi
            .get_contents(&c.name, 0.0, 1024.0, 0.0, 1024.0)
            .expect("Getting read spectrum contents");
        assert_eq!(0, contents.len());

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn ascii1d_2() {
        // Put counts in the two spectra, they should match:

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?file={}&format=ascii&spectrum=oned", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing write JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?filename={}&format=ascii&bind=false", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing read JSON");
        assert_eq!("OK", read_response.status);

        // get contents of both "oned" and "oned_0":

        let original_contents = sapi
            .get_contents("oned", 0.0, 1024.0, 0.0, 1024.0)
            .expect("original contents");
        let copy_contents = sapi
            .get_contents("oned_0", 0.0, 1024.0, 0.0, 1024.0)
            .expect("copy contents");

        assert_eq!(original_contents, copy_contents);
        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn jsong1_1() {
        // empty g1 spectrum (the metadata are correct):

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=gamma1&format=json&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=json&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let original = sapi
            .list_spectra("gamma1")
            .expect("getting gamma1 descriptions");
        assert_eq!(1, original.len());
        let o = &original[0];
        let copy = sapi
            .list_spectra("gamma1_0")
            .expect("Getting gamma1_0 desription");
        assert_eq!(1, copy.len());
        let c = &copy[0];

        assert_eq!(o.type_name, c.type_name);
        assert_eq!(o.xparams, c.xparams);
        assert_eq!(o.yparams, c.yparams);
        assert_eq!(o.xaxis, c.xaxis);
        assert_eq!(o.yaxis, c.yaxis);
        assert_eq!(Some(String::from("_snapshot_condition_")), c.gate);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn jsong1_2() {
        // FIll the spectra this time:
        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=gamma1&format=json&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=json&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let original_contents = sapi
            .get_contents("gamma1", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'gamma1' contents");
        let copy_contents = sapi
            .get_contents("gamma1_0", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'gamma1_0 contents");
        assert_eq!(original_contents, copy_contents);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn asciig1_1() {
        // empty g1 spectrum (the metadata are correct):

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=gamma1&format=ascii&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=ascii&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let original = sapi
            .list_spectra("gamma1")
            .expect("getting gamma1 descriptions");
        assert_eq!(1, original.len());
        let o = &original[0];
        let copy = sapi
            .list_spectra("gamma1_0")
            .expect("Getting gamma1_0 desription");
        assert_eq!(1, copy.len());
        let c = &copy[0];

        assert_eq!(o.type_name, c.type_name);
        assert_eq!(o.xparams, c.xparams);
        assert_eq!(o.yparams, c.yparams);
        assert_eq!(o.xaxis, c.xaxis);
        assert_eq!(o.yaxis, c.yaxis);
        assert_eq!(Some(String::from("_snapshot_condition_")), c.gate);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn asciig1_2() {
        // FIll the spectra this time:
        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=gamma1&format=ascii&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=ascii&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let original_contents = sapi
            .get_contents("gamma1", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'gamma1' contents");
        let copy_contents = sapi
            .get_contents("gamma1_0", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'gamma1_0 contents");
        assert_eq!(original_contents, copy_contents);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn jsong2_1() {
        // empty g2 spectrum (the metadata are correct):

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=gamma2&format=json&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=json&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let original = sapi
            .list_spectra("gamma2")
            .expect("getting gamma2 descriptions");
        assert_eq!(1, original.len());
        let o = &original[0];
        let copy = sapi
            .list_spectra("gamma2_0")
            .expect("Getting gamma2_0 desription");
        assert_eq!(1, copy.len());
        let c = &copy[0];

        assert_eq!(o.type_name, c.type_name);
        assert_eq!(o.xparams, c.xparams);
        assert_eq!(o.yparams, c.yparams);
        assert_eq!(o.xaxis, c.xaxis);
        assert_eq!(o.yaxis, c.yaxis);
        assert_eq!(Some(String::from("_snapshot_condition_")), c.gate);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn jsong2_2() {
        // FIll the spectra this time:
        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=gamma2&format=json&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=json&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let original_contents = sapi
            .get_contents("gamma2", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'gamma1' contents");
        let copy_contents = sapi
            .get_contents("gamma2_0", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'gamma1_0 contents");
        assert_eq!(original_contents, copy_contents);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn asciig2_1() {
        // empty g1 spectrum (the metadata are correct):

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=gamma2&format=ascii&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=ascii&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let original = sapi
            .list_spectra("gamma2")
            .expect("getting gamma2 descriptions");
        assert_eq!(1, original.len());
        let o = &original[0];
        let copy = sapi
            .list_spectra("gamma2_0")
            .expect("Getting gamma2_0 desription");
        assert_eq!(1, copy.len());
        let c = &copy[0];

        assert_eq!(o.type_name, c.type_name);
        assert_eq!(o.xparams, c.xparams);
        assert_eq!(o.yparams, c.yparams);
        assert_eq!(o.xaxis, c.xaxis);
        assert_eq!(o.yaxis, c.yaxis);
        assert_eq!(Some(String::from("_snapshot_condition_")), c.gate);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn asciig2_2() {
        // FIll the spectra this time:
        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=gamma2&format=ascii&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=ascii&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let original_contents = sapi
            .get_contents("gamma2", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'gamma2' contents");
        let copy_contents = sapi
            .get_contents("gamma2_0", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'gamma2_0 contents");
        assert_eq!(original_contents, copy_contents);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }

    #[test]
    fn jsonpg_1() {
        // empty g2 spectrum (the metadata are correct):

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!(
            "/swrite?spectrum=particle-gamma&format=json&file={}",
            filename
        );
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=json&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let original = sapi
            .list_spectra("particle-gamma")
            .expect("getting particle-gamma descriptions");
        assert_eq!(1, original.len());
        let o = &original[0];
        let copy = sapi
            .list_spectra("particle-gamma_0")
            .expect("Getting particle-gamma_0 desription");
        assert_eq!(1, copy.len());
        let c = &copy[0];

        assert_eq!(o.type_name, c.type_name);
        assert_eq!(o.xparams, c.xparams);
        assert_eq!(o.yparams, c.yparams);
        assert_eq!(o.xaxis, c.xaxis);
        assert_eq!(o.yaxis, c.yaxis);
        assert_eq!(Some(String::from("_snapshot_condition_")), c.gate);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn jsonpg_2() {
        // FIll the spectra this time:
        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!(
            "/swrite?spectrum=particle-gamma&format=json&file={}",
            filename
        );
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=json&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let original_contents = sapi
            .get_contents("particle-gamma", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'particle-gamma' contents");
        let copy_contents = sapi
            .get_contents("particle-gamma_0", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'particle-gamma_0 contents");
        assert_eq!(original_contents, copy_contents);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn asciipg_1() {
        // empty g1 spectrum (the metadata are correct):

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!(
            "/swrite?spectrum=particle-gamma&format=ascii&file={}",
            filename
        );
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=ascii&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let original = sapi
            .list_spectra("particle-gamma")
            .expect("getting particle-gamma descriptions");
        assert_eq!(1, original.len());
        let o = &original[0];
        let copy = sapi
            .list_spectra("particle-gamma_0")
            .expect("Getting particle-gamma_0 desription");
        assert_eq!(1, copy.len());
        let c = &copy[0];

        assert_eq!(o.type_name, c.type_name);
        assert_eq!(o.xparams, c.xparams);
        assert_eq!(o.yparams, c.yparams);
        assert_eq!(o.xaxis, c.xaxis);
        assert_eq!(o.yaxis, c.yaxis);
        assert_eq!(Some(String::from("_snapshot_condition_")), c.gate);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn asciipg_2() {
        // FIll the spectra this time:
        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!(
            "/swrite?spectrum=particle-gamma&format=ascii&file={}",
            filename
        );
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=ascii&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let original_contents = sapi
            .get_contents("particle-gamma", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'particle-gamma' contents");
        let copy_contents = sapi
            .get_contents("particle-gamma_0", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'particle-gamma_0 contents");
        //assert_eq!(original_contents, copy_contents);
        assert_eq!(original_contents, copy_contents);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }

    #[test]
    fn jsonsum_1() {
        // empty g2 spectrum (the metadata are correct):

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=summary&format=json&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=json&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let original = sapi
            .list_spectra("summary")
            .expect("getting summary descriptions");
        assert_eq!(1, original.len());
        let o = &original[0];
        let copy = sapi
            .list_spectra("summary_0")
            .expect("Getting summary_0 desription");
        assert_eq!(1, copy.len());
        let c = &copy[0];

        assert_eq!(o.type_name, c.type_name);
        assert_eq!(o.xparams, c.xparams);
        assert_eq!(o.yparams, c.yparams);
        assert_eq!(o.xaxis, c.xaxis);
        assert_eq!(o.yaxis, c.yaxis);
        assert_eq!(Some(String::from("_snapshot_condition_")), c.gate);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn jsonsum_2() {
        // FIll the spectra this time:
        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=summary&format=json&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=json&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let original_contents = sapi
            .get_contents("summary", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'summary' contents");
        let copy_contents = sapi
            .get_contents("summary_0", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'summary_0 contents");
        assert_eq!(original_contents, copy_contents);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn asciisum_1() {
        // empty g1 spectrum (the metadata are correct):

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=summary&format=ascii&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=ascii&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let original = sapi
            .list_spectra("summary")
            .expect("getting summary descriptions");
        assert_eq!(1, original.len());
        let o = &original[0];
        let copy = sapi
            .list_spectra("summary_0")
            .expect("Getting summary_0 desription");
        assert_eq!(1, copy.len());
        let c = &copy[0];

        assert_eq!(o.type_name, c.type_name);
        assert_eq!(o.xparams, c.xparams);
        assert_eq!(o.yparams, c.yparams);
        assert_eq!(o.xaxis, c.xaxis);
        assert_eq!(o.yaxis, c.yaxis);
        assert_eq!(Some(String::from("_snapshot_condition_")), c.gate);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn asciisum_2() {
        // FIll the spectra this time:
        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=summary&format=ascii&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=ascii&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let original_contents = sapi
            .get_contents("summary", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'summary' contents");
        let copy_contents = sapi
            .get_contents("summary_0", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'summary_0 contents");
        //assert_eq!(original_contents, copy_contents);
        assert_eq!(original_contents, copy_contents);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn json2d_1() {
        // Empty 2d spectrum:

        // empty g2 spectrum (the metadata are correct):

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=twod&format=json&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=json&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let original = sapi
            .list_spectra("twod")
            .expect("getting twod descriptions");
        assert_eq!(1, original.len());
        let o = &original[0];
        let copy = sapi
            .list_spectra("twod_0")
            .expect("Getting twod_0 desription");
        assert_eq!(1, copy.len());
        let c = &copy[0];

        assert_eq!(o.type_name, c.type_name);
        assert_eq!(o.xparams, c.xparams);
        assert_eq!(o.yparams, c.yparams);
        assert_eq!(o.xaxis, c.xaxis);
        assert_eq!(o.yaxis, c.yaxis);
        assert_eq!(Some(String::from("_snapshot_condition_")), c.gate);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn json2d_2() {
        // FIll the spectra this time:
        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=twod&format=json&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=json&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let original_contents = sapi
            .get_contents("twod", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'twod' contents");
        let copy_contents = sapi
            .get_contents("twod_0", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'twod_0 contents");
        assert_eq!(original_contents, copy_contents);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }

    #[test]
    fn ascii2d_1() {
        // Empty 2d spectrum:

        // empty g2 spectrum (the metadata are correct):

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=twod&format=ascii&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=ascii&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let original = sapi
            .list_spectra("twod")
            .expect("getting twod descriptions");
        assert_eq!(1, original.len());
        let o = &original[0];
        let copy = sapi
            .list_spectra("twod_0")
            .expect("Getting twod_0 desription");
        assert_eq!(1, copy.len());
        let c = &copy[0];

        assert_eq!(o.type_name, c.type_name);
        assert_eq!(o.xparams, c.xparams);
        assert_eq!(o.yparams, c.yparams);
        assert_eq!(o.xaxis, c.xaxis);
        assert_eq!(o.yaxis, c.yaxis);
        assert_eq!(Some(String::from("_snapshot_condition_")), c.gate);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn ascii2d_2() {
        // FIll the spectra this time:
        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=twod&format=ascii&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=ascii&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let original_contents = sapi
            .get_contents("twod", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'twod' contents");
        let copy_contents = sapi
            .get_contents("twod_0", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting 'twod_0 contents");
        assert_eq!(original_contents, copy_contents);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }

    #[test]
    fn json2dsum_1() {
        // Empty 2d spectrum:

        // empty g2 spectrum (the metadata are correct):

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=2d-sum&format=json&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=json&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let original = sapi
            .list_spectra("2d-sum")
            .expect("getting 2d-sum descriptions");
        assert_eq!(1, original.len());
        let o = &original[0];
        let copy = sapi
            .list_spectra("2d-sum_0")
            .expect("Getting 2d-sum_0 desription");
        assert_eq!(1, copy.len());
        let c = &copy[0];

        assert_eq!(o.type_name, c.type_name);
        assert_eq!(o.xparams, c.xparams);
        assert_eq!(o.yparams, c.yparams);
        assert_eq!(o.xaxis, c.xaxis);
        assert_eq!(o.yaxis, c.yaxis);
        assert_eq!(Some(String::from("_snapshot_condition_")), c.gate);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn json2dsum_2() {
        // FIll the spectra this time:
        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=2d-sum&format=json&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=json&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let original_contents = sapi
            .get_contents("2d-sum", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting '2d-sum' contents");
        let copy_contents = sapi
            .get_contents("2d-sum_0", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting '2d-sum_0 contents");
        assert_eq!(original_contents, copy_contents);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }

    #[test]
    fn ascii2dsum_1() {
        // Empty 2d spectrum:

        // empty g2 spectrum (the metadata are correct):

        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=2d-sum&format=ascii&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=ascii&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let original = sapi
            .list_spectra("2d-sum")
            .expect("getting 2d-sum descriptions");
        assert_eq!(1, original.len());
        let o = &original[0];
        let copy = sapi
            .list_spectra("2d-sum_0")
            .expect("Getting 2d-sum_0 desription");
        assert_eq!(1, copy.len());
        let c = &copy[0];

        assert_eq!(o.type_name, c.type_name);
        assert_eq!(o.xparams, c.xparams);
        assert_eq!(o.yparams, c.yparams);
        assert_eq!(o.xaxis, c.xaxis);
        assert_eq!(o.yaxis, c.yaxis);
        assert_eq!(Some(String::from("_snapshot_condition_")), c.gate);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn ascii2dsum_2() {
        // FIll the spectra this time:
        let filename = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .expect("making filename");
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        fill_test_spectra(&sapi);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let write_uri = format!("/swrite?spectrum=2d-sum&format=ascii&file={}", filename);
        let write_req = client.get(&write_uri);
        let write_response = write_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", write_response.status);

        let read_uri = format!("/sread?format=ascii&bind=false&filename={}", filename);
        let read_req = client.get(&read_uri);
        let read_response = read_req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing read JSON");
        assert_eq!("OK", read_response.status);

        // make sure the descriptions of gamma1 and gamma1_0 match (except,
        // of course the names and gates).

        let original_contents = sapi
            .get_contents("2d-sum", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting '2d-sum' contents");
        let copy_contents = sapi
            .get_contents("2d-sum_0", 0.0, 1024.0, 0.0, 1024.0)
            .expect("getting '2d-sum_0 contents");
        assert_eq!(original_contents, copy_contents);

        std::fs::remove_file(&filename).expect("removing test file");
        teardown(chan, &papi, &bind_api);
    }
}
