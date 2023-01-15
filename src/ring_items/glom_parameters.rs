use crate::ring_items;

/// These are the strategies glom uses to assign timestamps to events:
///
#[derive(Clone, Copy)]
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
    // conversions:

    pub fn from_raw(raw: &ring_items::RingItem) -> Option<GlomParameters> {
        if raw.type_id() == ring_items::GLOM_INFO {
            let mut result = Self::new(0, true, TimestampPolicy::First);
            let payload = raw.payload().as_slice();

            result.coincidence_ticks = u64::from_ne_bytes(payload[0..8].try_into().unwrap());
            result.is_building = u16::from_ne_bytes(payload[8..10].try_into().unwrap()) != 0;
            if let Some(policy) =
                Self::policy_from_code(u16::from_ne_bytes(payload[10..12].try_into().unwrap()))
            {
                result.timestamp_policy = policy;
                Some(result)
            } else {
                None
            }
        } else {
            None
        }
    }
    pub fn to_raw(&self) -> ring_items::RingItem {
        let mut result = ring_items::RingItem::new(ring_items::GLOM_INFO);

        let building: u16 = if self.is_building { 1 } else { 0 };
        let policy = Self::policy_to_code;
        result.add(self.coincidence_ticks).add(building).add(policy);

        result
    }
}