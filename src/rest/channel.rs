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

use super::*;

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
#[get("/set")]
pub fn set_chan() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Unsupported /spectcl/channel/set",
        "This is not SpecTcl",
    ))
}
/// If this were implemented,
/// the query paramters would be:
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
#[get("/get")]
pub fn get_chan() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Unsupported /spectcl/channel/get",
        "This is not SpecTcl",
    ))
}

#[cfg(test)]
mod channels_tests {
    use super::*;

    use crate::messaging;
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
    fn set_1() {
        let r = setup();
        let (hg, p, b) = get_state(&r);

        let c = Client::tracked(r).expect("Failed to make client");
        let request = c.get("/set");
        let reply = request.dispatch();
        let json = reply
            .into_json::<GenericResponse>()
            .expect("bad JSON parse");
        assert_eq!("Unsupported /spectcl/channel/set", json.status.as_str());
        assert_eq!("This is not SpecTcl", json.detail.as_str());

        teardown(hg, &p, &b);
    }

    #[test]
    fn get_1() {
        let r = setup();
        let (hg, p, b) = get_state(&r);

        let c = Client::tracked(r).expect("Failed to make client");
        let request = c.get("/get");
        let reply = request.dispatch();
        let json = reply
            .into_json::<GenericResponse>()
            .expect("bad JSON parse");
        assert_eq!("Unsupported /spectcl/channel/get", json.status.as_str());
        assert_eq!("This is not SpecTcl", json.detail.as_str());

        teardown(hg, &p, &b);
    }
}
