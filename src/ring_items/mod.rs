//!  Ring items are the structures used to transmit data in NSCLDAQ
//!  A ring items is self describing in the sense that it has a header
//!  That has a type and size.
//!
//!   This module and submodules define both a raw ring item (RingItem)
//!   which is what we'll use to serialize and de-serialize data as well
//!   as specific ring  item types.  Conversion traits allow for conversion
//!   from specific ring item types to RingItem (ToRaw trait), and
//!   attempted conversions from
//!   RingItem to specific ring item types (FromRaw trait).
//!  
//!    In addition all ring item types are printable since they
//!    implement the Display trait.
#![allow(dead_code)]
use std::fmt;
use std::io::Read;
use std::io::Write;
use std::mem;
use std::ops::Add;
use std::str;
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
impl fmt::Display for BodyHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Body header:").unwrap();
        writeln!(f, "   timestamp: {:0>8x}", self.timestamp).unwrap();
        writeln!(f, "   sourceid:  {}", self.source_id).unwrap();
        writeln!(f, "   barrier:   {}", self.barrier_type)
    }
}
#[derive(Debug)]
pub enum RingItemError {
    HeaderReadFailed,
    InvalidHeader,
    FileTooSmall,
}
impl fmt::Display for RingItemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderReadFailed => write!(f, "Header read failed"),
            Self::InvalidHeader => write!(f, "Invalid header"),
            Self::FileTooSmall => write!(f, "File not large enough for ring item"),
        }
    }
}
pub type RingItemResult = Result<RingItem, RingItemError>;

impl RingItem {
    // Private methods:

    // Read a u32:

    fn read_long<T: Read>(f: &mut T) -> Result<u32, u8> {
        let mut buf: [u8; 4] = [0; 4];

        if f.read_exact(&mut buf).is_ok() {
            Ok(u32::from_ne_bytes(buf))
        } else {
            Err(0)
        }
    }
    /// Write a u32:

    fn write_long<T: Write>(f: &mut T, l: u32) -> std::io::Result<usize> {
        let buf = l.to_ne_bytes();
        f.write_all(&buf)?;
        Ok(mem::size_of::<u32>())
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
    /// Fetch the body header from the payload... if there is one.
    ///
    pub fn get_bodyheader(&self) -> Option<BodyHeader> {
        if self.has_body_header() {
            Some(BodyHeader {
                timestamp: u64::from_ne_bytes(self.payload.as_slice()[0..8].try_into().unwrap()),
                source_id: u32::from_ne_bytes(self.payload.as_slice()[8..12].try_into().unwrap()),
                barrier_type: u32::from_ne_bytes(
                    self.payload.as_slice()[12..16].try_into().unwrap(),
                ),
            })
        } else {
            None
        }
    }
    pub fn payload(&self) -> &Vec<u8> {
        &(self.payload)
    }
    pub fn payload_mut(&mut self) -> &mut Vec<u8> {
        &mut (self.payload)
    }

    ///  Add an object of type T to the ring buffer.  Note
    /// That the raw bytes are added therefore the item must
    /// not contain e.g. pointers.
    ///   This is best used to put primitive types into the
    ///   payload
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
        self.size += mem::size_of::<T>() as u32;
        self
    }
    pub fn add_byte_vec(&mut self, v: &Vec<u8>) {
        for b in v {
            self.add(*b);
        }
    }
    /// Read a ring item from file.

    pub fn read_item<T: Read>(file: &mut T) -> RingItemResult {
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
            if file.read_exact(item.payload.as_mut_slice()).is_err() {
                return Err(RingItemError::FileTooSmall);
            }
        }

        Ok(item)
    }

    /// write the current ring item to file:
    /// The return value on success is the total number of
    /// bytes written.

    pub fn write_item<T: Write>(&self, file: &mut T) -> std::io::Result<usize> {
        let mut bytes_written: usize = 0;

        file.write_all(&u32::to_ne_bytes(self.size))?;
        file.write_all(&u32::to_ne_bytes(self.type_id))?;
        file.write_all(&u32::to_ne_bytes(self.body_header_size))?;
        file.write_all(&self.payload)?;
        bytes_written += self.size as usize;
        Ok(bytes_written)
    }
}

/// provide for textual formatting of a raw ring item:

