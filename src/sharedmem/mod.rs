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
#[derive(PartialEq, Copy, Clone)]
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

/// This struct manages storage in some external extent
/// with a certain fixed size.   It will be used to allocate
/// and free spectrum storage within the shared memory of
/// SpecTcl.
/// Each extent is a tuple of base offset and size.
///
type Extent = (usize, usize);
struct StorageAllocator {
    free_extents: Vec<Extent>,
    allocated_extents: Vec<Extent>,
}
impl StorageAllocator {
    // Defragment the free_extents array.
    // We assume that the number of extents is not that big so
    // in order to fully defragment, we
    // 1. sort the free extents by offsets.
    // 2. build a new free extents vector combining adjacent allocations.
    //
    fn defragment(&mut self) {
    
        // Sort the
        self.free_extents.sort_by_key(|e| e.0); // Sort by extent base:
        let mut result = vec![self.free_extents[0]];

        // This loop works correctly because we know that
        // free_extents is sorted by offset:

        for item in self.free_extents.iter().skip(1) {
            // item is contiguous with the last extent, just
            // modify the extent
            let index = result.len() - 1;
            if item.0 == (result[index].0 + result[index].1) {
                result[index].1 += item.1
            } else {
                // make a new extent not contiguous with the last one.

                result.push(*item);
            }
        }

        self.free_extents = result;
    }

    /// Create a  manager for an arena that's n units bit:
    ///
    fn new(n: usize) -> StorageAllocator {
        StorageAllocator {
            free_extents: vec![(0, n)],
            allocated_extents: vec![],
        }
    }
    /// Allocate - our allocator is stupid in that rather than looking
    /// for the best fit it just provides the first fit.
    ///
    fn allocate(&mut self, n: usize) -> Option<usize> {
        for (i, extent) in self.free_extents.iter().enumerate() {
            if extent.1 >= n {
                let result = extent.0;
                let remainder = extent.1 - n;

                // If the extent was fully used,
                // remove it else recompute the base/size in place:

                if remainder > 0 {
                    self.free_extents[i] = (self.free_extents[i].0 + n, self.free_extents[i].1 - n);
                } else {
                    self.free_extents.remove(i);
                }
                // Mark allocated.

                self.allocated_extents.push((result, n));
                return Some(result);
            }
        }
        None
    }
    /// Free storage.  Error if the freed storage is not in the
    /// allocated_extents list.
    /// We defragment on each free - which is a bit excessive but
    /// allocation/deallocation is believed to be realtively 'slow'.
    ///
    fn free(&mut self, offset: usize, size: usize) -> Result<(), String> {
        let allocation: Extent = (offset, size);
        for (i, extent) in self.allocated_extents.iter().enumerate() {
            if (extent.0 == allocation.0) && (extent.1 == allocation.1) {
                self.allocated_extents.remove(i);
                self.free_extents.push(allocation);
                self.defragment();
                return Ok(());
            }
        }
        Err(String::from("Attempted free of unallocated extent"))
    }
}

///  This struct, and its implementation, define an Xamine
/// compatible memory and map.
/// The implementation supports the operations we need on
/// the memory region.
pub struct SharedMemory {
    bindings: [String; XAMINE_MAXSPEC],
    backing_store: tempfile::NamedTempFile,
    map: memmap::MmapMut,
    spec_size: usize,
}

impl SharedMemory {
    /// Make an initialized bindings array that can be
    /// used to initialize the bindings member.
    fn init_bindings() -> [String; XAMINE_MAXSPEC] {
        // Maybe there's a better way to do this but I'm not sure
        // what it is:

        let mut b = Vec::<String>::new();
        for i in 0..XAMINE_MAXSPEC {
            b.push(String::new());
        }
        b.try_into().expect("Failed to unwrap the bindings vector")
    }
    /// Compute total memory size requirements.
    fn total_memory_size(spec_storage_size: usize) -> usize {
        spec_storage_size + mem::size_of::<XamineSharedMemory>()
    }
    /// Get base of spectrumstorage.
    fn spectrum_pointer(&mut self) -> *mut u8 {
        unsafe {
            self.map
                .as_mut_ptr()
                .offset(mem::size_of::<XamineSharedMemory>() as isize)
        }
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
    pub fn new(specsize: usize) -> Result<SharedMemory, String> {
        let total_size = Self::total_memory_size(specsize);
        let home_dir = if let Some(s) = dirs::home_dir() {
            s
        } else {
            return Err(String::from("Failed to get home directory"));
        };
        let file = match tempfile::NamedTempFile::new_in(home_dir) {
            Ok(f) => f,
            Err(e) => {
                return Err(format!("Failed to create temp file: {}", e.to_string()));
            }
        };
        // Now we need to set the file length:

        if let Err(e) = file.as_file().set_len(total_size as u64) {
            return Err(format!(
                "Failed to set the length of the backing store file: {}",
                e.to_string()
            ));
        }
        // Finally we can make the memory map:

        let mut map = match unsafe { memmap::MmapMut::map_mut(file.as_file()) } {
            Ok(m) => m,
            Err(e) => {
                return Err(format!("Failed to map file: {}", e.to_string()));
            }
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
            map: map,
            spec_size: specsize,
        })
    }
    /// Get a free slot number.

