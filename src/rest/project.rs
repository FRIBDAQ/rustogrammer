//!  This ReST interface implements projection spectra.
//!  In fact, projection spectra are no different than
//!  ordinary spectra.  A projection just describes how they
//!  come into being and get their initial contents.
//!
//!  Only spectra with 'sensible' underlying 2d histoigrams can
//!  be projected and these include:
//!  *  Ordinary 2-d spectra.
//!  *  Multiply incremented 2-d spectra (g2 in SpecTcl parlance).
//!  *  Particle Gamma spectra (gd in SpecTcl parlance).
//!  *  Twod sum spectra (m2 in SpecTcl parlance).
//!
//!  The resulting spectrum is populated by summing rows or columns
//!  In the direction other than the projection direction.  For example,
//!  In the case of an X projectsion, columns (y axis) are summed
//!  To project the image down onto the  X axis. Similarly, in the case of a
//!  Y projection, rows (x  axis) are summed onto the Y axis.  The resulting
//!  spectrum type depends on the original spectrum:
//!
//!  * 2d -> A 1d spectrum on the parameter of the appropriate axis
//! (e.g x parameter for x projection)
//!  * 2dmulti -> A multi1d spectrum on all of the parameters of the original
//! spectrum.
//!  * pgamma -> A 1d multi spectrum - The parameters are the parameters appropriate to
//! the axis but each parameter is repeated once per opposite parameter.  For example,
//! Suppose the original spectrum has 3 y parameters and we project in X,  The resultiing
//! Spectrum has the original X parameters but repeated 3 times (to maintain the appropriate projection).
//! * twodsum - a 1d multi spectrum.  The parmaeters are the parameters of the approprite axis.  For
//! example an X projection will have all of the X parameters of the original spectrum.
//!
//!  Projections can also be within any contour that can be displayed on the underlying spectrum.
//!
//!  The final gate of the spectrum will depend:
//!
//! * Snapshot spectra get the false gate _projection_snapshot_condition which, if necessary is created.
//! * Ungated  source spectra result in ungated non-snapshot projections... but see below:
//! * Gated Source spectra result in non-snapshots gated on the same condition that gates the original spectrum
//! * Spectra projected within a contour will retain that contour as a gate.  If the source spectrum
//! is gated, the gate used is the And of the original spectrum's gate and the contour.
//!

use super::*;
use rocket::serde::json::Json;
use rocket::State;

use crate::messaging::{condition_messages, spectrum_messages};
use crate::projections;
use crate::sharedmem::binder;
//------------------------------------------------------------------
// project:
#[allow(unused_variables)]
#[get("/?<snapshot>&<source>&<newname>&<direction>&<contour>&<bind>")]
pub fn project(
    snapshot: String,
    source: String,
    newname: String,
    direction: String,
    contour: OptionalString,
    bind: OptionalFlag,
    hgchannel: &State<SharedHistogramChannel>,
    bchannel: &State<SharedBinderChannel>,
) -> Json<GenericResponse> {
    // Make the spectrum and condition APIs:

    let sapi = spectrum_messages::SpectrumMessageClient::new(&(hgchannel.inner().lock().unwrap()));
    let capi =
        condition_messages::ConditionMessageClient::new(&(hgchannel.inner().lock().unwrap()));

    // Figure out direction:

    let projection_direction = match direction.as_str() {
        "x" | "X" => projections::ProjectionDirection::X,
        "y" | "Y" => projections::ProjectionDirection::Y,
        _ => {
            return Json(GenericResponse::err(
                "Invalid projection direction",
                "Must be 'X' or 'Y'",
            ));
        }
    };
    // Snapshot text to bool:

    let snapshot = match snapshot.as_str() {
        "Yes" | "yes" | "True" | "true" => true,
        "No" | "no" | "False" | "false" => false,
        _ => {
            return Json(GenericResponse::err(
                "Invalid value for 'snapshot'",
                "Must be in {yes, no, true, false} e.g.",
            ));
        }
    };

    // Can we make the spectrum?

    let mut reply = if let Err(s) = projections::project(
        &sapi,
        &capi,
        &source,
        projection_direction,
        &newname,
        snapshot,
        contour,
    ) {
        GenericResponse::err("Failed to crate projection spectrum", &s)
    } else {
        GenericResponse::ok("")
    };
    // On success, bind if requested:

    if "OK" == reply.status.as_str() {
        let do_bind = if let Some(b) = bind {
            b
        } else {
            false // SpecTcl does not support this flag and does not bind
        };
        if do_bind {
            let bapi = binder::BindingApi::new(&bchannel.inner().lock().unwrap());
            reply = match bapi.bind(&newname) {
                Ok(()) => GenericResponse::ok(""),
                Err(s) => GenericResponse::err("Could not bind projected spectrum", &s),
            };
        }
    }

    Json(reply)
}
// Tests of the REST interface.
#[cfg(test)]
mod project_rest_tests {
    use super::*;
    use crate::messaging;
    use crate::messaging::{condition_messages, parameter_messages, spectrum_messages};
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    // Setup Rocket and create parameters, conditions and spectra that
    // will be used by the tests:

