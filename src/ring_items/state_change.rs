use crate::ring_items;
use std::mem;
use std::ops::Add;
use std::time;
///
/// provide support for state change items.
/// these are actually four different item types.

/// Types of run state transitions in rustly form:
#[derive(Clone, Copy)]
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
}

impl StateChange {
    fn string_len(d: &[u8]) -> usize {
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
        }
    }
    pub fn new(
        type_id: StateChangeType,
        body_header: Option<ring_items::BodyHeader>,
        run: u32,
        offset: u32,
        divisor: u32,
        title: &str,
    ) -> StateChange {
        match body_header {
            Some(h) => Self::new_with_body_header(type_id, &h, run, offset, divisor, title),
            None => Self::new_without_body_header(type_id, run, offset, divisor, title),
        }
    }
    /// new state change item from a raw item:
    pub fn from_raw(raw: &ring_items::RingItem) -> Option<Self> {
        let body_header = raw.get_bodyheader(); // Option of body header.
        if let Some(type_enum) = Self::type_from_type_id(raw.type_id()) {
            let mut result = Self::new(type_enum, body_header, 0, 0, 1, "");
            // Body position depends on if body_header is defined:

            let body_pos = if result.has_body_header {
                mem::size_of::<u64>() + 2 * mem::size_of::<u32>()
            } else {
                0
            };
            // Now we can fetch stuff out of the body:

            let payload = raw.payload().as_slice();
            result.run_number =
                u32::from_ne_bytes(payload[body_pos..body_pos + 4].try_into().unwrap());
            result.time_offset =
                u32::from_ne_bytes(payload[body_pos + 4..body_pos + 8].try_into().unwrap());
            let stamp = time::Duration::from_secs(u32::from_ne_bytes(
                payload[body_pos + 8..body_pos + 12].try_into().unwrap(),
            ) as u64);
            result.absolute_time = time::UNIX_EPOCH.add(stamp);
            result.offset_divisor =
                u32::from_ne_bytes(payload[body_pos + 12..body_pos + 16].try_into().unwrap());
            let title_len = Self::string_len(&payload[body_pos + 16..]);
            result.run_title = String::from_utf8(
                payload[body_pos + 16..body_pos + 16 + title_len]
                    .try_into()
                    .unwrap(),
            )
            .unwrap();
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
        item.add(self.run_number);
        item.add(self.time_offset);
        let unix_stamp = self.absolute_time.duration_since(time::UNIX_EPOCH).unwrap();
        let secs = unix_stamp.as_secs();
        let secsu32: u32 = (secs & 0xffffffff) as u32;
        item.add(secsu32);
        item.add(self.offset_divisor);

        // Need the string as bytes -- truncate to 80 and put in as bytes
        // with null terminator.

        let mut title = self.run_title.clone();
        title.truncate(79);
        let title_bytes = String::into_bytes(title.clone());
        for c in title_bytes {
            item.add(c);
        }

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
}
