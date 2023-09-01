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
    GetChunkSize,     // Return chunksize.
    Exit,             // Exit thread (mostly for testing).
    List,
    Version(RingVersion),
    GetVersion,
}
pub struct Request {
    reply_chan: mpsc::Sender<Reply>,
    request: RequestType,
}

pub type Reply = Result<String, String>;

// for now stubs:

/// We'll need an API object so that we can hold
/// the channel we'll use to talk with it:
/// For now only one instance of this should be held and that's
/// in the REST state.
///  This is because chunk_size is cached.
#[derive(Clone)]
pub struct ProcessingApi {
    req_chan: mpsc::Sender<Request>,
}

impl ProcessingApi {
    // Utility for communicating with the thread:

    fn transaction(&self, req: RequestType) -> Reply {
        let (rep_send, rep_recv) = mpsc::channel();
        let request = Request {
            reply_chan: rep_send,
            request: req,
        };
        let result = self.req_chan.send(request);
        if result.is_err() {
            Err(String::from("Send to processing thread failed"))
        } else {
            let result = rep_recv.recv();
            if let Ok(result) = result {
                result
            } else {
                Err(String::from("Receive from processing thread failed"))
            }
        }
    }

    /// Note that theoretically this allows more than one
    /// event file to be processed at the same time,  however
    /// rustogrammer only actually creates one of these.

    pub fn new(chan: &mpsc::Sender<messaging::Request>) -> ProcessingApi {
        let (send, recv) = mpsc::channel();
        let api_chan = chan.clone();
        thread::spawn(move || processing_thread(recv, api_chan));
        ProcessingApi { req_chan: send }
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
    pub fn set_batching(&mut self, events: usize) -> Result<String, String> {
        self.transaction(RequestType::ChunkSize(events))
    }
    pub fn get_batching(&self) -> usize {
        let result = self.transaction(RequestType::GetChunkSize);
        if let Ok(s) = result {
            s.parse::<usize>().expect("Not a usize from get_batching")
        } else {
            panic!("Getting chunksize failed!");
        }
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
    pub fn set_ring_version(&self, version: RingVersion) -> Result<String, String> {
        self.transaction(RequestType::Version(version))
    }
    pub fn get_ring_version(&self) -> Result<RingVersion, String> {
        let raw_version = self.transaction(RequestType::GetVersion);
        match raw_version {
            Ok(str_version) => str_version.parse::<RingVersion>(),
            Err(s) => Err(s),
        }
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

    event_chunk: Vec<parameters::Event>,
    ring_version: RingVersion,
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
            Ok(String::from("file:") + s)
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
    // Start processing - if there's a data source just set
    // procesing true otherwise it's an error:

    fn start_processing(&mut self) -> Reply {
        if self.attach_name.is_none() {
            Err(String::from("No file is attached"))
        } else if self.processing {
            Err(format!(
                "Already processing {}",
                self.attach_name.as_ref().unwrap()
            ))
        } else {
            self.processing = true;
            Ok(String::from("Processing begins"))
        }
    }
    // Stop processing - if we're not processing this is an error.
    // Otherwise, set processing false and, when we return we'll stop.
    //
    fn stop_processing(&mut self) -> Reply {
        if self.processing {
            self.processing = false;
            Ok(String::from(""))
        } else {
            Err(String::from("Not processing data"))
        }
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
            if let Err(reason) = self.parameter_mapping.map(id, &name) {
                if reason == *"Duplicate Map" {
                    panic!("ProcessingThread failed to make a map due to duplication");
                }
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
                if param.is_empty() {
                    panic!(
                        "Just made parameter {} but got an empty list fetching it def",
                        name
                    );
                }
                let param = &param[0];
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
        }
    }
    // Build an event from a ParameterItem ring item:

    fn build_event(raw: &analysis_ring_items::ParameterItem) -> parameters::Event {
        let mut result = parameters::Event::new();
        for p in raw.iter() {
            result.push(parameters::EventParameter::new(p.id(), p.value()));
        }
        result
    }

    //
    // Flush the event batch to the histogramer:
    //
    fn flush_events(&mut self) {
        if self.event_chunk.is_empty() {
            if let Err(s) = self.spectrum_api.process_events(&self.event_chunk) {
                panic!("Unable to get the histogram thread to process events {}", s);
            }
            self.event_chunk.clear();
        }
    }
    // Process a ring item with event data.
    // We create an event from our ring item.
    // We ask the parameter map to create an event from it with the
    // parameter ids that are native to the histogramer.
    // For now we just send the event to the histogramer.
    // in a future implementation we'll send batches of events.
    //
    fn process_event(&mut self, event: &analysis_ring_items::ParameterItem) {
        let event = Self::build_event(event);
        let event = self.parameter_mapping.map_event(&event);

        self.event_chunk.push(event);
        if self.event_chunk.len() >= self.chunk_size {
            self.flush_events();
        }
    }

    // Process a ring item from the file we only process
    // *  Parameter definition records - which cause us to
    // rebuild the parameterm ap.
    // *  Parameter value records which get processed into an event,
    // mapped to an event in the server's parameter space and
    // sent to the histogram thread
    fn read_an_event(&mut self) -> bool {
        if let Some(fp) = self.attached_file.as_mut() {
            let try_item = RingItem::read_item(fp);

            // Any error will be treated as an end

            if let Err(reason) = try_item {
                println!("Failed to read a ring item: {}", reason.to_string());
                self.processing = false;
                return true;
            }
            let item = try_item.unwrap();
            match item.type_id() {
                ring_items::PARAMETER_DEFINITIONS => {
                    let definitions: Option<analysis_ring_items::ParameterDefinitions> =
                        item.to_specific(self.ring_version);
                    if definitions.is_none() {
                        panic!("Converting a parameter definitions ring item failed!");
                    }
                    let definitions = definitions.unwrap();
                    self.rebuild_parameter_map(&definitions);
                }
                ring_items::PARAMETER_DATA => {
                    let data: Option<analysis_ring_items::ParameterItem> =
                        item.to_specific(self.ring_version);
                    if data.is_none() {
                        panic!("Converting parameter encoded data from raw ring item failed!");
                    }
                    let event = data.unwrap();
                    self.process_event(&event);
                }
                _ => {} // Ignore all other ring item types.
            };
        }
        false
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
                eof = self.read_an_event();
            }
        }
        self.flush_events();
    }
    // Process any request received from other threads:

    fn process_request(&mut self, request: Request) {
        let reply = match request.request {
            RequestType::Attach(fname) => self.attach(&fname),
            RequestType::Detach => self.detach(),
            RequestType::Start => self.start_processing(),
            RequestType::Stop => self.stop_processing(),
            RequestType::ChunkSize(n) => {
                self.chunk_size = n;
                Ok(String::from(""))
            }
            RequestType::GetChunkSize => Ok(self.chunk_size.to_string()),
            RequestType::Exit => {
                self.keep_running = false;
                Ok(String::from(""))
            }
            RequestType::List => self.list(),
            RequestType::Version(v) => {
                self.ring_version = v;
                Ok(String::from(""))
            }
            RequestType::GetVersion => Ok(format!("{}", self.ring_version)),
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
            event_chunk: Vec::new(),
            ring_version: RingVersion::V11,
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
