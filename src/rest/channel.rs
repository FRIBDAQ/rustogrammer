//!  Implements the /spectcl/channel domain.  Note that in Rustogrammer,
//!  this is not implemented.  While implemented in SpecTcl I believe
//!  it's not used.  If it is required, it is possible to implement it
//!  in the spectrum server code (I think).
//!
//!  We have handlers for
//!
//!  set - sets a channel value.
//!  get - gets a channel value.
//!
//!  Both of these just return a GenericResponse::err.
//!

use rocket::serde::json::Json;
use rocket::State;

use super::*;
use crate::messaging::spectrum_messages;

/// We don't even bother with query parameters.
/// If we implement this the query parameters would be:
///
/// * spectrum (mandatory)- name of the spectrum.
/// * xchannel (mandatory)- xchannel number to set.
/// * ychannel (optional)- y channel number to set.
/// only makes sense for 2 d spectra.  Defaults to 0.0
/// if not supplied.
/// * value - value to set the selected channel to.
///
#[get("/set?<spectrum>&<xchannel>&<ychannel>&<value>")]
pub fn set_chan(
    spectrum: &str,
    xchannel: i32,
    ychannel: Option<i32>,
    value: f64,
    api_chan: &State<SharedHistogramChannel>,
) -> Json<GenericResponse> {
    let api = spectrum_messages::SpectrumMessageClient::new(&api_chan.lock().unwrap());

    let reply = match api.set_channel_value(spectrum, xchannel, ychannel, value) {
        Ok(()) => GenericResponse::ok(""),
        Err(s) => GenericResponse::err("Unable to set channel: ", &s),
    };
    Json(reply)
}
// Stuff needed for getchan:

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ChannelValueResponse {
    status: String,
    detail: f64,
}

/// Implement the channel get functionality:
///
/// *   spectrum (mandatory) - name of the spectrum being queried.
/// *   xchannel (mandatory) - X channel to get
/// *   ychannel (optional) - required only for 2d spectra. The
/// Y channel to get.
///
/// The return value on success would then be
/// *   status : _OK_
/// *   detail : the value in that channel.
///
/// Note that channels out of range would, unlike SpecTcl likely
/// fetch the over/underflow value depending.
///
#[get("/get?<spectrum>&<xchannel>&<ychannel>")]
pub fn get_chan(
    spectrum: &str,
    xchannel: i32,
    ychannel: Option<i32>,
    api_chan: &State<SharedHistogramChannel>,
) -> Json<ChannelValueResponse> {
    let api = spectrum_messages::SpectrumMessageClient::new(&api_chan.lock().unwrap());

    let reply = match api.get_channel_value(spectrum, xchannel, ychannel) {
        Ok(value) => ChannelValueResponse {
            status: String::from("OK"),
            detail: value,
        },
        Err(s) => ChannelValueResponse {
            status: format!("Could not get channel: {}", s),
            detail: 0.0,
        },
    };
    Json(reply)
}

#[cfg(test)]
mod channels_tests {
    use super::*;

    use crate::messaging;
    use crate::messaging::{parameter_messages, spectrum_messages};
    use crate::processing;
    use crate::sharedmem::binder;
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    fn setup() -> Rocket<Build> {
        rest_common::setup().mount("/", routes![set_chan, get_chan])
    }
    fn get_state(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
    ) {
        rest_common::get_state(r)
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        rest_common::teardown(c, p, b);
    }

    #[test]
    fn get_1() {
        // Get from 1d:

        let r = setup();
        let (hg, p, b) = get_state(&r);

        // Make a 1d spectrum - means making a parameter:

        let param_api = parameter_messages::ParameterMessageClient::new(&hg);
        param_api.create_parameter("p0").expect("Making p0");
        let spec_api = spectrum_messages::SpectrumMessageClient::new(&hg);
        spec_api
            .create_spectrum_1d("Test", "p0", 0.0, 1024.0, 1024)
            .expect("Making spectrum");

        let client = Client::untracked(r).expect("Making client");
        let req = client.get("/get?spectrum=Test&xchannel=512");
        let reply = req
            .dispatch()
            .into_json::<ChannelValueResponse>()
            .expect("Parsing json");

        assert_eq!("OK", reply.status);
        assert_eq!(0.0, reply.detail);

        teardown(hg, &p, &b);
    }
    #[test]
    fn get_2() {
        // get from 2d:

        let r = setup();
        let (hg, p, b) = get_state(&r);

        // Make a 1d spectrum - means making a parameter:

        let param_api = parameter_messages::ParameterMessageClient::new(&hg);
        param_api.create_parameter("p0").expect("Making p0");
        param_api.create_parameter("p1").expect("Making p1");
        let spec_api = spectrum_messages::SpectrumMessageClient::new(&hg);
        spec_api
            .create_spectrum_2d("test", "p0", "p1", 0.0, 512.0, 512, 0.0, 512.0, 512)
            .expect("Making spectrum");

        let client = Client::untracked(r).expect("Making client");
        let req = client.get("/get?spectrum=test&xchannel=100&ychannel=100");
        let reply = req
            .dispatch()
            .into_json::<ChannelValueResponse>()
            .expect("Parsing json");

        assert_eq!("OK", reply.status);
        assert_eq!(0.0, reply.detail);

        teardown(hg, &p, &b);
    }
    #[test]
    fn set_1() {
        // Set a good 1d channel:

        let r = setup();
        let (hg, p, b) = get_state(&r);

        // make a 1d spectrum:

        let param_api = parameter_messages::ParameterMessageClient::new(&hg);
        param_api.create_parameter("p1").expect("Making parameter");
        let spec_api = spectrum_messages::SpectrumMessageClient::new(&hg);
        spec_api
            .create_spectrum_1d("test", "p1", 0.0, 1024.0, 1024)
            .expect("Making spectrum");

        let client = Client::untracked(r).expect("Making client");
        let req = client.get("/set?spectrum=test&xchannel=512&value=100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        // Check the value:

        assert_eq!(
            100.0,
            spec_api
                .get_channel_value("test", 512, None)
                .expect("Getting value")
        );

        teardown(hg, &p, &b);
    }
    #[test]
    fn set_2() {
        // From 2d spectrum:

        let r = setup();
        let (hg, p, b) = get_state(&r);

        // Make a 1d spectrum - means making a parameter:

        let param_api = parameter_messages::ParameterMessageClient::new(&hg);
        param_api.create_parameter("p0").expect("Making p0");
        param_api.create_parameter("p1").expect("Making p1");
        let spec_api = spectrum_messages::SpectrumMessageClient::new(&hg);
        spec_api
            .create_spectrum_2d("test", "p0", "p1", 0.0, 512.0, 512, 0.0, 512.0, 512)
            .expect("Making spectrum");

        let client = Client::untracked(r).expect("Making client");
        let req = client.get("/set?spectrum=test&xchannel=256&ychannel=256&value=200");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        assert_eq!(
            200.0,
            spec_api
                .get_channel_value("test", 256, Some(256))
                .expect("getting value")
        );

        teardown(hg, &p, &b);
    }
}
