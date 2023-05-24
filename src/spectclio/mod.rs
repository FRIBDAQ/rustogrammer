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

use crate::rest::spectrum;
use crate::rest::spectrumio::{SpectrumChannel, SpectrumFileData, SpectrumProperties};
use chrono::prelude::*;
use std::io::{prelude::*, BufReader, Bytes, Lines, Read, Write};

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
        &format!("({} {}) {}\n", c.x_bin - 1, c.y_bin - 2, c.value),
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
//---------------------------------------------------------------------
// This section of code handles reading spectra from a Readable
// object

// Parse the parameter line of the header.  This is of the form:
//
// (xparam1 [xparam2 ...]) [(yparam1 [yparam2...])]
//
fn parse_parameters(line: &str) -> Result<(Vec<String>, Vec<String>), String> {
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

// Read one spectrum from a bytes iterator:

fn read_spectrum<T: Read>(l: &mut Lines<BufReader<T>>) -> Result<SpectrumFileData, String> {
    let hdr1 = l.next();
    if let None = hdr1 {
        return Err(String::from("end of file"));
    }
    let hdr1 = hdr1.unwrap();
    if let Err(s) = hdr1 {
        return Err(format!("I/O error of some sort: {}", s));
    }
    let hdr1 = hdr1.unwrap();

    // Try 2d first:

    let hdr1_result = scan_fmt!(&hdr1, "\"{}\" ({} {})", String, u32, u32);
    let mut xbins = 0;
    let mut ybins = 0;
    let mut name = String::new();
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

    let date_time = l.next();
    if let None = date_time {
        return Err(String::from("Premature end of file"));
    }
    let date_time = date_time.unwrap();
    if let Err(s) = date_time {
        return Err(format!("I/O error trying to read dat/time: {}", s));
    }
    // Next is the format version which we also skip:

    let version_line = l.next();
    if let None = version_line {
        return Err(String::from("Premature end of file"));
    }
    let version_line = version_line.unwrap();
    if let Err(s) = date_time {
        return Err(format!("I/O error trying to read dat/time: {}", s));
    }
    // Next there's the spectrum type and data type...

    let types = l.next();
    if let None = types {
        return Err(format!("Premature end file reading spectrum type/data type"));
    }
    let types = types.unwrap();
    if let Err(s) = types {
        return Err(format!("Error reading the spectrum and data type: {}", s));
    }
    let types = types.unwrap();
    let types_result = scan_fmt!(&types, "{} {}", String, String);
    if types_result.is_err() {
        println!(
            "Failed to decode types from {}: {}",
            types,
            types_result.unwrap_err()
        );
        return Err(format!(
            "Unable to decode spectrum and channel type from '{}'",
            types
        ));
    }
    let spectrum_type: String;
    let (spectrum_type, _) = types_result.unwrap();
    // the spectrum type needs to be converted to our spectrum type:

    let native_type = spectrum::spectcl_sptype_to_rustogramer(&spectrum_type);
    if let Err(s) = native_type {
        println!("Failed to convert {} to native type: {}", spectrum_type, s);
        return Err(format!(
            "Failed to convert spectrum type '{}' to native type: {}",
            spectrum_type, s
        ));
    }
    let native_type = native_type.unwrap();

    // Next is one or two lists of parameters.   This is tricky enough to unravel
    // it's worth a function all it's own:
    let param_line = l.next();
    if let None = param_line {
        return Err(String::from(
            "Premature end file while trying to read the parameter names from the header",
        ));
    }
    let param_line = param_line.unwrap();
    if let Err(s) = param_line {
        return Err(format!("Error reading parameters line: {}", s));
    }
    let param_line = param_line.unwrap();
    let parameters = parse_parameters(&param_line);
    if let Err(s) = parameters {
        println!("Unable to parse parameters from '{}': {}", param_line, s);
        return Err(format!(
            "Unable to parse parameters from '{}': {}",
            param_line, s
        ));
    }
    let (xparams, yparams) = parameters.unwrap();

    println!("Got spectrum type: {}", native_type);
    println!("Xparameters:");
    for p in xparams.iter() {
        println!("{}", *p);
    }
    println!("Yparameters:");
    for p in yparams.iter() {
        println!("{}", *p);
    }

    Err(String::from("Read spectrum unimplemented"))
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
