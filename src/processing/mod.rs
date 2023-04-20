//!  This module is reponsible for processing event data
//!  from the data source.
//!  It provides the following:
//!
//!  -  An API to interact with the processing thread.
//!  -  A processing thread.
//!  -  Request/response messaging structures that
//! support the API.
//!  
//! The API itself supports the following operations:
//!  
//! - Start the thread.
//! - Stop the thread.
//! - Set the thread event aggregation size.
//! - Attach the thread to a data source.
//! - Start processing from the data source.
//! - Stop processing data from the data source.
//!
//!
//!  When processing starts, if there are parameter description
//!  records, the processing thread creats a map between the
//!  parameter ids in the data and parameter ids known to the
//!  histograming thread.  If new parameters are encounterd in
//!  the data stream, they are created in the processing thread
//!  and added to that map.
//!  
//!  When paramter data records are encounted, the map is used
//!  to construct events (id/value pairs) from that and
//!  those events are then blocked up and sent to the
//!  histogramer from processing.
//!
use crate::messaging;
use crate::messaging::spectrum_messages;
use std::sync::mpsc;

// The request/reply structs are private:

enum Request {}
enum Reply {}

// for now stubs:

/// We'll need an API object so that we can hold
/// the channel we'll use to talk with it:
///

pub struct ProcessingApi {
    spectrum_api: spectrum_messages::SpectrumMessageClient,
    req_chan: mpsc::Sender<Request>,
    rcv_chan: Option<mpsc::Receiver<Request>>,
}
// For now a stub interface.
//
impl ProcessingApi {
    pub fn new(chan: &mpsc::Sender<messaging::Request>) -> ProcessingApi {
        let (send, recv) = mpsc::channel();
        ProcessingApi {
            spectrum_api: spectrum_messages::SpectrumMessageClient::new(chan),
            req_chan: send,
            rcv_chan: Some(recv),
        }
    }
    pub fn start_thread(&self) -> Result<(), String> {
        Ok(())
    }
    pub fn stop_thread(&self) -> Result<(), String> {
        Ok(())
    }
    pub fn attach(&self, source: &str) -> Result<(), String> {
        Ok(())
    }
    pub fn detach(&self) -> Result<(), String> {
        Ok(())
    }
    pub fn set_batching(&self, events: usize) -> Result<(), String> {
        Ok(())
    }
    pub fn start_analysis(&self) -> Result<(), String> {
        Ok(())
    }
    pub fn stop_analysis(&self) -> Result<(), String> {
        Ok(())
    }
}
