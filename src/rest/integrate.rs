//! 'Implements' the /spectcl/integrate domain.
//! This is not implemented in this version of Rustogramer.
//! This is a candidate for implementation in a future release
//! *However*, it seems to me that this functionality really belongs
//!  in a displayer such as CutiePie where users can interact with
//!  a visualization of the spectrum to peform their integrations.
//!
//!  There is only /spectcl/integrate, nothing underneath it.
//!
use super::*;
use crate::messaging::{condition_messages, spectrum_messages};
use crate::spectra::integration;
use rocket::serde::{json::Json, Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(crate = "rocket::serde")]
pub struct IntegrationDetail {
    centroid: Vec<f64>,
    fwhm: Vec<f64>,
    counts: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct IntegrationResponse {
    status: String,
    detail: IntegrationDetail,
}
// figure out if a spectrum is 1d or 2d

fn is_1d(desc: &spectrum_messages::SpectrumProperties) -> bool {
    match desc.type_name.as_str() {
        "Multi1d" | "1D" => true,
        "Multi2d" | "PGamma" | "Summary" | "2D" | "2DSum" => false,
        _ => false, // Maybe this should return an Option.
    }
}

// Given spectrum characteristics and the inputs that might
// describe the AOI, return an integration::AreaOfInterest

fn generate_aoi(
    api: &condition_messages::ConditionMessageClient,
    oned: bool,
    gate: OptionalString,
    low: Option<f64>,
    high: Option<f64>,
    xcoord: OptionalF64Vec,
    ycoord: OptionalF64Vec,
) -> Result<integration::AreaOfInterest, String> {
    if oned {
        // x/ycoord must be none.  Only either gate or both of low, high can be some
        // gate if Some, must be a cut.  We'll return either AreaOfInterest::All
        // or AreaOfInterest::Oned.

        if let Some(gate_name) = gate {
            if low.is_some() | high.is_some() {
                return Err(String::from(
                    "1d spectra can only have either a gate name or limits",
                ));
            } else {
                // get the gate information.

                match api.list_conditions(&gate_name) {
                    condition_messages::ConditionReply::Listing(l) => {
                        if l.len() != 1 {
                            return Err(format!("{} either is a non-existent condition or is pattern that has more than one match", gate_name));
                        }
                        let condition = l[0].clone();
                        if condition.type_name == "Cut" {
                            return Ok(integration::AreaOfInterest::Oned {
                                low: condition.points[0].0,
                                high: condition.points[1].0,
                            });
                        } else {
                            return Err(format!(
                                "{} is not a Cut and must be for 1-d integrations",
                                gate_name
                            ));
                        }
                    }
                    condition_messages::ConditionReply::Error(s) => {
                        return Err(format!(
                            "Failed to get information about gate {}: {}",
                            gate_name, s
                        ));
                    }
                    _ => {
                        return Err(format!(
                            "Unexpected response from getting gate properties for {}",
                            gate_name
                        ));
                    }
                };
            }
        } else {
            if low.is_some() && high.is_some() {
                return Ok(integration::AreaOfInterest::Oned {
                    low: low.unwrap(),
                    high: high.unwrap(),
                });
            } else {
                // be nice and allow one but not the other to be all:

                return Ok(integration::AreaOfInterest::All);
            }
        }
    } else {
        // 2d we're allowed to have gate or x/y coordinates of a contour.

        if let Some(gate_name) = gate {
            if xcoord.is_some() || ycoord.is_some() {
                return Err(String::from("For a 2d spectrum only the gate _OR_ the AOI coordinates are allowed, not both"));
            }
            // Get gate information - must be a contour and we
            // then reconstruct it to make it a 2d area of interest:

            match api.list_conditions(&gate_name) {
                condition_messages::ConditionReply::Listing(l) => {
                    if l.len() != 1 {
                        return Err(format!(
                            "{} either is a nonexistent condition or is a non-unique pattern",
                            gate_name
                        ));
                    }

                    match condition_messages::reconstitute_contour(l[0].clone()) {
                        Ok(c) => {
                            return Ok(integration::AreaOfInterest::Twod(c));
                        }
                        Err(s) => {
                            return Err(format!(
                                "Failed to construct a contour from {} : {}",
                                gate_name, s
                            ));
                        }
                    }
                }

                condition_messages::ConditionReply::Error(s) => {
                    return Err(format!(
                        "Unable to get {} condition description: {}",
                        gate_name, s
                    ));
                }
                _ => {
                    return Err(format!(
                        "Unexpected responses getting description of condition {}",
                        gate_name
                    ));
                }
            }
        } else {
            if xcoord.is_some() && ycoord.is_some() {
                let xcoord = xcoord.unwrap();
                let ycoord = ycoord.unwrap();
                if xcoord.len() != ycoord.len() {
                    return Err(String::from(
                        "The X and Y coordinate arrays must be the same length",
                    ));
                }
                let mut pts = Vec::<(f64, f64)>::new();
                for (i, x) in xcoord.iter().enumerate() {
                    pts.push((*x, ycoord[i]));
                }
                let props = condition_messages::ConditionProperties {
                    cond_name: String::from("junk"),
                    type_name: String::from("Contour"),
                    points: pts,
                    gates: vec![],
                    parameters: vec![0, 1],
                };
                match condition_messages::reconstitute_contour(props) {
                    Ok(c) => {
                        return Ok(integration::AreaOfInterest::Twod(c));
                    }
                    Err(s) => {
                        return Err(format!("Could not make a contour from x/y points: {}", s));
                    }
                }
            } else if xcoord.is_none() && ycoord.is_none() {
                return Ok(integration::AreaOfInterest::All);
            } else {
                return Err(String::from(
                    "When specifying a 2d AOI with points both xcoord and ycoord must be present",
                ));
            }
        }
    }
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
#[get("/?<name>&<gate>&<low>&<high>&<xcoord>&<ycoord>")]
pub fn integrate(
    name: String,
    gate: OptionalString,
    low: Option<f64>,
    high: Option<f64>,
    xcoord: OptionalF64Vec,
    ycoord: OptionalF64Vec,
    state: &State<SharedHistogramChannel>,
) -> Json<IntegrationResponse> {
    // A few errors to check for:
    // - the name must be for a valid spectrum - and we must be able to get
    //   the contents
    // - We can construct a valid area of interest from gate,low, high, xcoord, ycoord.
    //

    // Get spectrum validity and description/contents or error
    let sapi = spectrum_messages::SpectrumMessageClient::new(&state.inner().lock().unwrap());
    let capi = condition_messages::ConditionMessageClient::new(&state.inner().lock().unwrap());
    let description = sapi.list_spectra(&name);
    if let Err(s) = description {
        return Json(IntegrationResponse {
            status: format!("Unable to get spectrum description: {}", s),
            detail: IntegrationDetail {
                centroid: vec![0.0],
                fwhm: vec![0.0],
                counts: 0,
            },
        });
    }
    let description = description.unwrap();
    if description.len() != 1 {
        return Json(IntegrationResponse {
            status: format!(
                "{} either does not exist or is a pattern with more than one match",
                name
            ),
            detail: IntegrationDetail {
                centroid: vec![0.0],
                fwhm: vec![0.0],
                counts: 0,
            },
        });
    }
    let description = description[0].clone();
    let is_1d = is_1d(&description);
    let (xlow, xhigh) = if let Some(xaxis) = description.xaxis {
        (xaxis.low, xaxis.high)
    } else {
        (0.0, 0.0)
    };
    let (ylow, yhigh) = if let Some(yaxis) = description.yaxis {
        (yaxis.low, yaxis.high)
    } else {
        (0.0, 0.0)
    };
    let contents = sapi.get_contents(&name, xlow, xhigh, ylow, yhigh);
    if let Err(s) = contents {
        return Json(IntegrationResponse {
            status: format!("Unable to fetch contents for spectrum{}", s),
            detail: IntegrationDetail {
                centroid: vec![0.0],
                fwhm: vec![0.0],
                counts: 0,
            },
        });
    }
    let contents = contents.unwrap();

    let aoi = generate_aoi(&capi, is_1d, gate, low, high, xcoord, ycoord);
    if let Err(s) = aoi {
        return Json(IntegrationResponse {
            status: format!("Could not create integration AOI: {}", s),
            detail: IntegrationDetail {
                centroid: vec![0.0],
                fwhm: vec![0.0],
                counts: 0,
            },
        });
    }
    let aoi = aoi.unwrap().clone();

    // Now do the integration and marshall the response - how that's done depends
    // on the spectrum dimensionality.

    let result = integration::integrate(&contents, aoi);

    let response = if is_1d {
        IntegrationResponse {
            status: String::from("OK"),
            detail: IntegrationDetail {
                centroid: vec![result.centroid.0],
                fwhm: vec![result.fwhm.0],
                counts: result.sum as u64,
            },
        }
    } else {
        IntegrationResponse {
            status: String::from("OK"),
            detail: IntegrationDetail {
                centroid: vec![result.centroid.0, result.centroid.1],
                fwhm: vec![result.fwhm.0, result.fwhm.1],
                counts: result.sum as u64,
            },
        }
    };

    Json(response)
}
// Placeholder for tests if/when this is supported:

#[cfg(test)]
mod integrate_tests {
    use super::*;
    use crate::messaging;
    use crate::messaging::{condition_messages, parameter_messages, spectrum_messages};
    use crate::processing;
    use crate::sharedmem::binder;
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;

    fn setup() -> Rocket<Build> {
        let r = rest_common::setup().mount("/", routes![integrate::integrate]);

        make_parameters(&r);
        make_conditions(&r);
        make_spectra(&r);

        r
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: processing::ProcessingApi,
        b: binder::BindingApi,
    ) {
        rest_common::teardown(c, &p, &b);
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
    ) {
        rest_common::get_state(r)
    }
    fn make_parameters(r: &Rocket<Build>) {
        let (req, _, _) = getstate(r);
        let api = parameter_messages::ParameterMessageClient::new(&req);
        for i in 0..10 {
            api.create_parameter(&format!("param.{}", i))
                .expect("Making parameter");
        }
    }
    // Make a 2-d and a 2-d spectrum with spikes at well known places:
    fn make_spectra(r: &Rocket<Build>) {
        let (req, _, _) = getstate(r);
        let api = spectrum_messages::SpectrumMessageClient::new(&req);

        api.create_spectrum_1d("oned", "param.0", 0.0, 1024.0, 1024)
            .expect("Making 1d");
        api.create_spectrum_2d(
            "twod", "param.0", "param.1", 0.0, 1024.0, 512, 0.0, 1024.0, 512,
        )
        .expect("Making 2d");

        // Put a spike in ths 1d spectrum at 150.0:

        api.fill_spectrum(
            "oned",
            vec![spectrum_messages::Channel {
                chan_type: spectrum_messages::ChannelType::Bin,
                x: 150.0,
                y: 0.0,
                bin: 0,
                value: 1234.0,
            }],
        )
        .expect("Setting 1d contents.");

        // put a spike in the twod spectrum at 150, 150:

        api.fill_spectrum(
            "twod",
            vec![spectrum_messages::Channel {
                chan_type: spectrum_messages::ChannelType::Bin,
                x: 150.0,
                y: 150.0,
                bin: 0,
                value: 4321.0,
            }],
        )
        .expect("setting 2d contents");
    }
    fn make_conditions(r: &Rocket<Build>) {
        // Make a slice and a diamond condition;  Note these, for fun, are on different
        // parameters than the spectra.

        let (req, _, _) = getstate(r);
        let api = condition_messages::ConditionMessageClient::new(&req);

        match api.create_true_condition("true") {
            condition_messages::ConditionReply::Created => {}
            _ => panic!("Bad reply from true creation."),
        } // bad condition to use.

        match api.create_cut_condition("good-cut", 2, 100.0, 200.0) {
            condition_messages::ConditionReply::Created => {}
            _ => panic!("Bad reply from true creation."),
        } // Spike in this cut.

        match api.create_cut_condition("empty-cut", 2, 0.0, 50.0) {
            condition_messages::ConditionReply::Created => {}
            _ => panic!("Bad reply from true creation."),
        } // Spike not sin this cut.

        // Make a couple of squares one that contains the spike the other that does not

        match api.create_contour_condition(
            "good-contour",
            4,
            5,
            &vec![
                (100.0, 100.0),
                (500.0, 100.0),
                (500.0, 500.0),
                (100.0, 500.0),
            ],
        ) {
            condition_messages::ConditionReply::Created => {}
            _ => panic!("Bad reply from true creation."),
        } // Contains spike

        match api.create_contour_condition(
            "empty-contour",
            4,
            5,
            &vec![(0.0, 0.0), (50.0, 0.0), (50.0, 50.0), (0.0, 50.0)],
        ) {
            condition_messages::ConditionReply::Created => {}
            _ => panic!("Bad reply from true creation."),
        }
    }
    #[test]
    fn error_1() {
        // Integrate a nonexistent spectrum:

        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("Unable to create client");
        let req = c.get("/?name=junk");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert!("Ok" != reply.status); // Fails.

        teardown(chan, p, b);
    }
    #[test]
    fn error_2() {
        // Spectrum is ok but condition name is nonexistent
        // gate name.

        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=oned&gate=no-such");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert!("OK" != reply.status);

        teardown(chan, p, b);
    }
    #[test]
    fn error_3() {
        // Condition exists but is not a legal integration gate:

        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=oned&gate=true");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert!("OK" != reply.status);

        teardown(chan, p, b);
    }
    #[test]
    fn error_4() {
        // Condition exists but is a contour integrating a 1d

        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=oned&gate=good-contour");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert!("OK" != reply.status);

        teardown(chan, p, b);
    }
    #[test]
    fn error_5() {
        // Same as error 4 but 2d spectrum with cut gate:

        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=twod&gate=good-cut");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert!("OK" != reply.status);

        teardown(chan, p, b);
    }
    #[test]
    fn error_6() {
        // oned integration can have gate or low/high but not both

        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=oned&gate=good-cut&low=100&high=200");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert!("OK" != reply.status);

        teardown(chan, p, b);
    }
    #[test]
    fn error_7() {
        // 2d integration can have contour or (xcoorc/ycoord) but not both.
        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=twod&gate=good-contour?xcoord=10&xcoord=20&xcoord=10&ycoord=10&ycoord=20&ycoord=10");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert!("OK" != reply.status);

        teardown(chan, p, b);
    }
    #[test]
    fn oned_1() {
        // Integrate 1d with no gate

        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=oned");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(
            IntegrationDetail {
                centroid: vec![150.0],
                fwhm: vec![0.0],
                counts: 1234
            },
            reply.detail
        );

        teardown(chan, p, b);
    }
    #[test]
    fn oned_2() {
        // integration inside a cut:

        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=oned&gate=good-cut");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(
            IntegrationDetail {
                centroid: vec![150.0],
                fwhm: vec![0.0],
                counts: 1234
            },
            reply.detail
        );

        teardown(chan, p, b);
    }
    #[test]
    fn oned_3() {
        // integration with the peak outside the cut:

        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=oned&gate=empty-cut");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(
            IntegrationDetail {
                centroid: vec![0.0],
                fwhm: vec![0.0],
                counts: 0
            },
            reply.detail
        );

