use std::fs::File;
use std::io::prelude::*;
use std::mem;
use std::ops::Add;
use std::time;

pub mod abnormal_end;
pub mod analysis_ring_items;
pub mod event_item;
pub mod format_item;
pub mod glom_parameters;
pub mod scaler_item;
pub mod state_change;
pub mod text_item;
pub mod triggers_item;

/// This is an raw ring item.   Raw in the
/// sense that the payload is just a soup of bytes.
/// However it wil have methods that allow conversion of this item
/// to more structured ring items based on the 'type' field.
///
pub struct RingItem {
    size: u32,
    type_id: u32,
    body_header_size: u32,
    payload: Vec<u8>,
}
#[derive(Clone, Copy)]
pub struct BodyHeader {
    pub timestamp: u64,
    pub source_id: u32,
    pub barrier_type: u32,
}

pub enum RingItemError {
    HeaderReadFailed,
    InvalidHeader,
    FileTooSmall,
}
pub type RingItemResult = Result<RingItem, RingItemError>;

impl RingItem {
    // Private methods:

    // Read a u32:

    fn read_long(f: &mut File) -> Result<u32, u8> {
        let mut buf: [u8; 4] = [0; 4];

        if let Ok(_) = f.read_exact(&mut buf) {
            let long = u32::from_ne_bytes(buf);
            return Ok(long);
        }
        Err(0)
    }

    ///
    /// Create a new empty ring item of the given type.
    ///
    pub fn new(t: u32) -> RingItem {
        RingItem {
            size: 3 * mem::size_of::<u32>() as u32,
            type_id: t,
            body_header_size: mem::size_of::<u32>() as u32,
            payload: Vec::new(),
        }
    }
    /// create a new ring item with a 12.x body header.
    ///
    pub fn new_with_body_header(t: u32, stamp: u64, source: u32, barrier: u32) -> RingItem {
        let mut result = RingItem::new(t);
        result.body_header_size = (body_header_size() + mem::size_of::<u32>()) as u32;

        result.add(stamp);
        result.add(source);
        result.add(barrier);

        result
    }

    pub fn size(&self) -> u32 {
        self.size
    }
    pub fn type_id(&self) -> u32 {
        self.type_id
    }
    pub fn has_body_header(&self) -> bool {
        self.body_header_size > mem::size_of::<u32>() as u32
    }
    ///  Add an object of type T to the ring buffer.  Note
    /// That the raw bytes are added therefore the item must
    /// not contain e.g. pointers.
    ///
    pub fn add<T>(&mut self, item: T) -> &mut RingItem {
        let pt = &item as *const T;
        let mut p = pt.cast::<u8>();

        // Now I have a byte pointer I can push the bytes of data
        // into the vector payload:

        for _i in 0..mem::size_of::<T>() {
            unsafe {
                self.payload.push(*p);
                p = p.offset(1);
            }
        }
        self.size = self.size + mem::size_of::<T>() as u32;
        self
    }
    /// Read a ring item from file.

