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
use crate::ring_items;
use crate::ring_items::*;
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
    // Implement detach -
    // If we are attached (attach name is Some),
    // -  Set the attach name and file to none.
    // -  set processing -> false.
    // -  return an Ok
    // else return an error (not attached).
    //
    fn detach(&mut self) -> Reply {
        if self.attach_name.is_some() {
            self.attach_name = None;
            self.attached_file = None;
            self.processing = false;
            Ok(String::from(""))
        } else {
            Err(String::from("Not attached to a data source"))
        }
    }

    // Process any request received from other threads:

    fn process_request(&mut self, request: Request) {
        let reply = match request.request {
            RequestType::Attach(fname) => self.attach(&fname),
            RequestType::Detach => self.detach(),
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
    //  given a new set of parameter definitions, rebuild the parameter
    // map
    // - ask the parameter api to list the parameter.
    // - use those to stock the parameter dictionary of them ap.
    // - Use the parameter definitions in the parameter definition item:
    //   *  If the parameter does not exist in the dictionary,
    // define it to the histograme  Make the 1:1 map in our dictionary.
    //   * If the parameter does exist in the dictionary, make a map
    // from its id in the record to the id in the histogramer.
    //
    fn rebuild_parameter_map(&mut self, defs: &analysis_ring_items::ParameterDefinitions) {
        self.parameter_mapping = parameters::ParameterIdMap::new();
        let known_parameters = self
            .parameter_api
            .list_parameters("*")
            .expect("Could not get parameter defs from histogram thread");

        // Stock the map with the parameters the histogramer has defined:

        println!("Stocking existing params to the map");
        for p in known_parameters {
            self.parameter_mapping
                .get_dict_mut()
                .insert(p.get_name(), p.get_id());
        }
        // Iterate over the definitions in the parameter definition
        // item.  If making a map for a parameter fails, then
        // we need to add the parameter to the histogramer,
        // fetch its id and make an new map.
        // Duplicate mapping is cause for a panic.

        for def in defs.iter() {
            let name = def.name();
            let id = def.id();
            println!("Processing {} ({}) from the ring item", name, id);
            if let Err(reason) = self.parameter_mapping.map(id, &name) {
                if reason == String::from("Duplicate Map") {
                    panic!("ProcessingThread failed to make a map due to duplication");
                }
                println!("Need t omake new parameter {}", name);
                if let Err(s) = self.parameter_api.create_parameter(&name) {
                    panic!("Failed to create new parameter {} : {}", name, s);
                }
                // Get the id of the new parameter:

                let param = self.parameter_api.list_parameters(&name);
                if let Err(s) = param {
                    panic!(
                        "Just created parameter {} but failed to get its id: {}",
                        name, s
                    );
                }
                let param = param.unwrap();
                if param.len() == 0 {
                    panic!(
                        "Just made parameter {} but got an empty list fetching it def",
                        name
                    );
                }
                let param = &param[0];
                println!("Native id is {}", param.get_id());
                self.parameter_mapping
                    .get_dict_mut()
                    .insert(name.clone(), param.get_id());

                // If it's still an error then it's panic time:

                if let Err(reason) = self.parameter_mapping.map(id, &name) {
                    panic!(
                        "After creating parameter {}, failed to make map entry {}",
                        name, reason
                    );
                }
            }
            println!("Parameter map re-created");
        }
    }

    // Process a ring item from the file we only process
    // *  Parameter definition records - which cause us to
    // rebuild the parameterm ap.
    // *  Parameter value records which get processed into an event,
    // mapped to an event in the server's parameter space and
    // sent to the histogram thread (this version does not support
    // batching at this time).
    fn read_an_event(&mut self) {
        println!("Reading an event");
        if let Some(fp) = self.attached_file.as_mut() {
            let try_item = RingItem::read_item(fp);

            // Any error will be treated as an end

            if let Err(reason) = try_item {
                println!("Failed to read a ring item: {}", reason.to_string());

                // stop processing - flushing any partial batch.

                self.processing = false;
                return;
            }
            let item = try_item.unwrap();
            match item.type_id() {
                ring_items::PARAMETER_DEFINITIONS => {
                    println!("Parameter definition event");
                    let definitions: Option<analysis_ring_items::ParameterDefinitions> =
                        item.to_specific(RingVersion::V11);
                    if definitions.is_none() {
                        panic!("Converting a parameter definitions ring item failed!");
                    }
                    let definitions = definitions.unwrap();
                    self.rebuild_parameter_map(&definitions);
                }
                ring_items::PARAMETER_DATA => {}
                _ => {}
            };
        }
    }

    // This is the method that's used when processing a data file:
    // It gets entered from run when self.processing is true after
    // a request is processed.  It returns when:
    //   -  keep_running -> false.
    //   -  processing -> false.
    //   - and end file is encountered on the data source.
    //
    // We read items from the event file.
    //
    fn processing(&mut self) {
        let mut eof = false;
        while self.processing && self.keep_running && (!eof) {
            // If there are requests process them:

            match self.request_chan.try_recv() {
                Ok(r) => self.process_request(r),
                Err(why) => {
                    // if disconnected we exit:
                    if let mpsc::TryRecvError::Disconnected = why {
                        self.processing = false;
                        self.keep_running = false;
                        self.attached_file = None; // Closes any file.
                        self.attach_name = None;
                    } // Otherwise just means there's no request.
                }
            };
            // Request processing might have changed processing or keep_running so:
            // Gaurd event processing:

            if self.processing && self.keep_running {
                self.read_an_event();
            }
        }
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
            if self.processing {
                self.processing();
            }
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
