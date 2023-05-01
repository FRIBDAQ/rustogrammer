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
use std::sync::mpsc;
use std::time;

/// Memory statistics have this format:
///
pub struct MemoryStatistics {
    free_bytes: usize,
    largest_free_bytes: usize,
    used_bytes: usize,
    largest_used_bytes: usize,
    bound_indices: usize,
    total_indices: usize,
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
    Exit,
}
struct Request {
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
enum Reply {
    Generic(GenericResult),
    List(ListResult),
    Statistics(StatisticsResult),
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
            match self
                .shm
                .bind_spectrum(name, Self::axis(info.xaxis), Some(Self::axis(info.yaxis)))
            {
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
                    .send(Reply::Generic(GenericResult::Ok(())))
                    .expect("Failed to send reply to client from binding thread");
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
    }
}
