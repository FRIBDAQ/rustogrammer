//!  This module provides rest interfaces that bear on the
//! Xamine compatible shared memory mapping.
//! These include domains:
//!
//! * /spectcl/shm - Gets the shared memory information.
use super::*;
use crate::sharedmem::binder::BindingApi;
use rocket::{serde::json::Json, serde::Serialize, State};
use std::env;

//----------------------------------------------------------------
// /spectcl/shm domain implementation:
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
pub fn shmem_name(state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = BindingApi::new(&state.inner().binder.lock().unwrap().0);
    Json(match api.get_shname() {
        Ok(name) => GenericResponse::ok(&name),
        Err(reason) => GenericResponse::err("Failed to get shared memory name", &reason),
    })
}
//------------------------------------------------------------
// status

/// Returns the size of the shared memory region in the
/// status as a string.  This is the total size of the shared
/// memory region in bytes (not the size of the spectrum pool which is
/// what's used to instantiate the shared memory region to begin with)
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
pub fn shmem_size(state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = BindingApi::new(&state.inner().binder.lock().unwrap().0);
    let info = api.get_usage();
    let response = match info {
        Ok(stats) => GenericResponse::ok(&(stats.total_size.to_string())),
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
#[derive(Serialize)]
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
#[derive(Serialize)]
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
pub fn get_variables(state: &State<HistogramState>) -> Json<SpectclVarResult> {
    let shmapi = BindingApi::new(&state.inner().binder.lock().unwrap().0);
    let prcapi = state.inner().processing.lock().unwrap();
    let batching = prcapi.get_batching();
    let mut vars = SpectclVariables {
        display_megabytes: 0,
        online: false,
        event_list_size: batching,
        parameter_count: String::from(UNDEF),
        instdir: get_instdir(),
        last_seq: String::from(UNDEF),
        run_number: String::from(UNDEF),
        run_state: String::from(UNDEF),
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
