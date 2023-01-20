use crate::ring_items;
use std::time;
///
/// provide support for state change items.
/// these are actually four different item types.

/// Types of run state transitions in rustly form:
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum StateChangeType {
    Begin,
    End,
    Pause,
    Resume,
}
/// The rustly form of a state change item:
///
pub struct StateChange {
    change_type: StateChangeType,
    has_body_header: bool,
    body_header: ring_items::BodyHeader, // only valid if has_body_header true
    run_number: u32,
    time_offset: u32,
    offset_divisor: u32,
    absolute_time: time::SystemTime,
    run_title: String,
    original_sid: Option<u32>,
}

impl StateChange {
    fn string_len(d: &[u8]) -> usize {
        ring_items::string_len(d)
    }
    fn string_from_type(&self) -> String {
        match self.change_type {
            StateChangeType::Begin => String::from("Begin"),
            StateChangeType::End => String::from("End"),
            StateChangeType::Pause => String::from("Pause"),
            StateChangeType::Resume => String::from("Resume"),
        }
    }
    fn type_id(&self) -> u32 {
        match self.change_type {
            StateChangeType::Begin => ring_items::BEGIN_RUN,
            StateChangeType::End => ring_items::END_RUN,
            StateChangeType::Pause => ring_items::PAUSE_RUN,
            StateChangeType::Resume => ring_items::RESUME_RUN,
        }
    }
    fn type_from_type_id(type_id: u32) -> Option<StateChangeType> {
        match type_id {
            ring_items::BEGIN_RUN => Some(StateChangeType::Begin),
            ring_items::END_RUN => Some(StateChangeType::End),
            ring_items::PAUSE_RUN => Some(StateChangeType::Pause),
            ring_items::RESUME_RUN => Some(StateChangeType::Resume),
            _ => None,
        }
    }

    /// Create a new state change item type with no body header.
    pub fn new_without_body_header(
        type_id: StateChangeType,
        run: u32,
        offset: u32,
        divisor: u32,
        title: &str,
        original_sid: Option<u32>,
    ) -> StateChange {
        StateChange {
            change_type: type_id,
            has_body_header: false,
            body_header: ring_items::BodyHeader {
                timestamp: 0,
                source_id: 0,
                barrier_type: 0,
            },
            run_number: run,
            time_offset: offset,
            offset_divisor: divisor,
            absolute_time: time::SystemTime::now(),
            run_title: String::from(title),
            original_sid: original_sid,
        }
    }
    /// new state change item with body header.
    pub fn new_with_body_header(
        type_id: StateChangeType,
        body_header: &ring_items::BodyHeader,
        run: u32,
        offset: u32,
        divisor: u32,
        title: &str,
        original_sid: Option<u32>,
    ) -> StateChange {
        StateChange {
            change_type: type_id,
            has_body_header: true,
            body_header: *body_header,
            run_number: run,
            time_offset: offset,
            offset_divisor: divisor,
            absolute_time: time::SystemTime::now(),
            run_title: String::from(title),
            original_sid: original_sid,
        }
    }
    pub fn new(
        type_id: StateChangeType,
        body_header: Option<ring_items::BodyHeader>,
        run: u32,
        offset: u32,
        divisor: u32,
        title: &str,
        original_sid: Option<u32>,
    ) -> StateChange {
        match body_header {
            Some(h) => {
                Self::new_with_body_header(type_id, &h, run, offset, divisor, title, original_sid)
            }
            None => {
                Self::new_without_body_header(type_id, run, offset, divisor, title, original_sid)
            }
        }
    }
    /// new state change item from a raw item:
    pub fn from_raw(raw: &ring_items::RingItem, version: ring_items::RingVersion) -> Option<Self> {
        let body_header = raw.get_bodyheader(); // Option of body header.
        if let Some(type_enum) = Self::type_from_type_id(raw.type_id()) {
            let mut result = Self::new(type_enum, body_header, 0, 0, 1, "", None);
            // Body position depends on if body_header is defined:

            let body_pos = if result.has_body_header {
                ring_items::body_header_size()
            } else {
                0
            };
            // Now we can fetch stuff out of the body:

            let payload = raw.payload().as_slice();
            result.run_number =
                u32::from_ne_bytes(payload[body_pos..body_pos + 4].try_into().unwrap());
            result.time_offset =
                u32::from_ne_bytes(payload[body_pos + 4..body_pos + 8].try_into().unwrap());
            let raw_stamp =
                u32::from_ne_bytes(payload[body_pos + 8..body_pos + 12].try_into().unwrap());
            result.absolute_time = ring_items::raw_to_systime(raw_stamp);
            result.offset_divisor =
                u32::from_ne_bytes(payload[body_pos + 12..body_pos + 16].try_into().unwrap());
            // Might have an original sid:

            let mut title_pos = body_pos + 16;
            if version == ring_items::RingVersion::V12 {
                result.original_sid = Some(u32::from_ne_bytes(
                    payload[title_pos..title_pos + 4].try_into().unwrap(),
                ));
                title_pos = title_pos + 4;
            }

            result.run_title = ring_items::get_c_string(&mut title_pos, &payload);
            return Some(result);
        } else {
            return None;
        }
    }
    // new raw item from this:
    pub fn to_raw(&self) -> ring_items::RingItem {
        let mut item = if self.has_body_header {
            ring_items::RingItem::new_with_body_header(
                self.type_id(),
                self.body_header.timestamp,
                self.body_header.source_id,
                self.body_header.barrier_type,
            )
        } else {
            ring_items::RingItem::new(self.type_id())
        };
        // Put in the other stuff:
        item.add(self.run_number).add(self.time_offset);
        let secsu32 = ring_items::systime_to_raw(self.absolute_time);
        item.add(secsu32).add(self.offset_divisor);

        // If there's an original sid it goes here:

        if let Some(osid) = self.original_sid {
            item.add(osid);
        }

        // Need the string as bytes -- truncate to 80 and put in as bytes
        // with null terminator.

        let mut title = self.run_title.clone();
        title.truncate(79);
        let title_bytes = String::into_bytes(title.clone());
        item.add_byte_vec(&title_bytes);

        // Pad out with nulls and ensure there's a null terminator
        // which  is not put in title_bytes by into_bytes.
        for _i in title.len()..81 {
            item.add(0 as u8);
        }
        item
    }

