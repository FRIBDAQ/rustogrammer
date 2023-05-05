//!  This module is in charge of maintaining the contents
//!  of spectrum bindings in an Xamine compatible shared memory
//!  region.
//!  We include:
//!  *  A thread that accepts and processes requests from clients
//! (Usually REST handlers)
//!  *  An API for properlty formatting requests and relaying
//! return values to the caller.
//!
//!  See BindingThread for more information about how bound spectra
//!  work.

use crate::messaging;
use crate::messaging::spectrum_messages;
use glob::Pattern;
use std::ops;
use std::sync::mpsc;
use std::thread;
use std::time;

/// Memory statistics have this format:
///
pub struct MemoryStatistics {
    pub free_bytes: usize,
    pub largest_free_bytes: usize,
    pub used_bytes: usize,
    pub largest_used_bytes: usize,
    pub bound_indices: usize,
    pub total_indices: usize,
    pub total_size: usize,
}
// This enum represents the set of operations that can be
// requested of this thread:

/// Requests are sent to the BindingThread via this enum.
/// The enum is private because it is instantiated by the
/// pub elements of the API.
///
enum RequestType {
    Unbind(String),
    UnbindAll,
    Bind(String),
    List(String),
    Clear(String),
    SetUpdate(u64),
    Statistics,
    ShmName,
    Exit,
}
pub struct Request {
    reply_chan: mpsc::Sender<Reply>,
    request: RequestType,
}

// Thread repies are just Result objects that are
// the sent back to the caller without any interpretation:

/// Most request return Generic results:

pub type GenericResult = Result<(), String>;
/// Listing the bindings returns a ListResult.  The
///  Ok Vector contains a doublet that has spectrum names
/// and their corresponding binding numbers.
///
pub type ListResult = Result<Vec<(usize, String)>, String>;

/// What we get back from statisitcs requests:
pub type StatisticsResult = Result<MemoryStatistics, String>;
/// When replies just need a string:'
pub type StringResult = Result<String, String>;

enum Reply {
    Generic(GenericResult),
    List(ListResult),
    Statistics(StatisticsResult),
    String(StringResult),
}

/// The default number of seconds to allow the receive on
/// requests to dwell:
/// The actual timeout can be set via the API.
const DEFAULT_TIMEOUT: u64 = 2;

/// This struct contains the state associated with the BindingThread
/// See the implementation for more information.
///
/// The binding thread does a timed out wait for messages on
/// its receive channel.  If it receives a message is processes it
/// and continues to process messages until the receive times out.
/// Once the receive times out, the thread runs a refresh pass.
/// The refresh pass uses the contents of all spectra to update the
/// data of those spectra in shared memory.
///
/// Note that the update passes just set the non-zero channels of the
/// bound spectra.  The full contents of the spectra are cleared both
/// when the spectrum is initially bound and when the thread is
/// asked to clear spectra.
///
/// We need to maintain the following information:
///
///  * timeout -    Our timeout in seconds (which is settable via e.g. the
/// REST interface).
///  * spectrum_api -  The Spectrum messaging API.
///  * request_chan - The channel on which requests will be sent.
///  * shm - the Xamine compatible shared memory segment.
///
struct BindingThread {
    request_chan: mpsc::Receiver<Request>,
    spectrum_api: spectrum_messages::SpectrumMessageClient,
    timeout: u64,
    shm: super::SharedMemory,
}

impl BindingThread {
    //Return Some(n) if there's a matching binding slot:
    // where n is the binding number:

    fn find_binding(&mut self, name: &str) -> Option<usize> {
        let bindings = self.shm.get_bindings();
        let is_found = bindings.iter().find(|x| x.1 == String::from(name));
        if let Some(x) = is_found {
            Some(x.0)
        } else {
            None
        }
    }
    // Unbind a spectrum from shared memory:

    fn unbind(&mut self, name: &str) -> Result<(), String> {
        if let Some(slot) = self.find_binding(name) {
            self.shm.unbind(slot);
            Ok(())
        } else {
            Err(String::from("Spectrum is not bound"))
        }
    }
    // Bind a spectrum to shared memory and fill it in:

