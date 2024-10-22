//!  This module provides rest interfaces that bear on the
//! Xamine compatible shared memory mapping.
//! These include domains:
//!
//! * /spectcl/shmem - Gets the shared memory information.
use super::*;
use crate::sharedmem::binder::BindingApi;
use crate::sharedmem::XamineSharedMemory;
use rocket::{serde::json::Json, serde::Deserialize, serde::Serialize, State};
use std::env;
use std::mem;

//----------------------------------------------------------------
// /spectcl/shmem domain implementation:
//

//--------------------------------------------------------
// key
/// Return the shared memory name.  In Rustogramer,
/// this a  string of the form type:name
/// where the interpretation of name depends on the type.
/// See BindgApi::get_shname for more.
///
/// ### Parameters
/// * state - provides among other things the channel needed to
/// instantiate a BindingApi.
///
/// ### Return:
///   Json encoded GenericResponse where, on success, the detail
/// is the name of the region and on error, the reason for faiure.
///
#[get("/key")]
pub fn shmem_name(state: &State<SharedBinderChannel>) -> Json<GenericResponse> {
    let api = BindingApi::new(&state.inner().lock().unwrap());
    Json(match api.get_shname() {
        Ok(name) => GenericResponse::ok(&name),
        Err(reason) => GenericResponse::err("Failed to get shared memory name", &reason),
    })
}
//------------------------------------------------------------
// size

/// Returns the size of the spectrum part of the  shared memory region in the
/// status as a string.  
///
/// ### Parameters
/// *  state - the histogram state object which lets us construct a
/// BindingApi
///
/// ### Return
/// * An Json encoded version of a GenericResponse object.  On success,
/// the detail field contains the size of the memory region. On failure,
/// why the request failed.
///
#[get("/size")]
pub fn shmem_size(state: &State<SharedBinderChannel>) -> Json<GenericResponse> {
    let api = BindingApi::new(&state.inner().lock().unwrap());
    let info = api.get_usage();

    let response = match info {
        Ok(stats) => {
            let spectrum_size = stats.total_size - mem::size_of::<XamineSharedMemory>();
            GenericResponse::ok(&(spectrum_size.to_string()))
        }
        Err(reason) => GenericResponse::err("Could not get shared memory size", &reason),
    };
    Json(response)
}
//----------------------------------------------------------
// variables

/// This is the structure that will provide the SpecTcl variables
/// that we are able to produce.  The ones that we cannot produce,
/// will be filed in with the string _-undefined-_
///
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct SpectclVariables {
    #[serde(rename = "DisplayMegabytes")]
    display_megabytes: usize,
    #[serde(rename = "OnlineState")]
    online: bool,
    #[serde(rename = "EventListSize")]
    event_list_size: usize,
    #[serde(rename = "ParameterCount")]
    parameter_count: String, // undefined
    #[serde(rename = "SpecTclHome")]
    instdir: String,
    #[serde(rename = "LastSequence")]
    last_seq: String, // undefined
    #[serde(rename = "RunNumber")]
    run_number: String, // undefined
    #[serde(rename = "RunState")]
    run_state: String, // undefined
    #[serde(rename = "DisplayType")]
    display_type: String, // "None"
    #[serde(rename = "BuffersAnalyzed")]
    buffers_analyzed: String, // undefined
    #[serde(rename = "RunTitle")]
    title: String, // undefined.
}
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SpectclVarResult {
    status: String,
    detail: SpectclVariables,
}
pub fn get_instdir() -> String {
    let full_path = env::current_exe().expect("Could not get exe path");
    let dir_name = full_path
        .parent()
        .expect("Could not extract dir from exe path");
    String::from(
        dir_name
            .to_str()
            .expect("Could not convert dir name to string"),
    )
}

const UNDEF: &str = "-undefined-";

