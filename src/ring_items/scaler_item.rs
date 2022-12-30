use crate::ring_items;
use std::mem;
use std::ops::Add;
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
}