impl fmt::Display for RingItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Raw ring item").unwrap();
        writeln!(f, "Size: {}", self.size()).unwrap();
        writeln!(f, "type: {}", self.type_id()).unwrap();
        let mut offset = 0;
        if self.has_body_header() {
            let header = self.get_bodyheader().unwrap();
            writeln!(f, "{}\n", header).unwrap();
            offset = body_header_size();
        }

        let payload = self.payload().as_slice();
        if offset < payload.len() {
            for (i, p) in payload[offset..].iter().enumerate() {
                if i % 8 == 0 {
                    writeln!(f).unwrap();
                }
                write!(f, "{:0>2x} ", p).unwrap();
            }
        }
        writeln!(f)
    }
}
/// This trait defines conversion to raw.  I'd love to use From
/// however it's not true that to_raw is reflexive... it can only
/// convert to the ring item types for which its type_id is
/// valid. e.g. while ou can convert a StateChangeItem into a RingItem,
/// You can't always convert a RingItem into a StateChangeItem.
///
pub trait ToRaw {
    fn to_raw(&self) -> RingItem;
}
/// This can be implemented for each destination type
/// e.g. `ConvertRaw<StateChange>` for RingItem to convert a raw
/// item to a ring item if possible (correct type_id).
///
pub trait FromRaw<T> {
    fn to_specific(&self, vers: RingVersion) -> Option<T>;
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
/// Returns the number of bytes of body header size that are
/// required of the payload.  Note that the actual value of
/// the body header size will be one `mem::size_of::<u32>()`
/// larger to account for the size field itself which is held in the
/// formal fields of a ring_items::RingItem.
///
pub fn body_header_size() -> usize {
    mem::size_of::<u64>() + 2 * mem::size_of::<u32>()
}
///
/// Given a slice of bytes that is known to contain a c_string,
/// returnst the length of this string  This length does not
/// include the terminating null.
///
pub fn string_len(d: &[u8]) -> usize {
    let mut result = 0;
    for c in d {
        if *c == (0_u8) {
            break;
        } else {
            result += 1;
        }
    }

    result
}
pub fn get_c_string(offset: &mut usize, bytes: &[u8]) -> String {
    let o: usize = *offset;
    let slen = string_len(&bytes[o..]);
    *offset = o + slen + 1;
    String::from_utf8(bytes[o..o + slen].try_into().unwrap()).unwrap()
}

#[derive(PartialEq, Copy, Clone)]
pub enum RingVersion {
    V11,
    V12,
}
impl fmt::Display for RingVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RingVersion::V11 => write!(f, "V11"),
            RingVersion::V12 => write!(f, "V12"),
        }
    }
}

impl str::FromStr for RingVersion {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "11" | "V11" => Ok(RingVersion::V11),
            "12" | "V12" => Ok(RingVersion::V12),
            _ => Err(format!("{} is not a legal Ring format version", s)),
        }
    }
}

// Ring item types:

pub const BEGIN_RUN: u32 = 1;
pub const END_RUN: u32 = 2;
pub const PAUSE_RUN: u32 = 3;
pub const RESUME_RUN: u32 = 4;
pub const PACKET_TYPES: u32 = 10;
pub const MONITORED_VARIABLES: u32 = 11;
pub const FORMAT_ITEM: u32 = 12;
pub const PERIODIC_SCALERS: u32 = 20;
pub const PHYSICS_EVENT: u32 = 30;
pub const PHYSICS_EVENT_COUNT: u32 = 31;
pub const GLOM_INFO: u32 = 42;
pub const ABNORMAL_END: u32 = 5;

// These ring item types are products of the FRIB analysis pipeline:

/// Contains the correspondences between parameter names and ids.
pub const PARAMETER_DEFINITIONS: u32 = 32768;
/// Contains the values of steering variables
pub const VARIABLE_VALUES: u32 = 32769;
/// Contains the actual parameter_id:value pairs for an event.
pub const PARAMETER_DATA: u32 = 32770;