    fn setup() -> Rocket<Build> {
        let r = rest_common::setup().mount("/", routes![super::project]);

        create_test_objects(&r);

        r
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        rest_common::teardown(c, p, b);
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
    fn create_test_objects(r: &Rocket<Build>) {
        // Make some parameters, a contour, a gate and a pair
        // of spectra.

        let (hch, _, _) = get_state(r);

        let papi = parameter_messages::ParameterMessageClient::new(&hch);
        let sapi = spectrum_messages::SpectrumMessageClient::new(&hch);
        let capi = condition_messages::ConditionMessageClient::new(&hch);

        // Make some parameters:

        for i in 0..10 {
            papi.create_parameter(&(format!("param.{}", i)))
                .expect("Making parameters");
        }
        // A oned spectrum named '1' and a twod one createvely enough named '2'
        // THe oned spectrum is to test error handling.

        sapi.create_spectrum_1d("1", "param.0", 0.0, 1024.0, 1024)
            .expect("making 1d spectrum");
        sapi.create_spectrum_2d(
            "2", "param.0", "param.1", 0.0, 1024.0, 256, 0.0, 1024.0, 256,
        )
        .expect("Creating 2d spectrum");

        // A projection contour
        //                                   ids
        match capi.create_contour_condition(
            "aoi",
            0,
            1,
            &vec![
                (100.0, 100.0),
                (200.0, 100.0),
                (200.0, 200.0),
                (100.0, 200.0),
            ],
        ) {
            condition_messages::ConditionReply::Created => {}
            condition_messages::ConditionReply::Replaced => {}
            condition_messages::ConditionReply::Error(s) => {
                panic!("Failed to create contour {}", s)
            }
            _ => panic!("Failed to create contour"),
        };
        // A simple gate that can be applied to the spectrum if desired.

        match capi.create_cut_condition("cut", 2, 100.0, 200.0) {
            condition_messages::ConditionReply::Created => {}
            condition_messages::ConditionReply::Replaced => {}
            condition_messages::ConditionReply::Error(s) => {
                panic!("Failed to create cut {}", s)
            }
            _ => panic!("Failed to create cut"),
        };
    }

    #[test]
    fn fail_1() {
        // NO such source spectrum:

        let r = setup();
        let (hch, papi, bapi) = get_state(&r);

        let c = Client::untracked(r).expect("Creating clients");
        let r = c.get("/?snapshot=no&source=junk&newname=stuff&direction=X");
        let response = r
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("JSON Parse error");

        assert!("OK" != response.status.as_str());

        teardown(hch, &papi, &bapi);
    }
    #[test]
    fn fail_2() {
        // Destination spectrum exists

        let r = setup();
        let (hch, papi, bapi) = get_state(&r);

        let c = Client::untracked(r).expect("Creating clients");
        let r = c.get("/?snapshot=no&source=2&newname=1&direction=X");
        let response = r.dispatch().into_json::<GenericResponse>().expect("JSON Parse Error");

        assert!("OK" != response.status.as_str());

        teardown(hch, &papi, &bapi);
    }
    #[test]
    fn fail_3() {
        // projection contour does not exist.

        let r = setup();
        let (hch, papi, bapi) = get_state(&r);

        let c = Client::untracked(r).expect("Creating test client");
        let r = c.get("/?snapshot=no&source=2&newname=projection&direction=X&contour=junk");
        let response = r.dispatch().into_json::<GenericResponse>().expect("JSON Parse error");

        assert!("OK" != response.status.as_str());

        teardown(hch, &papi, &bapi);
        
    }
    #[test]
    fn fail_4() {
        let r = setup();
        let (hch, papi, bapi) = get_state(&r);

        let c = Client::untracked(r).expect("Creating test client");
        let r = c.get("/?snapshot=no&source=2&newname=projection&direction=X&contour=cut");
        let response = r.dispatch().into_json::<GenericResponse>().expect("JSON Parse error");

        assert!("OK" != response.status.as_str());

        teardown(hch, &papi, &bapi);
        
    }
    #[test]
    fn fail_5() {
        // invalid direction string

        let r = setup();
        let (hch, papi, bapi) = get_state(&r);

        let c = Client::untracked(r).expect("Creating test client");
        let r = c.get("/?snapshot=no&source=2&newname=projection&direction=Xyzzy");
        let response = r.dispatch().into_json::<GenericResponse>().expect("JSON Parse error");

        assert!("OK" != response.status.as_str());

        teardown(hch, &papi, &bapi);
        
    }
    #[test]
    fn fail_6() {
        // Invalid snapshot string

        let r = setup();
        let (hch, papi, bapi) = get_state(&r);

        let c = Client::untracked(r).expect("Creating test client");
        let r = c.get("/?snapshot=nooooooo&source=2&newname=projection&direction=X");
        let response = r.dispatch().into_json::<GenericResponse>().expect("JSON Parse error");

        assert!("OK" != response.status.as_str());

        teardown(hch, &papi, &bapi);
        
    }
}
