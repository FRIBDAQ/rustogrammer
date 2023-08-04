//! 'Implements' the /spectcl/integrate domain.
//! This is not implemented in this version of Rustogramer.
//! This is a candidate for implementation in a future release
//! *However*, it seems to me that this functionality really belongs
//!  in a displayer such as CutiePie where users can interact with
//!  a visualization of the spectrum to peform their integrations.
//!
//!  There is only /spectcl/integrate, nothing underneath it.
//!
use rocket::serde::{json::Json, Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct IntegrationDetail {
    centroid: f64,
    fwhm: f64,
    counts: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct IntegrationResponse {
    status: String,
    detail: IntegrationDetail,
}
/// integrate (unimplemented)
///
/// When implemented this would accept the following
/// query parameters depending on the type of integration being performed
///
/// * spectrum (mandatory) - The spectrum to be integrated.
/// * gate (optional) - If the gate can appear drawn on the spectrum,
/// the integration will be over the interior of the gate.
/// * low - If the spectrum is one dimensional and the integration is
/// not in a gate this is the low limit of the range of channels
/// over which to integrate.
/// * high - if the spectrum is 1d the high limit over which to integerate.
/// * xcoord - If the
/// integration is not in a gate and in a 2d spectrum, these are
/// the X coordinates of a contour within which an integration is performed.
/// * ycoord - if the integrations is not in a gate and  in a 2d spectrum,
/// these are the set of y coordinates of points that describe the
/// contour within which the integration will be done.
///
/// The reply is an IntegrationResponse.
///
#[get("/")]
pub fn integrate() -> Json<IntegrationResponse> {
    Json(IntegrationResponse {
        status: String::from("/spectcl/integrate is not supported - this is not SpecTcl"),
        detail: IntegrationDetail {
            centroid: 0.0,
            fwhm: 0.0,
            counts: 0,
        },
    })
}
// Placeholder for tests if/when this is supported:

#[cfg(test)]
mod integrate_tests {
    use super::*;
    use crate::histogramer;
    use crate::messaging;
    use crate::processing;
    use crate::rest::{HistogramState, SharedHistogramChannel};
    use crate::sharedmem::binder;
    use crate::trace;
    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::{mpsc, Arc, Mutex};

    fn setup() -> Rocket<Build> {
        let tracedb = trace::SharedTraceStore::new();
        let (_, hg_sender) = histogramer::start_server(tracedb.clone());
        let (binder_req, _rx): (
            mpsc::Sender<binder::Request>,
            mpsc::Receiver<binder::Request>,
        ) = mpsc::channel();

        // Construct the state:

        let state = HistogramState {
            binder: Mutex::new(binder_req),
            processing: Mutex::new(processing::ProcessingApi::new(&hg_sender)),
            portman_client: None,
            mirror_exit: Arc::new(Mutex::new(mpsc::channel::<bool>().0)),
            mirror_port: 0,
        };

        rocket::build()
            .manage(state)
            .manage(Mutex::new(hg_sender.clone()))
            .manage(tracedb.clone())
            .mount("/", routes![integrate])
    }
    fn teardown(c: mpsc::Sender<messaging::Request>, p: &processing::ProcessingApi) {
        histogramer::stop_server(&c);
        p.stop_thread().expect("Stopping processing thread");
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (mpsc::Sender<messaging::Request>, processing::ProcessingApi) {
        let chan = r
            .state::<SharedHistogramChannel>()
            .expect("Valid state")
            .lock()
            .unwrap()
            .clone();
        let papi = r
            .state::<HistogramState>()
            .expect("Valid State")
            .processing
            .lock()
            .unwrap()
            .clone();

        (chan, papi)
    }
    #[test]
    fn integrate_1() {
        // Make the request...

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let request = client.get("/");
        let response = request
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("parsing JSON");

        assert_eq!(
            "/spectcl/integrate is not supported - this is not SpecTcl",
            response.status
        );

        teardown(c, &papi);
    }
}
