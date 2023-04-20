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
use crate::messaging::spectrum_messaging;
use std::sync::mpsc;

// The request/reply structs are private:

enum Request {

}
enum Reply {

}


// for now stubs:

/// We'll need an API object so that we can hold
/// the channel we'll use to talk with it:
///

pub struct ProcessingApi {
    spectrum_api : spectrum_messaging::SpectrumMessageClient,
    req_chan     : Sender<Request>
}

impl ProcessingApi {
    
}