    fn bind(&mut self, name: &str) -> Result<(), String> {
        if let Some(n) = self.find_binding(name) {
            return Err(format!("{} is already bound", name));
        }
        if let Ok(info) = self.spectrum_info(name) {
            match self.shm.bind_spectrum(
                name,
                Self::get_xaxis(&info).expect("No x axis!!!"),
                Self::get_yaxis(&info),
            ) {
                Ok((slot, _)) => {
                    self.shm.clear_contents(slot);
                    self.update_spectrum((slot, String::from(name)));
                    Ok(())
                }
                Err(s) => Err(s),
            }
        } else {
            Err(format!("Spectrum {} might not exist", name))
        }
    }
    // Get spectrum information given its name.  This returns a result
    // Ok means that the request worke and there was exactly one reponse
    // else ther's an error string.
    fn spectrum_info(
        &mut self,
        name: &str,
    ) -> Result<spectrum_messages::SpectrumProperties, String> {
        match self.spectrum_api.list_spectra(name) {
            spectrum_messages::SpectrumServerListingResult::Ok(spectra) => {
                if spectra.len() == 0 {
                    Err(format!("No such spectrum {}", name))
                } else if spectra.len() > 1 {
                    Err(format!("Ambiguous spectrum name {}", name))
                } else {
                    Ok(spectra[0].clone())
                }
            }
            spectrum_messages::SpectrumServerListingResult::Err(s) => Err(s),
        }
    }
    fn get_yaxis(info: &spectrum_messages::SpectrumProperties) -> Option<(f64, f64, u32)> {
        // This is just extracted fom th Y axis if it's there:

        if let Some(y) = info.yaxis {
            Some((y.low, y.high, y.bins))
        } else {
            None
        }
    }
    fn get_xaxis(info: &spectrum_messages::SpectrumProperties) -> Option<(f64, f64, u32)> {
        // Normally this will just be the X axis but for summary
        // spectra we constuct this from the number of parameters.

        if info.type_name != String::from("Summary") {
            if let Some(x) = info.xaxis {
                Some((x.low, x.high, x.bins))
            } else {
                None
            }
        } else {
            let len = info.xparams.len();
            Some((0.0, len as f64, len as u32))
        }
    }
    // Given a Option<AxisSpecification returns a triplet of low, high, size
    // Note that None gives (0.0, 1.0, 1)
    fn axis(a: Option<spectrum_messages::AxisSpecification>) -> (f64, f64, u32) {
        if let Some(ax) = a {
            (ax.low, ax.high, ax.bins)
        } else {
            (0.0, 1.0, 1)
        }
    }
    // Given a spectrum specification, return
    // (xlow,xhigh, ylow, yhigh).  If an axis does not exist, then 0,0
    // is placed instead.

    fn get_axes(info: &spectrum_messages::SpectrumProperties) -> (f64, f64, f64, f64) {
        let mut result = (0.0, 0.0, 0.0, 0.0);
        if let Some(xaxis) = info.xaxis {
            result.0 = xaxis.low;
            result.1 = xaxis.high;
        }
        if let Some(yaxis) = info.yaxis {
            result.2 = yaxis.low;
            result.3 = yaxis.high;
        }
        // Summary spectra have a y axis specification and the
        // X axis is determined by the number of x parameters.
        if info.type_name == String::from("Summary") {
            result.0 = 0.0;
            result.1 = info.xparams.len() as f64;
        }
        result
    }

    // Update a single spectrum's contents
    fn update_spectrum(&mut self, binding: (usize, String)) {
        let slot = binding.0;
        let name = binding.1;
        // Get the contents.   If that fails, we assume the spectrum
        // was deleted and get rid of the binding:

        if let Ok(info) = self.spectrum_info(&name) {
            let axis_spec = Self::get_axes(&info);
            if let Ok(contents) = self.spectrum_api.get_contents(
                &name,
                axis_spec.0,
                axis_spec.1,
                axis_spec.2,
                axis_spec.3,
            ) {
                self.shm.set_contents(slot, &contents);
            } else {
                self.shm.unbind(slot);
            }
        } else {
            self.shm.unbind(slot);
        }
    }
    /// Get only the bindings that match a pattern.

