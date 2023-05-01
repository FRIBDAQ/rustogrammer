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
pub type ListResult = Result<Vec<(String, u32)>, String>;

enum Reply {
    Generic(GenericResult),
    List(ListResult),
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

        if let Ok(info) = self.spectrum_api.list_spectra(&name) {
            if info.len() != 1 {
                self.shm.unbind(slot); // probably no such spectrum.
            } else {
                let axis_spec = Self::get_axes(&info[0]);
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
            }
        } else {
            self.shm.unbind(slot);
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
        let result = match req.request {
            RequestType::Unbind(name) => true,
            RequestType::UnbindAll => true,
            RequestType::Bind(name) => true,
            RequestType::List(pattern) => true,
            RequestType::Clear(pattern) => true,
            RequestType::SetUpdate(secs) => {
                self.timeout = secs;
                true
            }
            RequestType::Exit => false,
        };
        req.reply_chan
            .send(Reply::Generic(GenericResult::Ok(())))
            .expect("Failed to send reply to client from binding thread");
        result
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
            shm: super::SharedMemory::new(spec_size).expect("Failed to create shared memory region!!"),
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
