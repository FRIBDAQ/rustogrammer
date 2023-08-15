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
//use std::fs::File;
use std::mem;
use tempfile;

use crate::messaging::spectrum_messages;
pub mod binder;
pub mod mirror;

// These constants are used to size the fixed sized arrays in the
// shared memory header:

/// Number of spectrum slots.
pub const XAMINE_MAXSPEC: usize = 10000;

/// Size of a spectrum title:

pub const TITLE_LENGTH: usize = 128;

/// Types of spectra:

#[allow(dead_code)]
#[repr(C)]
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum SpectrumTypes {
    Undefined = 0,
    TwodLong = 5,
    OnedLong = 4,
    TwodWord = 3,
    TwodByte = 1,
}

/// The dimension of a spectrum:

#[repr(C)]
pub struct SpectrumDimension {
    xchans: u32,
    ychans: u32,
}

/// A title or label:

type SpectrumTitle = [u8; TITLE_LENGTH];

/// Statistics (not used but still present):

#[repr(C)]
pub struct Statistics {
    overflows: [u32; 2],
    underflows: [u32; 2],
}

/// This struct is used to define
/// -  The mapping between axis coordinates and bins and
/// -  X and Y axis labels:

#[repr(C)]
pub struct SpectrumMap {
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
pub struct XamineSharedMemory {
    dsp_xy: [SpectrumDimension; XAMINE_MAXSPEC],
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
    /// Trusting free means that we trust the caller to understand
    /// the size of the extent to be freed.
    ///
    fn free_trusted(&mut self, offset: usize) -> Result<(), String> {
        for extent in self.allocated_extents.iter() {
            if extent.0 == offset {
                self.free(extent.0, extent.1)
                    .expect("Failed to free an extent");
                return Ok(());
            }
        }
        Err(format!(
            "Failed to find an allocation at offset: {}",
            offset
        ))
    }
    /// Return the usage statistics.  These are four usize numbers
    /// returned in a tuple in this order:
    ///
    /// *   Total free space.
    /// *   Size of largest free chunk.
    /// *   Total used space.
    /// *   Size of largest used chunk.
    ///
    pub fn statistics(&self) -> (usize, usize, usize, usize) {
        let mut total_free = 0;
        let mut biggest_free = 0;
        let mut total_alloc = 0;
        let mut biggest_alloc = 0;

        // get the free info:

        for extent in self.free_extents.iter() {
            let size = extent.1;
            total_free += size;
            if size > biggest_free {
                biggest_free = size;
            }
        }
        // Get used info:

        for extent in self.allocated_extents.iter() {
            let size = extent.1;
            total_alloc += size;
            if size > biggest_alloc {
                biggest_alloc = size;
            }
        }

        (total_free, biggest_free, total_alloc, biggest_alloc)
    }
}

///  This struct, and its implementation, define an Xamine
/// compatible memory and map.
/// The implementation supports the operations we need on
/// the memory region.

pub struct SharedMemory {
    bindings: Vec<String>,
    backing_store: tempfile::NamedTempFile,
    map: memmap::MmapMut,
    allocator: StorageAllocator,
    total_size: usize,
}

impl SharedMemory {
    /// Make an initialized bindings array that can be
    /// used to initialize the bindings member.
    fn init_bindings(shm: &mut SharedMemory) -> &Self {
        // Maybe there's a better way to do this but I'm not sure
        // what it is:

        for _ in 0..XAMINE_MAXSPEC {
            shm.bindings.push(String::new());
        }
        shm
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
    fn get_header(&mut self) -> &mut XamineSharedMemory {
        let header = self.map.as_mut_ptr() as *mut XamineSharedMemory;
        unsafe { header.as_mut().unwrap() }
    }
    fn slot_as_pointer(&mut self, slot: usize) -> *mut u32 {
        let header = self.get_header();
        // this is why for 1ds we initialized ychans to 1 not zero.

        let offset = header.dsp_offsets[slot];

        // Make a *mut u32 pointer to the spectrum data:

        unsafe { (self.spectrum_pointer() as *mut u32).offset(offset as isize) }
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
                (*header).dsp_types[i] = SpectrumTypes::Undefined;
            }
        }
        println!(
            "Created shared memory file: {} with size {}",
            file.path()
                .as_os_str()
                .to_str()
                .expect("Failed to get shared memory path"),
            total_size
        );
        //
        let mut result = SharedMemory {
            bindings: vec![],
            backing_store: file,
            map: map,
            allocator: StorageAllocator::new(specsize),
            total_size: total_size,
        };
        Self::init_bindings(&mut result);
        Ok(result)
    }
    /// Get a free slot number.

