use crate::ring_items;
use std::mem;
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
    pub fn scaler_values(&mut self, scalers: &mut Vec<u32>) {
        scalers.append(&mut self.scalers);
    }

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
            let offset : usize = if let Some(_b) = body_header {
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

        result.add(self.start_offset);
        result.add(self.end_offset);
        result.add(ring_items::systime_to_raw(self.absolute_time));
        result.add(self.divisor);
        result.add(self.scalers.len() as u32);
        result.add(self.is_incremental);
        if let Some(osid) = self.original_sid {
            result.add(osid);
        }
        for sc in &self.scalers {
            result.add(*sc);
        }

        result
    }
}
