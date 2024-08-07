use crate::ring_items;
use std::fmt;
use std::mem;
use std::ptr;

///
/// This module contains code to handle physics event items.
/// What we're going to do is treat an event item body as a vector
/// of u8 but supply a cursor and methods to use that cursor to
/// fetch generically from the soup of bytes with cursor movement.
///  We'll also provide for insertion as the raw item can do.

#[derive(Clone)]
pub struct PhysicsEvent {
    body_header: Option<ring_items::BodyHeader>,
    get_cursor: usize,
    event_data: Vec<u8>,
}

impl PhysicsEvent {
    /// Create a new Physics event from nothing.
    /// As the event is acquired it can be filled in with the add generics.
    ///
    pub fn new(bh: Option<ring_items::BodyHeader>) -> PhysicsEvent {
        PhysicsEvent {
            body_header: bh,
            get_cursor: 0,
            event_data: Vec::<u8>::new(),
        }
    }

    // Add data to the payload:

    pub fn add<T>(&mut self, item: T) -> &mut PhysicsEvent {
        let pt = &item as *const T;
        let mut p = pt.cast::<u8>(); // u8 pointer to the item.

        // Sort of what we did in RingItem:

        for _i in 0..mem::size_of::<T>() {
            unsafe {
                self.event_data.push(*p);
                p = p.offset(1);
            }
        }

        self // So we can chain.
    }
    // Get an item of a type from the event_data incrementing the
    // cursor.  T must implement a copy trait.

    pub fn get<T: Copy>(&mut self) -> Option<T> {
        // Make sure there;s enough stuff in the event for item T.

        if self.get_cursor < (self.event_data.len() + 1 - mem::size_of::<T>()) {
            let mut p = self.event_data.as_ptr();
            unsafe {
                p = p.add(self.get_cursor);
            }
            // Need to cast this to a pointer of type T:

            let pt = p.cast::<T>();
            let result = unsafe { ptr::read_unaligned(pt) };
            self.get_cursor += mem::size_of::<T>();
            Some(result)
        } else {
            None // Out of range.
        }
    }
    ///
    /// Reset the get -cursor to  the beginning.
    ///
    pub fn rewind(&mut self) {
        self.get_cursor = 0;
    }
    ///
    /// Return the body header:
    ///
    pub fn get_bodyheader(&self) -> Option<ring_items::BodyHeader> {
        self.body_header
    }
    ///  Size of event body:
    ///
    pub fn body_size(&self) -> usize {
        self.event_data.len()
    }
}

impl Iterator for PhysicsEvent {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(result) = self.get() {
            Some(result)
        } else {
            self.rewind();
            None
        }
    }
}

impl fmt::Display for PhysicsEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Physics Event:").unwrap();
        if let Some(bh) = self.get_bodyheader() {
            writeln!(f, "Body Header:\n {}", bh).unwrap();
        }

        // We're a bit hampered by the fact that the signature
        // requires immutability so:

        let mut offset = 0;
        let u32s = mem::size_of::<u16>();

        let mut in_line = 0;
        loop {
            if offset >= self.event_data.len() {
                break;
            } else {
                let mut p = self.event_data.as_ptr();
                unsafe {
                    p = p.add(offset);
                }
                let pt = p.cast::<u16>();
                let word = { unsafe { *pt } };
                offset += u32s;

                write!(f, "{:0>4x} ", word).unwrap();
                in_line += 1;
                if in_line == 8 {
                    writeln!(f).unwrap();
                    in_line = 0;
                }
            }
        }
        if in_line != 0 {
            writeln!(f).unwrap();
        }
        write!(f, "")
    }
}
/// ToRaw is a trait that allows conversion to raw ring items from
/// self:

impl ring_items::ToRaw for PhysicsEvent {
    fn to_raw(&self) -> ring_items::RingItem {
        let mut result = if let Some(bh) = self.body_header {
            ring_items::RingItem::new_with_body_header(
                ring_items::PHYSICS_EVENT,
                bh.timestamp,
                bh.source_id,
                bh.barrier_type,
            )
        } else {
            ring_items::RingItem::new(ring_items::PHYSICS_EVENT)
        };
        // Now just Append our data to the payload:

        result.add_byte_vec(&self.event_data);
        result
    }
}

/// From raw, a generic trait for RingItem allows an attempt to
/// convert a ring item to a specific type:

