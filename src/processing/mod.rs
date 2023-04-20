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
//! - List the currently attached file.
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
use std::thread;

// The request/reply structs are private:

pub enum Request {}
pub enum Reply {}

// for now stubs:

/// We'll need an API object so that we can hold
/// the channel we'll use to talk with it:
///
pub struct ProcessingApi {
    spectrum_api: spectrum_messages::SpectrumMessageClient,
    req_chan: mpsc::Sender<Request>,
}

impl ProcessingApi {
    /// Note that theoretically this allows more than one
    /// event file to be processed at the same time,  however
    /// rustogrammer only actually creates one of these.

    pub fn new(chan: &mpsc::Sender<messaging::Request>) -> ProcessingApi {
        let (send, recv) = mpsc::channel();
        thread::spawn(move || processing_thread(recv));
        ProcessingApi {
            spectrum_api: spectrum_messages::SpectrumMessageClient::new(chan),
            req_chan: send,
        }
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
    pub fn list(&self) -> Result<String, String> {
        Ok(String::from("File: /some/file"))
    }
}

/// This private function is the file processing thread.
/// It has two states of operation:
///
/// - Not processing in which case it hangs blocking reads
/// its request channel responding to requests.
/// - Processing in which case it creates blocks of events
/// and tests the request channel (via try_recv) to see if there
/// are requests that need processing.
///
/// Transitions between states NP (non processing) P (processing)
///  are done by:
///  *  NP -> P a source is attaached and the start request is received.
///  *  P -> NP end of data, or a read error is encountered on a data source.
///  *  P -> NP between event batches, a stop request was received.
///  *  P -> NP between event batches an attach or detach request
/// is received.
///
fn processing_thread(req: mpsc::Receiver<Request>) {
    // we implement the not processing mode.

    loop {
        let request = req.recv();
        if request.is_err() {
            break;
        }
    }
}
