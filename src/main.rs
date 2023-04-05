mod ring_items;
mod conditions;
mod histogramer;
mod messaging;
mod parameters;
mod spectra;

use std::sync::{Mutex};
// Pull in Rocket features:

#[macro_use]
extern crate rocket;

// This is now the entry point as Rocket has the main
//
#[launch]
fn rocket() -> _ {
    // For now to ensure the join handle and channel don't get
    // dropped start the histogram server in a thread:
    //

    let (jh, channel) = histogramer::start_server();
    rocket::build().manage(Mutex::new((jh, channel)))
}