    fn get_bindings(&mut self, pattern: &str) -> ListResult {
        let p = Pattern::new(pattern);
        if let Err(reason) = p {
            Err(format!("Bad glob pattern {} :{}", pattern, reason.msg))
        } else {
            let p = p.unwrap();
            let mut listing = vec![];
            for b in self.shm.get_bindings() {
                if p.matches(&b.1) {
                    listing.push((b.0, b.1.clone()));
                }
            }
            Ok(listing)
        }
    }
    /// Clear the contents of bound spectra with names that match the
    /// pattern
    fn clear_spectra(&mut self, pattern: &str) {
        let spectra = self.get_bindings(pattern).unwrap();
        for info in spectra {
            let slot = info.0;
            self.shm.clear_contents(slot);
        }
    }
    /// Return a MemoryStatistics struct that describes the current
    /// memory and slot usage.
    fn get_statistics(&mut self) -> MemoryStatistics {
        let memory_stats = self.shm.statistics();

        MemoryStatistics {
            free_bytes: memory_stats.0,
            largest_free_bytes: memory_stats.1,
            used_bytes: memory_stats.2,
            largest_used_bytes: memory_stats.3,
            bound_indices: memory_stats.4,
            total_indices: memory_stats.5,
            total_size: memory_stats.6,
        }
    }
    /// Update the contents of all spectra bound to shared memory:

    fn update_contents(&mut self) {
        for binding in self.shm.get_bindings() {
            self.update_spectrum(binding);
        }
    }

