//!  This module provides the REST interface to the procesing
//!  thread.  The assumption is that he field _processing_ in the
//!  HistogramState object contains a Mutex wrapped
//!  ProcessingApi object, and the analysis thread has already
//!  been started.
//!  
//! Two mount points are provided:
//!  
//!  *  /attach which provides the attach, detach and list methods.
//!  *  /analyze which provides the start, stop and eventchunk
//! methods.

// Imports:

use rocket::serde::json::Json;
use rocket::State;

use super::*;

//---------------------------------------------------------------
// The /attach mount point:

/// Attach a data source.
/// Note that this version of rustogrammer only support type=file
/// Query parameters:
///
/// *  type - the type of attach (file is the only one supported).
/// *  source - in this case the name of the data file to attach.
/// *  size (ignored) - for compatiblity with SpecTcl's API.
///
/// The response is a generic resposne with the detail empty on
/// success and containing more detailed error message on failure
/// than that in status.
#[allow(unused_variables)]
#[get("/attach?<type>&<source>&<size>")]
pub fn attach_source(
    r#type: String,
    source: String,
    size: OptionalString,
    state: &State<HistogramState>,
) -> Json<GenericResponse> {
    let reply = if r#type == "file" {
        let api = state.inner().processing.lock().unwrap();
        if let Err(s) = api.attach(&source) {
            GenericResponse::err("Attach failed", &s)
        } else {
            GenericResponse::ok("")
        }
    } else {
        GenericResponse::err(
            &format!("Data source type '{}' is not supported", r#type),
            "This is Rustogramer not SpecTcl",
        )
    };
    Json(reply)
}

/// list the current data source.
/// this has no query parameters:
///  On success, detail contains the data source.
///  on failure, the error from the api.
#[get("/list")]
pub fn list_source(state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = state.inner().processing.lock().unwrap();
    Json(match api.list() {
        Ok(s) => GenericResponse::ok(&s),
        Err(s) => GenericResponse::err("Failed to get data source", &s),
    })
}
/// Detach from the current data source.
///  This is specific to Rustogramer.
///
#[get("/detach")]
pub fn detach_source(state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = state.inner().processing.lock().unwrap();
    Json(match api.detach() {
        Ok(s) => GenericResponse::ok(&s),
        Err(s) => GenericResponse::err("Failed to detach", &s),
    })
}

//--------------------------------------------------------------
// The /analyze mount point.
//

/// start - starts analyzing data on the currently attached
/// data source.  No query parameters are required/accepted.
#[get("/start")]
pub fn start_processing(state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = state.inner().processing.lock().unwrap();
    Json(match api.start_analysis() {
        Ok(_) => GenericResponse::ok(""),
        Err(s) => GenericResponse::err("Failed to start analysis", &s),
    })
}
///
/// stop stops analyzing data on the current data source.
/// No query parameters are required.
///
#[get("/stop")]
pub fn stop_processing(state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = state.inner().processing.lock().unwrap();
    Json(match api.stop_analysis() {
        Ok(_) => GenericResponse::ok(""),
        Err(s) => GenericResponse::err("Failed to stop analysis", &s),
    })
}
/// Set the analysis block size.  This is the number of events that
/// will be sent to the histograming thread for each analysis request.
///
/// The query parameter _events_ must be the number of events.
///
#[get("/size?<events>")]
pub fn set_event_batch(events: usize, state: &State<HistogramState>) -> Json<GenericResponse> {
    let mut api = state.inner().processing.lock().unwrap();
    Json(match api.set_batching(events) {
        Ok(_) => GenericResponse::ok(""),
        Err(s) => GenericResponse::err("Failed to set event processing batch size", &s),
    })
}
