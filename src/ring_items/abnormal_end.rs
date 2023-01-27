use crate::ring_items;
use std::fmt;
///
/// Abnormal ends are empty actually.
///
pub struct AbnormalEnd {}

impl AbnormalEnd {
    pub fn new() -> AbnormalEnd {
        AbnormalEnd {}
    }
}
impl fmt::Display for AbnormalEnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Abnormal end Item")
    }
}
/// ToRaw trait allows the abnormal end items to convert to
/// ring_items::RingItem  This cannot fail:

impl ring_items::ToRaw for AbnormalEnd {
    fn to_raw(&self) -> ring_items::RingItem {
        ring_items::RingItem::new(ring_items::ABNORMAL_END)
    }
}
impl ring_items::FromRaw<AbnormalEnd> for ring_items::RingItem {
    fn to_specific(
        self: &ring_items::RingItem,
        _v: ring_items::RingVersion,
    ) -> Option<AbnormalEnd> {
        if self.type_id() == ring_items::ABNORMAL_END {
            Some(AbnormalEnd::new())
        } else {
            None
        }
    }
}

//------------------------------------------------------------------
// unit tests
//
#[cfg(test)]
mod abend_tests {
    use crate::ring_items::*;
    use abnormal_end::*;
    use std::mem::size_of;
    #[test]
    fn fromraw_1() {
        let raw = RingItem::new(crate::ring_items::ABNORMAL_END);
        let result: Option<AbnormalEnd> = raw.to_specific(RingVersion::V11);
        assert!(result.is_some());
    }
    #[test]
    fn fromraw_2() {
        let raw = RingItem::new(crate::ring_items::BEGIN_RUN);
        let result: Option<AbnormalEnd> = raw.to_specific(RingVersion::V11);
        assert!(result.is_none());
    }
    #[test]
    fn toraw_1() {
        let end = abnormal_end::AbnormalEnd::new();
        let raw = end.to_raw();
        assert_eq!(crate::ring_items::ABNORMAL_END, raw.type_id());
        assert!(!raw.has_body_header());
        assert_eq!(3 * size_of::<u32>() as u32, raw.size());
    }
}
