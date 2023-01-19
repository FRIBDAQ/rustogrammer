use crate::ring_items;
///
/// Provides a format item and interface:
/// Format items provide the ring buffer format level.
/// and were introduced in NSCLDAQ-11.x
///
pub struct FormatItem {
    major: u16,
    minor: u16,
}

impl FormatItem {
    pub fn major(&self) -> u16 {
        self.major
    }
    pub fn minor(&self) -> u16 {
        self.minor
    }
    pub fn new(major: u16, minor: u16) -> FormatItem {
        FormatItem {
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
#[cfg(test)]
mod fmt_tests {
    use crate::format_item::*;
    use crate::ring_items::*;
    use std::mem::size_of;
    #[test]
    fn new_1() {
        // Also tests getters.

        let item = FormatItem::new(11, 5);
        assert_eq!(11, item.major());
        assert_eq!(5, item.minor());
    }
    #[test]
    fn to_raw_1() {
        let item = FormatItem::new(11, 26);
        let raw = item.to_raw();

        assert_eq!(FORMAT_ITEM, raw.type_id());
        assert!(!raw.has_body_header());
        let p = raw.payload().as_slice();
        assert_eq!(
            11,
            u16::from_ne_bytes(p[0..size_of::<u16>()].try_into().unwrap())
        );
        assert_eq!(
            26,
            u16::from_ne_bytes(
                p[size_of::<u16>()..2 * size_of::<u16>()]
                    .try_into()
                    .unwrap()
            )
        );
    }
    #[test]
    fn from_raw_1() {
        let item = FormatItem::new(11, 26);
        let raw = item.to_raw();
        let recons = FormatItem::from_raw(&raw);

        assert!(recons.is_some());
        let recons = recons.unwrap();
        assert_eq!(11, recons.major());
        assert_eq!(26, recons.minor());
    }
    #[test]
    fn from_raw_2() {
        let raw = RingItem::new(FORMAT_ITEM + 1); // should fail.
        assert!(FormatItem::from_raw(&raw).is_none());
    }
}
