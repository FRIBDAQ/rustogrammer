//!  This module implements the output part of
//!  SpecTcl compatible I/O.  The form of a spectrum output
//! file from SpecTcl is a header followed by  a line of dashes
//! Followed by spetrum data itself.  This is all fully ASCII.  The
//! header consists of:
//!
//!  *  The spectrum name surrounded by double quotes and a parenthesized
//! space separated list of dimensions (bin coordinates) e.g:
//!     "test.00" (500)
//!
//!  * The data/time at which the file was written e.g.:
//!     Wed May 10 11:34:16 2023
//!
//!  *  The spectrum file format version level, currently _3_
//!  *  The spectrum SpecTcl type and data type e.g.:
//!         1 long
//!  *  The list of parameters space separated, quoted and surrounded
//! by parentheses e.g.:
//!        ("event.raw.00")
//! Note that PGamma spectra have two parameter lists e.g.:
//!      ("x1" "x2", "x3")  ("y1", "y2")
//!  *  The spectrum axis definitions.  This is one or two
//! parenthesized space separated pairs of low/high values e.g.:
//!      (0 1023) (-1.5 1.5)
//!
//! This is all followed by the header ending line of
//! 40 or so "-" characters (probably one is enough but I'm not checking).
//!
//! Spectrumdata consists of either:
//!
//! * (xbin) counts   <br />
//! lines for 1-d spectra and
//! * (xbin ybin) counts <br />
//! lines for 2-d spectra.
//!

use crate::messaging::spectrum_messages::ChannelType;
use crate::rest::spectrum;
use crate::rest::spectrumio::{SpectrumChannel, SpectrumFileData, SpectrumProperties};
use chrono::prelude::*;
use std::io::{prelude::*, BufReader, Lines, Read, Write};

//---------------------------------------------------------------------
// This section of code handles writing spectra to a writable object.

fn fdwrite(fd: &mut dyn Write, b: &str) -> Result<(), String> {
    if let Err(e) = fd.write(b.as_bytes()) {
        Err(format!("{}", e))
    } else {
        Ok(())
    }
}

// Is a spectrum type string for a 1d?

fn is_1d(t: &str) -> bool {
    match t {
        "1" => true,
        "g1" => true,
        "g2" => false,
        "gd" => false,
        "s" => false,
        "2" => false,
        "m2" => false,
        _ => {
            panic!("Unrecognized spectrum type {}", t);
        }
    }
}

// Write a 1d channel:

fn write_1(fd: &mut dyn Write, c: &SpectrumChannel) -> Result<(), String> {
    fdwrite(fd, &format!("({}) {}\n", c.x_bin - 1, c.value))
}
// write a 2-d channel
fn write_2(fd: &mut dyn Write, c: &SpectrumChannel) -> Result<(), String> {
    fdwrite(
        fd,
        &format!("({} {}) {}\n", c.x_bin - 1, c.y_bin - 1, c.value),
    )
}

fn write_channels(
    fd: &mut dyn Write,
    chans: &Vec<SpectrumChannel>,
    f: fn(&mut dyn Write, &SpectrumChannel) -> Result<(), String>,
) -> Result<(), String> {
    for c in chans.iter() {
        f(fd, c)?;
    }
    fdwrite(fd, "(-1 -1)\n")?; // End of data sentinel
    Ok(())
}

fn write_axis_def(fd: &mut dyn Write, low: f64, high: f64) -> Result<(), String> {
    fdwrite(fd, &format!("({} {}) ", low, high))
}

fn write_string_list(fd: &mut dyn Write, s: &Vec<String>) -> Result<(), String> {
    fdwrite(fd, "(")?;

    for px in s.iter() {
        fdwrite(fd, &format!("\"{}\" ", px))?;
    }
    fdwrite(fd, ") ")
}

/// This method writes a spectrum to any object that supports the Write
/// trait.