    pub fn read_item(file: &mut File) -> RingItemResult {
        // Create a new ring item - type is unimportant since
        // it'll get overwitten.

        let mut item = RingItem::new(0);

        // The header fields must be read individually b/c
        // rust could have rearranged them  read only reads
        // to u8 arrays so we need to read and then copy into
        // the fields:

        if let Ok(n) = RingItem::read_long(file) {
            item.size = n;
        } else {
            return Err(RingItemError::HeaderReadFailed);
        }
        if item.size < 3 * mem::size_of::<u32>() as u32 {
            return Err(RingItemError::InvalidHeader);
        }

        if let Ok(n) = RingItem::read_long(file) {
            item.type_id = n;
        } else {
            return Err(RingItemError::HeaderReadFailed);
        }

        if let Ok(n) = RingItem::read_long(file) {
            item.body_header_size = n;
        } else {
            return Err(RingItemError::HeaderReadFailed);
        }

        // Figure out how many bytes are in the body
        // and read those into the veftor:

        let body_size: usize = (item.size as usize) - 3 * mem::size_of::<u32>();
        if body_size > 0 {
            item.payload.resize(body_size, 0);
            if let Err(_) = file.read_exact(item.payload.as_mut_slice()) {
                return Err(RingItemError::FileTooSmall);
            }
        }

        Ok(item)
    }
    /// Fetch the body header from the payload... if there is one.
    ///
    pub fn get_bodyheader(&self) -> Option<BodyHeader> {
        if self.has_body_header() {
            return Some(BodyHeader {
                timestamp: u64::from_ne_bytes(self.payload.as_slice()[0..8].try_into().unwrap()),
                source_id: u32::from_ne_bytes(self.payload.as_slice()[8..12].try_into().unwrap()),
                barrier_type: u32::from_ne_bytes(
                    self.payload.as_slice()[12..16].try_into().unwrap(),
                ),
            });
        } else {
            return None;
        }
    }
    pub fn payload(&self) -> &Vec<u8> {
        &(self.payload)
    }
    pub fn payload_mut(&mut self) -> &mut Vec<u8> {
        &mut (self.payload)
    }
    pub fn add_byte_vec(&mut self, v: &Vec<u8>) {
        for b in v {
            self.add(*b);
        }
    }
}
/// convert a u32 into a SystemTime:

///
/// Some items have variant shapes depending on their version.
///
pub fn raw_to_systime(raw: u32) -> time::SystemTime {
    let stamp = time::Duration::from_secs(raw as u64);
    time::UNIX_EPOCH.add(stamp)
}
/// convert a SystemTime into  a u32 for inclusion in to a raw item:
///
pub fn systime_to_raw(stamp: time::SystemTime) -> u32 {
    let unix_stamp = stamp.duration_since(time::UNIX_EPOCH).unwrap();
    let secs = unix_stamp.as_secs();
    (secs & 0xffffffff) as u32
}

pub fn body_header_size() -> usize {
    mem::size_of::<u64>() + 2 * mem::size_of::<u32>()
}

pub fn string_len(d: &[u8]) -> usize {
    let mut result = 0;
    for c in d {
        if *c == (0 as u8) {
            break;
        } else {
            result = result + 1;
        }
    }

    return result;
}
pub fn get_c_string(offset: &mut usize, bytes: &[u8]) -> String {
    let o: usize = *offset;
    let slen = string_len(&bytes[o..]);
    *offset = o + slen + 1;
    String::from_utf8(bytes[o..o + slen].try_into().unwrap()).unwrap()
}

#[derive(PartialEq)]
pub enum RingVersion {
    V11,
    V12,
}

// Ring item types:

const BEGIN_RUN: u32 = 1;
const END_RUN: u32 = 2;
const PAUSE_RUN: u32 = 3;
const RESUME_RUN: u32 = 4;
const PACKET_TYPES: u32 = 10;
const MONITORED_VARIABLES: u32 = 11;
const FORMAT_ITEM: u32 = 12;
const PERIODIC_SCALERS: u32 = 20;
const PHYSICS_EVENT: u32 = 30;
const PHYSICS_EVENT_COUNT: u32 = 31;
const GLOM_INFO: u32 = 42;
const ABNORMAL_END: u32 = 5;

// These ring item types are products of the FRIB analysis pipeline:

/// Contains the correspondences between parameter names and ids.
const PARAMETER_DEFINITIONS: u32 = 32768;
/// Contains the values of steering variables
const VARIABLE_VALUES: u32 = 32769;
/// Contains the actual parameter_id:value pairs for an event.
const PARAMETER_DATA: u32 = 32770;

#[cfg(test)]
mod tests {
    use crate::ring_items::RingItem;
    use std::mem;
    #[test]
    fn new1() {
        let item = RingItem::new(1234);
        assert_eq!(1234, item.type_id);
        assert_eq!(mem::size_of::<u32>() as u32, item.body_header_size);
        assert_eq!(0, item.payload.len());
        assert_eq!(3 * mem::size_of::<u32>() as u32, item.size);
    }
}
