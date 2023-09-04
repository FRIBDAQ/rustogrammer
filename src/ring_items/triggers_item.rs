use crate::ring_items;
use humantime;
use std::fmt;
use std::time;
///
/// EventCountItems count the nunmber of triggers that have
/// been seen since the start of run.  This can be used for
/// determining the accepted event rate as well as, in a sampled client,
/// computing the fraction of events analyzed.
///

pub struct PhysicsEventCountItem {
    body_header: Option<ring_items::BodyHeader>,
    time_offset: u32,
    time_divisor: u32,
    absolute_time: time::SystemTime,
    original_sid: Option<u32>,
    event_count: u64,
}

impl PhysicsEventCountItem {
    pub fn new(
        bheader: Option<ring_items::BodyHeader>,
        offset: u32,
        divisor: u32,
        orsid: Option<u32>,
        evtcount: u64,
    ) -> PhysicsEventCountItem {
        PhysicsEventCountItem {
            body_header: bheader,
            time_offset: offset,
            time_divisor: divisor,
            absolute_time: time::SystemTime::now(),
            original_sid: orsid,
            event_count: evtcount,
        }
    }
    pub fn get_bodyheader(&self) -> Option<ring_items::BodyHeader> {
        self.body_header
    }
    pub fn get_timeoffset(&self) -> u32 {
        self.time_offset
    }
    pub fn get_time_divisor(&self) -> u32 {
        self.time_divisor
    }
    pub fn get_offset_time(&self) -> f32 {
        (self.time_offset as f32) / (self.time_divisor as f32)
    }
    pub fn get_original_sid(&self) -> Option<u32> {
        self.original_sid
    }
    pub fn get_event_count(&self) -> u64 {
        self.event_count
    }
    pub fn get_absolute_time(&self) -> time::SystemTime {
        self.absolute_time
    }
}
impl fmt::Display for PhysicsEventCountItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Trigger count information:").unwrap();
        if let Some(bh) = self.get_bodyheader() {
            write!(f, "BodyHeader: \n   {}", bh).unwrap();
        }
        writeln!(
            f,
            "{} Seconds in the run at {} : {} Triggers",
            self.get_offset_time(),
            humantime::format_rfc3339(self.get_absolute_time()),
            self.get_event_count()
        )
        .unwrap();
        if let Some(sid) = self.get_original_sid() {
            writeln!(f, "Original sid: {}", sid).unwrap();
        }
        write!(f, "")
    }
}

impl ring_items::ToRaw for PhysicsEventCountItem {
    fn to_raw(&self) -> ring_items::RingItem {
        let mut result = if let Some(bh) = self.body_header {
            ring_items::RingItem::new_with_body_header(
                ring_items::PHYSICS_EVENT_COUNT,
                bh.timestamp,
                bh.source_id,
                bh.barrier_type,
            )
        } else {
            ring_items::RingItem::new(ring_items::PHYSICS_EVENT_COUNT)
        };
        result
            .add(self.time_offset)
            .add(self.time_divisor)
            .add(ring_items::systime_to_raw(self.absolute_time));
        if let Some(sid) = self.original_sid {
            result.add(sid);
        }
        result.add(self.event_count);

        result
    }
}

impl ring_items::FromRaw<PhysicsEventCountItem> for ring_items::RingItem {
    fn to_specific(&self, version: ring_items::RingVersion) -> Option<PhysicsEventCountItem> {
        if self.type_id() == ring_items::PHYSICS_EVENT_COUNT {
            let mut result = PhysicsEventCountItem::new(None, 0, 1, None, 0);
            result.body_header = self.get_bodyheader();
            let offset = if result.body_header.is_some() {
                ring_items::body_header_size()
            } else {
                0
            };
            let payload = self.payload().as_slice();
            result.time_offset =
                u32::from_ne_bytes(payload[offset..offset + 4].try_into().unwrap());
            result.time_divisor =
                u32::from_ne_bytes(payload[offset + 4..offset + 8].try_into().unwrap());
            result.absolute_time = ring_items::raw_to_systime(u32::from_ne_bytes(
                payload[offset + 8..offset + 12].try_into().unwrap(),
            ));
            if version == ring_items::RingVersion::V11 {
                result.event_count =
                    u64::from_ne_bytes(payload[offset + 12..offset + 20].try_into().unwrap());
            } else {
                result.original_sid = Some(u32::from_ne_bytes(
                    payload[offset + 12..offset + 16].try_into().unwrap(),
                ));
                result.event_count =
                    u64::from_ne_bytes(payload[offset + 16..offset + 24].try_into().unwrap());
            }
            Some(result)
        } else {
            None
        }
    }
}
#[cfg(test)]
mod triggers_test {
    use super::*;
    use crate::ring_items::*;
    use std::mem::size_of;
    use std::time::SystemTime;

