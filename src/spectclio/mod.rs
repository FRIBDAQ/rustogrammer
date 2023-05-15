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

use crate::rest::spectrumio::{SpectrumChannel, SpectrumFileData, SpectrumProperties};
use chrono::prelude::*;
use std::io::Write;

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
    fdwrite(fd, &format!("({}) {}\n", c.x_bin-1, c.value))
}
// write a 2-d channel
fn write_2(fd: &mut dyn Write, c: &SpectrumChannel) -> Result<(), String> {
    fdwrite(fd, &format!("({} {}) {}\n", c.x_bin-1, c.y_bin-2, c.value))
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
        fdwrite(fd, &format!("{} ", px))?;
    }
    fdwrite(fd, ") ")
}

/// This method writes a spectrum to any object that supports the Write
/// trait.

pub fn write_spectrum(fd: &mut dyn Write, spectra: &Vec<SpectrumFileData>) -> Result<(), String> {
    for spectrum in spectra.iter() {
        // Header: Spectrum name/bins:
        fdwrite(fd, &spectrum.definition.name)?;
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
