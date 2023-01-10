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
