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
    use crate::rest::MirrorState;
    use crate::sharedmem::binder;
    use crate::trace;
    use rocket;
    use rocket::Build;
    use rocket::Rocket;
    use std::sync::{mpsc, Arc, Mutex};
    /// Sets up the state and rocket.
    /// The caller must still mount the appropriate set of
    /// routes.
    ///
    pub fn setup() -> Rocket<Build> {
        let tracedb = trace::SharedTraceStore::new();
        let (_, hg_sender) = histogramer::start_server(tracedb.clone());
        let (binder_req, _rx): (
            mpsc::Sender<binder::Request>,
            mpsc::Receiver<binder::Request>,
        ) = mpsc::channel();
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
    pub fn teardown(c: mpsc::Sender<messaging::Request>, p: &processing::ProcessingApi) {
        histogramer::stop_server(&c);
        p.stop_thread().expect("Stopping processing thread");
    }
}
