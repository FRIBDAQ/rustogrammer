use crate::ring_items;
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
            let mut orsid: Option<u32> = None;
            let mut offset = offset; // new offset.
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
            .add(self.scalers.len() as u32)
            .add(self.is_incremental);
        if let Some(osid) = self.original_sid {
            result.add(osid);
        }
        for sc in &self.scalers {
            result.add(*sc);
        }

        result
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
    fn getters_1() {}
}
