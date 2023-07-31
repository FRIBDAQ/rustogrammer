mod conditions;
mod histogramer;
mod messaging;
mod parameters;
mod processing;
mod rest;
mod ring_items;
mod sharedmem;
mod spectclio;
mod spectra;
mod trace;

use clap::Parser;
use portman_client;
use rest::{
    apply, channel, data_processing, evbunpack, exit, filter, fit, fold, gates, getstats,
    integrate, mirror_list, rest_parameter, ringversion, sbind, shm, spectrum, spectrumio, unbind,
    unimplemented, version,
};
use sharedmem::{binder, mirror};
use std::env;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

// Pull in Rocket features:

#[macro_use]
extern crate rocket;

// Pull in scan_fmt:

#[macro_use]
extern crate scan_fmt;

const DEFAULT_SHM_SPECTRUM_MBYTES: usize = 32;

// Program parameters as parsed by Clap:

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    #[arg(short, long, default_value_t=DEFAULT_SHM_SPECTRUM_MBYTES)]
    shm_mbytes: usize,
    #[arg(short, long, default_value_t = 8000)]
    rest_port: u16,
    #[arg(long)]
    rest_service: Option<String>,
    #[arg(long, default_value_t = 8001)]
    mirror_port: u16,
    #[arg(long)]
    mirror_service: Option<String>,
}

// This is now the entry point as Rocket has the main
//
#[launch]
fn rocket() -> _ {
    let args = Args::parse();

    // Create the trace database and start its prune thread.
    // we will make a separate state for it.

    let trace_store = trace::SharedTraceStore::new();
    trace_store.start_prune_thread();

    // start the histogram server in a thread:
    //

    let (_, histogramer_channel) = histogramer::start_server(trace_store.clone());
    let processor = processing::ProcessingApi::new(&histogramer_channel);
    let binder = binder::start_server(&histogramer_channel, args.shm_mbytes * 1024 * 1024, &trace_store);

    let (rest_port, mirror_port, client) = get_ports(&args);

    // Start the mirror server:

    let shm_name = binder::BindingApi::new(&binder.0)
        .get_shname()
        .expect("Unable to get shared memoryname");
    // Remove the file: prefix:
    let colon = shm_name
        .find(':')
        .expect("Finding end of file: in shm name");
    let shm_name = String::from(&shm_name.as_str()[colon + 1..]);
    println!("Mirroring {} port: {}", shm_name, mirror_port);

    let (mirror_send, mirror_rcv) = mpsc::channel();
    let mirror_directory = Arc::new(Mutex::new(mirror::Directory::new()));
    let server_dir = mirror_directory.clone();
    thread::spawn(move || {
        let mut server = mirror::MirrorServer::new(mirror_port, &shm_name, mirror_rcv, server_dir);
        server.run();
    });

    let state = rest::HistogramState {
        histogramer: Mutex::new(histogramer_channel),
        binder: Mutex::new(binder.0),
        processing: Mutex::new(processor),
        portman_client: client,
        mirror_exit: Arc::new(Mutex::new(mirror_send)),
        mirror_port: mirror_port,
    };

    // Set the rocket port then fire it off:

    env::set_var("ROCKET_PORT", rest_port.to_string());

    rocket::build()
        .manage(mirror_directory.clone())
        .manage(state)
        .manage(trace_store.clone())
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
        .mount("/spectcl/mirror", routes![mirror_list::mirror_list])
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
        .mount("/spectcl/specstats", routes![getstats::get_statistics])
        .mount("/spectcl/swrite", routes![spectrumio::swrite_handler])
        .mount("/spectcl/sread", routes![spectrumio::sread_handler])
}
///
/// Gets the port to use for our REST service.
/// This uses command line argument that have been Parsed into
/// the Args struct.   Here's how we determine the port to advertise:
///
/// * If service is supplied, then port is assumed to be the port manager's
/// service port and we advertise given the service specified.
/// * if service is not supplied, then we advertise on the value of the port
/// field.
///
/// The Client is part of what's returned as it must remain alive to
/// keep the allocation.
fn get_ports(args: &Args) -> (u16, u16, Option<portman_client::Client>) {
    let mut result = (0, 0, None);

    // The rest port/service:

    if args.rest_service.is_some() {
        let mut client = portman_client::Client::new(args.rest_port);
        let port = client
            .get(args.rest_service.as_ref().unwrap())
            .expect("Could not allocate service port");

        result.0 = port;
        result.2 = Some(client);
    } else {
        result.0 = args.rest_port;
    }
    // Mirror port/service:

    if args.mirror_service.is_some() {
        if let Some(c) = result.2.as_mut() {
            result.1 = c
                .get(args.rest_service.as_ref().unwrap())
                .expect("Getting mirror port");
        } else {
            let mut client = portman_client::Client::new(args.rest_port);
            result.1 = client
                .get(args.rest_service.as_ref().unwrap())
                .expect("Getting mirror port");
            result.2 = Some(client);
        }
    } else {
        result.1 = args.mirror_port;
    }
    result
}
