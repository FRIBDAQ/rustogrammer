///
/// provide support for state change items.
/// these are actually four different item types.
use std::time;

/// Types of run state transitions in rustly form:

pub enum StateChangeType {
    Begin,
    End,
    Pause,
    Resume,
}
/// The rustly form of a state change item:
///
pub struct StateChange {
    change_type: StateChangeType,
    has_body_header: bool,
    body_header: ring_items::BodyHeader, // only valid if has_body_header true
    run_number: u32,
    time_offset: u32,
    offset_divisor: u32,
    absolute_time: time::SystemTime,
    run_title: String,
}


impl StateChangeType {
    pub fn new(
        type : StateChangeType, run : u32, offset : u32, divisor : u32, 
        title: &str) -> StateChange {
        StateChange {
            change_type : type,
            has_body_header : false,
            body_header : {
                timestamp: 0,
                source_id :  0,
                barrier_type : 0
            },
            run_number : run,
            time_offset : offset,
            offset_divisor : divisor,
            absolute_time  : time::SystemTime::now(),
            run_title : String::from(title)
        }
    }
    pub fn new_with_body_header(
        type : StateChangeType, 
        evb_timestamp: u64, source_id : u32, barrier_type : u32,
        run : u32, offset : u32, divisor : u32, title: &str
    ) -> StateChange {
        StateChange {
            change_tpye : type,
            has_body_header : true,
            body_header {
                timestamp: evb_timestamp, 
                source_id :  source_id,
                barrier_type : barrier_type
            },
            run_number : run,
            time_offset : offset,
            offset_divisor : divisor,
            absolute_time  : time::SystemTime::now(),
            run_title : String::from(title)
        }
    }
}