use crate::ring_items;
use std::fmt;
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
    body_header: Option<ring_items::BodyHeader>, // issue 2 make an option of this.
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
            body_header: None,
            run_number: run,
            time_offset: offset,
            offset_divisor: divisor,
            absolute_time: time::SystemTime::now(),
            run_title: String::from(title),
            original_sid,
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
            body_header: Some(*body_header),
            run_number: run,
            time_offset: offset,
            offset_divisor: divisor,
            absolute_time: time::SystemTime::now(),
            run_title: String::from(title),
            original_sid,
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

    // getters:

    pub fn change_type(&self) -> StateChangeType {
        self.change_type
    }

    pub fn change_type_string(&self) -> String {
        self.string_from_type()
    }
    pub fn body_header(&self) -> Option<ring_items::BodyHeader> {
        self.body_header
    }
    pub fn has_body_header(&self) -> bool {
        self.body_header.is_some()
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

impl fmt::Display for StateChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "State Change: {}", self.change_type_string()).unwrap();
        if let Some(bh) = self.body_header {
            write!(f, "Body header\n  {}", bh).unwrap();
        }
        writeln!(
            f,
            " run: {} offset {} seconds",
            self.run_number(),
            self.time_offset(),
        )
        .unwrap();
        if let Some(osid) = self.original_sid() {
            writeln!(f, " original sid: {}", osid).unwrap();
        }
        writeln!(f, "Title: {}", self.title()).unwrap();
        write!(
            f,
            " Stamp {}",
            humantime::format_rfc3339(self.absolute_time())
        )
    }
}
impl ring_items::ToRaw for StateChange {
    fn to_raw(&self) -> ring_items::RingItem {
        let mut item = if self.has_body_header() {
            ring_items::RingItem::new_with_body_header(
                self.type_id(),
                self.body_header.unwrap().timestamp,
                self.body_header.unwrap().source_id,
                self.body_header.unwrap().barrier_type,
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
        let title_bytes = title.into_bytes();
        item.add_byte_vec(&title_bytes);

        // Pad out with nulls and ensure there's a null terminator
        // which  is not put in title_bytes by into_bytes.
        for _i in title_bytes.len()..81 {
            item.add(0_u8);
        }
        item
    }
}
impl ring_items::FromRaw<StateChange> for ring_items::RingItem {
    fn to_specific(&self, version: ring_items::RingVersion) -> Option<StateChange> {
        let body_header = self.get_bodyheader(); // Option of body header.
        if let Some(type_enum) = StateChange::type_from_type_id(self.type_id()) {
            let mut result = StateChange::new(type_enum, body_header, 0, 0, 1, "", None);
            // Body position depends on if body_header is defined:

            let body_pos = if result.has_body_header() {
                ring_items::body_header_size()
            } else {
                0
            };
            // Now we can fetch stuff out of the body:

            let payload = self.payload().as_slice();
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
                title_pos += 4;
            }

            result.run_title = ring_items::get_c_string(&mut title_pos, payload);
            Some(result)
        } else {
            None
        }
    }
}
#[cfg(test)]
mod state_tests {
    use super::*;
    use crate::ring_items::*;
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
        assert_eq!(false, item.has_body_header());
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
        assert_eq!(true, item.body_header.is_some());
        let ibh = item.body_header.unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

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
        assert_eq!(true, item.body_header.is_some());
        let ibh = item.body_header.unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

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
        let _t = SystemTime::now();
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
    #[test]
    fn getter_2() {
        // Raw items:

        // V12 body header:

        let bh = BodyHeader {
            timestamp: 0x123456789abcdef,
            source_id: 2,
            barrier_type: 1,
        };
        let _t = SystemTime::now();
        let item = StateChange::new(
            StateChangeType::End,
            Some(bh),
            13,
            100,
            1,
            "Some title",
            Some(5),
        );

        assert_eq!(13, item.run_number());
        assert_eq!(100, item.raw_time_offset());
        assert_eq!(1, item.offset_divisor());
        assert_eq!(String::from("Some title"), item.title());
    }
    #[test]
    fn getter_3() {
        // absolute time:

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
        assert!(item.absolute_time().duration_since(t).unwrap().as_secs() <= 1);
    }
    #[test]
    fn getter_4() {
        // time in seconds into run.

        // V12 body header:

        let bh = BodyHeader {
            timestamp: 0x123456789abcdef,
            source_id: 2,
            barrier_type: 1,
        };
        let _t = SystemTime::now();
        let item = StateChange::new(
            StateChangeType::End,
            Some(bh),
            13,
            100,
            2,
            "Some title",
            Some(5),
        );
        assert_eq!(50.0, item.time_offset());
    }
    fn getter_5() {
        let bh = BodyHeader {
            timestamp: 0x123456789abcdef,
            source_id: 2,
            barrier_type: 1,
        };
        let _t = SystemTime::now();
        let item = StateChange::new(
            StateChangeType::End,
            Some(bh),
            13,
            100,
            2,
            "Some title",
            Some(5),
        );
        assert!(item.original_sid().is_some());
        assert_eq!(5, item.original_sid().unwrap());

        let item = StateChange::new(
            StateChangeType::End,
            Some(bh),
            13,
            100,
            2,
            "Some title",
            None,
        );
        assert!(item.original_sid().is_none());
    }
    // as has become ususal...test to_raw first so that it
    // can be used to generate raw items for from_raw tests.
    #[test]
    fn to_raw_1() {
        // No body header V11 format:

        let _t = SystemTime::now();
        let item = StateChange::new(StateChangeType::End, None, 13, 100, 2, "Some title", None);
        let raw = item.to_raw();
        assert_eq!(END_RUN, raw.type_id());
        assert!(!raw.has_body_header());

        let p = raw.payload().as_slice();
        let mut offset = 0; // payload
        let u32s = size_of::<u32>();
        assert_eq!(
            13,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;
        assert_eq!(
            100,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;
        assert_eq!(
            systime_to_raw(item.absolute_time()),
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        assert_eq!(
            2,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        // there's no original sid so the title is next:

        offset += u32s;
        let mut o = offset;
        let title = get_c_string(&mut o, p);
        assert_eq!(String::from("Some title"), title);

        // The title is a fixed size block so:

        offset += 81;
        assert_eq!(offset, p.len());
    }
    #[test]
    fn to_raw_2() {
        // V11 body header .

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
            2,
            "Some title",
            None,
        );
        let raw = item.to_raw();

        assert_eq!(END_RUN, raw.type_id());
        assert!(raw.has_body_header());
        let rbh = raw.get_bodyheader().unwrap();
        assert_eq!(bh.timestamp, rbh.timestamp);
        assert_eq!(bh.source_id, rbh.source_id);
        assert_eq!(bh.barrier_type, rbh.barrier_type);

        let p = raw.payload().as_slice();
        let mut offset = body_header_size(); // Payload starts here:
        let u32s = size_of::<u32>();

        assert_eq!(
            13,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;
        assert_eq!(
            100,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;
        assert_eq!(
            systime_to_raw(t),
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        assert_eq!(
            2,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        // there's no original sid so the title is next:

        offset += u32s;
        let mut o = offset;
        let title = get_c_string(&mut o, p);
        assert_eq!(String::from("Some title"), title);

        // The title is a fixed size block so:

        offset += 81;
        assert_eq!(offset, p.len());
    }
    #[test]
    fn to_raw_3() {
        // Body header in v12+ format:

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
            2,
            "Some title",
            Some(5),
        );

        let raw = item.to_raw();

        assert_eq!(END_RUN, raw.type_id());

        assert!(raw.has_body_header());
        let rbh = raw.get_bodyheader().unwrap();
        assert_eq!(bh.timestamp, rbh.timestamp);
        assert_eq!(bh.source_id, rbh.source_id);
        assert_eq!(bh.barrier_type, rbh.barrier_type);

        let p = raw.payload().as_slice();
        let mut offset = body_header_size(); // Payload starts here:
        let u32s = size_of::<u32>();

        assert_eq!(
            13,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;
        assert_eq!(
            100,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;
        assert_eq!(
            systime_to_raw(t),
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        assert_eq!(
            2,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        // there's an original sid so
        offset += u32s;
        assert_eq!(
            5,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );

        // The title is next:

        offset += u32s;
        let mut o = offset;
        let title = get_c_string(&mut o, p);
        assert_eq!(String::from("Some title"), title);

        // The title is a fixed size block so:

        offset += 81;
        assert_eq!(offset, p.len());
    }
    // Ok so to_raw seems to work.  We can use it to produce
    // raw items for from_raw to work on for testing:

    #[test]
    fn from_raw_1() {
        // no body header, v11 format:

        let t = SystemTime::now();
        let item = StateChange::new(StateChangeType::End, None, 13, 100, 2, "Some title", None);
        let raw = item.to_raw();
        let recons: Option<StateChange> = raw.to_specific(RingVersion::V11);
        assert!(recons.is_some());
        let recons = recons.unwrap();

        assert_eq!(StateChangeType::End, recons.change_type());
        assert!(recons.body_header().is_none());
        assert_eq!(13, recons.run_number());
        assert_eq!(100, recons.raw_time_offset());
        assert_eq!(2, recons.offset_divisor());
        assert_eq!(systime_to_raw(t), systime_to_raw(recons.absolute_time()));
        assert_eq!(String::from("Some title"), recons.title());
        assert!(recons.original_sid().is_none());
    }
    #[test]
    fn from_raw_2() {
        // v11 with body header:

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
            2,
            "Some title",
            None,
        );
        let raw = item.to_raw();
        let recons: Option<StateChange> = raw.to_specific(RingVersion::V11);
        assert!(recons.is_some());
        let recons = recons.unwrap();

        assert_eq!(StateChangeType::End, recons.change_type());
        assert!(recons.body_header().is_some());
        let rbh = recons.body_header().unwrap();
        assert_eq!(bh.timestamp, rbh.timestamp);
        assert_eq!(bh.source_id, rbh.source_id);
        assert_eq!(bh.barrier_type, rbh.barrier_type);

        assert_eq!(13, recons.run_number());
        assert_eq!(100, recons.raw_time_offset());
        assert_eq!(2, recons.offset_divisor());
        assert_eq!(systime_to_raw(t), systime_to_raw(recons.absolute_time()));
        assert_eq!(String::from("Some title"), recons.title());
        assert!(recons.original_sid().is_none());
    }
    #[test]
    fn from_raw_3() {
        // v12 with body header:

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
            2,
            "Some title",
            Some(5),
        );

        let raw = item.to_raw();
        let recons: Option<StateChange> = raw.to_specific(RingVersion::V12);
        assert!(recons.is_some());
        let recons = recons.unwrap();

        assert_eq!(StateChangeType::End, recons.change_type());
        assert!(recons.body_header().is_some());
        let rbh = recons.body_header().unwrap();
        assert_eq!(bh.timestamp, rbh.timestamp);
        assert_eq!(bh.source_id, rbh.source_id);
        assert_eq!(bh.barrier_type, rbh.barrier_type);

        assert_eq!(13, recons.run_number());
        assert_eq!(100, recons.raw_time_offset());
        assert_eq!(2, recons.offset_divisor());
        assert_eq!(systime_to_raw(t), systime_to_raw(recons.absolute_time()));
        assert_eq!(String::from("Some title"), recons.title());
        assert!(recons.original_sid().is_some());
        assert_eq!(5, recons.original_sid().unwrap());
    }
    #[test]
    fn from_raw_4() {
        // bad type -> None:

        let raw = RingItem::new(PHYSICS_EVENT);
        let recons: Option<StateChange> = raw.to_specific(RingVersion::V11);
        assert!(recons.is_none());
        let recons: Option<StateChange> = raw.to_specific(RingVersion::V12);
        assert!(recons.is_none());
    }
}
