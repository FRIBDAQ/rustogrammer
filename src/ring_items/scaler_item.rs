use crate::ring_items;
use humantime;
use std::fmt;
use std::slice::Iter;
use std::time;
///
/// Provide an internalt representation of scaler items
/// with methods that allow one to also get the
/// item from a raw item and create a raw item from the internal item.
///

pub struct ScalerItem {
    body_header: Option<ring_items::BodyHeader>,
    start_offset: u32,
    end_offset: u32,
    absolute_time: time::SystemTime,
    divisor: u32,
    is_incremental: bool,
    original_sid: Option<u32>,
    scalers: Vec<u32>,
}

impl ScalerItem {
    pub fn get_body_header(&self) -> Option<ring_items::BodyHeader> {
        self.body_header
    }
    pub fn get_start_offset(&self) -> u32 {
        self.start_offset
    }
    pub fn get_start_secs(&self) -> f32 {
        (self.start_offset as f32) / (self.divisor as f32)
    }
    pub fn get_end_offset(&self) -> u32 {
        self.end_offset
    }
    pub fn get_end_secs(&self) -> f32 {
        (self.end_offset as f32) / (self.divisor as f32)
    }
    pub fn get_absolute_time(&self) -> time::SystemTime {
        self.absolute_time
    }
    pub fn is_incremental(&self) -> bool {
        self.is_incremental
    }
    pub fn original_sid(&self) -> Option<u32> {
        self.original_sid
    }

    pub fn get_scaler_values(&self) -> &Vec<u32> {
        &self.scalers
    }
    pub fn len(&self) -> usize {
        self.scalers.len()
    }
    pub fn iter(&self) -> Iter<'_, u32> {
        self.scalers.iter()
    }

    pub fn append_scaler(&mut self, sc: u32) -> &mut Self {
        self.scalers.push(sc);
        self
    }
    /// Note that the scalers are consumed by this method
    /// because append is used to block append them to an empty vector.
    ///
    pub fn new(
        body_header: Option<ring_items::BodyHeader>,
        start: u32,
        end: u32,
        time: time::SystemTime,
        divisor: u32,
        incremental: bool,
        orsid: Option<u32>,
        scalers: &mut Vec<u32>,
    ) -> ScalerItem {
        let mut result = ScalerItem {
            body_header: body_header,
            start_offset: start,
            end_offset: end,
            absolute_time: time,
            divisor: divisor,
            is_incremental: incremental,
            original_sid: orsid,
            scalers: Vec::<u32>::new(),
        };
        result.scalers.append(scalers);

        result
    }

    pub fn from_raw(
        raw: &ring_items::RingItem,
        fmt: ring_items::RingVersion,
    ) -> Option<ScalerItem> {
        if raw.type_id() == ring_items::PERIODIC_SCALERS {
            // Pull parameters from the raw item:

            let body_header = raw.get_bodyheader();
            let offset: usize = if let Some(_b) = body_header {
                ring_items::body_header_size()
            } else {
                0
            };
            let p = raw.payload().as_slice();
            let start = u32::from_ne_bytes(p[offset..offset + 4].try_into().unwrap());
            let end = u32::from_ne_bytes(p[offset + 4..offset + 8].try_into().unwrap());
            let raw_stamp = u32::from_ne_bytes(p[offset + 8..offset + 12].try_into().unwrap());
            let divisor = u32::from_ne_bytes(p[offset + 12..offset + 16].try_into().unwrap());
            let nscalers = u32::from_ne_bytes(p[offset + 16..offset + 20].try_into().unwrap());
            let incr = u32::from_ne_bytes(p[offset + 20..offset + 24].try_into().unwrap()) != 0;
            let mut offset = offset + 24; // new offset.

            let mut orsid: Option<u32> = None;
            if fmt == ring_items::RingVersion::V12 {
                orsid = Some(u32::from_ne_bytes(
                    p[offset..offset + 4].try_into().unwrap(),
                ));
                offset = offset + 4;
            }
            // Offset now points at the scalers regardless of the format:

            let mut scalers: Vec<u32> = Vec::new();
            for _ in 0..nscalers {
                scalers.push(u32::from_ne_bytes(
                    p[offset..offset + 4].try_into().unwrap(),
                ));
                offset = offset + 4;
            }
            Some(Self::new(
                body_header,
                start,
                end,
                ring_items::raw_to_systime(raw_stamp),
                divisor,
                incr,
                orsid,
                &mut scalers,
            ))
        } else {
            None
        }
    }
    pub fn to_raw(&self) -> ring_items::RingItem {
        let mut result = if let Some(bh) = self.body_header {
            ring_items::RingItem::new_with_body_header(
                ring_items::PERIODIC_SCALERS,
                bh.timestamp,
                bh.source_id,
                bh.barrier_type,
            )
        } else {
            ring_items::RingItem::new(ring_items::PERIODIC_SCALERS)
        };

        // Now the rest of the item:

        result
            .add(self.start_offset)
            .add(self.end_offset)
            .add(ring_items::systime_to_raw(self.absolute_time))
            .add(self.divisor)
            .add(self.scalers.len() as u32);
        let incr: u32 = if self.is_incremental { 1 } else { 0 };
        result.add(incr);
        if let Some(osid) = self.original_sid {
            result.add(osid);
        }
        for sc in &self.scalers {
            result.add(*sc);
        }

        result
    }
}

