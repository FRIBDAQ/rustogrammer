use crate::ring_items;
use std::mem;

///
/// This module contains code to handle physics event items.
/// What we're going to do is treat an event item body as a vector
/// of u8 but supply a cursor and methods to use that cursor to
/// fetch generically from the soup of bytes with cursor movement.
///  We'll also provide for insertion as the raw item can do.

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
    /// Given a raw ring item, if it is a PHYSICS_EVENT, build a
    /// new PhysicsEvent item from it.
    ///
    pub fn from_raw(raw: &ring_items::RingItem) -> Option<PhysicsEvent> {
        if raw.type_id() == ring_items::PHYSICS_EVENT {
            let mut result = PhysicsEvent::new(None);

            // If there's a body header we start taking payload after it
            // and put the body header in our body header:

            let mut payload_offset = 0;
            if let Some(bh) = raw.get_bodyheader() {
                result.body_header = Some(bh);
                payload_offset = ring_items::body_header_size();
            }
            result
                .event_data
                .extend_from_slice(&raw.payload().as_slice()[payload_offset..]);
            Some(result)
        } else {
            None
        }
    }
    ///
    /// Convert self to a raw ring item.
    ///
    pub fn to_raw(&mut self) -> ring_items::RingItem {
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

        result.add_byte_vec(&mut self.event_data);
        result
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
                p = p.offset(self.get_cursor as isize);
            }
            // Need to cast this to a pointer of type T:

            let pt = p.cast::<T>();
            let result = unsafe { *pt };
            self.get_cursor = self.get_cursor + mem::size_of::<T>();
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
#[cfg(test)]
mod test_event {
    use crate::event_item::*;
    use crate::ring_items::*;
    use std::mem::size_of;
    #[test]
    fn new_1() {
        let item = PhysicsEvent::new(None);
        assert!(item.body_header.is_none());
        assert_eq!(0, item.get_cursor);
        assert_eq!(0, item.event_data.len());
    }
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
}
