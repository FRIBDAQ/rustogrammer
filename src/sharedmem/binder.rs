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

use crate::messaging::spectrum_messages;
use crate::messaging;
use glob::Pattern;
use std::sync::mpsc;

// This enum represents the set of operations that can be
// requested of this thread:

/// Requests are sent to the BindingThread via this enum.
/// The enum is private because it is instantiated by the
/// pub elements of the API.
///
enum RequestTypes {
    Unbind(String),
    UnbindAll,
    Bind(String),
    List(String),
    Clear(String),
    SetUpdate(u64),
}
struct Request {
    reply_chan: mpsc::Sender<Reply>,
    request: RequestTypes,
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
struct BindingThread {
    request_chan: mpsc::Receiver<Request>,
    spectrum_api: spectrum_messages::SpectrumMessageClient,
    timeout: u64,
}

impl BindingThread {
    /// Create the binding state.  Note that in general,
    /// this is done within the binding thread which then
    /// invokes the run  method on the newly created object.
    /// This is similar to how the Histogrammer thread is
    /// created/started.
    pub fn new(
        req: mpsc::Receiver<Request>,
        api_chan: &mpsc::Sender<messaging::Request>,
    ) -> BindingThread {
        BindingThread {
            request_chan: req,
            spectrum_api: spectrum_messages::SpectrumMessageClient::new(api_chan),
            timeout: DEFAULT_TIMEOUT,
        }
    }
}
