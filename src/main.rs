mod conditions;
mod histogramer;
mod messaging;
mod parameters;
mod rest;
mod ring_items;
mod spectra;

use rest::rest_parameter;
use std::sync::Mutex;

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
    let state = rest::HistogramState {
        state: Mutex::new((jh, channel)),
    };
    rocket::build().manage(state).mount(
        "/spectcl/parameter",
        routes![
            rest_parameter::list_parameters,
            rest_parameter::parameter_version,
            rest_parameter::create_parameter,
            rest_parameter::edit_parameter,
            rest_parameter::promote_parameter,
            rest_parameter::check_parameter,
            rest_parameter::uncheck_parameter
        ],
    )
}