pub fn write_spectrum(fd: &mut dyn Write, spectra: &Vec<SpectrumFileData>) -> Result<(), String> {
    for spectrum in spectra.iter() {
        // Header: Spectrum name/bins:
        fdwrite(fd, &format!("\"{}\"", spectrum.definition.name))?;
        fdwrite(fd, " (")?;
        if let Some((_, _, bins)) = spectrum.definition.x_axis {
            let bins = bins - 2;
            fdwrite(fd, &bins.to_string())?;
            fdwrite(fd, " ")?;
        }
        if let Some((_, _, bins)) = spectrum.definition.y_axis {
            let bins = bins - 2;
            fdwrite(fd, &bins.to_string())?;
        }
        fdwrite(fd, ")\n")?;
        // Date/time stamp.

        fdwrite(fd, &format!("{}\n", Local::now()))?;

        // Format version:

        fdwrite(fd, "3\n")?;

        // Spectrum type and data type:   data type is always 'long'

        fdwrite(fd, &format!("{} long\n", spectrum.definition.type_string,))?;

        // Parenthesized names of parameters (x) - if not pgamma this is one
        // list otherwise two:

        if spectrum.definition.type_string.as_str() != "gd" {
            let mut params = spectrum.definition.x_parameters.clone();
            params.extend(spectrum.definition.y_parameters.clone());
            write_string_list(fd, &params)?;
        } else {
            // X and y parameters are in separate lists:

            write_string_list(fd, &spectrum.definition.x_parameters)?;
            write_string_list(fd, &spectrum.definition.y_parameters)?;
        }
        fdwrite(fd, "\n")?;

        // axis low and high for all defined axes:
        // Note for summary we just write the y axis.

        if spectrum.definition.type_string.as_str() != "s" {
            if let Some((lo, hi, _)) = spectrum.definition.x_axis {
                write_axis_def(fd, lo, hi)?;
            }
            if let Some((lo, hi, _)) = spectrum.definition.y_axis {
                write_axis_def(fd, lo, hi)?;
            }
        } else {
            let (lo, hi, _) = spectrum.definition.y_axis.unwrap();
            write_axis_def(fd, lo, hi)?;
        }
        fdwrite(fd, "\n")?;

        // Header terminator:

        fdwrite(fd, "--------------------------------------------\n")?;

        // Write the channels:

        if is_1d(&spectrum.definition.type_string) {
            write_channels(fd, &spectrum.channels, write_1)?;
        } else {
            write_channels(fd, &spectrum.channels, write_2)?;
        }
    }
    Ok(())
}
// This section of code handles reading spectra from a Readable
//---------------------------------------------------------------------
// object

// Get a line from a line iterator -- simplifying error handling:

fn read_line<T: Read>(l: &mut Lines<BufReader<T>>) -> Result<String, String> {
    let line = l.next();
    if let None = line {
        return Err(String::from("End of file"));
    }
    let line = line.unwrap();
    if let Err(s) = line {
        return Err(format!("Error trying to get a line : {}", s));
    }
    Ok(line.unwrap())
}

// Axis definitions are 2 element vectors that parse to  f64:
fn parse_axis(straxis: &Vec<String>) -> Result<(f64, f64), String> {
    if straxis.len() != 2 {
        return Err(format!(
            "The axis definitions must have 2 elements {:?}",
            straxis
        ));
    }
    let lo = straxis[0].parse::<f64>();
    if let Err(s) = lo {
        return Err(format!(
            "Low value cold not be parsed: {} : {}",
            straxis[0], s
        ));
    }
    let lo = lo.unwrap();
    let hi = straxis[1].parse::<f64>();
    if let Err(s) = hi {
        return Err(format!(
            "High value cold not be parsed: {} : {}",
            straxis[1], s
        ));
    }
    Ok((lo, hi.unwrap()))
}

// A couple of places in the header, we have one or two parenthesized lists
// for example parameters are:
//
// (xparam1 [xparam2 ...]) [(yparam1 [yparam2...])]
//
// and axis defs are:
//
//  (low high) [(low high)]
//
// Depending on the underlying spectrum dimensionality.
//
fn parse_paren_list(line: &str) -> Result<(Vec<String>, Vec<String>), String> {
    let mut xparams: Vec<String> = vec![];
    let mut yparams: Vec<String> = vec![];

    // There must be a "(" and a ")"  If not that's an error:

    let open = line.find('(');
    let close = line.find(')');
    if open.is_none() || close.is_none() {
        return Err(format!("Can't find the X parameter list in '{}'", line));
    }
    let open = open.unwrap();
    let close = close.unwrap();

    let x: Vec<&str> = line[open + 1..close].split_whitespace().collect();
    for xp in x {
        xparams.push(String::from(xp));
    }

    // Remainder is the string after the first )

    let remainder: String = String::from(line).drain(close + 1..).collect();

    // Now look for the Y ()  it's ok to not find a ( but having found it
    // It's an error not to find the ).  Note that we're a bit tolerant in that
    // if there are characters after the last list we don't care.  If there are
    // characters but no ( after the x list we don't care.
    //

    let open = remainder.find('(');
    if let Some(open) = open {
        if let Some(close) = remainder.find(')') {
            let y: Vec<&str> = remainder[open + 1..close].split(" ").collect();
            for yp in y {
                yparams.push(String::from(yp));
            }
        } else {
            return Err(String::from("Found a '('  for the y parameters but no ')'"));
        }
    }

    Ok((xparams, yparams))
}