//---------------------------------------------------------------
// unit tests
//
#[cfg(test)]
mod tests {
    use crate::ring_items::RingItem;
    use std::io::{Seek, Write};
    use std::mem;
    use std::ptr;
    use std::time;
    use tempfile::tempfile;
    #[test]
    fn new_1() {
        let item = RingItem::new(1234);
        assert_eq!(1234, item.type_id);
        assert_eq!(mem::size_of::<u32>() as u32, item.body_header_size);
        assert_eq!(0, item.payload.len());
        assert_eq!(3 * mem::size_of::<u32>() as u32, item.size);
    }
    #[test]
    fn new_2() {
        let item = RingItem::new_with_body_header(1234, 0xffffffffffffffff, 2, 0);
        assert_eq!(1234, item.type_id);
        assert_eq!(
            (3 * mem::size_of::<u32>() + mem::size_of::<u64>()) as u32,
            item.body_header_size
        );
        assert_eq!(
            (5 * mem::size_of::<u32>() + mem::size_of::<u64>()) as u32,
            item.size
        );
        assert_eq!(
            2 * mem::size_of::<u32>() + mem::size_of::<u64>(),
            item.payload.len()
        );
    }
    #[test]
    fn new_3() {
        let item = RingItem::new_with_body_header(1234, 0xffffffffffffffff, 2, 0);
        let p = item.payload().as_slice();
        assert_eq!(
            0xffffffffffffffff,
            u64::from_ne_bytes(p[0..8].try_into().unwrap())
        );
        assert_eq!(2, u32::from_ne_bytes(p[8..12].try_into().unwrap()));
        assert_eq!(0, u32::from_ne_bytes(p[12..16].try_into().unwrap()));
    }
    #[test]
    fn getters_1() {
        let item = RingItem::new(1234);
        assert_eq!(item.size, item.size());
        assert_eq!(item.type_id, item.type_id());
        assert!(!item.has_body_header());
        assert!(!matches!(item.get_bodyheader(), Some(_)));
    }
    #[test]
    fn getters_2() {
        let mut item = RingItem::new(1234);
        assert_eq!(item.payload.len(), item.payload().len());
        assert_eq!(item.payload.len(), item.payload_mut().len());
    }
    #[test]
    fn payload_1() {
        let item = RingItem::new(1234);
        assert!(ptr::eq(&item.payload, item.payload()));
    }
    #[test]
    fn payload_2() {
        let mut item = RingItem::new(2134);
        assert!(ptr::eq(&item.payload, item.payload_mut()));
    }
    #[test]
    fn add_1() {
        let mut item = RingItem::new(1234);
        item.add(0xa5_u8);
        let s = mem::size_of::<u8>();
        assert_eq!(s, item.payload.len());
        assert_eq!(0xa5_u8, item.payload[0]);
    }
    #[test]
    fn add_2() {
        let mut item = RingItem::new(1234);
        item.add(0xa55a_u16);
        let s = mem::size_of::<u16>();
        assert_eq!(s, item.payload.len());
        assert_eq!(
            0xa55a_u16,
            u16::from_ne_bytes(item.payload.as_slice()[0..s].try_into().unwrap())
        );
    }
    #[test]
    fn add_3() {
        let mut item = RingItem::new(1234);
        item.add(0x12345678_u32);
        let s = mem::size_of::<u32>();
        assert_eq!(s, item.payload.len());
        assert_eq!(
            0x12345678_u32,
            u32::from_ne_bytes(item.payload.as_slice()[0..s].try_into().unwrap())
        );
    }
    #[test]
    fn add_4() {
        let mut item = RingItem::new(1234);
        item.add(0x1234567876543210_u64);
        let s = mem::size_of::<u64>();
        assert_eq!(s, item.payload.len());
        assert_eq!(
            0x1234567876543210_u64,
            u64::from_ne_bytes(item.payload.as_slice()[0..s].try_into().unwrap())
        );
    }
    #[test]
    fn add_5() {
        let mut item = RingItem::new(1234);
        item.add(3.14159_f32);
        let s = mem::size_of::<f32>();
        assert_eq!(s, item.payload.len());
        assert_eq!(
            3.14159_f32,
            f32::from_ne_bytes(item.payload.as_slice()[0..s].try_into().unwrap())
        );
    }
    #[test]
    fn add_6() {
        let mut item = RingItem::new(1234);
        item.add(2.71828182_f64);
        let s = mem::size_of::<f64>();
        assert_eq!(s, item.payload.len());
        assert_eq!(
            2.71828182_f64,
            f64::from_ne_bytes(item.payload.as_slice()[0..s].try_into().unwrap())
        );
    }
    #[test]
    fn add_7() {
        // test add chaining:
        let data: Vec<u8> = vec![1, 2, 3, 4]; // So simple test:
        let mut item = RingItem::new(1234);
        item.add(data[0]).add(data[1]).add(data[2]).add(data[3]);
        assert_eq!(data, item.payload);
    }
    #[test]
    fn addbvec_1() {
        let data: Vec<u8> = vec![1, 2, 3, 4]; // So simple test:
        let mut item = RingItem::new(1234);
        item.add_byte_vec(&data);
        assert_eq!(data, item.payload);
    }
    #[test]
    fn read_1() {
        // Minimal ring item:

        let mut file = tempfile().unwrap();
        let size = u32::to_ne_bytes(3 * mem::size_of::<u32>() as u32);
        let item_type = u32::to_ne_bytes(1);
        let bh = u32::to_ne_bytes(mem::size_of::<u32>() as u32);
        file.write_all(&size).unwrap();
        file.write_all(&item_type).unwrap();
        file.write_all(&bh).unwrap();
        file.rewind().unwrap();

        let res = RingItem::read_item(&mut file);
        assert!(res.is_ok());
        let item = res.ok().unwrap();
        assert_eq!(3 * mem::size_of::<u32>() as u32, item.size);
        assert_eq!(1_u32, item.type_id);
        assert_eq!(mem::size_of::<u32>() as u32, item.body_header_size);
    }
    #[test]
    fn read_2() {
        // Minimal but with body header:

        let mut file = tempfile().unwrap();
        let size = u32::to_ne_bytes((5 * mem::size_of::<u32>() + mem::size_of::<u64>()) as u32);
        let item_type = u32::to_ne_bytes(1);
        let bhsize = u32::to_ne_bytes((3 * mem::size_of::<u32>() + mem::size_of::<u64>()) as u32);
        let tstamp = u64::to_ne_bytes(0x1234567812345678);
        let sid = u32::to_ne_bytes(5);
        let btype = u32::to_ne_bytes(0);

        file.write_all(&size).unwrap();
        file.write_all(&item_type).unwrap();
        file.write_all(&bhsize).unwrap();
        file.write_all(&tstamp).unwrap();
        file.write_all(&sid).unwrap();
        file.write_all(&btype).unwrap();
        file.rewind().unwrap();

        let read_status = RingItem::read_item(&mut file);
        assert!(read_status.is_ok());
        let item = read_status.ok().unwrap();

        assert_eq!(
            (5 * mem::size_of::<u32>() + mem::size_of::<u64>()) as u32,
            item.size
        );
        assert_eq!(1, item.type_id);
        assert_eq!(
            (3 * mem::size_of::<u32>() + mem::size_of::<u64>()) as u32,
            item.body_header_size
        );
        let s1 = mem::size_of::<u64>();
        assert_eq!(
            0x1234567812345678_u64,
            u64::from_ne_bytes(item.payload.as_slice()[0..s1].try_into().unwrap())
        );
        let s2 = s1 + mem::size_of::<u32>();
        assert_eq!(
            5_u32,
            u32::from_ne_bytes(item.payload.as_slice()[s1..s2].try_into().unwrap())
        );
        let s3 = s2 + mem::size_of::<u32>();
        assert_eq!(
            0_u32,
            u32::from_ne_bytes(item.payload.as_slice()[s2..s3].try_into().unwrap())
        );
    }
    #[test]
    fn read_3() {
        // no body header but payload -- let ring item compute size etc:

        let mut out_item = RingItem::new(12);
        let payload: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8];
        out_item.add_byte_vec(&payload);

