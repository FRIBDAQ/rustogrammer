mod ring_items;
use ring_items::abnormal_end;
use ring_items::analysis_ring_items;
use ring_items::event_item;
use ring_items::format_item;
use ring_items::glom_parameters;
use ring_items::scaler_item;
use ring_items::state_change;
use ring_items::text_item;
use ring_items::triggers_item;
use ring_items::FromRaw;
use std::fs::File;
use std::sync::{Arc, Mutex};

mod conditions;
mod histogramer;
mod messaging;
mod parameters;
mod spectra;

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
