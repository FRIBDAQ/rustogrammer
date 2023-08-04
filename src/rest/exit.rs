//! Supports the /spectcl/exit URL.
//! this provides the ability to, in order:
//!
//! * Shutdown the rustogramer threads via their APIs.
//! * Reply to the caller that, yes we're shutting down.
//! * Notify rocket that when the request is complete it too should
//! shutdown which, in turn exits th main program.
//!

use rocket::serde::json::Json;
use rocket::Shutdown;

use super::*; // For generic response.
use crate::histogramer;
use crate::sharedmem::binder::BindingApi;
use crate::trace;
use std::fs;
use std::net::TcpStream;
use std::path::Path;
use std::thread;
use std::time;

/// This performs the shutdown:
///
#[get["/"]]
pub fn shutdown(
    shutdown: Shutdown,
    state: &State<MirrorState>,
    hg_chan: &State<SharedHistogramChannel>,
    b_chan: &State<SharedBinderChannel>,
    p_api: &State<SharedProcessingApi>,
    tracedb: &State<trace::SharedTraceStore>,
) -> Json<GenericResponse> {
    // Stop the trace prune thread (or rather schedule it to stop - within
    // one second it will stop).

    tracedb.inner().stop_prune();

    // Shutdown the processor:

    let prc_api = p_api.inner().lock().unwrap();
    if let Err(s) = prc_api.stop_thread() {
        println!(
            "Note failed to stop processing thread -might have already stopped {}",
            s
        );
    }
    // Kill off the mirror server:...again ignore errors.

    let _ = state.inner().mirror_exit.lock().unwrap().send(true); // ignore errors;
    let _ = TcpStream::connect(&format!("127.0.0.1:{}", state.inner().mirror_port));

    // Shutdown the shared memory program.

    let bind_api = BindingApi::new(&b_chan.inner().lock().unwrap());

    match bind_api.exit() {
        Ok(s) => {
            // Let the thread exit first...
            thread::sleep(time::Duration::from_millis(500));
            if let Err(e) = fs::remove_file(Path::new(&s)) {
                println!("Failed to remove shared memory backing store {}: {}", s, e);
            }
        }
        Err(s) => {
            println!(
                "Note failed to stop shared memory thread - might have already stopped {}",
                s
            );
        }
    }

    // Shutdown the histogrammer

    let hg = hg_chan.inner().lock().unwrap();
    histogramer::stop_server(&hg);

    //  Tell rocket to shutdown when processing of all requests is complete:
    shutdown.notify();
    Json(GenericResponse::ok("")) // Client may not get this.
}
