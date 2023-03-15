//!  This module contains message formats and message support code.
//!  The module is needed in order to pull the histograming part
//!  of the application out into a separate thread.  We envision
//!  the final application to consist of the following threads:
//!  - Main thread handles interactive commands -- maybe GUI too.
//!  - Histograming thread contains the dictionaries for parameters,
//! conditions, and spectra.  It accepts messages from the other thread
//! telling it what to do.  The messages can create or query one of the
//! dictionaries or hand it data from the:
//!   - Data input thread - runs when data analysis is in progress
//! sending raw data from the data source to the histograming thread.
//!   - Shared memory mirror thread.   This interacts, periodically,
//! with the histogramer to maintain Xamine compatible shared memory
//! regions from which external presenters can be run.
//!    - REST server thread which can interact with clients to provide
//! a SpecTcl compatible REST interface to the software state.
//!    - Mirror servers fired up on-demand from the REST server. these
//! provide a SpecTcl compatible shared memory mirroring service
//! which can allow remote presenters access to the spectra in a
//! SpecTcl compatible manner.
//!
//! All of this threading requires messages to be sent to the
//! histograming thread and for replies to come back.
//!
use std::sync::mpsc;

/// The MessageType enum defines which subset of functionality
/// a message is adressed to.

#[derive(Clone)]
pub enum MessageType {
    Parameter,
}

/// The Reply enum defines the sorts of things that can be sent
/// back along the reply channel.  It too is a coarse enum which
/// is further refined for each reply type within the
/// subdivision.
#[derive(Clone)]
pub enum Reply {
    Parameter,
}

///
/// The Request struct is the format of the message that is sent
/// to the histogramer.  It consists of a channel on which the
/// reply message is sent (See the Reply struct) and
/// a payload that is an enum (MessageType) that contains:
///   -    The actual message type differentiation.
////  -    The payload of each message type which contains
/// any additional information required by the request.
///
/// Note that the MessageType enum is a broad categorization
/// that describes which sub-set of histogrammer functionality
/// is adressed by that request and the payloads are
/// usually enums which define the actual request of that subset.
/// for example the Parameter MessageType is, itself an enum whose
/// options define the actual request of the Parameter handling code
/// section.
///
#[derive(Clone)]
pub struct Request {
    reply_channel: mpsc::Sender<Reply>,
    message: MessageType,
}

/// These functions send/receive raw messages and
/// provide for a transaction (send message/receive reply)
/// It is recommended, however that submodule detailed functions
/// be used instead.  Those layer on top of these methods to
/// provide an API that hides the actual message passing.
/// Note that send/receive failures can only happen if something
/// serious has happened like all senders have exited or
/// the receive end exited.  These are all treated with PANICs.
///

impl Request {
    /// Send request message to the histogramer along chan.
    ///
    pub fn send(&self, chan: mpsc::Sender<Request>) {
        chan.send(self.clone())
            .expect("Send to histogramer failed!");
    }
    /// Get a request (by the histogramer).
    ///
    pub fn get_request(&self, chan: mpsc::Receiver<Request>) -> Request {
        let result = chan.recv().expect("Receive by histogramer failed!");
        result
    }
    /// Send a reply to the client:
    ///
    pub fn send_reply(&self, msg: Reply) {
        self.reply_channel
            .send(msg)
            .expect("Histogramer failed to send reply to client")
    }

    /// Get a reply from the historamer
    ///
    pub fn get_reply(chan: mpsc::Receiver<Reply>) -> Reply {
        chan.recv().expect("Read of reply from histogramer failed!")
    }
    /// Transaction with the histogramer:
    /// Sends a request and returns its reply.  This is the
    /// Method client methods should use unless they want to
    /// overlap some work between the request/reply
    ///
    pub fn transaction(&self, req: mpsc::Sender<Request>, reply: mpsc::Receiver<Reply>) -> Reply {
        self.send(req);
        Self::get_reply(reply)
    }
}
