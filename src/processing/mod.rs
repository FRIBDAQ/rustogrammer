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
use crate::messaging::parameter_messages;
use crate::messaging::spectrum_messages;
use crate::parameters;
use std::fs;
use std::fs::File;
use std::sync::mpsc;
use std::thread;

const DEFAULT_EVENT_CHUNKSIZE: usize = 100;

pub enum RequestType {
    Attach(String),   // Attach this file.
    Detach,           // Stop analyzing and close source
    Start,            // Start analyzing source
    Stop,             // Stop analyzing, keep file open.
    ChunkSize(usize), // Set # events per request to Histogramer
    Exit,             // Exit thread (mostly for testing).
    List,
}
pub struct Request {
    reply_chan: mpsc::Sender<Reply>,
    request: RequestType,
}

pub type Reply = Result<String, String>;

// for now stubs:

/// We'll need an API object so that we can hold
/// the channel we'll use to talk with it:
///
pub struct ProcessingApi {
    spectrum_api: spectrum_messages::SpectrumMessageClient,
    req_chan: mpsc::Sender<Request>,
}

impl ProcessingApi {
    // Utility for communicating with the thread:

    fn transaction(&self, req: RequestType) -> Result<String, String> {
        let (rep_send, rep_recv) = mpsc::channel();
        let request = Request {
            reply_chan: rep_send,
            request: req,
        };
        self.req_chan
            .send(request)
            .expect("Failed send to read thread");
        rep_recv.recv().expect("Failed read from read thread")
    }

    /// Note that theoretically this allows more than one
    /// event file to be processed at the same time,  however
    /// rustogrammer only actually creates one of these.

    pub fn new(chan: &mpsc::Sender<messaging::Request>) -> ProcessingApi {
        let (send, recv) = mpsc::channel();
        let api_chan = chan.clone();
        thread::spawn(move || processing_thread(recv, api_chan));
        ProcessingApi {
            spectrum_api: spectrum_messages::SpectrumMessageClient::new(chan),
            req_chan: send,
        }
    }

    pub fn stop_thread(&self) -> Result<String, String> {
        self.transaction(RequestType::Exit)
    }
    pub fn attach(&self, source: &str) -> Result<String, String> {
        self.transaction(RequestType::Attach(String::from(source)))
    }
    pub fn detach(&self) -> Result<String, String> {
        self.transaction(RequestType::Detach)
    }
    pub fn set_batching(&self, events: usize) -> Result<String, String> {
        self.transaction(RequestType::ChunkSize(events))
    }
    pub fn start_analysis(&self) -> Result<String, String> {
        self.transaction(RequestType::Start)
    }
    pub fn stop_analysis(&self) -> Result<String, String> {
        self.transaction(RequestType::Stop)
    }
    pub fn list(&self) -> Result<String, String> {
        self.transaction(RequestType::List)
    }
}
/// The processing thread requires state that's held across
/// several functions.  That implies a struct and implementation.
///
/// * request_chan is the recevier on which we'll process requests.
/// * spectrum_api is used to communicate with the histogram server's
/// spectrum interface.
/// * parameter_api is used to communicate with the histogram server's
/// parameter api.
/// * attach_name - contains the name of the data source. None indicates we're not attached.
/// * attached_file - contains the file descriptor of the file we're attached
/// None indicates we are not attached.
/// * parameter_mapping is a mapping between the parameter ids in the
/// histogram server's parameter dictionary and the ones in the event file.
/// this will be regenerated on each attach since it's possible that
/// these mappings change from file to file.
/// * chunk_size is the number of events that are batched together
/// in calls to spectrum_api.process_events.
/// * processing means that we are analyzing data from a file.
/// * keep_running - when an exit request is received, this is
/// set to false indicating that when convenienct the thread should
/// cleanly exit.
///
struct ProcessingThread {
    request_chan: mpsc::Receiver<Request>,

    spectrum_api: spectrum_messages::SpectrumMessageClient,
    parameter_api: parameter_messages::ParameterMessageClient,

    attach_name: Option<String>,
    attached_file: Option<fs::File>,
    parameter_mapping: parameters::ParameterIdMap,
    chunk_size: usize,
    processing: bool,
    keep_running: bool,
}
impl ProcessingThread {
    // Handle the Attach request:
    // Attempt to open the file.  If it exists,
    // store the attached fil and attached name as some.
    // additionaly, set processing -> false in order to
    // halt processing of the old file...if it was in progress.
    // On error, return that as the error string:
    //
    fn attach(&mut self, fname: &str) -> Reply {
        match File::open(fname) {
            Ok(fp) => {
                self.attach_name = Some(String::from(fname));
                self.attached_file = Some(fp);
                self.processing = false;
                Ok(String::from(""))
            }
            Err(e) => Err(e.to_string()),
        }
    }
    // Implement the List request - this is always
    // successful
    // If attach_name is Some, return its contents
    // If attach_name is None return "Not Attached"

    fn list(&mut self) -> Reply {
        if let Some(s) = &self.attach_name {
            Ok(s.clone())
        } else {
            Ok(String::from("Not Attached"))
        }
    }

    // Process any request received from other threads:

    fn process_request(&mut self, request: Request) {
        let reply = match request.request {
            RequestType::Attach(fname) => self.attach(&fname),
            RequestType::Detach => Ok(String::from("")),
            RequestType::Start => Ok(String::from("")),
            RequestType::Stop => Ok(String::from("")),
            RequestType::ChunkSize(n) => {
                self.chunk_size = n;
                Ok(String::from(""))
            }
            RequestType::Exit => {
                self.keep_running = false;
                Ok(String::from(""))
            }
            RequestType::List => self.list(),
        };
        request
            .reply_chan
            .send(reply)
            .expect("ProcessingThread failed to send reply to request");
    }

    /// Create a new processing thread.
    ///
    /// * req_chan is the channel on which we will accept new requests.
    /// * api_chan is the channel on which we send request to the histogram
    /// server it is used to create the API objects for the spectrum
    /// and event interfaces that we need.
    ///
    pub fn new(
        req_chan: mpsc::Receiver<Request>,
        api_chan: mpsc::Sender<messaging::Request>,
    ) -> ProcessingThread {
        ProcessingThread {
            request_chan: req_chan,
            spectrum_api: spectrum_messages::SpectrumMessageClient::new(&api_chan),
            parameter_api: parameter_messages::ParameterMessageClient::new(&api_chan),
            attach_name: None,
            attached_file: None,
            parameter_mapping: parameters::ParameterIdMap::new(),
            chunk_size: DEFAULT_EVENT_CHUNKSIZE,
            processing: false,
            keep_running: true,
        }
    }
    /// run the thread.
    /// So the idea is that there's a thread starting function
    /// that does the new and then invokes this method on the
    /// constructed object.

    pub fn run(&mut self) {
        while self.keep_running {
            let request = self.request_chan.recv();
            if request.is_err() {
                break;
            }
            let request = request.unwrap();
            self.process_request(request);
        }
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
fn processing_thread(req: mpsc::Receiver<Request>, api_chan: mpsc::Sender<messaging::Request>) {
    let mut thread = ProcessingThread::new(req, api_chan);
    thread.run();
}
