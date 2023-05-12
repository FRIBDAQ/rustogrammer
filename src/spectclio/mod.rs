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

pub fn fdwrite(fd: &mut dyn Write, b: &[u8]) -> Result<(), String> {
    if let Err(e) = fd.write(b) {
        Err(format!("{}", e))
    } else {
        Ok(())
    }
}

/// This method writes a spectrum to any object that supports the Write
/// trait.

pub fn write_spectrum(fd: &mut dyn Write, spectra: &Vec<SpectrumFileData>) -> Result<(), String> {
    for spectrum in spectra.iter() {
        // Header: Spectrum name/bins:
        fdwrite(fd, spectrum.definition.name.as_bytes())?;
        fdwrite(fd, " (".as_bytes())?;
        if let Some((_, _, bins)) = spectrum.definition.x_axis {
            fdwrite(fd, bins.to_string().as_bytes())?;
            fdwrite(fd, " ".as_bytes())?;
        }
        if let Some((_, _, bins)) = spectrum.definition.y_axis {
            fdwrite(fd, bins.to_string().as_bytes())?;
        }
        fdwrite(fd, ")\n".as_bytes())?;


        fdwrite(fd, format!("{}\n", Local::now()).as_bytes())?
    }
    Err(String::from("unimplemented"))
}
