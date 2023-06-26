//!  Implements the /spectcl/specstats operation.
//!  See the get_statisics function below.
//!

use super::*;
use crate::messaging::spectrum_messages;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;

///  Spectrum statistics are in the following struct
///
#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct SpectrumStatistics {
    name: String,
    underflows: [u32; 2],
    overflows: [u32; 2],
}
/// This is turned into Json for the response:

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct SpectrumStatisticsReply {
    status: String,
    detail: Vec<SpectrumStatistics>,
}

///  Process the /spectcl/specstats RES method.
///  - Enumerate the spectra that match the pattern.
///  - For each of thise get statisics and add it to the
///set of statistics entry in the reply.
///
/// ### Parameters
/// *  pattern :  Glob pattern, we get statistics for each spectrum whose name
/// matches _pattern_
/// *  state : The REST server state object which includes the
/// request channel needed to build an API Object.
/// ### Returns:
/// * JSON encoded SpectrumStatisticsReply.  On success, status is _OK_ on failure
/// it is an error nessage describing the problem.
/// ### Note:
///  Because the operation of enumerating matching spectra and getting their
/// statistics is not atomic (thing multiple server threads e.g.),
/// we just omit failed statistics responses from the output.
///
#[get("/?<pattern>")]
pub fn get_statistics(
    pattern: OptionalString,
    state: &State<HistogramState>,
) -> Json<SpectrumStatisticsReply> {
    let pat = if let Some(p) = pattern {
        p
    } else {
        String::from("*")
    };

    let api =
        spectrum_messages::SpectrumMessageClient::new(&state.inner().histogramer.lock().unwrap());
    let spectra = api.list_spectra(&pat);
    if let Err(s) = spectra {
        return Json(SpectrumStatisticsReply {
            status: format!("Failed to get spectrum list for {} : {}", pat, s),
            detail: vec![],
        });
    }
    let spectra = spectra.unwrap();
    let mut response = SpectrumStatisticsReply {
        status: String::from("OK"),
        detail: vec![],
    };
    for s in spectra {
        let stats = api.get_statistics(&s.name);
        if let Ok(st) = stats {
            response.detail.push(SpectrumStatistics {
                name: s.name.clone(),
                underflows: [st.0, st.1],
                overflows: [st.2, st.3],
            });
        }
    }

    Json(response)
}

#[cfg(test)]
mod getstats_tests {
    use super::*;
    use crate::histogramer;
    use crate::messaging;
    use crate::messaging::{parameter_messages, spectrum_messages};
    use crate::processing;
    use crate::sharedmem::binder;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;
    use std::sync::Mutex;

    fn setup() -> Rocket<Build> {
        let (_, hg_sender) = histogramer::start_server();
        let (binder_req, _rx): (
            mpsc::Sender<binder::Request>,
            mpsc::Receiver<binder::Request>,
        ) = mpsc::channel();

        // Construct the state:

        let state = HistogramState {
            histogramer: Mutex::new(hg_sender.clone()),
            binder: Mutex::new(binder_req),
            processing: Mutex::new(processing::ProcessingApi::new(&hg_sender)),
            portman_client: None,
        };

        // Create a pair of parmaeters, p1, p2 and a pair of histograms
        // we can play with in the tests:

        let param_api = parameter_messages::ParameterMessageClient::new(&hg_sender);
        param_api
            .create_parameter("p1")
            .expect("Creating parameter p1"); // id 1
        param_api
            .create_parameter("p2")
            .expect("Creating parameter p2"); // id 2

        let hist_api = spectrum_messages::SpectrumMessageClient::new(&hg_sender);
        hist_api
            .create_spectrum_1d("p1", "p1", 0.0, 1024.0, 1024)
            .expect("Creating spectrum p1");
        hist_api
            .create_spectrum_2d("2", "p1", "p2", 0.0, 1024.0, 1024, 0.0, 1024.0, 1024)
            .expect("Creating spectrum 2");

        // finally start rocket:

        rocket::build()
            .manage(state)
            .mount("/", routes![get_statistics])
    }
    fn teardown(c: mpsc::Sender<messaging::Request>, p: &processing::ProcessingApi) {
        histogramer::stop_server(&c);
        p.stop_thread().expect("Stopping processing thread");
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (mpsc::Sender<messaging::Request>, processing::ProcessingApi) {
        let chan = r
            .state::<HistogramState>()
            .expect("Valid state")
            .histogramer
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
    fn sortdetail(inp : &Vec<SpectrumStatistics>) -> Vec<SpectrumStatistics> {
        let mut result = inp.clone();
        result.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
        result
    }
    #[test]
    fn getstats_1() {
        // With no counts and no pattern, I get both spectra and ther are
        // neither under nor overflows.

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let request = client.get("/");
        let reply = request
            .dispatch()
            .into_json::<SpectrumStatisticsReply>()
            .expect("Parsing Json");

        assert_eq!("OK", reply.status);
        assert_eq!(2, reply.detail.len());

        // Get the results sorted by name:
        // 2 then p1:
        let detail = sortdetail(&reply.detail);
        assert_eq!("2", detail[0].name);
        assert_eq!(vec![0,0], detail[0].underflows);
        assert_eq!(vec![0,0], detail[0].overflows);

        assert_eq!("p1", detail[1].name);
        assert_eq!(vec![0,0], detail[1].underflows);
        assert_eq!(vec![0,0], detail[1].overflows);
        

        teardown(c, &papi);
    }
    #[test]
    fn getstats_2() {
        // No counts but a filter pattern:

        let rocket = setup();
        let (c, papi) = getstate(&rocket);

        let client = Client::tracked(rocket).expect("Creating client");
        let request = client.get("/?pattern=p*");  // gets p1
        let reply = request
            .dispatch()
            .into_json::<SpectrumStatisticsReply>()
            .expect("Parsing Json");

        assert_eq!("OK", reply.status);
        assert_eq!(1, reply.detail.len());

        let detail = &reply.detail[0];  // for notational brevity.
        assert_eq!("p1", detail.name);
        assert_eq!(vec![0,0], detail.underflows);
        assert_eq!(vec![0,0], detail.overflows);

        teardown(c, &papi);
    }
}
