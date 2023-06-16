//!  Implements the /spectcl/specstats operation.
//!  See the get_statisics function below.
//!

use super::*;
use crate::messaging::spectrum_messages;
use rocket::serde::{json::Json, Serialize};
use rocket::State;

///  Spectrum statistics are in the following struct
///
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SpectrumStatistics {
    name: String,
    underflows: [u32; 2],
    overflows: [u32; 2],
}
/// This is turned into Json for the response:

#[derive(Serialize)]
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