impl ring_items::FromRaw<PhysicsEvent> for ring_items::RingItem {
    fn to_specific(&self, _v: ring_items::RingVersion) -> Option<PhysicsEvent> {
        if self.type_id() == ring_items::PHYSICS_EVENT {
            let mut payload_offset = 0;
            let mut result = PhysicsEvent::new(self.get_bodyheader());
            if self.has_body_header() {
                payload_offset = ring_items::body_header_size();
            }
            result
                .event_data
                .extend_from_slice(&self.payload().as_slice()[payload_offset..]);
            Some(result)
        } else {
            None
        }
    }
}
#[cfg(test)]
mod test_event {
    use super::*;
    use crate::ring_items::*;
    use std::mem::size_of;
    #[test]
    fn new_1() {
        let item = PhysicsEvent::new(None);
        assert!(item.body_header.is_none());
        assert_eq!(0, item.get_cursor);
        assert_eq!(0, item.event_data.len());
    }
    #[test]
    fn new_2() {
        let item = PhysicsEvent::new(Some(BodyHeader {
            timestamp: 0x1234567890,
            source_id: 2,
            barrier_type: 0,
        }));
        assert!(item.body_header.is_some());
        let bh = item.body_header.unwrap();
        assert_eq!(0x1234567890, bh.timestamp);
        assert_eq!(2, bh.source_id);
        assert_eq!(0, bh.barrier_type);
    }
    // Adding data to the event:
    #[test]
    fn add_1() {
        let mut item = PhysicsEvent::new(None);

        item.add(0xa5_u8);

        assert_eq!(1, item.event_data.len());
        assert_eq!(0xa5, item.event_data[0]);
    }
    #[test]
    fn add_2() {
        let mut item = PhysicsEvent::new(None);

        item.add(0xa5a5_u16);
        assert_eq!(size_of::<u16>(), item.event_data.len());
        let s = item.event_data.as_slice();
        assert_eq!(
            0xa5a5_u16,
            u16::from_ne_bytes(s[0..size_of::<u16>()].try_into().unwrap())
        );
    }
    #[test]
    fn add_3() {
        let mut item = PhysicsEvent::new(None);

        item.add(0xa5a5a5a5_u32);
        assert_eq!(size_of::<u32>(), item.event_data.len());
        let s = item.event_data.as_slice();
        assert_eq!(
            0xa5a5a5a5_u32,
            u32::from_ne_bytes(s[0..size_of::<u32>()].try_into().unwrap())
        );
    }
    #[test]
    fn add_4() {
        //chaining:

        let mut item = PhysicsEvent::new(None);

        item.add(0xa5_u8).add(0xa5a5_u16).add(0xa5a5a5a5_u32);
        assert_eq!(
            size_of::<u8>() + size_of::<u16>() + size_of::<u32>(),
            item.event_data.len()
        );

        let mut offset = 0;
        assert_eq!(0xa5_u8, item.event_data[offset]);
        offset += size_of::<u8>();
        let s = item.event_data.as_slice();

        assert_eq!(
            0xa5a5_u16,
            u16::from_ne_bytes(s[offset..offset + size_of::<u16>()].try_into().unwrap())
        );
        offset += size_of::<u16>();
        assert_eq!(
            0xa5a5a5a5_u32,
            u32::from_ne_bytes(s[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
    }
    // getting data from the event:

    #[test]
    fn get_1() {
        let mut item = PhysicsEvent::new(None);
        assert!(item.get::<u8>().is_none());
    }
    #[test]
    fn get_2() {
        let mut item = PhysicsEvent::new(None);
        item.add(0xa5_u8);
        let gotten = item.get::<u8>();
        assert!(gotten.is_some());
        let gotten = gotten.unwrap();
        assert_eq!(0xa5_u8, gotten);
        assert!(item.get::<u8>().is_none()); // Nothing more to get.
        assert_eq!(item.event_data.len(), item.get_cursor);
    }
    #[test]
    fn get_3() {
        let mut item = PhysicsEvent::new(None);
        item.add(0xa5a5_u16);
        let gotten = item.get::<u16>();
        assert!(gotten.is_some());
        let gotten = gotten.unwrap();
        assert_eq!(0xa5a5_u16, gotten);
        assert!(item.get::<u8>().is_none()); // Nothing more to get.
        assert_eq!(item.event_data.len(), item.get_cursor);
    }
    #[test]
    fn get_4() {
        let mut item = PhysicsEvent::new(None);
        item.add(0xa5a5a5a5_u32);
        let gotten = item.get::<u32>();
        assert!(gotten.is_some());
        let gotten = gotten.unwrap();
        assert_eq!(0xa5a5a5a5_u32, gotten);
        assert!(item.get::<u8>().is_none()); // Nothing more to get.
        assert_eq!(item.event_data.len(), item.get_cursor);
    }
    #[test]
    fn get_5() {
        // Several things to get:

        let mut item = PhysicsEvent::new(None);

        item.add(0xa5_u8).add(0xa5a5_u16).add(0xa5a5a5a5_u32);

        let got = item.get::<u8>();
        assert!(got.is_some());
        assert_eq!(0xa5_u8, got.unwrap());

        let got = item.get::<u16>();
        assert!(got.is_some());
        assert_eq!(0xa5a5_u16, got.unwrap());

        let got = item.get::<u32>();
        assert!(got.is_some());
        assert_eq!(0xa5a5a5a5_u32, got.unwrap());

        // Nothing left:

        assert!(item.get::<u8>().is_none());
    }
    #[test]
    fn rewind_1() {
        // Several things to get:

        let mut item = PhysicsEvent::new(None);

        item.add(1_u8).add(2_u16).add(3_u32);

        // Consume the data:

        let _got = item.get::<u8>();
        let _got = item.get::<u16>();
        let _got = item.get::<u32>();

        item.rewind(); // Reset the cursor.
        let got = item.get::<u8>();
        assert!(got.is_some());
        assert_eq!(1_u8, got.unwrap());
    }
    #[test]
    fn get_bodyheader_1() {
        let item = PhysicsEvent::new(None);
        assert!(item.get_bodyheader().is_none());
    }
    #[test]
    fn get_bodyheader_2() {
        let item = PhysicsEvent::new(Some(BodyHeader {
            timestamp: 0x1234567890,
            source_id: 2,
            barrier_type: 0,
        }));
        let bh = item.get_bodyheader();
        assert!(bh.is_some());
        let bh = bh.unwrap();
        assert_eq!(0x1234567890_u64, bh.timestamp);
        assert_eq!(2, bh.source_id);
        assert_eq!(0, bh.barrier_type);
    }
    #[test]
    fn body_size_1() {
        let mut item = PhysicsEvent::new(None);

        item.add(0xa5_u8).add(0xa5a5_u16).add(0xa5a5a5a5_u32);

        assert_eq!(item.event_data.len(), item.body_size());
    }
    // First we'll test to_raw because then we can use it to generate
    // raw items that we can use to test from_raw with:

    #[test]
    fn to_raw_1() {
        // Empty no body header:

        let item = PhysicsEvent::new(None);
        let raw = item.to_raw();
        assert_eq!(PHYSICS_EVENT, raw.type_id());
        assert!(!raw.has_body_header());
        assert_eq!(0, raw.payload().len());
    }
    #[test]
    fn to_raw_2() {
        // Empty but with a body header:

        let item = PhysicsEvent::new(Some(BodyHeader {
            timestamp: 0x12345,
            source_id: 1,
            barrier_type: 0,
        }));
        let raw = item.to_raw();
        assert!(raw.has_body_header());
        let bh = raw.get_bodyheader().unwrap();
        assert_eq!(item.body_header.unwrap().timestamp, bh.timestamp);
        assert_eq!(item.body_header.unwrap().source_id, bh.source_id);
        assert_eq!(item.body_header.unwrap().barrier_type, bh.barrier_type);
        assert_eq!(size_of::<u64>() + 2 * size_of::<u32>(), raw.payload().len());
    }
    #[test]
    fn to_raw_3() {
        // no body header but contents:

        let mut item = PhysicsEvent::new(None);
        item.add(0xa5_u8).add(0xa5a5_u16).add(0xa5a5a5a5_u32);
        let raw = item.to_raw();
        assert_eq!(
            size_of::<u32>() + size_of::<u16>() + size_of::<u8>(),
            raw.payload().len()
        );

        let mut offset = 0;
        let p = raw.payload().as_slice();
        assert_eq!(
            0xa5_u8,
            u8::from_ne_bytes(p[offset..offset + size_of::<u8>()].try_into().unwrap())
        );
        offset += size_of::<u8>();
        assert_eq!(
            0xa5a5_u16,
            u16::from_ne_bytes(p[offset..offset + size_of::<u16>()].try_into().unwrap())
        );
        offset += size_of::<u16>();
        assert_eq!(
            0xa5a5a5a5_u32,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
    }
    #[test]
    fn to_raw_4() {
        // body header with contents:

        let mut item = PhysicsEvent::new(Some(BodyHeader {
            timestamp: 0x1234567890,
            source_id: 2,
            barrier_type: 0,
        }));
        item.add(0xa5_u8).add(0xa5a5_u16).add(0xa5a5a5a5_u32);
        let raw = item.to_raw();

        assert_eq!(
            body_header_size() + size_of::<u32>() + size_of::<u16>() + size_of::<u8>(),
            raw.payload().len()
        );
        let mut offset = body_header_size();
        let p = raw.payload().as_slice();
        assert_eq!(
            0xa5_u8,
            u8::from_ne_bytes(p[offset..offset + size_of::<u8>()].try_into().unwrap())
        );
        offset += size_of::<u8>();
        assert_eq!(
            0xa5a5_u16,
            u16::from_ne_bytes(p[offset..offset + size_of::<u16>()].try_into().unwrap())
        );
        offset += size_of::<u16>();
        assert_eq!(
            0xa5a5a5a5_u32,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
    }
    // to_raw works so generate raw items and go full circle:

    #[test]
    fn from_raw_1() {
        // empty no body header:

        let item = PhysicsEvent::new(None);
        let raw = item.to_raw();
        let event: Option<PhysicsEvent> = raw.to_specific(RingVersion::V11);
        assert!(event.is_some());
        let event = event.unwrap();

        assert!(event.body_header.is_none());
        assert_eq!(0, event.get_cursor);
        assert_eq!(0, event.event_data.len());
    }
    #[test]
    fn from_raw_2() {
        //empty but with body header:

        let item = PhysicsEvent::new(Some(BodyHeader {
            timestamp: 0xabcdef012345,
            source_id: 2,
            barrier_type: 0,
        }));
        let raw = item.to_raw();
        let event: Option<PhysicsEvent> = raw.to_specific(RingVersion::V11);
        assert!(event.is_some());
        let event = event.unwrap();
        assert!(event.body_header.is_some());
        let bh = event.body_header.unwrap();
        assert_eq!(item.body_header.unwrap().timestamp, bh.timestamp);
        assert_eq!(item.body_header.unwrap().source_id, bh.source_id);
        assert_eq!(item.body_header.unwrap().barrier_type, bh.barrier_type);
    }
    #[test]
    fn from_raw_3() {
        // no body header but a payload:

        let mut item = PhysicsEvent::new(None);
        item.add(0xa5_u8).add(0xa5a5_u16).add(0xa5a5a5a5_u32);
        let raw = item.to_raw();
        let event: Option<PhysicsEvent> = raw.to_specific(RingVersion::V11);
        assert!(event.is_some());
        let mut event = event.unwrap();

        assert_eq!(
            size_of::<u8>() + size_of::<u16>() + size_of::<u32>(),
            event.event_data.len()
        );
        assert_eq!(0xa5_u8, event.get::<u8>().unwrap());
        assert_eq!(0xa5a5_u16, event.get::<u16>().unwrap());
        assert_eq!(0xa5a5a5a5_u32, event.get::<u32>().unwrap());
        assert!(event.get::<u8>().is_none());
    }
    #[test]
    fn from_raw_4() {
        // body header and payload:

        let mut item = PhysicsEvent::new(Some(BodyHeader {
            timestamp: 0x12345678abdeef,
            source_id: 2,
            barrier_type: 5,
        }));
        item.add(0x1_u8).add(0x2_u16).add(0x3_u32);
        let raw = item.to_raw();
        let event: Option<PhysicsEvent> = raw.to_specific(RingVersion::V11);
        assert!(event.is_some());
        let mut event = event.unwrap();
        assert!(event.get_bodyheader().is_some());
        assert_eq!(
            size_of::<u8>() + size_of::<u16>() + size_of::<u32>(),
            event.event_data.len()
        );
        assert_eq!(1_u8, event.get::<u8>().unwrap());
        assert_eq!(2_u16, event.get::<u16>().unwrap());
        assert_eq!(3_u32, event.get::<u32>().unwrap());
        assert!(event.get::<u8>().is_none());
    }
    #[test]
    fn from_raw_5() {
        // failed conversion:

        let raw = RingItem::new(PHYSICS_EVENT + 1);
        let failed: Option<PhysicsEvent> = raw.to_specific(RingVersion::V11);
        assert!(failed.is_none());
    }
}
