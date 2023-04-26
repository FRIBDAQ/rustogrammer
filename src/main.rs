mod conditions;
mod histogramer;
mod messaging;
mod parameters;
mod processing;
mod rest;
mod ring_items;
mod spectra;

use rest::apply;
use rest::channel;
use rest::data_processing;
use rest::evbunpack;
use rest::filter;
use rest::fit;
use rest::gates;
use rest::rest_parameter;
use rest::spectrum;
use rest::fold;

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
        .mount(
            "/spectcl/attach",
            routes![
                data_processing::attach_source,
                data_processing::list_source,
                data_processing::detach_source
            ],
        )
        .mount(
            "/spectcl/analyze",
            routes![
                data_processing::start_processing,
                data_processing::stop_processing,
                data_processing::set_event_batch
            ],
        )
        .mount(
            "/spectcl/apply",
            routes![apply::apply_gate, apply::apply_list],
        )
        .mount("/spectcl/ungate", routes![apply::ungate_spectrum])
        .mount(
            "/spectcl/channel",
            routes![channel::set_chan, channel::get_chan],
        )
        .mount(
            "/spectcl/evbunpack",
            routes![
                evbunpack::create_evbunpack,
                evbunpack::add_evbunpack,
                evbunpack::list_evbunpack,
            ],
        )
        .mount(
            "/spectcl/filter",
            routes![
                filter::new,
                filter::delete,
                filter::enable,
                filter::disable,
                filter::regate,
                filter::file,
                filter::list
            ],
        )
        .mount("/spectcl/fit", 
        routes![
            fit::create, fit::update, fit::delete, fit::list, fit::proc
        ])
        .mount("/spectcl/fold", routes![fold::apply, fold::list, fold::remove])
}