// Reorganize the parameter lists according to spectrum type:
// At present, the only reorganization is that a "2" spectrum requires
// Xparams be the first element of the x params array and yaparms the second:
//
fn reorganize_params(
    xparams: Vec<String>,
    yparams: Vec<String>,
    sptype: &str,
) -> (Vec<String>, Vec<String>) {
    if sptype != "2" {
        (xparams, yparams)
    } else {
        (vec![xparams[0].clone()], vec![xparams[1].clone()])
    }
}
// Read a channel line:

fn read_channel<T: Read>(l: &mut Lines<BufReader<T>>) -> Option<SpectrumChannel> {
    let line = read_line(l);
    if let Err(_) = line {
        return None;
    }
    let mut line = line.unwrap();

    // The coordinates can be gotten from the parse_paren_list

    let bins = parse_paren_list(&line);
    if let Err(_) = bins {
        return None;
    }
    let bins = bins.unwrap();
    let bins = bins.0;

    // There must be at least 1 bin:

    if bins.len() == 0 {
        return None;
    }

    let xbin = bins[0].parse::<i32>();
    if let Err(_) = xbin {
        return None;
    }
    let xbin = xbin.unwrap();
    if xbin == -1 {
        return None; // end sentinel.
    }
    let mut ybin = 0; // a default value since - bins are not options.
    if bins.len() > 1 {
        let ybinstr = bins[1].parse::<i32>();
        if let Err(_) = ybinstr {
            return None;
        }
        ybin = ybinstr.unwrap();
    }

    // The string following the ) will be the value of the channel:
    // external forces will need to compute the x/y real coords.
    // We also know there is a close because otherwise parse_paren_list would fail.

    let close = line.find(')').unwrap();
    let remainder: String = line.drain(close + 1..).collect();
    let remainder = remainder.trim();
    if remainder.len() == 0 {
        return None;
    }
    let height = remainder.parse::<u64>();
    if let Err(_) = height {
        return None;
    }
    let height = height.unwrap();

    Some(SpectrumChannel {
        chan_type: ChannelType::Bin,
        x_coord: 0.0,
        y_coord: 0.0,
        x_bin: xbin as usize,
        y_bin: ybin as usize,
        value: height,
    })
}

// Remove quotes from vector of string the quotes are leading and trailing chars:
//

fn unquote(strings: &mut Vec<String>) -> Vec<String> {
    let mut result = vec![];
    for s in strings {
        let mut st: String = s.drain(1..).collect(); // chop off leading quote.
        st.truncate(st.len() - 1); // chop off trailing quote.
        result.push(st);
    }
    result
}

// Read the header from the data:

fn read_header<T: Read>(l: &mut Lines<BufReader<T>>) -> Result<SpectrumProperties, String> {
    let hdr1 = read_line(l);
    if let Err(s) = hdr1 {
        return Err(format!("Failed to read first header line: {}", s));
    }
    let hdr1 = hdr1.unwrap();

    // Try 2d first:

    let hdr1_result = scan_fmt!(&hdr1, "\"{}\" ({} {})", String, u32, u32);
    let xbins;
    let mut ybins = 0;
    let name;
    if hdr1_result.is_err() {
        // try as 1d:

        let result = scan_fmt!(&hdr1, "\"{}\" ({}", String, u32);
        if let Err(s) = result {
            return Err(format!("Unable to decode header1: {}", s));
        }
        (name, xbins) = result.unwrap();
    } else {
        (name, xbins, ybins) = hdr1_result.unwrap();
    }
    // Next is the date/time which we just skip:

    let date_time = read_line(l);
    if let Err(s) = date_time {
        return Err(format!(
            "Failed to read to read date/time header line: {}",
            s
        ));
    }
    // Next is the format version which we also skip:

    let _version_line = read_line(l);
    if let Err(s) = date_time {
        return Err(format!("Failed to read version header line: {}", s));
    }
    // Next there's the spectrum type and data type...

    let types = read_line(l);
    if let Err(s) = types {
        return Err(format!("Error reading the spectrum and data type: {}", s));
    }
    let types = types.unwrap();
    let types_result = scan_fmt!(&types, "{} {}", String, String);
    if types_result.is_err() {
        return Err(format!(
            "Unable to decode spectrum and channel type from '{}'",
            types
        ));
    }

    let (spectrum_type, _) = types_result.unwrap();
    // the spectrum type needs to be converted to our spectrum type:

    let native_type = spectrum::spectcl_sptype_to_rustogramer(&spectrum_type);
    if let Err(s) = native_type {
        return Err(format!(
            "Failed to convert spectrum type '{}' to native type: {}",
            spectrum_type, s
        ));
    }

    // Next is one or two lists of parameters.   This is tricky enough to unravel
    // it's worth a function all it's own:
    let param_line = read_line(l);

    if let Err(s) = param_line {
        return Err(format!("Error reading parameters line: {}", s));
    }
    let param_line = param_line.unwrap();
    let parameters = parse_paren_list(&param_line);
    if let Err(s) = parameters {
        return Err(format!(
            "Unable to parse parameters from '{}': {}",
            param_line, s
        ));
    }
    let (mut xparams, mut yparams) = parameters.unwrap();

    // Each parameter leads and ends with which must be stripped off

    let xparams = unquote(&mut xparams);
    let yparams = unquote(&mut yparams);

    // Have to process x/y parameters according to spectrum type:
    // Note that by now the spectrum type is supported.

    let (xparams, yparams) = reorganize_params(xparams, yparams, &spectrum_type);

    // Now axis definitions:

    let axis = read_line(l);
    if let Err(s) = axis {
        return Err(format!("Failed to read axis definition line: {}", s));
    }
    let axis = axis.unwrap();
    let axes = parse_paren_list(&axis);
    if let Err(s) = axes {
        return Err(format!(
            "Failed to parse axis definition line {} : {}",
            axis, s
        ));
    }
    let (xaxis_str, yaxis_str) = axes.unwrap();
    // Convert axis strings to low, high -- if possible:

    let xaxis = parse_axis(&xaxis_str);
    if let Err(s) = xaxis {
        return Err(format!("Failed to parse x axis: {}", s));
    }
    let xaxis = xaxis.unwrap();

    let mut yaxis: Result<(f64, f64), String> = Ok((0.0, 0.0));
    if yaxis_str.len() > 0 {
        yaxis = parse_axis(&yaxis_str);
    }
    if let Err(s) = yaxis {
        return Err(format!("Failed to parse y axis: {}", s));
    }
    let yaxis = yaxis.unwrap();

    // Skip the ---- line:

    let ignore = read_line(l);
    if let Err(s) = ignore {
        return Err(format!("Failed to read/skip marker line: {}", s));
    }
    // Marshall the stuff we got into a SpectrumProperties that can be
    // returned:

    let result = SpectrumProperties {
        name: name,
        type_string: spectrum_type,
        x_parameters: xparams.clone(),
        y_parameters: yparams.clone(),
        x_axis: Some((xaxis.0, xaxis.1, xbins)),
        y_axis: if ybins == 0 {
            None
        } else {
            Some((yaxis.0, yaxis.1, ybins))
        },
    };
    Ok(result)
}

fn transform(bins: u32, low: f64, high: f64, chan: usize) -> f64 {
    if bins == 0 {
        return 0.0;
    }
    (chan as f64) * (high - low) / (bins as f64)
}
// Compute the coordinates of a channel given its
// definition:

fn compute_coords(c: &mut SpectrumChannel, def: &SpectrumProperties) {
    let xaxis = def.x_axis.unwrap(); // there's always an x:
    let x = transform(xaxis.2, xaxis.0, xaxis.1, c.x_bin);

    let y = if let Some(yaxis) = def.y_axis {
        transform(yaxis.2, yaxis.0, yaxis.1, c.y_bin)
    } else {
        0.0
    };
    c.x_coord = x;
    c.y_coord = y;
}

// Read one spectrum from a bytes iterator:

fn read_spectrum<T: Read>(l: &mut Lines<BufReader<T>>) -> Result<SpectrumFileData, String> {
    let definition = read_header(l);
    if let Err(s) = definition {
        return Err(format!("Failed to read header: {}", s));
    }
    let definition = definition.unwrap();

    let mut contents = Vec::<SpectrumChannel>::new();
    loop {
        let channel = read_channel(l);
        if channel.is_none() {
            break;
        }
        let mut channel = channel.unwrap();
        compute_coords(&mut channel, &definition);

        contents.push(channel);
    }
    Ok(SpectrumFileData {
        definition: definition,
        channels: contents,
    })
}

///
/// Read a spectrum.
///
/// ### Parameters:
/// *   f - anything that implements the readable trait.
///
/// ### Returns:
/// * Vec<SpectrumFileData>  the spectra read from the file (could be more than
/// one).  
///
///  ### Note
///  An empty vector indicates an error of some sort that prevented even
///  one spectrum from being read. Errors of this sort include an empty file.
///  A non-empty vector may still indicate an error parsing the file where
///  the vector will contain as many spectra as were properly read.
///
pub fn read_spectra<T>(f: &mut T) -> Vec<SpectrumFileData>
where
    T: Read,
{
    let reader = BufReader::new(f);
    let mut lines = reader.lines(); // Iterates over lines.
    let mut result: Vec<SpectrumFileData> = vec![];
    loop {
        let try_spec = read_spectrum(&mut lines);
        if try_spec.is_err() {
            break;
        } else {
            result.push(try_spec.unwrap());
        }
    }

    result
}