impl fmt::Display for ScalerItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, " Scaler: \n").unwrap();
        if let Some(bh) = self.body_header {
            write!(f, "Body header:\n  {}\n", bh).unwrap();
        }
        write!(
            f,
            "  Start: {} End {}\n",
            self.get_start_secs(),
            self.get_end_secs()
        )
        .unwrap();
        write!(
            f,
            "  At: {}\n",
            humantime::format_rfc3339(self.get_absolute_time())
        )
        .unwrap();
        if let Some(osid) = self.original_sid() {
            write!(f, " Original source id {}\n", osid).unwrap();
        }

        write!(f, " {} scalers:\n", self.len()).unwrap();
        for s in self.iter() {
            write!(f, "    {} counts\n", *s).unwrap();
        }
        write!(f, "")
    }
}

#[cfg(test)]
mod scaler_tests {
    use crate::ring_items::*;
    use crate::scaler_item::*;
    use std::mem::size_of;
    use std::time::*;
    #[test]
    fn new_1() {
        // Empty scaler item with no body header:
        // 11.x style.
        let mut scalers = Vec::<u32>::new();
        let t = SystemTime::now();
        let item = ScalerItem::new(None, 0, 10, t, 1, true, None, &mut scalers);
        assert!(item.body_header.is_none());
        assert_eq!(0, item.start_offset);
        assert_eq!(10, item.end_offset);
        assert_eq!(t, item.absolute_time);
        assert_eq!(1, item.divisor);
        assert!(item.original_sid.is_none());
        assert_eq!(0, item.scalers.len());
    }
    #[test]
    fn new_2() {
        // empty scaler item with body header, 11.x style:

        let mut scalers = Vec::<u32>::new();
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0x123456789,
            source_id: 2,
            barrier_type: 0,
        };
        let item = ScalerItem::new(Some(bh), 0, 10, t, 1, true, None, &mut scalers);
        assert!(item.body_header.is_some());
        let ibh = item.body_header.unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);
    }
    #[test]
    fn new_3() {
        // Empty scaler no body header, 12.x format.

        let mut scalers = Vec::<u32>::new();
        let t = SystemTime::now();
        let item = ScalerItem::new(None, 0, 10, t, 1, true, Some(5), &mut scalers);

        assert!(item.original_sid.is_some());
        assert_eq!(5, item.original_sid.unwrap());
    }
    #[test]
    fn new_4() {
        // empty scaler body header, 12.x format:

        let mut scalers = Vec::<u32>::new();
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0x123456789,
            source_id: 2,
            barrier_type: 0,
        };
        let item = ScalerItem::new(Some(bh), 0, 10, t, 1, true, Some(5), &mut scalers);

        assert!(item.original_sid.is_some());
        assert_eq!(5, item.original_sid.unwrap());
    }
    #[test]
    fn new_5() {
        // nonempty 11.x form:

        let mut scalers: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
        let t = SystemTime::now();
        let item = ScalerItem::new(None, 0, 10, t, 1, true, None, &mut scalers);

        assert_eq!(6, item.scalers.len());
        for i in 0..6 {
            assert_eq!((i + 1) as u32, item.scalers[i]);
        }
    }
    #[test]
    fn new_6() {
        // nonempty 12.x form
        let mut scalers: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
        let t = SystemTime::now();
        let item = ScalerItem::new(None, 0, 10, t, 1, true, Some(5), &mut scalers);

        assert_eq!(6, item.scalers.len());
        for i in 0..6 {
            assert_eq!((i + 1) as u32, item.scalers[i]);
        }
    }
    #[test]
    fn getters_1() {
        // nonempty 12.x form
        let mut scalers: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
        let t = SystemTime::now();
        let item = ScalerItem::new(None, 0, 10, t, 1, true, None, &mut scalers);

        assert!(item.get_body_header().is_none());
        assert_eq!(0, item.get_start_offset());
        assert_eq!(0.0, item.get_start_secs());
        assert_eq!(10, item.get_end_offset());
        assert_eq!(10.0, item.get_end_secs());
        assert_eq!(t, item.get_absolute_time());
        assert!(item.is_incremental());
        assert!(item.original_sid.is_none());
    }
    #[test]
    fn getters_2() {
        let mut scalers: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0xabcdef987654321,
            source_id: 2,
            barrier_type: 0,
        };
        let item = ScalerItem::new(Some(bh), 1, 10, t, 2, true, Some(5), &mut scalers);

        assert_eq!(5.0, item.get_end_secs());
        assert_eq!(0.5, item.get_start_secs());

        assert!(item.get_body_header().is_some());
        let ibh = item.get_body_header().unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert!(item.original_sid().is_some());
        assert_eq!(5, item.original_sid().unwrap());
    }
    #[test]
    fn getters_3() {
        // get scaler values:

        let mut scalers: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0xabcdef987654321,
            source_id: 2,
            barrier_type: 0,
        };
        let item = ScalerItem::new(Some(bh), 1, 10, t, 2, true, Some(5), &mut scalers);

        let values = item.get_scaler_values();
        for i in 0..values.len() {
            assert_eq!((i + 1) as u32, values[i]);
        }
    }
    #[test]
    fn getters_4() {
        let mut scalers: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0xabcdef987654321,
            source_id: 2,
            barrier_type: 0,
        };
        let item = ScalerItem::new(Some(bh), 1, 10, t, 2, true, Some(5), &mut scalers);

        assert_eq!(item.scalers.len(), item.len());
    }
    #[test]
    fn getters_5() {
        let mut scalers: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0xabcdef987654321,
            source_id: 2,
            barrier_type: 0,
        };
        let item = ScalerItem::new(Some(bh), 1, 10, t, 2, true, Some(5), &mut scalers);

        let mut i = 1;
        for s in item.iter() {
            assert_eq!(i, *s);
            i += 1;
        }
    }
    #[test]
    fn append_1() {
        // append single scaler:

        let mut scalers = Vec::<u32>::new();
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0xabcdef987654321,
            source_id: 2,
            barrier_type: 0,
        };
        let mut item = ScalerItem::new(Some(bh), 1, 10, t, 2, true, Some(5), &mut scalers);

        item.append_scaler(1234);
        assert_eq!(1, item.len());
        assert_eq!(1234, item.get_scaler_values()[0]);
    }
    #[test]
    fn append_2() {
        // chain:
        let mut scalers = Vec::<u32>::new();
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0xabcdef987654321,
            source_id: 2,
            barrier_type: 0,
        };
        let mut item = ScalerItem::new(Some(bh), 1, 10, t, 2, true, Some(5), &mut scalers);

        item.append_scaler(0).append_scaler(1).append_scaler(2);
        assert_eq!(3, item.len());
        for i in 0..3 {
            assert_eq!(i as u32, item.get_scaler_values()[i]);
        }
    }
    #[test]
    fn append_3() {
        //Appends to existing values:

        let mut scalers: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0xabcdef987654321,
            source_id: 2,
            barrier_type: 0,
        };
        let mut item = ScalerItem::new(Some(bh), 1, 10, t, 2, true, Some(5), &mut scalers);

        item.append_scaler(7).append_scaler(8);
        assert_eq!(8, item.len());
        for i in 0..item.len() {
            assert_eq!((i + 1) as u32, item.get_scaler_values()[i]);
        }
    }
    // Test to_raw so that we can use it to generate raw items. for
    // from_raw tests:

    #[test]
    fn to_raw_1() {
        // Empty scaler item with no body header:
        // 11.x style.
        let mut scalers = Vec::<u32>::new();
        let t = SystemTime::now();
        let item = ScalerItem::new(None, 0, 10, t, 1, true, None, &mut scalers);

        let raw = item.to_raw();
        assert_eq!(PERIODIC_SCALERS, raw.type_id());
        assert!(!raw.has_body_header());

        let p = raw.payload().as_slice();
        let mut offset = 0;
        assert_eq!(
            0,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            systime_to_raw(item.get_absolute_time()),
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            0, // there are no scalers.
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        // V11 has no original sid so with no scalers that's the end of
        // the item .

        offset += size_of::<u32>();
        assert_eq!(offset, p.len());
    }
    #[test]
    fn to_raw_2() {
        // body header empty, v11

        let mut scalers = Vec::<u32>::new();
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0x123456789,
            source_id: 2,
            barrier_type: 0,
        };
        let item = ScalerItem::new(Some(bh), 0, 10, t, 1, true, None, &mut scalers);

        let raw = item.to_raw();
        assert!(raw.has_body_header());
        let bhr = raw.get_bodyheader().unwrap(); //Ok since it has one.
        assert_eq!(bh.timestamp, bhr.timestamp);
        assert_eq!(bh.source_id, bhr.source_id);
        assert_eq!(bh.barrier_type, bhr.barrier_type);

        let p = raw.payload().as_slice();
        let mut offset = body_header_size();

        assert_eq!(
            0,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            systime_to_raw(item.get_absolute_time()),
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            0, // there are no scalers.
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        // V11 has no original sid so with no scalers that's the end of
        // the item .

        offset += size_of::<u32>();
        assert_eq!(offset, p.len());
    }
    #[test]
    fn to_raw_3() {
        // No scalers, body header and v 12 style

        let mut scalers = Vec::<u32>::new();
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0x123456789,
            source_id: 2,
            barrier_type: 0,
        };
        let item = ScalerItem::new(Some(bh), 0, 10, t, 1, true, Some(5), &mut scalers);

        let raw = item.to_raw();
        assert!(raw.has_body_header());
        let bhr = raw.get_bodyheader().unwrap(); //Ok since it has one.
        assert_eq!(bh.timestamp, bhr.timestamp);
        assert_eq!(bh.source_id, bhr.source_id);
        assert_eq!(bh.barrier_type, bhr.barrier_type);

        let p = raw.payload().as_slice();
        let mut offset = body_header_size();

        assert_eq!(
            0,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            systime_to_raw(item.get_absolute_time()),
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            0, // there are no scalers.
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );

        // V12 has an original sid here:

        offset += size_of::<u32>();
        assert_eq!(
            5,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );

        // should be nothing more:

        offset += size_of::<u32>();
        assert_eq!(offset, p.len());
    }
    #[test]
    fn to_raw_4() {
        // no body header v11, some scalers:

        let mut scalers: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
        let t = SystemTime::now();
        let item = ScalerItem::new(None, 0, 10, t, 1, true, None, &mut scalers);

        let raw = item.to_raw();

        let p = raw.payload().as_slice();
        let mut offset = 0;
        assert_eq!(
            0,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            systime_to_raw(item.get_absolute_time()),
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            6, // there are 6 scalers:
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );

        // scaler values:

        offset += size_of::<u32>(); // First scaler offset:

        for i in 0..6 {
            let expected: u32 = i + 1;
            assert_eq!(
                expected,
                u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
            );
            offset += size_of::<u32>();
        }
        // offset should now be off the end so:

        assert_eq!(offset, p.len());
    }
    #[test]
    fn to_raw_5() {
        // body header, scalers and v11:

        let mut scalers: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0xacdef0123456789,
            source_id: 1,
            barrier_type: 0,
        };
        let item = ScalerItem::new(Some(bh), 0, 10, t, 1, true, None, &mut scalers);

        let raw = item.to_raw();

        let p = raw.payload().as_slice();
        let mut offset = body_header_size(); // all starts after bh.
        assert_eq!(
            0,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            systime_to_raw(item.get_absolute_time()),
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            6, // there are 6 scalers:
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );

        // scaler values:

        offset += size_of::<u32>(); // First scaler offset:

        for i in 0..6 {
            let expected: u32 = i + 1;
            assert_eq!(
                expected,
                u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
            );
            offset += size_of::<u32>();
        }
        // offset should now be off the end so:

        assert_eq!(offset, p.len());
    }
    #[test]
    fn to_raw_6() {
        // body header, v12, contents:

        let mut scalers: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0xacdef0123456789,
            source_id: 1,
            barrier_type: 0,
        };
        let item = ScalerItem::new(Some(bh), 0, 10, t, 1, true, Some(5), &mut scalers);

        let raw = item.to_raw();

        let p = raw.payload().as_slice();
        let mut offset = body_header_size(); // all starts after bh.
        assert_eq!(
            0,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            systime_to_raw(item.get_absolute_time()),
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            6, // there are 6 scalers:
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );

        // osid:

        offset += size_of::<u32>();
        assert_eq!(
            5,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );

        // scaler values:

        offset += size_of::<u32>(); // First scaler offset:

        for i in 0..6 {
            let expected: u32 = i + 1;
            assert_eq!(
                expected,
                u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
            );
            offset += size_of::<u32>();
        }
        // offset should now be off the end so:

        assert_eq!(offset, p.len());
    }
    // We know we can now use to_raw to generate raw items for
    // from_raw:

    #[test]
    fn from_raw_1() {
        // no body header, v11, no scalers:

        let mut scalers = Vec::<u32>::new();
        let t = SystemTime::now();
        let item = ScalerItem::new(None, 0, 10, t, 1, true, None, &mut scalers);

        let raw = item.to_raw();
        let recons = ScalerItem::from_raw(&raw, RingVersion::V11);
        assert!(recons.is_some());
        let recons = recons.unwrap(); // The scaler item itself:

        assert!(recons.get_body_header().is_none());
        assert_eq!(0, recons.get_start_offset());
        assert_eq!(10, recons.get_end_offset());
        assert_eq!(
            systime_to_raw(t),
            systime_to_raw(recons.get_absolute_time())
        );
        assert!(recons.is_incremental());
        assert!(recons.original_sid().is_none());
        assert_eq!(0, recons.len());
    }
    #[test]
    fn from_raw_2() {
        // body header, v11, no scalers:

        let mut scalers = Vec::<u32>::new();
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0x123456789,
            source_id: 2,
            barrier_type: 0,
        };
        let item = ScalerItem::new(Some(bh), 0, 10, t, 1, true, None, &mut scalers);

        let raw = item.to_raw();
        let recons = ScalerItem::from_raw(&raw, RingVersion::V11);
        assert!(recons.is_some());
        let recons = recons.unwrap();

        assert!(recons.get_body_header().is_some());
        let rbh = recons.get_body_header().unwrap();
        assert_eq!(bh.timestamp, rbh.timestamp);
        assert_eq!(bh.source_id, rbh.source_id);
        assert_eq!(bh.barrier_type, rbh.barrier_type);

        assert_eq!(0, recons.get_start_offset());
        assert_eq!(10, recons.get_end_offset());
        assert_eq!(
            systime_to_raw(t),
            systime_to_raw(recons.get_absolute_time())
        );
        assert!(recons.is_incremental());
        assert!(recons.original_sid().is_none());
        assert_eq!(0, recons.len());
    }
    #[test]
    fn from_raw_3() {
        // no scalers, body header and v12 version:

        let mut scalers = Vec::<u32>::new();
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0x123456789,
            source_id: 2,
            barrier_type: 0,
        };
        let item = ScalerItem::new(Some(bh), 0, 10, t, 1, true, Some(5), &mut scalers);

        let raw = item.to_raw();
        let recons = ScalerItem::from_raw(&raw, RingVersion::V12);
        assert!(recons.is_some());
        let recons = recons.unwrap();

        assert!(recons.get_body_header().is_some());
        let rbh = recons.get_body_header().unwrap();
        assert_eq!(bh.timestamp, rbh.timestamp);
        assert_eq!(bh.source_id, rbh.source_id);
        assert_eq!(bh.barrier_type, rbh.barrier_type);

        assert_eq!(0, recons.get_start_offset());
        assert_eq!(10, recons.get_end_offset());
        assert_eq!(
            systime_to_raw(t),
            systime_to_raw(recons.get_absolute_time())
        );
        assert!(recons.is_incremental());
        assert!(recons.original_sid().is_some());
        assert_eq!(5, recons.original_sid().unwrap());
        assert_eq!(0, recons.len());
    }
    #[test]
    fn from_raw_4() {
        // no body header, v11, some scalers:

        let scalers: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
        let t = SystemTime::now();
        let item = ScalerItem::new(None, 0, 10, t, 1, true, None, &mut scalers.clone());

        let raw = item.to_raw();
        let recons = ScalerItem::from_raw(&raw, RingVersion::V11);

        assert!(recons.is_some());
        let recons = recons.unwrap(); // The scaler item itself:

        assert!(recons.get_body_header().is_none());
        assert_eq!(0, recons.get_start_offset());
        assert_eq!(10, recons.get_end_offset());
        assert_eq!(
            systime_to_raw(t),
            systime_to_raw(recons.get_absolute_time())
        );
        assert!(recons.is_incremental());
        assert!(recons.original_sid().is_none());
        assert_eq!(6, recons.len()); // 6 scalers:
        for i in 0..recons.len() {
            assert_eq!(scalers[i], recons.get_scaler_values()[i]);
        }
    }
    #[test]
    fn from_raw_5() {
        // body header, scalers and v11:

        let scalers: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0xacdef0123456789,
            source_id: 1,
            barrier_type: 0,
        };
        let item = ScalerItem::new(Some(bh), 0, 10, t, 1, true, None, &mut scalers.clone());

        let raw = item.to_raw();

        let recons = ScalerItem::from_raw(&raw, RingVersion::V11);
        assert!(recons.is_some());
        let recons = recons.unwrap(); // The scaler item itself:

        assert!(recons.get_body_header().is_some());
        let rbh = recons.get_body_header().unwrap();
        assert_eq!(bh.timestamp, rbh.timestamp);
        assert_eq!(bh.source_id, rbh.source_id);
        assert_eq!(bh.barrier_type, rbh.barrier_type);

        assert_eq!(0, recons.get_start_offset());
        assert_eq!(10, recons.get_end_offset());
        assert_eq!(
            systime_to_raw(t),
            systime_to_raw(recons.get_absolute_time())
        );
        assert!(recons.is_incremental());
        assert!(recons.original_sid().is_none());
        assert_eq!(6, recons.len()); // 6 scalers:
        for i in 0..recons.len() {
            assert_eq!(scalers[i], recons.get_scaler_values()[i]);
        }
    }
    #[test]
    fn from_raw_6() {
        let scalers: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0xacdef0123456789,
            source_id: 1,
            barrier_type: 0,
        };
        let item = ScalerItem::new(Some(bh), 0, 10, t, 1, true, Some(5), &mut scalers.clone());

        let raw = item.to_raw();
        let recons = ScalerItem::from_raw(&raw, RingVersion::V12);

        assert!(recons.is_some());
        let recons = recons.unwrap(); // The scaler item itself:

        assert!(recons.get_body_header().is_some());
        let rbh = recons.get_body_header().unwrap();
        assert_eq!(bh.timestamp, rbh.timestamp);
        assert_eq!(bh.source_id, rbh.source_id);
        assert_eq!(bh.barrier_type, rbh.barrier_type);

        assert_eq!(0, recons.get_start_offset());
        assert_eq!(10, recons.get_end_offset());
        assert_eq!(
            systime_to_raw(t),
            systime_to_raw(recons.get_absolute_time())
        );
        assert!(recons.is_incremental());
        assert!(recons.original_sid().is_some());
        assert_eq!(5, recons.original_sid().unwrap());
        assert_eq!(6, recons.len()); // 6 scalers:
        for i in 0..recons.len() {
            assert_eq!(scalers[i], recons.get_scaler_values()[i]);
        }
    }
    #[test]
    fn from_raw_7() {
        // Give none if the type is wrong:

        let raw = RingItem::new(PERIODIC_SCALERS + 1); // bad type.
        assert!(ScalerItem::from_raw(&raw, RingVersion::V11).is_none());
        assert!(ScalerItem::from_raw(&raw, RingVersion::V12).is_none());
    }
}
