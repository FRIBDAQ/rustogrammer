mod conditions;
mod histogramer;
mod messaging;
mod parameters;
mod rest;
mod ring_items;
mod spectra;

use messaging::Request;
use rest::rest_parameter;
use std::sync::{mpsc, Mutex};
use std::thread;

// Pull in Rocket features:

#[macro_use]
extern crate rocket;

///  This type is used by all handler in a state request guard
///  to obtain the send channel and join handle of the histogram
///  server:

pub struct HistogramState {
    junk: String,
}
// This is now the entry point as Rocket has the main
//
#[launch]
fn rocket() -> _ {
    // For now to ensure the join handle and channel don't get
    // dropped start the histogram server in a thread:
    //

    let (jh, channel) = histogramer::start_server();
    let state = rest::HistogramState {
        state: Mutex::new((jh, channel)),
    };
    rocket::build()
        .manage(state)
        .mount("/spectcl/parameter", routes![rest_parameter::list_parameters])
}
