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

use crate::messaging::*;
use crate::trace;
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
    pub fn process_message(
        &mut self,
        message: MessageType,
        tracedb: &trace::SharedTraceStore,
    ) -> Reply {
        match message {
            MessageType::Parameter(req) => {
                Reply::Parameter(self.parameters.process_request(req, tracedb))
            }
            MessageType::Condition(req) => {
                Reply::Condition(self.conditions.process_request(req, tracedb))
            }
            MessageType::Spectrum(req) => Reply::Spectrum(self.spectra.process_request(
                req,
                self.parameters.get_dict(),
                self.conditions.get_dict(),
                tracedb,
            )),
            MessageType::Exit => Reply::Exiting,
        }
    }
}

/// The histogramer struct is essentially the the thread.
/// This layer encapsulates a RequestProcessor and the
/// `Receiver<Request>` channel on which requests are received.
///
struct Histogramer {
    processor: RequestProcessor,
    chan: mpsc::Receiver<Request>,
    tracdb: trace::SharedTraceStore,
}
impl Histogramer {
    pub fn new(chan: mpsc::Receiver<Request>, tracedb: trace::SharedTraceStore) -> Histogramer {
        Histogramer {
            processor: RequestProcessor::new(),
            chan,
            tracdb: tracedb.clone(),
        }
    }
    ///
    /// Invoke this to run the server until it's told to exit.
    ///
    pub fn run(&mut self) {
        loop {
            let req = self.chan.recv();
            if req.is_err() {
                return;
            }
            let req = req.unwrap();
            let reply = self.processor.process_message(req.message, &self.tracdb);

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

// Stolen from the tests so we already know they work:

/// Start the histogram server the returned tuple contains
/// the thread's join handle and the channel on which to  send the
/// server requests.
/// Note that there are well developed API classes for formating
/// and sending request message to this server...use them.
///
pub fn start_server(
    tracdb: trace::SharedTraceStore,
) -> (thread::JoinHandle<()>, mpsc::Sender<Request>) {
    let (req_send, req_recv) = mpsc::channel();

    let db = tracdb.clone();
    let join_handle = thread::spawn(move || {
        let mut processor = Histogramer::new(req_recv, db);
        processor.run();
    });

    (join_handle, req_send)
}
/// Stop the histogram server:
///
/// * jh - the join handle for the server thread.  On exit from this
/// function the join has been done.
/// * req_send - the channel on which requests get sent to the server.
/// (second element of the tuple returned from the start_server function).
///
pub fn stop_server(req_send: &mpsc::Sender<Request>) {
    let (rep_send, rep_recv) = mpsc::channel();
    let req = Request {
        reply_channel: rep_send,
        message: MessageType::Exit,
    };

    req.transaction(req_send.clone(), rep_recv);
}

// Note we're just going to try some simple requests for each
// type to ensure all branches of the match in process_message work.
// We assume each request processor has already been extensively
// tested in its own module's tests.
#[cfg(test)]
mod request_tests {
    use super::*;
    use crate::messaging;
    use crate::trace;
    use std::matches;
    #[test]
    fn param_create_1() {
        let mut req = RequestProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        let msg = MessageType::Parameter(ParameterRequest::Create(String::from("test")));
        assert!(matches!(
            req.process_message(msg, &tracedb),
            messaging::Reply::Parameter(ParameterReply::Created)
        ));

        let d = req.parameters.get_dict();
        d.lookup("test").expect("failed to find 'test' parameters");
    }
    #[test]
    fn cond_create_1() {
        let mut req = RequestProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        let msg = MessageType::Condition(ConditionRequest::CreateTrue(String::from("true")));
        assert!(matches!(
            req.process_message(msg, &tracedb),
            Reply::Condition(ConditionReply::Created)
        ));

        let d = req.conditions.get_dict();
        d.get(&String::from("true"))
            .expect("Failed condition lookup");
    }
    #[test]
    fn spec_clear_1() {
        // Clear because we don't actually need any
        // spectra for that.
        //
        let mut req = RequestProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        let msg = MessageType::Spectrum(SpectrumRequest::Clear(String::from("*")));
        assert!(matches!(
            req.process_message(msg, &tracedb),
            Reply::Spectrum(SpectrumReply::Cleared)
        ));
    }
    #[test]
    fn exit_1() {
        let mut req = RequestProcessor::new();
        let tracedb = trace::SharedTraceStore::new();
        let msg = MessageType::Exit;
        assert!(matches!(req.process_message(msg, &tracedb), Reply::Exiting));
    }
}
#[cfg(test)]
mod hgrammer_tests {
    use super::*;
    use crate::messaging;
    use crate::test::histogramer_common;
    use std::matches;
    use std::sync::mpsc;
    use std::thread;

    fn setup() -> (thread::JoinHandle<()>, mpsc::Sender<Request>) {
        let (req, jh) = histogramer_common::setup();
        (jh, req)
    }
    fn teardown(ch: mpsc::Sender<Request>, jh: thread::JoinHandle<()>) {
        histogramer_common::teardown(ch, jh);
    }
    fn stop_server(req_send: mpsc::Sender<Request>) {
        super::stop_server(&req_send);
    }
    #[test]
    fn exit_1() {
        // start and stop the thread...all test are in that
        // Tests server response to Request::Exit
        let (jh, ch) = setup();
        stop_server(ch);
        jh.join().unwrap();
    }
    #[test]
    fn params_1() {
        // test parameters:

        let (jh, ch) = setup();
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

        teardown(ch, jh);
    }
    #[test]
    fn conditions_1() {
        // test interactions via conditions API:

        let (jh, ch) = setup();

        let client = messaging::condition_messages::ConditionMessageClient::new(&ch);

        let reply = client.create_true_condition("true");
        assert!(matches!(
            reply,
            messaging::condition_messages::ConditionReply::Created
        ));

        let reply = client.list_conditions("*");
        assert!(
            if let messaging::condition_messages::ConditionReply::Listing(l) = reply {
                assert_eq!(1, l.len());
                assert_eq!(String::from("true"), l[0].cond_name);
                true
            } else {
                false
            }
        );

        teardown(ch, jh);
    }
    #[test]
    fn spectra_1() {
        // Test interactions with spectrum API.

        let (jh, ch) = setup();
        let client = messaging::spectrum_messages::SpectrumMessageClient::new(&ch);

        // Simplest thing we cand without needing to add any parameters
        // is get the empty list of spectra:

        let l = client.list_spectra("*").expect("Failed to list spectra");
        assert_eq!(0, l.len());

        teardown(ch, jh);
    }
    #[test]
    fn getchan_1() {
        // Get a channel from a spectrum...don't bother to load data
        // just see that the round trip request/response works:

        let (jh, ch) = setup();
        let client = messaging::spectrum_messages::SpectrumMessageClient::new(&ch);
        let params = messaging::parameter_messages::ParameterMessageClient::new(&ch);

        // Make a 1d histogram:

        params
            .create_parameter("test")
            .expect("Making a parameters");
        client
            .create_spectrum_1d("test", "test", 0.0, 1024.0, 1024)
            .expect("Making a spectrum");

        let result = client
            .get_channel_value("test", 512, None)
            .expect("Getting channel value");
        assert_eq!(0.0, result);

        teardown(ch, jh);
    }
    #[test]
    fn getchan_2() {
        // Same as above but for a 2d spectrum:

        let (jh, ch) = setup();
        let client = messaging::spectrum_messages::SpectrumMessageClient::new(&ch);
        let params = messaging::parameter_messages::ParameterMessageClient::new(&ch);

        // Make a 1d histogram:

        params.create_parameter("p1").expect("Making a parameters");
        params.create_parameter("p2").expect("Making a parameter");
        client
            .create_spectrum_2d("test", "p1", "p2", 0.0, 1024.0, 512, 0.0, 1024.0, 512)
            .expect("Making the spectrum");

        let result = client
            .get_channel_value("test", 256, Some(256))
            .expect("Getting channel value");
        assert_eq!(0.0, result);

        teardown(ch, jh);
    }
}