    pub fn get_free_slot(&self) -> Option<usize> {
        let header = self.map.as_ptr() as *mut XamineSharedMemory;
        for i in 0..XAMINE_MAXSPEC {
            if unsafe { (*header).dsp_types[i] } == SpectrumTypes::Undefined {
                return Some(i);
            }
        }
        return None; // No free slots.
    }
    /// Allocate a free spectrum pointer that
    /// points to sufficient storage for a spectrum _size_ bytes long.
    ///
    /// On success returns doublet containing the long offset in the
    /// spectrum storage area and the pointer to that stroage.
    pub fn get_free_spectrum_pointer(&mut self, size: usize) -> Option<(usize, *mut u8)> {
        // See if we have any that fit:
        if let Some(offset) = self.allocator.allocate(size) {
            Some((offset, unsafe {
                self.spectrum_pointer().offset(offset as isize)
            }))
        } else {
            None
        }
    }
    /// Make a binding for a specific named spectrum.
    ///
    /// * name - name of the spectrum.
    /// * xaxis - (low, high, bins)
    /// * yaxis - (low, high, bins).
    ///
    /// Notes:
    ///  1.   We only create onedlong and twodlong spectra and its the
    /// presence of both axes that determine that a spectrum is 2d.
    //// 2.   The axis specifications are used to fill in the mapping structs.
    ///  3.    In addition to allocating spectrum storage we
    /// add the spectrum to the bindings array.
    /// The slot is automatically offset by one
    ///
    /// On success, we return a double of the slot number and a pointer
    /// to where the spectrum data should be mirrored.
    /// It's up to the caller to arrange
    /// for the data to be transferred from the local histogram
    /// to the spectrum.  The caller should not assume the
    /// spectrum storage is initialized.

    pub fn bind_spectrum(
        &mut self,
        sname: &str,
        xaxis: (f64, f64, u32),
        yaxis: Option<(f64, f64, u32)>,
    ) -> Result<(usize, *mut u8), String> {
        // If the name is too long we need to truncate it to
        // TITLE_LENGTH -1 so there's a null termination

        let mut name = String::from(sname);
        name.truncate(TITLE_LENGTH - 1);
        name.push('\0'); // Ensure it's all null terminated.

        // Let's try to get a slot:

        let slot = self.get_free_slot();
        if slot.is_none() {
            return Err(String::from("All spectrum slots are in use"));
        }
        let slot = slot.unwrap();

        // See if we have sufficent spectrum storage:
        // We allow for the hidden under/overflow channels here too:

        let mut required = xaxis.2;
        let mut spectrum_type = SpectrumTypes::OnedLong;
        if let Some(y) = yaxis {
            required = required * (y.2);
            spectrum_type = SpectrumTypes::TwodLong;
        }
        let storage = self.get_free_spectrum_pointer((required as usize) * mem::size_of::<u32>());
        if storage.is_none() {
            return Err(format!(
                "Unable to allocate spectrum storage for {} bytes",
                required
            ));
        }
        let (offset, ptr) = storage.unwrap();

        //  Fill in the appropriate header slot.

        let header = self.get_header();
        header.dsp_xy[slot].xchans = xaxis.2;
        if let Some(y) = yaxis {
            header.dsp_xy[slot].ychans = y.2;
        } else {
            header.dsp_xy[slot].ychans = 1;
        }
        for (i, c) in name.chars().enumerate() {
            header.dsp_titles[slot][i] = c as u8;
            header.dsp_info[slot][i] = c as u8;
        }
        header.dsp_offsets[slot] = (offset / mem::size_of::<u32>()) as u32;
        header.dsp_map[slot].xmin = xaxis.0 as f32;
        header.dsp_map[slot].xmax = xaxis.1 as f32;
        if let Some(y) = yaxis {
            header.dsp_map[slot].ymin = y.0 as f32;
            header.dsp_map[slot].ymax = y.1 as f32;
        } else {
            header.dsp_map[slot].ymin = 0.0;
            header.dsp_map[slot].ymax = 0.0;
        }
        //Empty  axis titles:

        header.dsp_map[slot].xlabel[0] = 0;
        header.dsp_map[slot].ylabel[0] = 0;
        header.dsp_statistics[slot].overflows = [0, 0];
        header.dsp_statistics[slot].underflows = [0, 0];

        // Finally the type

        header.dsp_types[slot] = spectrum_type;

        // Make the binding

        self.bindings[slot] = String::from(sname); // Use origial name.

        Ok((slot, ptr))
    }
    /// unbind a spectrum from shared memory:
    /// Set the binding string empty.
    /// set the header spectrum type id to undefined.
    /// Release the storage from our allocator.
    ///
    pub fn unbind(&mut self, slot: usize) {
        self.bindings[slot] = String::new();
        let header = self.get_header();
        header.dsp_types[slot] = SpectrumTypes::Undefined;
        let offset = (header.dsp_offsets[slot] as usize) * mem::size_of::<u32>();
        self.allocator
            .free_trusted(offset)
            .expect("BUG: Failed to free spectrum storage");
    }
    /// Clear the contents of a spectrum.
    ///
    pub fn clear_contents(&mut self, slot: usize) {
        // figure out where and how much:

        let header = self.get_header();

        // this is why for 1ds we initialized ychans to 1 not zero.

        let size = header.dsp_xy[slot].xchans * header.dsp_xy[slot].ychans;

        // Make a *mut u32 pointer to the spectrum data:

        let mut pspectrum = self.slot_as_pointer(slot);
        for _ in 0..size {
            unsafe {
                *pspectrum = 0;
                pspectrum = pspectrum.offset(1);
            };
        }
    }
    /// Given a reference to SpectrumContents and a spectrum slot,
    /// Copies the channel values into the target spectrum.
    /// note that no clear is done prior to the copy.  
    /// That's something the caller needs to do if necessary.

