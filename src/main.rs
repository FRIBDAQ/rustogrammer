mod conditions;
mod histogramer;
mod messaging;
mod parameters;
mod processing;
mod rest;
mod ring_items;
mod spectra;

use rest::gates;
use rest::rest_parameter;
use rest::spectrum;
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
    let processor = processing::ProcessingApi::new(&channel);
    processor
        .start_thread()
        .expect("Unable to start processor thread");

    let state = rest::HistogramState {
        state: Mutex::new((jh, channel)),
        processing: Mutex::new(processor),
    };
    rocket::build()
        .manage(state)
        .mount(
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
        .mount(
            "/spectcl/rawparameter",
            routes![
                rest_parameter::new_rawparameter,
                rest_parameter::list_rawparameter,
                rest_parameter::delete_rawparameter
            ],
        )
        .mount(
            "/spectcl/gate",
            routes![gates::list_gates, gates::delete_gate, gates::edit_gate],
        )
        .mount(
            "/spectcl/spectrum",
            routes![
                spectrum::list_spectrum,
                spectrum::delete_spectrum,
                spectrum::create_spectrum,
                spectrum::get_contents,
                spectrum::clear_spectra,
            ],
        )
        .mount("/spectcl/attach", routes![])
        .mount("/spectcl/analyze", routes![])
}
