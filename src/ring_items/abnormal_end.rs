use crate::ring_items;

///
/// Abnormal ends are empty actually.
///
pub struct AbnormalEnd {}

impl AbnormalEnd {
    pub fn new() -> AbnormalEnd {
        AbnormalEnd {}
    }
    pub fn from_raw(item: &ring_items::RingItem) -> Option<AbnormalEnd> {
        if item.type_id() == ring_items::ABNORMAL_END {
            Some(Self::new())
        } else {
            None
        }
    }
    pub fn to_raw(&self) -> ring_items::RingItem {
        ring_items::RingItem::new(ring_items::ABNORMAL_END)
    }
}

//------------------------------------------------------------------
// unit tests
//
#[cfg(test)]
mod tests {
    use crate::abnormal_end::AbnormalEnd;
    use crate::ring_items::RingItem;
    use std::mem::size_of;
    #[test]
    fn fromraw_1() {
        let raw = RingItem::new(crate::ring_items::ABNORMAL_END);
        assert!(AbnormalEnd::from_raw(&raw).is_some());
    }
    #[test]
    fn fromraw_2() {
        let raw = RingItem::new(crate::ring_items::BEGIN_RUN);
        assert!(AbnormalEnd::from_raw(&raw).is_none());
    }
    #[test]
    fn toraw_1() {
        let end = AbnormalEnd::new();
        let raw = end.to_raw();
        assert_eq!(crate::ring_items::ABNORMAL_END, raw.type_id());
        assert!(!raw.has_body_header());
        assert_eq!(3 * size_of::<u32>() as u32, raw.size());
    }
}
