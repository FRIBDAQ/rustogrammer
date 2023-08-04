//!  Implements the /spectcl/fit domain of URIs.
//!  This entire domain is implemented to return to the
//!  client that this functionality is not supported by Rustogramer.
//!  With the development of the, e.g. CutiePie, visualizer,
//!  it seems that doing fitting in the visualizer is the appropriate
//!  place for this functionality.
//!
//!  The /spectcl/fit domain has the following URIs that will
//!  have handlers:
//!
//!  *  create - creates a new fit object.
//!  *  update - Update fit parameters based on current data.
//!  *  delete - Delete a fit object.
//!  *  list   - list the fit objects that exist.
//!  *  proc   - Returns the name of the fit proc associated with the fit.
//! (In SpecTcl this allowed evaulation of the fit).
//!  
use super::*;
use rocket::serde::{json::Json, Deserialize, Serialize};

/// create - create a new fit object. (unimplemented).
/// If this is implemented the following query parameters
/// would required:
///
///  name - name of the fit object.
///  spectrum - Name of the spectrum on which the fit is evaulated.
///  low  - Low channel limit of the fitted region.
///  high - high channel limit of the fitted region.
///  type - Type of the fit (e.g. 'gaussian')
///
/// Idf implemented a GenericResponse is perfectly appropriate.
///
#[get("/create")]
pub fn create() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/fit/create is not supported",
        "This is not Spectcl",
    ))
}

/// update (not implemented) Give a set of fits that match a pattern,
/// the fit paramaeters are re-computed using the current spectrum
/// data.  The concept is that as the data are processed,fit parameters
/// will shift both because
///
/// * Additional statisitcs may shift slightly the fit parameters.
/// * After clearing the spectra and attaching a different data file,
/// the data could change significantly (consider an experimental
/// data set that includes an energy scan or multiple beam species
/// for example).
///
/// The query parameter that would be accepted if implemented would be
/// _pattern_ which is a glob pattern.  Fits with matching names
/// only will be recomputed.
///
#[get("/update")]
pub fn update() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/fit/update is not supported",
        "This is not Spectcl",
    ))
}

/// delete (unimplemented)
/// Deletes an existing fit object.  The only query parameter is
/// _name_ which specifies the the name of the fit to delete.
///
/// A GenericResponse is perfectly ok for any future implementation
/// of this URI.
///
#[get("/delete")]
pub fn delete() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/fit/delete is not supported",
        "This is not Spectcl",
    ))
}
//
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct FitParameter {
    name: String,
    value: f64,
}
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct FitDescription {
    name: String,
    spectrum: String,
    r#type: String,
    low: f64,
    high: f64,
    parameters: Vec<FitParameter>,
}
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct FitListReply {
    status: String,
    detail: Vec<FitDescription>,
}

///
/// list (unimplemented).
///
/// Lists the set of fits that match the optional _pattern_ query
/// parameter (defaults to "*").  The returned reply will be of the
/// form described by FitListReply above.  Note that the
/// FitParameter is different from what SpecTcl would produce which
/// is just a set of name/value pairs... which I don't quite know how
/// to produce (Maybe a tuple would be better?).
///
/// Not important at this time since we're going to
/// return an unimplemented reply.'
///
#[get("/list")]
pub fn list() -> Json<FitListReply> {
    Json(FitListReply {
        status: String::from("/spectcl/fit/list is not implemented - this is not SpecTcl"),
        detail: vec![],
    })
}
///
/// proc (unimplemented)
///
/// This would, in SpecTcl return the name of a proc that can be
/// invoked to evaulate a fit at a specific channel.  This would be
/// done bia the script interface.
///
/// A GenericResponse is fine if implemented as the detail is just
/// the string proc name.
///
#[get("/proc")]
pub fn proc() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "/spectcl/fit/proc is not supported",
        "This is not Spectcl",
    ))
}

#[cfg(test)]
mod fit_tests {
    use super::*;
    use crate::histogramer;
    use crate::messaging;
    use crate::processing;
    use crate::sharedmem::binder;
    use crate::trace;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::{mpsc, Arc, Mutex};
    // note these are all unimplemented URLS so...

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
            .mount("/", routes![create, update, delete, list, proc])
    }
    fn teardown(c: mpsc::Sender<messaging::Request>, p: &processing::ProcessingApi) {
        histogramer::stop_server(&c);
        p.stop_thread().expect("Stopping processing thread");
    }
    fn get_state(
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
    fn create_1() {
        let rocket = setup();
        let (r, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Failed to make client");
        let req = client.get("/create");
        let response = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Bad JSON");

        assert_eq!(
            "/spectcl/fit/create is not supported",
            response.status.as_str()
        );
        assert_eq!("This is not Spectcl", response.detail.as_str());

        teardown(r, &papi);
    }
    #[test]
    fn update_1() {
        let rocket = setup();
        let (r, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Failed to make client");
        let req = client.get("/update");
        let response = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Bad JSON");

        assert_eq!(
            "/spectcl/fit/update is not supported",
            response.status.as_str()
        );
        assert_eq!("This is not Spectcl", response.detail.as_str());

        teardown(r, &papi);
    }
    #[test]
    fn delete_1() {
        let rocket = setup();
        let (r, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Failed to make client");
        let req = client.get("/delete");
        let response = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Bad JSON");

        assert_eq!(
            "/spectcl/fit/delete is not supported",
            response.status.as_str()
        );
        assert_eq!("This is not Spectcl", response.detail.as_str());

        teardown(r, &papi);
    }
    #[test]
    fn list_1() {
        let rocket = setup();
        let (r, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Failed to make client");
        let req = client.get("/list");
        let response = req
            .dispatch()
            .into_json::<FitListReply>()
            .expect("Bad JSON");

        assert_eq!(
            "/spectcl/fit/list is not implemented - this is not SpecTcl",
            response.status.as_str()
        );
        assert_eq!(0, response.detail.len());

        teardown(r, &papi);
    }
    #[test]
    fn proc_1() {
        let rocket = setup();
        let (r, papi) = get_state(&rocket);

        let client = Client::tracked(rocket).expect("Failed to make client");
        let req = client.get("/proc");
        let response = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Bad JSON");

        assert_eq!(
            "/spectcl/fit/proc is not supported",
            response.status.as_str()
        );
        assert_eq!("This is not Spectcl", response.detail.as_str());

        teardown(r, &papi);
    }
}