        let mut file = tempfile().unwrap();
        file.write_all(&u32::to_ne_bytes(out_item.size)).unwrap();
        file.write_all(&u32::to_ne_bytes(out_item.type_id)).unwrap();
        file.write_all(&u32::to_ne_bytes(out_item.body_header_size))
            .unwrap();
        file.write_all(&out_item.payload).unwrap();
        file.rewind().unwrap();

        let item = RingItem::read_item(&mut file).unwrap();
        assert_eq!(out_item.size, item.size);
        assert_eq!(out_item.type_id, item.type_id);
        assert_eq!(out_item.body_header_size, item.body_header_size);
        assert_eq!(out_item.payload, item.payload);
    }
    #[test]
    fn read_4() {
        // with body header and payload.

        let mut out_item = RingItem::new_with_body_header(1, 0x5555555555, 2, 0);
        let payload: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8];
        out_item.add_byte_vec(&payload);
        let mut file = tempfile().unwrap();
        file.write_all(&u32::to_ne_bytes(out_item.size)).unwrap();
        file.write_all(&u32::to_ne_bytes(out_item.type_id)).unwrap();
        file.write_all(&u32::to_ne_bytes(out_item.body_header_size))
            .unwrap();
        file.write_all(&out_item.payload).unwrap();
        file.rewind().unwrap();

        let item = RingItem::read_item(&mut file).unwrap();
        assert_eq!(out_item.size, item.size);
        assert_eq!(out_item.type_id, item.type_id);
        assert_eq!(out_item.body_header_size, item.body_header_size);
        assert_eq!(out_item.payload, item.payload);
    }
    #[test]
    fn write_1() {
        // Write minimal item should read bnack the same.

        let out_item = RingItem::new(1);
        let mut file = tempfile().unwrap();
        let s = out_item.write_item(&mut file).unwrap();
        assert_eq!(s as u32, out_item.size);

        file.rewind().unwrap();
        let in_item = RingItem::read_item(&mut file).unwrap();

        assert_eq!(out_item.size, in_item.size);
        assert_eq!(out_item.type_id, in_item.type_id);
        assert_eq!(out_item.body_header_size, in_item.body_header_size);
        assert_eq!(out_item.payload, in_item.payload);
    }
    #[test]
    fn write_2() {
        // write minimal item with body header.

        let out_item = RingItem::new_with_body_header(1, 0x8877665544332211, 2, 0);
        let mut file = tempfile().unwrap();
        let s = out_item.write_item(&mut file).unwrap();
        assert_eq!(s as u32, out_item.size);

        file.rewind().unwrap();
        let in_item = RingItem::read_item(&mut file).unwrap();

        assert_eq!(out_item.size, in_item.size);
        assert_eq!(out_item.type_id, in_item.type_id);
        assert_eq!(out_item.body_header_size, in_item.body_header_size);
        assert_eq!(out_item.payload, in_item.payload);
    }
    #[test]
    fn write_3() {
        // Write ring item with payload:

        let mut out_item = RingItem::new(1);
        let payload: Vec<u8> = vec![5, 4, 3, 2, 1, 0];
        out_item.add_byte_vec(&payload);

        let mut file = tempfile().unwrap();
        let s = out_item.write_item(&mut file).unwrap();
        assert_eq!(s as u32, out_item.size);

        file.rewind().unwrap();
        let in_item = RingItem::read_item(&mut file).unwrap();

        assert_eq!(out_item.size, in_item.size);
        assert_eq!(out_item.type_id, in_item.type_id);
        assert_eq!(out_item.body_header_size, in_item.body_header_size);
        assert_eq!(out_item.payload, in_item.payload);
    }
    #[test]
    fn write_4() {
        // Write ring item with body header and payload.

        let mut out_item = RingItem::new_with_body_header(1, 0x1245123412, 2, 0);
        let payload: Vec<u8> = vec![5, 4, 3, 2, 1, 0];
        out_item.add_byte_vec(&payload);

        let mut file = tempfile().unwrap();
        let s = out_item.write_item(&mut file).unwrap();
        assert_eq!(s as u32, out_item.size);

        file.rewind().unwrap();
        let in_item = RingItem::read_item(&mut file).unwrap();

        assert_eq!(out_item.size, in_item.size);
        assert_eq!(out_item.type_id, in_item.type_id);
        assert_eq!(out_item.body_header_size, in_item.body_header_size);
        assert_eq!(out_item.payload, in_item.payload);
    }
    // Unbound functions:
    // Round time time conversion:

    #[test]
    fn time_1() {
        let now = time::SystemTime::now();
        let stamp = crate::ring_items::systime_to_raw(now);
        let now2 = crate::ring_items::raw_to_systime(stamp);

        assert_eq!(
            format!("{}", humantime::format_rfc3339_seconds(now)),
            format!("{}", humantime::format_rfc3339(now2))
        );
    }
    #[test]
    fn bhdrsize_1() {
        let item = RingItem::new_with_body_header(1, 0, 0, 0);
        assert_eq!(crate::ring_items::body_header_size(), item.payload.len());
    }
    #[test]
    fn strlen_1() {
        let test_string = String::from("Hello world");
        let mut bvec = test_string.as_bytes().to_vec();
        bvec.push(0);
        assert_eq!(test_string.len(), crate::ring_items::string_len(&bvec));
    }
    #[test]
    fn getcstr_1() {
        let test_string = String::from("Hello world");
        let mut bvec = test_string.as_bytes().to_vec();
        bvec.push(0);
        let mut offset = 0;
        assert_eq!(
            test_string,
            crate::ring_items::get_c_string(&mut offset, &bvec)
        );
        assert_eq!(test_string.len() + 1, offset);
    }
}
