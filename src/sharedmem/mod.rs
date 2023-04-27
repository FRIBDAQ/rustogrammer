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
extern crate dirs;
use memmap;
use std::fs::File;
use std::mem;
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
pub struct SharedMemory {
    bindings : [String; XAMINE_MAXSPEC],
    backing_store : tempfile::NamedTempFile,
    map : memmap::MmapMut,
    spec_size: usize
}

impl SharedMemory {
    fn init_bindings() -> [String; XAMINE_MAXSPEC] {
        // Maybe there's a better way to do this but I'm not sure
        // what it is:

        let mut b = Vec::<String>::new();
        for i in 0..XAMINE_MAXSPEC {
            b.push(String::new());
        }
        b.try_into().expect("Failed to unwrap the bindings vector")
    }
    fn total_memory_size(spec_storage_size : usize) -> usize{
        spec_storage_size + mem::size_of::<XamineSharedMemory>()
    }
    /// Create a new Xamine shared memory region and initialize
    /// it so that there are no spectra in it.
    /// We use tmpfile::NamedTempFile so we can get a name that
    /// can be passed to other processes that want to view our
    /// spectra.
    ///
    ///  *   specsize - Number of bytes of spectrum storage.
    ///
    /// If there is an error in creating the memory map
    /// (one reason we know of in Linux is a size limit on 
    /// shared memory segments), a textual description of the error
    /// is returned as Err, otherwise Ok returns the struct.
    ///
    pub fn new(specsize : usize) -> Result<SharedMemory, String> {
        let total_size = Self::total_memory_size(specsize);
        let home_dir = if let Some(s) = dirs::home_dir() {
            s
        } else {
            return Err(String::from("Failed to get home directory"));
        };
        let file = match tempfile::NamedTempFile::new_in(home_dir) {
            Ok(f) => f,
            Err(e) => {return Err(format!("Failed to create temp file: {}", e.to_string()));}
        };
        // Now we need to set the file length:

        if let Err(e) = file.as_file().set_len(total_size as u64) {
            return Err(format!("Failed to set the length of the backing store file: {}", e.to_string()));
        }
        // Finally we can make the memory map:

        let mut map =  match unsafe {memmap::MmapMut::map_mut(file.as_file())} {
            Ok(m) => m,
            Err(e) => {return Err(format!("Failed to map file: {}", e.to_string()));}
        };
        // Initialize the map by setting it to zeros.

        let header = map.as_mut_ptr() as *mut XamineSharedMemory;
        unsafe {
            for i in 0..XAMINE_MAXSPEC {
                (*header).dsp_types[i] = SpectrumTypes::undefined;
            }
        }

        // All is good.

        Ok(SharedMemory {
            bindings: Self::init_bindings(),
            backing_store: file,
            map : map,
            spec_size : specsize
        })
    }
}