    /// Process all requests and reply to them.
    /// If we have an Exit request, we're going to return false.
    fn process_request(&mut self, req: Request) -> bool {
        match req.request {
            RequestType::Unbind(name) => {
                if let Err(s) = self.unbind(&name) {
                    req.reply_chan
                        .send(Reply::Generic(GenericResult::Err(format!(
                            "Spectrum {} could not be unbound: {}",
                            name, s
                        ))))
                        .expect("Failed to send error response from binding thread to client");
                } else {
                    req.reply_chan
                        .send(Reply::Generic(GenericResult::Ok(())))
                        .expect("Failed to send reply to client from binding thread");
                }
                true
            }
            RequestType::UnbindAll => {
                for b in self.shm.get_bindings() {
                    // Too simple to need an fn.
                    self.shm.unbind(b.0);
                }
                req.reply_chan
                    .send(Reply::Generic(GenericResult::Ok(())))
                    .expect("Failed to send reply to client from binding thread");
                true
            }
            RequestType::Bind(name) => {
                if let Err(s) = self.bind(&name) {
                    req.reply_chan
                        .send(Reply::Generic(GenericResult::Err(format!(
                            "Could not bind spectrum {}; {}",
                            name, s
                        ))))
                        .expect("Failed to send error result from binding thread to client");
                } else {
                    req.reply_chan
                        .send(Reply::Generic(GenericResult::Ok(())))
                        .expect("Failed to send reply to client from binding thread");
                }
                true
            }
            RequestType::List(pattern) => {
                req.reply_chan
                    .send(Reply::List(self.get_bindings(&pattern)))
                    .expect("Failed to send bindings list to client");
                true
            }
            RequestType::Clear(pattern) => {
                self.clear_spectra(&pattern);
                req.reply_chan
                    .send(Reply::Generic(GenericResult::Ok(())))
                    .expect("Failed to send reply to client from binding thread");
                true
            }
            RequestType::SetUpdate(secs) => {
                self.timeout = secs;
                req.reply_chan
                    .send(Reply::Generic(GenericResult::Ok(())))
                    .expect("Failed to send reply to client from binding thread");
                true
            }
            RequestType::Statistics => {
                req.reply_chan
                    .send(Reply::Statistics(Ok(self.get_statistics())))
                    .expect("Failed to send reply to client from binding thread");
                true
            }
            RequestType::ShmName => {
                req.reply_chan
                    .send(Reply::String(Ok(self.shm.get_shm_name())))
                    .expect("Failed to send reply to client from bindng thread");
                true
            }
            RequestType::Exit => {
                req.reply_chan
                    .send(Reply::Generic(GenericResult::Ok(())))
                    .expect("Failed to send reply to client from binding thread");
                false
            }
        }
    }
    /// Create the binding state.  Note that in general,
    /// this is done within the binding thread which then
    /// invokes the run  method on the newly created object.
    /// This is similar to how the Histogrammer thread is
    /// created/started.
    pub fn new(
        req: mpsc::Receiver<Request>,
        api_chan: &mpsc::Sender<messaging::Request>,
        spec_size: usize,
    ) -> BindingThread {
        BindingThread {
            request_chan: req,
            spectrum_api: spectrum_messages::SpectrumMessageClient::new(api_chan),
            timeout: DEFAULT_TIMEOUT,
            shm: super::SharedMemory::new(spec_size)
                .expect("Failed to create shared memory region!!"),
        }
    }
    /// Runs the thread.  See the struct comments for a reasonably
    /// complete description of how the thread works.
    ///
    pub fn run(&mut self) {
        loop {
            match self
                .request_chan
                .recv_timeout(time::Duration::from_secs(self.timeout))
            {
                Ok(request) => {
                    if !self.process_request(request) {
                        break;
                    }
                }
                Err(tmo) => {
                    if let mpsc::RecvTimeoutError::Timeout = tmo {
                        // Timeout so update contents.

                        self.update_contents();
                    } else {
                        // Sender disconnected the channel.
                        println!("Binding thread sender disconnected -- exiting");
                        break;
                    }
                }
            }
        }
        drop(self);
    }
}
/// This is the function to call to initiate a BindingThread.
/// We return the request channel and the join handle.
///
pub fn start_server(
    hreq_chan: &mpsc::Sender<messaging::Request>,
    spectrum_bytes: usize,
) -> (mpsc::Sender<Request>, thread::JoinHandle<()>) {
    let (sender, receiver) = mpsc::channel();
    let hreq = hreq_chan.clone();
    let join_handle = thread::spawn(move || {
        let mut t = BindingThread::new(receiver, &hreq, spectrum_bytes);
        t.run();
    });
    (sender, join_handle)
}
/// This struct and its implementation provide an API to
/// make requests of a running BindingThread.  Note that theoretically,
/// more than one binding thread could be run, each managing a separate
/// instance of an Xamine compatible shared memory.
/// In the current version of Rustogramer we don't do that.
/// In the future, however, if an allocated shared memory region fills
/// (e.g. a bind fails), we could try to create a new shared memory
/// memory region (binding thread) and retry the bind in it instead.
/// that means a few changes to how the shared memory is reported
/// to the REST client and we don't do that now because:
///
/// *  We need to figure out the REST implications.
/// *  We need to figure out how to manage the state so that we know
/// not just we have binding thread but which ones have which spectra.
/// *  We need to figure out what to do to (un)bind a spectrum in the
/// presence of more than one BindingThread especially how to bind
/// a new spectrum once a 'full' memory region has had one or more
/// spectra unbound.
///
pub struct BindingApi {
    req_chan: mpsc::Sender<Request>,
}
impl BindingApi {
    // Private method to make a request.
    // - Creates the reply channel,
    // - Sends the request on the req_chan.
    // - Returns the reply without interpretation:
    //
    fn transaction(&self, req: RequestType) -> Reply {
        let (rep_send, rep_rcv) = mpsc::channel();
        if let Err(_) = self.req_chan.send(Request {
            reply_chan: rep_send,
            request: req,
        }) {
            return Reply::Generic(GenericResult::Err(String::from(
                "Failed to send request to Binding Thread",
            )));
        }
        let reply = rep_rcv.recv();
        if let Err(_) = reply {
            return Reply::Generic(GenericResult::Err(String::from(
                "Failed to receive reply from Binding thread request",
            )));
        }
        reply.unwrap()
    }
    /// Creates a binding API instance given a BindingThread's
    /// request channel.  Note that this is cloned so multiple
    /// API Instances talking to the same thread are fully supported.
    ///
    /// ### Parameters:
    /// *   req - request channel to the binding thread.
    ///
    /// ### Returns:
    /// *   BindingApi instance.
    ///
    pub fn new(req: &mpsc::Sender<Request>) -> BindingApi {
        BindingApi {
            req_chan: req.clone(),
        }
    }
    /// Unbind a spectrum from the shared memory.
    /// On success:
    /// - The binding slot used by that spectrum will be freed.
    /// - The memory used by the spectrum will be returned to the
    /// shared memory free pool.
    /// - the shared memory free pool will be defragmented.
    ///
    /// ### Paramters:
    /// *   name -name of the spectrum to unbind.
    ///
    /// ### Returns:
    /// * GenericResult instance.
    pub fn unbind(&self, name: &str) -> GenericResult {
        match self.transaction(RequestType::Unbind(String::from(name))) {
            Reply::Generic(result) => result,
            _ => Err(String::from("Unexpected return type from BindingThread")),
        }
    }
    /// Unbind all spectra from the shared memory that are currently bound.
    /// On success:
    /// -  All binding slots will be free.
    /// -  The spectrum storage pool will be one single extent that is the
    /// size of the spectrum memory.
    ///
    /// ### Parameters
    /// (none)
    ///
    /// ### Returns
    /// *  GenericResult instance.
    ///
    pub fn unbind_all(&self) -> GenericResult {
        match self.transaction(RequestType::UnbindAll) {
            Reply::Generic(result) => result,
            _ => Err(String::from("Unexpected return type from binding thread")),
        }
    }
    /// Bind the named spectrum into the shared memory.
    /// On success:
    /// -  A binding slot is allocated to the spectrum.
    /// -  Memory is allocated from the spectrum storage pool to hold
    /// the spectrum.
    /// - The current contents of the spectrum are copied into the
    /// shared memory assigned to the spectrum.
    /// - As long as the spectrum remains bound, the BindingThread will
    /// periodically (see set_update_period) update the contents
    /// of the spectrum storage from the spectrum contents.
    /// - Note the clear_spectra request will clear the spectrum contents
    /// until then ext refresh pass by BindingThread and should be
    /// called with "*") whenver histogrammer spectra are aslo cleared.
    ///
    /// ### Parameters
    /// *  name - name of the spectrum to bind.
    ///
    /// ### Returns
    /// * GenericResult instance.
    pub fn bind(&self, name: &str) -> GenericResult {
        match self.transaction(RequestType::Bind(String::from(name))) {
            Reply::Generic(result) => result,
            _ => Err(String::from("Unexpected return type from binding thread")),
        }
    }
    /// List the bindings that are currently in force in the
    /// shared memory.  This makes no modifications to the share
    // memory contents.
    ///
    /// ### Parameters
    /// *  pattern  - This is a glob pattern.  Only the bindings for
    /// spectra that match _pattern_ are returned.  Note that
    /// to get all bindings use the pattern "*"
    ///
    /// ### Returns
    /// *  ListResult instnace.
    ///
    pub fn list_bindings(&self, pattern: &str) -> ListResult {
        match self.transaction(RequestType::List(String::from(pattern))) {
            Reply::List(r) => r,
            _ => Err(String::from("Unexpected return type from binding thread")),
        }
    }
    /// Clear the contents of a collection of spectra in the shared memory.
    /// note that almost immediatetly the server will run a pass over
    /// the set of bound spectra, updating their contents.  
    /// This is required because the update only updates the non-zero
    /// channels of a spectrum.  Therefore, spectra must be cleared
    /// manually when
    /// *  When first bound (done by bind).
    /// *  When the underlying spectrum is cleared in the histogrammer
    ///
    /// This implies that whenever spectra are cleared inthe histogramer,
    /// this method must be invoked.
    ///
    /// ### Parameters
    /// *  pattern - Glob pattern.  Only bound spectra whose names
    /// match the _pattern_ paramter willi be cleared.
    ///
    /// ### Returns
    ///  *  GenericResult instance.
    ///
    pub fn clear_spectra(&self, pattern: &str) -> GenericResult {
        match self.transaction(RequestType::Clear(String::from(pattern))) {
            Reply::Generic(r) => r,
            _ => Err(String::from("Unexpected reply type from BindingServer")),
        }
    }
    /// Sets the rate at which the BindingThread updates the contents
    /// of the bound spectra in shared memory. Note that updates change
    /// the non-zero channels only.  See clear_spectra above.
    ///
    /// ### Parameters
    ///  * period_secs - number of seconds between updates.
    /// Note that this is approximate.  The actual period depends on the
    /// time required to perform the update as well as the latency between
    /// updates and the frequency of requests.   The BindingThread processing
    /// main loop reads requests with period_secs for a timeout. Updates
    /// take place only after such a read times out.  Therefore, the actual
    /// period can be slower because:
    ///     -  The bindings thread is busy processing requests that come in
    /// faster than period_secs and each processed request holds off the next
    /// timeout.
    ///     -  The actual update takes a significant amount of time
    /// relative to the update period.  In that case, given no requests
    /// to process, the actual period is period_secs + time required to do the update.
    ///
    /// ### Returns:
    /// *   GenericResult instance.
    ///
    pub fn set_update_period(&self, period_secs: u64) -> GenericResult {
        match self.transaction(RequestType::SetUpdate(period_secs)) {
            Reply::Generic(r) => r,
            _ => Err(String::from("Unexpected reply type from BindingServer")),
        }
    }
    /// Obtains the usage statistics for the shared memory region.
    ///
    /// ### Returns:
    ///    An instance of StatisticsResult
    ///
    pub fn get_usage(&self) -> StatisticsResult {
        match self.transaction(RequestType::Statistics) {
            Reply::Statistics(stats) => stats,
            _ => Err(String::from("Unexpected reply type from BindingServer")),
        }
    }
    /// Asks the binding thread to tell us the name of the shared
    /// memory region. The name includes  a prefix separated from
    /// a name that makes sense given the prefix by a colon.
    /// Valid prefixes are:
    ///
    /// *  file  - The shared memory is a mapped file.
    /// *  posix - The shared memory is a Posix shared memory name.
    /// *  sysv  - The shared memory is a SYSV shared memory segment.
    ///
    /// ### Examples:
    ///
    ///    file:/user/fox/some_name
    ///
    /// Is a mapped file named /user/fox/some_name.
    ///
    ///    posix:/junk
    ///
    /// Is a posix shared memory region named /junk.
    ///
    ///    sysv:Xa32
    ///
    /// Is a SYSV shared memory region with the token Xa32
    ///
    /// ### Returns:
    /// *  StringResult instance.
    ///
    pub fn get_shname(&self) -> StringResult {
        match self.transaction(RequestType::ShmName) {
            Reply::String(result) => result,
            _ => Err(String::from("Unexpected reply type from BindingServer")),
        }
    }
    /// Asks the binding thread to exit.  On successful return all
    /// further requests of this and other API objects that talk to the
    /// same BindingServer will fail attempting to do the send part
    /// of any transaction, as the thread will not process any more
    /// requests and will soon exit after sending its reply to us.
    ///
    /// ### Returns:
    /// *   GenericResult instance.
    ///
    pub fn exit(&self) -> GenericResult {
        match self.transaction(RequestType::Exit) {
            Reply::Generic(r) => r,
            _ => Err(String::from("Unexpected reply type from BindingServer")),
        }
    }
}
