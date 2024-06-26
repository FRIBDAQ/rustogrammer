use crate::ring_items;
use std::fmt;

/// These are the strategies glom uses to assign timestamps to events:
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimestampPolicy {
    First,
    Last,
    Average,
}
/// In raw data these will be represented by:

const GLOM_TIMESTAMP_FIRST: u16 = 0;
const GLOM_TIMESTAMP_LAST: u16 = 1;
const GLOM_TIMESTAMP_AVERAGE: u16 = 2;

///
/// The GlomParameters struct documents the settings for the
/// event bulder.  These are emitted by the glom stage of the event builder
/// to document how it builds events and assigns timestamps to built events
/// from the fragments that make up each event.
///
pub struct GlomParameters {
    coincidence_ticks: u64,
    is_building: bool,
    timestamp_policy: TimestampPolicy,
}

impl GlomParameters {
    // Private utilities:

    fn policy_to_code(&self) -> u16 {
        match self.timestamp_policy {
            TimestampPolicy::First => GLOM_TIMESTAMP_FIRST,
            TimestampPolicy::Last => GLOM_TIMESTAMP_LAST,
            TimestampPolicy::Average => GLOM_TIMESTAMP_AVERAGE,
        }
    }
    fn policy_from_code(code: u16) -> Option<TimestampPolicy> {
        match code {
            GLOM_TIMESTAMP_FIRST => Some(TimestampPolicy::First),
            GLOM_TIMESTAMP_LAST => Some(TimestampPolicy::Last),
            GLOM_TIMESTAMP_AVERAGE => Some(TimestampPolicy::Average),
            _ => None,
        }
    }

    /// Construction:
    pub fn new(ticks: u64, building: bool, policy: TimestampPolicy) -> GlomParameters {
        GlomParameters {
            coincidence_ticks: ticks,
            is_building: building,
            timestamp_policy: policy,
        }
    }
    // Getters:

    pub fn get_coincidence_interval(&self) -> u64 {
        self.coincidence_ticks
    }
    pub fn is_building(&self) -> bool {
        self.is_building
    }
    pub fn get_ts_policy(&self) -> TimestampPolicy {
        self.timestamp_policy
    }
    pub fn policy_string(&self) -> String {
        match self.timestamp_policy {
            TimestampPolicy::First => String::from("First"),
            TimestampPolicy::Last => String::from("Last"),
            TimestampPolicy::Average => String::from("Averaged"),
        }
    }
}
impl fmt::Display for GlomParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Glom parameters").unwrap();
        let building = if self.is_building { "On" } else { "Off" };
        write!(
            f,
            "  Coincidence interval {} ticks, Event building is {} Timestamp Policy {}",
            self.coincidence_ticks,
            building,
            self.policy_string()
        )
    }
}
/// ToRaw provides failure free conversion from a specific item type
/// to a RingItem (generic type).

impl ring_items::ToRaw for GlomParameters {
    fn to_raw(&self) -> ring_items::RingItem {
        let mut result = ring_items::RingItem::new(ring_items::GLOM_INFO);

        let building: u16 = if self.is_building { 1 } else { 0 };
        let policy: u16 = self.policy_to_code();
        result.add(self.coincidence_ticks).add(building).add(policy);

        result
    }
}
/// FromRaw implementations of the generic allow attempts to convert
/// from a generic RingItem to a specific type (e.g. GlomParameters)

impl ring_items::FromRaw<GlomParameters> for ring_items::RingItem {
    fn to_specific(&self, _v: ring_items::RingVersion) -> Option<GlomParameters> {
        if self.type_id() == ring_items::GLOM_INFO {
            let mut result = GlomParameters::new(0, true, TimestampPolicy::First);
            let payload = self.payload().as_slice();

            result.coincidence_ticks = u64::from_ne_bytes(payload[0..8].try_into().unwrap());
            result.is_building = u16::from_ne_bytes(payload[8..10].try_into().unwrap()) != 0;
            if let Some(policy) = GlomParameters::policy_from_code(u16::from_ne_bytes(
                payload[10..12].try_into().unwrap(),
            )) {
                result.timestamp_policy = policy;
                Some(result)
            } else {
                None
            }
        } else {
            None
        }
    }
}
#[cfg(test)]
mod glom_tests {
    use super::*;
    use crate::ring_items::*;
    use std::mem::size_of;
    #[test]
    fn new_1() {
        let item = GlomParameters::new(1000, true, TimestampPolicy::First);
        assert_eq!(1000, item.coincidence_ticks);
        assert!(item.is_building);
        assert_eq!(TimestampPolicy::First, item.timestamp_policy);
    }
    #[test]
    fn getters_1() {
        let item = GlomParameters::new(1000, true, TimestampPolicy::First);
        assert_eq!(1000, item.get_coincidence_interval());
        assert!(item.is_building());
        assert_eq!(TimestampPolicy::First, item.get_ts_policy());
    }
    #[test]
    fn getters_2() {
        let item = GlomParameters::new(1000, true, TimestampPolicy::First);
        assert_eq!(String::from("First"), item.policy_string());

        let item = GlomParameters::new(1000, true, TimestampPolicy::Last);
        assert_eq!(String::from("Last"), item.policy_string());

        let item = GlomParameters::new(1000, true, TimestampPolicy::Average);
        assert_eq!(String::from("Averaged"), item.policy_string());
    }
    // Test for to_raw - so that we can use it to generate raw items to
    // test from_raw.

