//!   histogramer  - This provides the histograming thread.
//!   The histogramer is implemented as
//! * A request processor
//! function - which can be tested in isolation in an unthreaded
//! environment:
//! * A communication function that gets requests, hands them off
//! to the request processing function and transmits the reply to
//! that request to the requestor.
//! * A public function to start the thread.
//! * A public function to stop the thread.
//!

use super::*;
use crate::messaging::*;
use std::sync::mpsc;
use std::thread;

///  The Request processor is implemented as a struct which holds
/// to the request processing structs for each of the categories of
/// messages.
///  The implementation can instantiate these and provides for message
/// processing once a message has been received.
///

struct RequestProcessor {
    parameters: parameter_messages::ParameterProcessor,
    conditions: condition_messages::ConditionProcessor,
    spectra: spectrum_messages::SpectrumProcessor,
}

impl RequestProcessor {
    /// Create an instance of RequestProcessor.

    pub fn new() -> RequestProcessor {
        RequestProcessor {
            parameters: parameter_messages::ParameterProcessor::new(),
            conditions: condition_messages::ConditionProcessor::new(),
            spectra: spectrum_messages::SpectrumProcessor::new(),
        }
    }
    /// Process a message and return the response.
    /// The top level is just a match on the top level message type
    /// which delivers the message to the appropriate member's
    /// message processing interface and wraps the reply in a
    /// response message.
    ///  Note that we ignore the Exit message since that must be
    /// handled at the level of the thread that runs us:
    ///
    pub fn process_message(&mut self, message: messaging::MessageType) -> messaging::Reply {
        match message {
            MessageType::Parameter(req) => Reply::Parameter(self.parameters.process_request(req)),
            MessageType::Condition(req) => Reply::Condition(self.conditions.process_request(req)),
            MessageType::Spectrum(req) => Reply::Spectrum(self.spectra.process_request(
                req,
                self.parameters.get_dict(),
                self.conditions.get_dict(),
            )),
            MessageType::Exit => Reply::Exiting,
        }
    }
}

/// The histogramer struct is essentially the the thread.
/// This layer encapsulates a RequestProcessor and the
/// Receiver<Request> channel on which requests are received.
///
struct Histogramer {
    processor: RequestProcessor,
    chan: mpsc::Receiver<Request>,
}
impl Histogramer {
    pub fn new(chan: mpsc::Receiver<Request>) -> Histogramer {
        Histogramer {
            processor: RequestProcessor::new(),
            chan: chan,
        }
    }
    ///
    /// Invoke this to run the server until it's told to exit.
    ///
    pub fn run(&mut self) {
        loop {
            let req = self.chan.recv().expect("Failed to read a request");
            let reply = self.processor.process_message(req.message);

            // The reply is sent to the client but if it's an exit we
            // return
            // Since send consumes the reply we need to
            // do it as per below or clone the reply which
            // might be computationally expensive (imagine it contains
            // the contents of a dense, large, 2d histogram e.g.).

            if let Reply::Exiting = reply {
                req.reply_channel
                    .send(reply)
                    .expect("Failed to send reply to request");
                break;
            } else {
                req.reply_channel
                    .send(reply)
                    .expect("Failed to send reply to request");
            }
        }
    }
}

// Note we're just going to try some simple requests for each
// type to ensure all branches of the match in process_message work.
// We assume each request processor has already been extensively
// tested in its own module's tests.
#[cfg(test)]
mod request_tests {
    use super::*;
    #[test]
    fn param_create_1() {
        let mut req = RequestProcessor::new();
        let msg = MessageType::Parameter(ParameterRequest::Create(String::from("test")));
        assert!(
            if let messaging::Reply::Parameter(ParameterReply::Created) = req.process_message(msg) {
                true
            } else {
                false
            }
        );
        let d = req.parameters.get_dict();
        d.lookup("test").expect("failed to find 'test' parameters");
    }
    #[test]
    fn cond_create_1() {
        let mut req = RequestProcessor::new();
        let msg = MessageType::Condition(ConditionRequest::CreateTrue(String::from("true")));
        assert!(
            if let Reply::Condition(ConditionReply::Created) = req.process_message(msg) {
                true
            } else {
                false
            }
        );
        let d = req.conditions.get_dict();
        d.get(&String::from("true")).expect("Failed gate lookup");
    }
    #[test]
    fn spec_clear_1() {
        // Clear because we don't actually need any
        // spectra for that.
        //
        let mut req = RequestProcessor::new();
        let msg = MessageType::Spectrum(SpectrumRequest::Clear(String::from("*")));
        assert!(
            if let Reply::Spectrum(SpectrumReply::Cleared) = req.process_message(msg) {
                true
            } else {
                false
            }
        );
    }
    #[test]
    fn exit_1() {
        let mut req = RequestProcessor::new();
        let msg = MessageType::Exit;
        assert!(if let Reply::Exiting = req.process_message(msg) {
            true
        } else {
            false
        });
    }
}
#[cfg(test)]
mod hgrammer_tests {
    use super::*;
    use crate::messaging;
    use std::sync::mpsc;
    use std::thread;

    fn start_server() -> (thread::JoinHandle<()>, mpsc::Sender<Request>) {
        let (req_send, req_recv) = mpsc::channel();

        let join_handle = thread::spawn(move || {
            let mut processor = Histogramer::new(req_recv);
            processor.run();
        });

        (join_handle, req_send)
    }
    fn stop_server(jh: thread::JoinHandle<()>, req_send: mpsc::Sender<Request>) {
        let (rep_send, rep_recv) = mpsc::channel();
        let req = messaging::Request {
            reply_channel: rep_send,
            message: messaging::MessageType::Exit,
        };
        assert!(
            if let messaging::Reply::Exiting = req.transaction(req_send, rep_recv) {
                true
            } else {
                false
            }
        );

        jh.join().expect("Failed to join server thread");
    }
    #[test]
    fn exit_1() {
        // start and stop the thread...all test are in that
        // Tests server response to Request::Exit
        let (jh, ch) = start_server();
        stop_server(jh, ch);
    }
    #[test]
    fn params_1() {
        // test parameters:

        let (jh, ch) = start_server();
        let client = messaging::parameter_messages::ParameterMessageClient::new(&ch);
        let lr = client.list_parameters("*").expect("list failed"); // should be empty but work.
        assert_eq!(0, lr.len());

        // want a bit more:

        client
            .create_parameter("test")
            .expect("create test parameter failed");
        let lr = client.list_parameters("*").expect("list2 failed");
        assert_eq!(1, lr.len());
        assert_eq!(String::from("test"), lr[0].get_name());
    }
}