    pub fn get_free_slot(&self) -> Option<usize> {
        let header = self.map.as_ptr() as *mut XamineSharedMemory;
        for i in 0..XAMINE_MAXSPEC {
            if unsafe { (*header).dsp_types[i] } == SpectrumTypes::undefined {
                return Some(i);
            }
        }
        return None; // No free slots.
    }
}

//// Tests for the allocator:

#[cfg(test)]
mod allocator_tests {
    use super::*;
    #[test]
    fn alloc_1() {
        let mut arena = StorageAllocator::new(100);
        let result = arena.allocate(10).expect("Allocatio of 10 failed"); // should work.
        assert_eq!(0, result);
    }
    #[test]
    fn alloc_2() {
        let mut arena = StorageAllocator::new(100);
        let result1 = arena.allocate(50).expect("First alloc failed");
        let result2 = arena.allocate(50).expect("second alloc failed");
        let result3 = arena.allocate(1);
        assert!(result3.is_none());

        assert_eq!(0, result1);
        assert_eq!(50, result2);
    }
    #[test]
    fn alloc_3() {
        // Test that each allocation winds up in the free list:
        let mut arena = StorageAllocator::new(100);
        for i in 0..10 {
            let result = arena
                .allocate(2)
                .expect(&format!("Allocation {} failed", i));
        }
        assert_eq!(10, arena.allocated_extents.len());
    }
    #[test]
    fn free_1() {
        let mut arena = StorageAllocator::new(100);
        let extent = (arena.allocate(10).expect("failed allocation"), 10);

        //If I free this there should be:
        // 1. No allocated extents.
        // 2. One free exent after garbage collection:

        arena
            .free(extent.0, extent.1)
            .expect("Failed to free allocation");
        assert_eq!(0, arena.allocated_extents.len());
        assert_eq!(1, arena.free_extents.len());
        assert_eq!(100, arena.free_extents[0].1);
        assert_eq!(0, arena.free_extents[0].0);
    }
    #[test]
    fn free_2() {
        // Defragmentation not possible:

        let mut arena = StorageAllocator::new(100);
        let mut extents = vec![];
        for i in 0..10 {
            extents.push((
                arena
                    .allocate(2)
                    .expect(&format!("Allocation {} failed", i)),
                2,
            ));
        }
        // Free only every other one of these:

        for (i, even) in extents
            .iter()
            .enumerate()
            .filter(|(index, _)| index % 2 == 0)
        {
            arena
                .free(even.0, even.1)
                .expect(&format!("failed to delete allocation {}", i));
        }

        // Last one is 8 which is not contiguous with the remaining
        // free space so should be 5 allocations and 6 free areas (I think).

        assert_eq!(6, arena.free_extents.len());
        assert_eq!(5, arena.allocated_extents.len());
    }
    #[test]
    fn free_3() {
        let mut arena = StorageAllocator::new(100);
        let mut extents = vec![];
        for i in 0..10 {
            extents.push((
                arena
                    .allocate(2)
                    .expect(&format!("Allocation {} failed", i)),
                2,
            ));
        }
        // Free only every other one of these:

        for (i, even) in extents
            .iter()
            .enumerate()
            .filter(|(index, _)| index % 2 == 0)
        {
            arena
                .free(even.0, even.1)
                .expect(&format!("failed to delete allocation {}", i));
        }

        // Last one is 8 which is not contiguous with the remaining
        // free space so should be 5 allocations and 6 free areas (I think).
        // But, if we free #5, that's contiguous with both #4,and #6
        // so the allocated extents should number 5 after that.

        arena
            .free(extents[5].0, extents[5].1)
            .expect("Final free failed");
        assert_eq!(5, arena.free_extents.len());
    }
}