    #[test]
    fn new_1() {
        // NO body header v11:
        let t = SystemTime::now();
        let item = PhysicsEventCountItem::new(None, 10, 1, None, 100);
        assert!(item.body_header.is_none());
        assert_eq!(10, item.time_offset);
        assert_eq!(1, item.time_divisor);
        assert!(item.absolute_time.duration_since(t).unwrap().as_secs() <= 1);
        assert!(item.original_sid.is_none());
        assert_eq!(100, item.event_count);
    }
    #[test]
    fn new_2() {
        // body header, v11:

        let bh = BodyHeader {
            timestamp: 0x12345abdef,
            source_id: 2,
            barrier_type: 0,
        };
        let t = SystemTime::now();
        let item = PhysicsEventCountItem::new(Some(bh), 10, 1, None, 100);
        assert!(item.body_header.is_some());
        let ibh = item.body_header.unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert_eq!(10, item.time_offset);
        assert_eq!(1, item.time_divisor);
        assert!(item.absolute_time.duration_since(t).unwrap().as_secs() <= 1);
        assert!(item.original_sid.is_none());
        assert_eq!(100, item.event_count);
    }
    #[test]
    fn new_3() {
        // body header, v12:

        let bh = BodyHeader {
            timestamp: 0x12345abdef,
            source_id: 2,
            barrier_type: 0,
        };
        let t = SystemTime::now();
        let item = PhysicsEventCountItem::new(Some(bh), 10, 1, Some(5), 100);
        assert!(item.body_header.is_some());
        let ibh = item.body_header.unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert_eq!(10, item.time_offset);
        assert_eq!(1, item.time_divisor);
        assert!(item.absolute_time.duration_since(t).unwrap().as_secs() <= 1);
        assert!(item.original_sid.is_some());
        assert_eq!(5, item.original_sid.unwrap());
        assert_eq!(100, item.event_count);
    }
    // Getters:
    #[test]
    fn getters_1() {
        // NO body header v11:
        let t = SystemTime::now();
        let item = PhysicsEventCountItem::new(None, 10, 2, None, 100);

        assert!(item.get_bodyheader().is_none());
        assert_eq!(10, item.get_timeoffset());
        assert_eq!(2, item.get_time_divisor());
        assert_eq!(5.0, item.get_offset_time());
        assert!(item.get_original_sid().is_none());
        assert_eq!(100, item.get_event_count());
        assert!(
            item.get_absolute_time()
                .duration_since(t)
                .unwrap()
                .as_secs()
                <= 1
        );
    }
    #[test]
    fn getters_2() {
        // v12 with body header:

        // body header, v12:

        let bh = BodyHeader {
            timestamp: 0x12345abdef,
            source_id: 2,
            barrier_type: 0,
        };
        let _t = SystemTime::now();
        let item = PhysicsEventCountItem::new(Some(bh), 10, 1, Some(5), 100);
        assert!(item.get_bodyheader().is_some());
        let ibh = item.get_bodyheader().unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert!(item.get_original_sid().is_some());
        assert_eq!(5, item.get_original_sid().unwrap());
    }
    // As usual we test the to_raw method so that later, we can
    // use it to generate raw items for from_raw tests.
    #[test]
    fn to_raw_1() {
        // No body header v11:

        let _t = SystemTime::now();
        let item = PhysicsEventCountItem::new(None, 10, 1, None, 100);
        let raw = item.to_raw();
        assert_eq!(PHYSICS_EVENT_COUNT, raw.type_id());
        assert!(!raw.has_body_header());

        let u32s = size_of::<u32>();
        let p = raw.payload().as_slice();
        let mut offset = 0;

        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset = offset + u32s;

        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        assert_eq!(
            systime_to_raw(item.get_absolute_time()),
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        assert_eq!(
            100,
            u64::from_ne_bytes(p[offset..offset + size_of::<u64>()].try_into().unwrap())
        );
        offset += size_of::<u64>();

        assert_eq!(offset, p.len());
    }
    #[test]
    fn to_raw_2() {
        let bh = BodyHeader {
            timestamp: 0x12345abdef,
            source_id: 2,
            barrier_type: 0,
        };
        let _t = SystemTime::now();
        let item = PhysicsEventCountItem::new(Some(bh), 10, 1, None, 100);
        let raw = item.to_raw();

        assert_eq!(PHYSICS_EVENT_COUNT, raw.type_id());
        assert!(raw.has_body_header());

        let ibh = raw.get_bodyheader().unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        let u32s = size_of::<u32>();
        let p = raw.payload().as_slice();
        let mut offset = body_header_size();

        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset = offset + u32s;

        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        assert_eq!(
            systime_to_raw(item.get_absolute_time()),
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        assert_eq!(
            100,
            u64::from_ne_bytes(p[offset..offset + size_of::<u64>()].try_into().unwrap())
        );
        offset += size_of::<u64>();

        assert_eq!(offset, p.len());
    }
    #[test]
    fn to_raw_3() {
        // body header and original source id:

        let bh = BodyHeader {
            timestamp: 0x12345abdef,
            source_id: 2,
            barrier_type: 0,
        };
        let _t = SystemTime::now();
        let item = PhysicsEventCountItem::new(Some(bh), 10, 1, Some(5), 100);
        let raw = item.to_raw();

        let u32s = size_of::<u32>();
        let p = raw.payload().as_slice();
        let mut offset = body_header_size();

        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset = offset + u32s;

        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        assert_eq!(
            systime_to_raw(item.get_absolute_time()),
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;
        // has original sid:
        assert_eq!(
            5,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        assert_eq!(
            100,
            u64::from_ne_bytes(p[offset..offset + size_of::<u64>()].try_into().unwrap())
        );
        offset += size_of::<u64>();

        assert_eq!(offset, p.len());
    }
    // can now test from_raw().

    #[test]
    fn from_raw_1() {
        // no body header v11
        let _t = SystemTime::now();
        let item = PhysicsEventCountItem::new(None, 10, 1, None, 100);
        let raw = item.to_raw();
        let recons: Option<PhysicsEventCountItem> = raw.to_specific(RingVersion::V11);

        assert!(recons.is_some());
        let recons = recons.unwrap();
        assert!(recons.get_bodyheader().is_none());
        assert_eq!(item.get_timeoffset(), recons.get_timeoffset());
        assert_eq!(item.get_time_divisor(), recons.get_time_divisor());
        assert!(recons.get_original_sid().is_none());
        assert_eq!(item.get_event_count(), recons.get_event_count());
        assert_eq!(
            systime_to_raw(item.get_absolute_time()),
            systime_to_raw(recons.get_absolute_time())
        );
    }
    #[test]
    fn from_raw_2() {
        // V11, body header.
        let bh = BodyHeader {
            timestamp: 0x12345abdef,
            source_id: 2,
            barrier_type: 0,
        };
        let _t = SystemTime::now();
        let item = PhysicsEventCountItem::new(Some(bh), 10, 1, None, 100);
        let raw = item.to_raw();
        let recons: Option<PhysicsEventCountItem> = raw.to_specific(RingVersion::V11);

        assert!(recons.is_some());
        let recons = recons.unwrap();
        assert!(recons.get_bodyheader().is_some());
        let ibh = recons.get_bodyheader().unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert_eq!(item.get_timeoffset(), recons.get_timeoffset());
        assert_eq!(item.get_time_divisor(), recons.get_time_divisor());
        assert!(recons.get_original_sid().is_none());
        assert_eq!(item.get_event_count(), recons.get_event_count());
        assert_eq!(
            systime_to_raw(item.get_absolute_time()),
            systime_to_raw(recons.get_absolute_time())
        );
    }
    #[test]
    fn from_raw_3() {
        // V12 body header:

        let bh = BodyHeader {
            timestamp: 0x12345abdef,
            source_id: 2,
            barrier_type: 0,
        };
        let _t = SystemTime::now();
        let item = PhysicsEventCountItem::new(Some(bh), 10, 1, Some(5), 100);
        let raw = item.to_raw();
        let recons: Option<PhysicsEventCountItem> = raw.to_specific(RingVersion::V12);

        assert!(recons.is_some());
        let recons = recons.unwrap();
        assert!(recons.get_bodyheader().is_some());
        let ibh = recons.get_bodyheader().unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert_eq!(item.get_timeoffset(), recons.get_timeoffset());
        assert_eq!(item.get_time_divisor(), recons.get_time_divisor());
        assert!(recons.get_original_sid().is_some());
        assert_eq!(
            item.get_original_sid().unwrap(),
            recons.get_original_sid().unwrap()
        );
        assert_eq!(item.get_event_count(), recons.get_event_count());
        assert_eq!(
            systime_to_raw(item.get_absolute_time()),
            systime_to_raw(recons.get_absolute_time())
        );
    }
    #[test]
    fn from_raw_4() {
        // Invalid conversion:

        let raw = RingItem::new(PHYSICS_EVENT_COUNT + 1);
        let recons: Option<PhysicsEventCountItem> = raw.to_specific(RingVersion::V11);
        assert!(recons.is_none());
        let recons: Option<PhysicsEventCountItem> = raw.to_specific(RingVersion::V12);
        assert!(recons.is_none());
    }
}
