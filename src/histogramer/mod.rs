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
    pub fn process_message(&mut self, message: messaging::MessageType) -> Reply {
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

#[cfg(test)]
mod request_tests {
    use super::*;

    #[test]
    fn new_1() {
        
    }
}