    fn to_raw_1() {
        let item = GlomParameters::new(1000, true, TimestampPolicy::Last);
        let raw = item.to_raw();

        assert_eq!(GLOM_INFO, raw.type_id());
        assert!(!raw.has_body_header());
        let p = raw.payload().as_slice();
        let mut offset = 0;
        assert_eq!(
            1000,
            u64::from_ne_bytes(p[offset..offset + size_of::<u64>()].try_into().unwrap())
        );
        offset += size_of::<u64>();
        assert_eq!(
            1,
            u16::from_ne_bytes(p[offset..offset + size_of::<u16>()].try_into().unwrap())
        );
        offset += size_of::<u16>();
        assert_eq!(
            GLOM_TIMESTAMP_LAST,
            u16::from_ne_bytes(p[offset..offset + size_of::<u16>()].try_into().unwrap())
        );
    }
    #[test]
    fn to_raw_2() {
        let item = GlomParameters::new(1000, true, TimestampPolicy::First);
        let raw = item.to_raw();

        assert_eq!(GLOM_INFO, raw.type_id());
        assert!(!raw.has_body_header());
        let p = raw.payload().as_slice();
        let mut offset = 0;
        assert_eq!(
            1000,
            u64::from_ne_bytes(p[offset..offset + size_of::<u64>()].try_into().unwrap())
        );
        offset += size_of::<u64>();
        assert_eq!(
            1,
            u16::from_ne_bytes(p[offset..offset + size_of::<u16>()].try_into().unwrap())
        );
        offset += size_of::<u16>();
        assert_eq!(
            GLOM_TIMESTAMP_FIRST,
            u16::from_ne_bytes(p[offset..offset + size_of::<u16>()].try_into().unwrap())
        );
    }
    #[test]
    fn to_raw_3() {
        let item = GlomParameters::new(1000, true, TimestampPolicy::Average);
        let raw = item.to_raw();

        assert_eq!(GLOM_INFO, raw.type_id());
        assert!(!raw.has_body_header());
        let p = raw.payload().as_slice();
        let mut offset = 0;
        assert_eq!(
            1000,
            u64::from_ne_bytes(p[offset..offset + size_of::<u64>()].try_into().unwrap())
        );
        offset += size_of::<u64>();
        assert_eq!(
            1,
            u16::from_ne_bytes(p[offset..offset + size_of::<u16>()].try_into().unwrap())
        );
        offset += size_of::<u16>();
        assert_eq!(
            GLOM_TIMESTAMP_AVERAGE,
            u16::from_ne_bytes(p[offset..offset + size_of::<u16>()].try_into().unwrap())
        );
    }
    // to_raw works we can use it to generate raw items for from_raw:

    #[test]
    fn from_raw_1() {
        let item = GlomParameters::new(1000, true, TimestampPolicy::First);
        let raw = item.to_raw();
        let back: Option<GlomParameters> = raw.to_specific(RingVersion::V11);
        assert!(back.is_some());
        let back = back.unwrap();

        assert_eq!(1000, back.get_coincidence_interval());
        assert!(back.is_building());
        assert_eq!(TimestampPolicy::First, back.get_ts_policy());
    }
    #[test]
    fn from_raw_2() {
        let item = GlomParameters::new(1000, true, TimestampPolicy::Last);
        let raw = item.to_raw();
        let back: Option<GlomParameters> = raw.to_specific(RingVersion::V11);
        assert!(back.is_some());
        let back = back.unwrap();

        assert_eq!(1000, back.get_coincidence_interval());
        assert!(back.is_building());
        assert_eq!(TimestampPolicy::Last, back.get_ts_policy());
    }
    #[test]
    fn from_raw_3() {
        let item = GlomParameters::new(1000, true, TimestampPolicy::Average);
        let raw = item.to_raw();
        let back: Option<GlomParameters> = raw.to_specific(RingVersion::V11);
        assert!(back.is_some());
        let back = back.unwrap();

        assert_eq!(1000, back.get_coincidence_interval());
        assert!(back.is_building());
        assert_eq!(TimestampPolicy::Average, back.get_ts_policy());
    }
    #[test]
    fn from_raw_4() {
        // invalid type -> None:

        let raw = RingItem::new(GLOM_INFO + 1);
        let bad: Option<GlomParameters> = raw.to_specific(RingVersion::V11);
        assert!(bad.is_none());
    }
}
