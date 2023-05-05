//! Supports the /spectcl/exit URL.
//! this provides the ability to, in order:
//!
//! * Shutdown the rustogramer threads via their APIs.
//! * Reply to the caller that, yes we're shutting down.
//! * Notify rocket that when the request is complete it too should
//! shutdown which, in turn exits th main program.
//!

use rocket::serde::{json::Json, Serialize};
use rocket::Shutdown;

use super::*; // For generic response.
use crate::histogramer;
use crate::sharedmem::binder::BindingApi;

/// This performs the shutdown:
///
#[get["/"]]
pub fn shutdown(shutdown: Shutdown, state: &State<HistogramState>) -> Json<GenericResponse> {
    // Shutdown the processor:

    let prc_api = state.inner().processing.lock().unwrap();
    prc_api
        .stop_thread()
        .expect("Failed to stop processing thread!");

    // Shutdown the shared memory program.

    let bind_api = BindingApi::new(&state.inner().binder.lock().unwrap().0);
    bind_api.exit().expect("Unable to stop the bind thread");

    // Shutdown the histogrammer

    let hg = state.inner().state.lock().unwrap();
    histogramer::stop_server(&hg.1);

    //  Tell rocket to shutdown when processing of all requests is complete:
    shutdown.notify();
    Json(GenericResponse::ok("")) // Client may not get this.
}
