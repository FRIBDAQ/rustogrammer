use crate::ring_items;
///
/// Provides a format item and interface:
/// Format items provide the ring buffer format level.
/// and were introduced in NSCLDAQ-11.x
///
pub struct FormatItem {
    type_id: u32,
    major: u16,
    minor: u16,
}

impl FormatItem {
    pub fn type_id(&self) -> u32 {
        self.type_id
    }
    pub fn major(&self) -> u16 {
        self.major
    }
    pub fn minor(&self) -> u16 {
        self.minor
    }
    pub fn new(major: u16, minor: u16) -> FormatItem {
        FormatItem {
            type_id: ring_items::FORMAT_ITEM,
            major: major,
            minor: minor,
        }
    }
    pub fn from_raw(raw: &ring_items::RingItem) -> Option<Self> {
        if raw.type_id() != ring_items::FORMAT_ITEM {
            return None;
        }
        let mut result = FormatItem::new(0, 0);
        let payload = raw.payload();

        // The first u16 is the major, the second u16 is the
        // minor:

        result.major = u16::from_ne_bytes(payload.as_slice()[0..2].try_into().unwrap());
        result.minor = u16::from_ne_bytes(payload.as_slice()[2..4].try_into().unwrap());
        Some(result)
    }
    pub fn to_raw(&self) -> ring_items::RingItem {
        let mut result = ring_items::RingItem::new(ring_items::FORMAT_ITEM);
        result.add(self.major);
        result.add(self.minor);
        result
    }
}