    pub fn set_contents(&mut self, slot: usize, contents: &spectrum_messages::SpectrumContents) {
        let pspectrum = self.slot_as_pointer(slot);
        for c in contents.iter() {
            unsafe {
                let p = pspectrum.offset(c.bin as isize);
                *p = c.value as u32;
            }
        }
    }
    /// return the name of the shared memory segment.
    /// This will be "file:" + backing_store's filename.
    ///
    pub fn get_shm_name(&self) -> String {
        let mut result = String::from("file:");
        let filepath = self
            .backing_store
            .path()
            .as_os_str()
            .to_str()
            .expect("Failed to get shared memory path");
        result = result + filepath;
        result
    }
    /// Return the binding indices that are in use:
    pub fn bound_indices(&mut self) -> Vec<usize> {
        let mut indices = vec![];

        {
            let header = self.get_header();
            for i in 0..XAMINE_MAXSPEC {
                if header.dsp_types[i] != SpectrumTypes::Undefined {
                    indices.push(i);
                }
            }
        }
        indices
    }

    /// Get information about the spectra that have been bound
    /// to the shared memory region.
    /// This is returned as a vector of doublets containing
    /// slots of bound spectra and their names.
    /// The return value is a tuple of usizes containing:
    ///

    ///
    pub fn get_bindings(&mut self) -> Vec<(usize, String)> {
        let mut result = vec![];
        let indices = self.bound_indices();
        for index in indices {
            // self.immutable borrow.
            result.push((index, self.bindings[index].clone()));
        }
        result
    }
    /// Provide memory allocation statistics:
    /// *   Total free space.
    /// *   Size of largest free chunk.
    /// *   Total used space.
    /// *   Size of largest used chunk.
    /// *   total indices bound.
    /// *   total indices
    /// *   Total memory size.
    pub fn statistics(&mut self) -> (usize, usize, usize, usize, usize, usize, usize) {
        let memstats = self.allocator.statistics();
        let bindinginfo = self.bound_indices();

        (
            memstats.0,
            memstats.1,
            memstats.2,
            memstats.3,
            bindinginfo.len(),
            XAMINE_MAXSPEC,
            self.total_size,
        )
    }
    pub fn get_backing_store(&self) -> String {
        String::from(self.backing_store.path().to_string_lossy())
    }
}
// Tests for the allocator:

#[cfg(test)]
mod allocator_tests {
    use super::*;
    #[test]
    fn alloc_1() {
        let mut arena = StorageAllocator::new(100);
        let result = arena.allocate(10).expect("Allocation of 10 failed"); // should work.
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
            arena
                .allocate(2)
                .expect(&format!("Allocation {} failed", i));
        }
        assert_eq!(10, arena.allocated_extents.len());
    }
    #[test]
    fn alloc_4() {
        // initial over big allocation fails:
        // but exactly 100 works:
        let mut arena = StorageAllocator::new(100);
        let result1 = arena.allocate(101);
        assert!(result1.is_none());
        arena.allocate(100).expect("Exact size allocation failed");
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
    #[test]
    fn free_4() {
        // Is issue 69 due to first freeing allocations in order:

        let mut arena = StorageAllocator::new(1000);
        let extent1 = arena.allocate(100).expect("Allocation1 failed");
        let extent2 = arena.allocate(200).expect("allocation 2 failed");

        // Kill off extent 1:

        arena.free_trusted(extent1).expect("Failed to free extent1");
        arena.free_trusted(extent2).expect("Failed to free extent2");
    }
}