        teardown(chan, p, b);
    }
    #[test]
    fn oned_4() {
        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=oned&low=100&high200");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(
            IntegrationDetail {
                centroid: vec![150.0],
                fwhm: vec![0.0],
                counts: 1234
            },
            reply.detail
        );

        teardown(chan, p, b);
    }
    #[test]
    fn oned_5() {
        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=oned&low=0&high=50");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(
            IntegrationDetail {
                centroid: vec![0.0],
                fwhm: vec![0.0],
                counts: 0
            },
            reply.detail
        );

        teardown(chan, p, b);
    }
    #[test]
    fn twod_1() {
        // 2d with no gate:

        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=twod");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(
            IntegrationDetail {
                centroid: vec![150.0, 150.0],
                fwhm: vec![0.0, 0.0],
                counts: 4321
            },
            reply.detail
        );

        teardown(chan, p, b);
    }
    #[test]
    fn twod_2() {
        // Integration when the peak's in a contour:

        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=twod&gate=good-contour");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(
            IntegrationDetail {
                centroid: vec![150.0, 150.0],
                fwhm: vec![0.0, 0.0],
                counts: 4321
            },
            reply.detail
        );

        teardown(chan, p, b);
    }
    #[test]
    fn twod_3() {
        // Integration when the peak's not in the contour:

        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=twod&gate=empty-contour");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(
            IntegrationDetail {
                centroid: vec![0.0, 0.0],
                fwhm: vec![0.0, 0.0],
                counts: 0
            },
            reply.detail
        );

        teardown(chan, p, b);
    }
    #[test]
    fn twod_4() {
        // Integration is in side a contour defined by x/ycoors:

        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=twod&xcoord=100&xcoord=500&xcoord=500&xcoord=100&ycoord=100&ycoord=100&ycoord=500&ycoord=500");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(
            IntegrationDetail {
                centroid: vec![150.0, 150.0],
                fwhm: vec![0.0, 0.0],
                counts: 4321
            },
            reply.detail
        );

        teardown(chan, p, b);
    }
    #[test]
    fn twod_5() {
        // integration spike is outside the contour defined by x/ycoords:

        let r = setup();
        let (chan, p, b) = getstate(&r);

        let c = Client::untracked(r).expect("unable to create client");
        let req = c.get("/?name=twod&xcoord=0&xcoord=50&xcoord=50&xcoord=0&ycoord=0&ycoord=0&ycoord=50&ycoord=50");
        let reply = req
            .dispatch()
            .into_json::<IntegrationResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(
            IntegrationDetail {
                centroid: vec![0.0, 0.0],
                fwhm: vec![0.0, 0.0],
                counts: 0
            },
            reply.detail
        );

        teardown(chan, p, b);
    }
}