    // getters:

    pub fn change_type(&self) -> StateChangeType {
        self.change_type
    }

    pub fn change_type_string(&self) -> String {
        self.string_from_type()
    }
    pub fn body_header(&self) -> Option<ring_items::BodyHeader> {
        if self.has_body_header {
            return Some(self.body_header);
        } else {
            return None;
        }
    }
    pub fn run_number(&self) -> u32 {
        self.run_number
    }
    pub fn time_offset(&self) -> f32 {
        self.time_offset as f32 / self.offset_divisor as f32
    }
    pub fn raw_time_offset(&self) -> u32 {
        self.time_offset
    }
    pub fn offset_divisor(&self) -> u32 {
        self.offset_divisor
    }
    pub fn absolute_time(&self) -> time::SystemTime {
        self.absolute_time
    }
    pub fn title(&self) -> String {
        self.run_title.clone()
    }
    pub fn original_sid(&self) -> Option<u32> {
        self.original_sid
    }
}
#[cfg(test)]
mod state_tests {
    use crate::ring_items::*;
    use crate::state_change::*;
    use std::mem::size_of;
    use std::time::*;

    // just test new, rather than the qualified versions
    // since they delegate -- considser making the qualified
    // versions private?

    #[test]
    fn new_1() {
        // V11, no body header.
        let t = SystemTime::now();
        let item = StateChange::new(
            StateChangeType::Begin,
            None,
            12,
            0,
            1,
            "This is a title",
            None,
        ); // will be a later time.

        assert_eq!(StateChangeType::Begin, item.change_type);
        assert_eq!(false, item.has_body_header);
        assert_eq!(12, item.run_number);
        assert_eq!(0, item.time_offset);
        assert_eq!(1, item.offset_divisor);
        // >Should< be less than a second between t and the absolute_time stamp.
        assert!(item.absolute_time.duration_since(t).unwrap().as_secs() <= 1);
        assert_eq!(String::from("This is a title"), item.run_title);
        assert!(item.original_sid.is_none());
    }
    #[test]
    fn new_2() {
        // V11 body header:

        let bh = BodyHeader {
            timestamp: 0x123456789abcdef,
            source_id: 2,
            barrier_type: 1,
        };
        let t = SystemTime::now();
        let item = StateChange::new(
            StateChangeType::End,
            Some(bh),
            13,
            100,
            1,
            "Some title",
            None,
        );
        assert_eq!(StateChangeType::End, item.change_type);
        assert_eq!(true, item.has_body_header);
        assert_eq!(bh.timestamp, item.body_header.timestamp);
        assert_eq!(bh.source_id, item.body_header.source_id);
        assert_eq!(bh.barrier_type, item.body_header.barrier_type);

        assert_eq!(13, item.run_number);
        assert_eq!(100, item.time_offset);
        assert_eq!(1, item.offset_divisor);
        // >Should< be less than a second between t and the absolute_time stamp.
        assert!(item.absolute_time.duration_since(t).unwrap().as_secs() <= 1);
        assert!(item.original_sid.is_none());
        assert_eq!(String::from("Some title"), item.run_title);
    }
    #[test]
    fn new_3() {
        // V12 body header:

        let bh = BodyHeader {
            timestamp: 0x123456789abcdef,
            source_id: 2,
            barrier_type: 1,
        };
        let t = SystemTime::now();
        let item = StateChange::new(
            StateChangeType::End,
            Some(bh),
            13,
            100,
            1,
            "Some title",
            Some(5),
        );
        assert_eq!(StateChangeType::End, item.change_type);
        assert_eq!(true, item.has_body_header);
        assert_eq!(bh.timestamp, item.body_header.timestamp);
        assert_eq!(bh.source_id, item.body_header.source_id);
        assert_eq!(bh.barrier_type, item.body_header.barrier_type);

        assert_eq!(13, item.run_number);
        assert_eq!(100, item.time_offset);
        assert_eq!(1, item.offset_divisor);
        // >Should< be less than a second between t and the absolute_time stamp.
        assert!(item.absolute_time.duration_since(t).unwrap().as_secs() <= 1);
        assert!(item.original_sid.is_some());
        assert_eq!(5, item.original_sid.unwrap());
        assert_eq!(String::from("Some title"), item.run_title);
    }
    #[test]
    fn getter_1() {
        // change types:

        // V12 body header:

        let bh = BodyHeader {
            timestamp: 0x123456789abcdef,
            source_id: 2,
            barrier_type: 1,
        };
        let t = SystemTime::now();
        let item = StateChange::new(
            StateChangeType::End,
            Some(bh),
            13,
            100,
            1,
            "Some title",
            Some(5),
        );
        assert_eq!(StateChangeType::End, item.change_type());
        assert_eq!(String::from("End"), item.change_type_string());
    }
}
