//! Todo:
//!  *  Get the port number from the commandl ine
//!  *  Allow that port number to be 'managed' which means get it
//!from the port manager.
//!  *  Get the shared memory size from the command line.
//!
mod conditions;
mod histogramer;
mod messaging;
mod parameters;
mod processing;
mod rest;
mod ring_items;
mod sharedmem;
mod spectra;

use clap::Parser;
use rest::{
    apply, channel, data_processing, evbunpack, exit, filter, fit, fold, gates, integrate,
    rest_parameter, ringversion, sbind, shm, spectrum, unbind, unimplemented, version,
};
use sharedmem::binder;
use std::sync::Mutex;

// Pull in Rocket features:

#[macro_use]
extern crate rocket;

const DEFAULT_SHM_SPECTRUM_MBYTES: usize = 32;

// Program parameters as parsed by Clap:

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    #[arg(short, long, default_value_t=DEFAULT_SHM_SPECTRUM_MBYTES)]
    shm_mbytes: usize,
}

// This is now the entry point as Rocket has the main
//
#[launch]
fn rocket() -> _ {
    let args = Args::parse();

    // For now to ensure the join handle and channel don't get
    // dropped start the histogram server in a thread:
    //

    let (jh, channel) = histogramer::start_server();
    let processor = processing::ProcessingApi::new(&channel);
    let binder = binder::start_server(&channel, args.shm_mbytes*1024*1024);

    let state = rest::HistogramState {
        state: Mutex::new((jh, channel)),
        binder: Mutex::new(binder),
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
        .mount(
            "/spectcl/fit",
            routes![fit::create, fit::update, fit::delete, fit::list, fit::proc],
        )
        .mount(
            "/spectcl/fold",
            routes![fold::apply, fold::list, fold::remove],
        )
        .mount("/spectcl/integrate", routes![integrate::integrate])
        .mount(
            "/spectcl/shmem",
            routes![shm::shmem_name, shm::shmem_size, shm::get_variables],
        )
        .mount(
            "/spectcl/sbind",
            routes![sbind::sbind_all, sbind::sbind_list, sbind::sbind_bindings],
        )
        .mount(
            "/spectcl/unbind",
            routes![
                unbind::unbind_byname,
                unbind::unbind_byid,
                unbind::unbind_all
            ],
        )
        .mount("/spectcl/mirror", routes![unimplemented::mirror_list])
        .mount(
            "/spectcl/pman",
            routes![
                unimplemented::pman_create,
                unimplemented::pman_list,
                unimplemented::pman_current,
                unimplemented::pman_listall,
                unimplemented::pman_list_event_processors,
                unimplemented::pman_choose_pipeline,
                unimplemented::pman_add_processor,
                unimplemented::pman_rm_processor,
                unimplemented::pman_clear,
                unimplemented::pman_clone
            ],
        )
        .mount("/spectcl/project", routes![unimplemented::project])
        .mount(
            "/spectcl/pseudo",
            routes![
                unimplemented::pseudo_create,
                unimplemented::pseudo_list,
                unimplemented::pseudo_delete
            ],
        )
        .mount(
            "/spectcl/roottree",
            routes![
                unimplemented::roottree_create,
                unimplemented::roottree_delete,
                unimplemented::roottree_list
            ],
        )
        .mount("/spectcl/script", routes![unimplemented::script_execute])
        .mount(
            "/spectcl/trace",
            routes![
                unimplemented::trace_establish,
                unimplemented::trace_done,
                unimplemented::trace_fetch
            ],
        )
        .mount(
            "/spectcl/treevariable",
            routes![
                unimplemented::treevariable_list,
                unimplemented::treevariable_set,
                unimplemented::treevariable_check,
                unimplemented::treevariable_set_changed,
                unimplemented::treevariable_fire_traces
            ],
        )
        .mount("/spectcl/version", routes![version::get_version])
        .mount("/spectcl/exit", routes![exit::shutdown])
        .mount(
            "/spectcl/ringformat",
            routes![ringversion::ringversion_get, ringversion::ringversion_set],
        )
}
