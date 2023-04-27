//! This module provides support for binding spectra in to
//! Xamine like shared memory.  Included in this support are
//! -  Support for creating and manipulating an Xamine compatible
//! shared memory region.
//! -  Support for a database of bindings between named spectra and
//! slots in the Xamine shared memory region.
//! -  Support for copying spectrum contents into shared memory
//! -  Support for entering graphical objecs onto shared memory
//! spectra.
//!
use dirs;
use memmap;
use std::fs::File;
use std::process;
use tempfile;

// These constants are used to size the fixed sized arrays in the
// shared memory header:

/// Number of spectrum slots.
const XAMINE_MAXSPEC: usize = 10000;

/// Size of a spectrum title:

const TITLE_LENGTH: usize = 128;

/// Types of spectra:

#[repr(C)]
enum SpectrumTypes {
    undefined = 0,
    twodlong = 5,
    onedlong = 4,
    twodword = 3,
    twodbyte = 1,
}

/// The dimension of a spectrum:

#[repr(C)]
struct SpectrumDimension {
    xchans: u32,
    ychans: u32,
}

/// A title or label:

type SpectrumTitle = [char; TITLE_LENGTH];

/// Statistics (not used but still present):

#[repr(C)]
struct Statistics {
    overflows: [u32; 2],
    underflows: [u32; 2],
}

/// This struct is used to define
/// -  The mapping between axis coordinates and bins and
/// -  X and Y axis labels:

#[repr(C)]
struct SpectrumMap {
    xmin: f32,
    xmax: f32,
    ymin: f32,
    ymax: f32,
    xlabel: SpectrumTitle,
    ylabel: SpectrumTitle,
}

/// The Xamine Shared memory header.
/// Immediately following that is channel soup.
///

#[repr(C)]
struct XamineSharedMemory {
    dsp_xy: SpectrumDimension,
    dsp_titles: [SpectrumTitle; XAMINE_MAXSPEC],
    dsp_info: [SpectrumTitle; XAMINE_MAXSPEC],
    dsp_offsets: [u32; XAMINE_MAXSPEC],
    dsp_types: [SpectrumTypes; XAMINE_MAXSPEC],
    dsp_map: [SpectrumMap; XAMINE_MAXSPEC],
    dsp_statistics: [Statistics; XAMINE_MAXSPEC],
}

///  This struct, and its implementation, define an Xamine
/// compatible memory and map.
/// The implementation supports the operations we need on
/// the memory region.
struct SharedMemory {}