/// Return the SpecTcl Variables.
/// Note that some of these have no correpondence in Rustogrammer,
/// those will be given values of _-undefined-_
///
/// ### Parameters
/// * state - the histogram state used to construct or get the APIs we need.
///
/// ### Returns
/// * Json encoded SpectclVariables struct with a bunch of renaming
/// If there are errors, getting this information, the status
/// field will contain full information and the detail field should be
/// ignored.
///
#[get("/variables")]
pub fn get_variables(
    state: &State<SharedProcessingApi>,
    b_state: &State<SharedBinderChannel>,
) -> Json<SpectclVarResult> {
    let shmapi = BindingApi::new(&b_state.inner().lock().unwrap());
    let prcapi = state.inner().lock().unwrap();
    let batching = prcapi.get_batching();
    let mut vars = SpectclVariables {
        display_megabytes: 0,
        online: false,
        event_list_size: batching,
        parameter_count: String::from(UNDEF),
        instdir: get_instdir(),
        last_seq: String::from(UNDEF),
        run_number: String::from(UNDEF),
        run_state: prcapi.processing_state().unwrap(),
        display_type: String::from("None"),
        buffers_analyzed: String::from(UNDEF),
        title: String::from(UNDEF),
    };
    // now fix up the fields we can fix up

    let result = if let Ok(stats) = shmapi.get_usage() {
        vars.display_megabytes = (stats.free_bytes + stats.used_bytes) / (1024 * 1024);
        SpectclVarResult {
            status: String::from("OK"),
            detail: vars,
        }
    } else {
        SpectclVarResult {
            status: String::from("Failed to get the display megabytes from BindingThread"),
            detail: vars,
        }
    };
    // Ok

    Json(result)
}
#[cfg(test)]
mod shm_tests {
    use super::*;
    use crate::messaging;
    use crate::processing;
    use crate::sharedmem::{binder, XamineSharedMemory};
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;

    fn setup() -> Rocket<Build> {
        rest_common::setup().mount("/", routes![shmem_name, shmem_size, get_variables])
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
    ) {
        rest_common::get_state(r)
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        rest_common::teardown(c, p, b);
    }
    #[test]
    fn key_1() {
        // Get the key name and check it should be file://memory_name:

        let rocket = setup();
        let (chan, papi, binder_api) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/key");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Decoding JSON");

        assert_eq!("OK", reply.status);

        // Get the correct key:

        let mem_name = binder_api
            .get_shname()
            .expect("Getting memory name via API");
        assert_eq!(mem_name.as_str(), reply.detail);

        teardown(chan, &papi, &binder_api);
    }
    #[test]
    fn size_1() {
        // get the memory total size and see it's right:

        let rocket = setup();
        let (chan, papi, binder_api) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/size");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Deocding JSON");

        assert_eq!("OK", reply.status);
        let usage = binder_api.get_usage().expect("Getting usage via API");
        let expected = usage.total_size - mem::size_of::<XamineSharedMemory>();
        assert_eq!(expected.to_string(), reply.detail);

        teardown(chan, &papi, &binder_api);
    }
    #[test]
    fn vars_1() {
        // Check the variables.

        let rocket = setup();
        let (chan, papi, binder_api) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Making client");
        let req = client.get("/variables");
        let reply = req
            .dispatch()
            .into_json::<SpectclVarResult>()
            .expect("Parsing json");

        assert_eq!("OK", reply.status);
        let vars = &reply.detail;

        // The  undefined ones:

        assert_eq!(UNDEF, vars.parameter_count);
        assert_eq!(UNDEF, vars.last_seq);
        assert_eq!(UNDEF, vars.run_number);
        assert_eq!("Inactive", vars.run_state);
        assert_eq!(UNDEF, vars.buffers_analyzed);
        assert_eq!(UNDEF, vars.title);

        // Now the ones with values.. which may need us to get
        // the usage:

        let usage = binder_api.get_usage().expect("getting usage via API");
        let batching = papi.get_batching();

        assert_eq!(
            (usage.free_bytes + usage.used_bytes) / (1024 * 1024),
            vars.display_megabytes
        );
        assert!(!vars.online);
        assert_eq!(batching, vars.event_list_size);
        assert_eq!(get_instdir(), vars.instdir);
        assert_eq!("None", vars.display_type);

        teardown(chan, &papi, &binder_api);
    }
}
