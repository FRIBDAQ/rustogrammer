//!  This module will collect code common
//!  to the tests.  It is also possible that
//!  At some point, if the test code within source
//!  modules becomes too distracting, the tests themselves
//!  will become submodules here.

#[cfg(test)]

pub mod rest_common {
    use crate::histogramer;
    use crate::messaging;
    use crate::processing;
    use crate::rest::{
        MirrorState, SharedBinderChannel, SharedHistogramChannel, SharedProcessingApi,
    };
    use crate::sharedmem::binder;
    use crate::trace;

    use rocket;
    use rocket::Build;
    use rocket::Rocket;

    use std::fs;
    use std::path::Path;
    use std::sync::{mpsc, Arc, Mutex};
    use std::thread;
    use std::time;

    /// Sets up the state and rocket.
    /// The caller must still mount the appropriate set of
    /// routes.
    ///
    pub fn setup() -> Rocket<Build> {
        let tracedb = trace::SharedTraceStore::new();
        let (_, hg_sender) = histogramer::start_server(tracedb.clone());
        let (binder_req, _jh) = binder::start_server(&hg_sender, 32 * 1024 * 1024, &tracedb);

        let state = MirrorState {
            mirror_exit: Arc::new(Mutex::new(mpsc::channel::<bool>().0)),
            mirror_port: 0,
        };
        rocket::build()
            .manage(state)
            .manage(Mutex::new(hg_sender.clone()))
            .manage(Mutex::new(binder_req))
            .manage(Mutex::new(processing::ProcessingApi::new(
                &hg_sender.clone(),
            )))
            .manage(tracedb.clone())
    }
    /// Teardown the infrastructure that was created by the
    /// setup function:
    pub fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        let backing_file = b.exit().expect("Forcing binding thread to exit");
        thread::sleep(time::Duration::from_millis(100));
        let _ = fs::remove_file(Path::new(&backing_file)); // faliure is ok.

        p.stop_thread().expect("Stopping processing thread");
        histogramer::stop_server(&c);
    }
    pub fn get_state(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
    ) {
        let chan = r
            .state::<SharedHistogramChannel>()
            .expect("Valid state")
            .lock()
            .unwrap()
            .clone();
        let papi = r
            .state::<SharedProcessingApi>()
            .expect("Valid State")
            .lock()
            .unwrap()
            .clone();
        let binder_api = binder::BindingApi::new(
            &r.state::<SharedBinderChannel>()
                .expect("Valid State")
                .lock()
                .unwrap(),
        );
        (chan, papi, binder_api)
    }
}
// Common test code for tests that need a histogramer active:
#[cfg(test)]
pub mod histogramer_common {
    use crate::histogramer;
    use crate::messaging;
    use crate::trace;

    use std::sync::mpsc;
    use std::thread;

    pub fn setup() -> (mpsc::Sender<messaging::Request>, thread::JoinHandle<()>) {
        let (jh, send) = histogramer::start_server(trace::SharedTraceStore::new());
        (send, jh)
    }
    pub fn teardown(ch: mpsc::Sender<messaging::Request>, jh: thread::JoinHandle<()>) {
        histogramer::stop_server(&ch);
        jh.join().unwrap();
    }
}
