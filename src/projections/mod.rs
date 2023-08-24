//!
//!   This module contains code to support making
//!  projections of histograms.
//!  It is intended that this set of functions run outside of the
//!  histogram server and, therefore the data it works with  are
//!  data gotten from message exchanges with that server
//!  defined in crate::messaging::*
//!
//!  

use crate::conditions::twod;
use crate::messaging::{condition_messages, spectrum_messages};

///
/// Legal projection directions:
///

#[derive(Copy, Clone, PartialEq)]
pub enum ProjectionDirection {
    X,
    Y,
}

///  Given a spectrum_messages::SpectrumProperties reference,
///  returns a pre-zeroed vector that can hold a projection
///  along the specified axis.
///
/// ### Parameters:
///
/// * def - references the spectrum definition.
/// * direction - Specifies ths projection direction.
///
/// ### Returns:
///    Result<Vec<f64>, String> On success, this is an initialized
///  vector large enough to contain the projection.  On failure a string
///  error message (e.g. "Spectrum requires both an X and a Y axis.")
///
fn make_sum_vector(
    def: &spectrum_messages::SpectrumProperties,
    direction: ProjectionDirection,
) -> Result<Vec<f64>, String> {
    if def.xaxis.is_some() && def.yaxis.is_some() {
        let n_element = if direction == ProjectionDirection::X {
            def.xaxis.unwrap().bins
        } else {
            def.yaxis.unwrap().bins
        } as usize;
        let mut result = vec![];
        result.resize(n_element, 0.0 as f64);
        Ok(result)
    } else {
        Err(String::from("To project requires an X and a Y axis."))
    }
}

///
/// Projection is summing in either the X or y direction into a vector
/// created by make_sum_vector with a function used to determine if
/// a specific channel shouild be included in the sum.  The
/// function supports projection only within some region of interest
/// such as a contour -- but is much more general than a contour.
///
/// ### Parameters:
///  *  desc - references SpectrumProperties that define the spectrum being projected.
///  *  contents - References SpectrumContents that contain the non-zero
/// spectrum channels.
///  *  direction - ProjectionDirection that describes which axis the projection
/// is onto.
///  * f a function that takes the x/y values of a channel and returns true
/// if that channel should be inlcuded in the sum.
///
///  ### Returns:
///    Result<Vec<f64>, String> where on success the vector is the projection
/// while on failure it is a diagnostic string describing the reason for failure.
///
pub fn project_spectrum<F>(
    desc: &spectrum_messages::SpectrumProperties,
    contents: &spectrum_messages::SpectrumContents,
    direction: ProjectionDirection,
    f: F,
) -> Result<Vec<f64>, String>
where
    F: Fn(f64, f64) -> bool,
{
    match make_sum_vector(&desc, direction) {
        Ok(mut v) => {
            // Get the axis specification:

            let axis = if let ProjectionDirection::X = direction {
                desc.xaxis.unwrap()
            } else {
                desc.yaxis.unwrap()
            };
            for c in contents {
                if f(c.x, c.y) {
                    let coord = if let ProjectionDirection::X = direction {
                        c.x
                    } else {
                        c.y
                    };
                    let bin = spectrum_messages::coord_to_bin(coord, axis);
                    v[bin as usize] += c.value;
                }
            }
            Ok(v)
        }
        Err(s) => Err(s),
    }
}
///
/// This function reconstructs a contour in terms of the information
/// that is passed to it by the condition_messaging API.  This is needed
/// in order to construct a closure that can properly work for project_spectrum
/// when the projection is inside s contour.
///
/// ### Parameters:
///   *  props - the condition properties. Note these are consumed.
/// ### Returns:
///   Result<conditions::twod::Contour, String>  - where:
///   *  Ok encapsulates the reconstituted contour
///   *  Err encapsulates an error string (normally if props are not a
/// contour).
///
/// ### NOTE:
///   Dummy parameter numbers 0 and 1 are used for the parameter ids.
///
fn reconstitute_contour(
    props: condition_messages::ConditionProperties,
) -> Result<twod::Contour, String> {
    if props.type_name == "Contour" {
        let mut pts = Vec::<twod::Point>::new();
        for (x, y) in props.points {
            pts.push(twod::Point::new(x, y));
        }
        match twod::Contour::new(0, 1, pts) {
            Some(c) => Ok(c),
            None => Err(String::from(
                "Failed to reconstitute contour in constructor - maybe too few points?",
            )),
        }
    } else {
        Err(String::from(
            "Error reconstituting a contour - input is not a contour",
        ))
    }
}

///
/// Make projection spectrum
///   Given an input spectrum description, produces the output spectrum for
/// a projection (in the histogram server) and stuffs it with the
/// the contents of the projection.
///
///  ### Parameters:
/// *   api - the spectrum messaging API used to request the creation of the
/// spectrum.
/// *   new_name - name for the projected spectrum.
/// *   desc - the input spectrum description.
/// *   direction -the projection direction.
/// *   data  - Results of the project_spectrum function.
///
/// ### Returns:
///   Result<(), String>
/// * Err - encapsulates an error message.
/// * Ok - encapsulates nothing.
///
/// ### Note:
///  Consumes the data
///
pub fn make_projection_spectrum(
    api: &spectrum_messages::SpectrumMessageClient,
    new_name: &str,
    desc: &spectrum_messages::SpectrumProperties,
    direction: ProjectionDirection,
    data: Vec<f64>,
) -> Result<(), String> {
    // in general the axis is the axis of the projection direction:

    let mut resulting_axis = match direction {
        ProjectionDirection::X => {
            if let Some(a) = desc.xaxis {
                a
            } else {
                return Err(String::from("Required X axis missing from source spectrum"));
            }
        }
        ProjectionDirection::Y => {
            if let Some(a) = desc.yaxis {
                a
            } else {
                return Err(String::from("Required Y axis missing from source spectrum"));
            }
        }
    };
    resulting_axis.bins -= 2; // they'll get added back when the ndhistogram is created.
                              // For most cases this is true:

    let params = match direction {
        ProjectionDirection::X => desc.xparams.clone(),
        ProjectionDirection::Y => desc.yparams.clone(),
    };

    // What we do depends on both the spectrum type and direction.
    // Would be nice figure that out all in one swoop but sadly not

    let status = match desc.type_name.as_str() {
        "Multi2D" => {
            // Multi-1d spectrum

            let params = desc.xparams.clone();
            api.create_spectrum_multi1d(
                new_name,
                &params,
                resulting_axis.low,
                resulting_axis.high,
                resulting_axis.bins,
            )
        }
        "PGamma" => {
            // pgamma spectra, to continue to faithfully increment
            // need to build a parameter array that is n copies of the
            // input array where n is the number of elements in the other axis:

            let (base_params, n) = match direction {
                ProjectionDirection::X => (desc.xparams.clone(), desc.yparams.len()),
                ProjectionDirection::Y => (desc.yparams.clone(), desc.xparams.len()),
            };
            let mut params = vec![];
            for _ in 0..n {
                params.append(&mut base_params.clone());
            }

            api.create_spectrum_multi1d(
                new_name,
                &params,
                resulting_axis.low,
                resulting_axis.high,
                resulting_axis.bins,
            )
        }
        "2D" => api.create_spectrum_1d(
            new_name,
            &params[0],
            resulting_axis.low,
            resulting_axis.high,
            resulting_axis.bins,
        ),
        "2DSum" => api.create_spectrum_multi1d(
            new_name,
            &params,
            resulting_axis.low,
            resulting_axis.high,
            resulting_axis.bins,
        ),
        _ => Err(format!("{} spectra cannot be projected", desc.type_name)),
    };
    // Still need to fill the spectrum....

    // Build a vector of channels (SpectrumContents)
    if let Ok(()) = status {
        let mut s_contents = Vec::<spectrum_messages::Channel>::new();
        resulting_axis.bins += 2; // Need the under/overflow channels back
        for (i, value) in data.iter().enumerate() {
            // Only add channels for non-zero values:
            // note that set_contents ignore the channel type:
            if *value != 0.0 {
                let coord = spectrum_messages::bin_to_coord(i as u32, resulting_axis);
                s_contents.push(spectrum_messages::Channel {
                    chan_type: spectrum_messages::ChannelType::Bin,
                    x: coord,
                    y: 0.0, // 1-d type.
                    bin: i,
                    value: *value,
                });
            }
        }
        api.fill_spectrum(new_name, s_contents)
    } else {
        status
    }
}

// Tests for make_sum_vector
#[cfg(test)]
mod make_sum_tests {
    use super::*;
    use crate::messaging::spectrum_messages;

    #[test]
    fn invalid_1() {
        // Only x axus is insufficient to project in x
        let props = spectrum_messages::SpectrumProperties {
            name: String::from("test"),
            type_name: String::from("1d"),
            xparams: vec![], // Parameters are ignored.
            yparams: vec![],
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            yaxis: None,
            gate: None,
        };
        assert!(make_sum_vector(&props, ProjectionDirection::X).is_err());
    }
    #[test]
    fn invalid_2() {
        // Only x axis insufficient to project in y.
        let props = spectrum_messages::SpectrumProperties {
            name: String::from("test"),
            type_name: String::from("1d"),
            xparams: vec![], // Parameters are ignored.
            yparams: vec![],
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            yaxis: None,
            gate: None,
        };
        assert!(make_sum_vector(&props, ProjectionDirection::Y).is_err());
    }
    #[test]
    fn invalid_3() {
        // only y axis is  insufficient to project in x or y:

        let props = spectrum_messages::SpectrumProperties {
            name: String::from("test"),
            type_name: String::from("1d"),
            xparams: vec![], // Parameters are ignored.
            yparams: vec![],
            xaxis: None,
            yaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            gate: None,
        };

        assert!(make_sum_vector(&props, ProjectionDirection::X).is_err());
        assert!(make_sum_vector(&props, ProjectionDirection::Y).is_err());
    }
    // If there are both axes we can project in both directions:

    #[test]
    fn ok_1() {
        let props = spectrum_messages::SpectrumProperties {
            name: String::from("test"),
            type_name: String::from("1d"),
            xparams: vec![], // Parameters are ignored.
            yparams: vec![],
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 512,
            }),
            yaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            gate: None,
        };
        assert!(make_sum_vector(&props, ProjectionDirection::X).is_ok());
        assert!(make_sum_vector(&props, ProjectionDirection::Y).is_ok());
    }
    #[test]
    fn ok_2() {
        // Ensure x projections get the size right:

        let props = spectrum_messages::SpectrumProperties {
            name: String::from("test"),
            type_name: String::from("1d"),
            xparams: vec![], // Parameters are ignored.
            yparams: vec![],
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 512,
            }),
            yaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            gate: None,
        };
        let v = make_sum_vector(&props, ProjectionDirection::X)
            .expect("could not make x projection vector");
        assert_eq!(props.xaxis.unwrap().bins as usize, v.len());
    }
    #[test]
    fn ok_3() {
        let props = spectrum_messages::SpectrumProperties {
            name: String::from("test"),
            type_name: String::from("1d"),
            xparams: vec![], // Parameters are ignored.
            yparams: vec![],
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 512,
            }),
            yaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            gate: None,
        };
        let v = make_sum_vector(&props, ProjectionDirection::Y)
            .expect("could not make x projection vector");
        assert_eq!(props.yaxis.unwrap().bins as usize, v.len());
    }
}
#[cfg(test)]
mod project_tests {
    use super::*;
    use crate::messaging::spectrum_messages;

    #[test]
    fn err_1() {
        // Spectra can only be projected if they have x and y axes:
        // No y axis:
        //
        let props = spectrum_messages::SpectrumProperties {
            name: String::from("test"),
            type_name: String::from("1d"),
            xparams: vec![], // Parameters are ignored.
            yparams: vec![],
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            yaxis: None,
            gate: None,
        };
        let contents = vec![];
        assert!(project_spectrum(&props, &contents, ProjectionDirection::X, |_, _| true).is_err());
        assert!(project_spectrum(&props, &contents, ProjectionDirection::Y, |_, _| true).is_err());
    }
    #[test]
    fn err_2() {
        // No X axis:

        let props = spectrum_messages::SpectrumProperties {
            name: String::from("test"),
            type_name: String::from("1d"),
            xparams: vec![], // Parameters are ignored.
            yparams: vec![],
            xaxis: None,
            yaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            gate: None,
        };
        let contents = vec![];
        assert!(project_spectrum(&props, &contents, ProjectionDirection::X, |_, _| true).is_err());
        assert!(project_spectrum(&props, &contents, ProjectionDirection::Y, |_, _| true).is_err());
    }
    #[test]
    fn ok_1() {
        // x/y axis allows projection - no contents so zeroes for sums:

        let props = spectrum_messages::SpectrumProperties {
            name: String::from("test"),
            type_name: String::from("1d"),
            xparams: vec![], // Parameters are ignored.
            yparams: vec![],
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 512,
            }),
            yaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            gate: None,
        };
        let contents = vec![];

        assert!(project_spectrum(&props, &contents, ProjectionDirection::X, |_, _| true).is_ok());
        assert!(project_spectrum(&props, &contents, ProjectionDirection::Y, |_, _| true).is_ok());
    }
    #[test]
    fn ok_2() {
        // Sizes should be correct:

        let props = spectrum_messages::SpectrumProperties {
            name: String::from("test"),
            type_name: String::from("1d"),
            xparams: vec![], // Parameters are ignored.
            yparams: vec![],
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 512,
            }),
            yaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            gate: None,
        };
        let contents = vec![];

        assert_eq!(
            props.xaxis.unwrap().bins as usize,
            project_spectrum(&props, &contents, ProjectionDirection::X, |_, _| true)
                .expect("Projecting x")
                .len()
        );
        assert_eq!(
            props.yaxis.unwrap().bins as usize,
            project_spectrum(&props, &contents, ProjectionDirection::Y, |_, _| true)
                .expect("Projecting y")
                .len()
        );
    }
    #[test]
    fn ok_3() {
        // in these cases the sums should be zero:

        let props = spectrum_messages::SpectrumProperties {
            name: String::from("test"),
            type_name: String::from("1d"),
            xparams: vec![], // Parameters are ignored.
            yparams: vec![],
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 512,
            }),
            yaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            gate: None,
        };
        let contents = vec![];
        for (i, n) in project_spectrum(&props, &contents, ProjectionDirection::X, |_, _| true)
            .expect("Projecting x")
            .iter()
            .enumerate()
        {
            assert_eq!(0.0, *n, "xprojection Bin {} nonzero", i);
        }
        for (i, n) in project_spectrum(&props, &contents, ProjectionDirection::Y, |_, _| true)
            .expect("Projecting Y")
            .iter()
            .enumerate()
        {
            assert_eq!(0.0, *n, "yprojection Bin {} nonzero", i);
        }
    }
    #[test]
    fn ok_4() {
        // If there is a single channel with non-zero data this shouild
        // result in a non-zero projection:

        let props = spectrum_messages::SpectrumProperties {
            name: String::from("test"),
            type_name: String::from("1d"),
            xparams: vec![], // Parameters are ignored.
            yparams: vec![],
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 512,
            }),
            yaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            gate: None,
        };
        let contents = vec![spectrum_messages::Channel {
            chan_type: spectrum_messages::ChannelType::Bin,
            x: 256.0,
            y: 256.0,
            bin: 0,
            value: 1234.0,
        }];
        let xproj = project_spectrum(&props, &contents, ProjectionDirection::X, |_, _| true)
            .expect("X Projection");
        assert_eq!(1234.0, xproj[128]); // due to 2:1 binning.

        let yproj = project_spectrum(&props, &contents, ProjectionDirection::Y, |_, _| true)
            .expect("Y Projection");
        assert_eq!(1234.0, yproj[256]); // 1:1 binning
    }
    #[test]
    fn ok_5() {
        // Using a closure that always returns false gives zeros in the  projection:

        let props = spectrum_messages::SpectrumProperties {
            name: String::from("test"),
            type_name: String::from("1d"),
            xparams: vec![], // Parameters are ignored.
            yparams: vec![],
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 512,
            }),
            yaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            gate: None,
        };
        let contents = vec![spectrum_messages::Channel {
            chan_type: spectrum_messages::ChannelType::Bin,
            x: 256.0,
            y: 256.0,
            bin: 0,
            value: 1234.0,
        }];
        let xproj = project_spectrum(&props, &contents, ProjectionDirection::X, |_, _| false)
            .expect("X Projection");
        for (i, x) in xproj.iter().enumerate() {
            assert_eq!(0.0, *x, "Nonzero value in X chanel {}", i)
        }
        let yproj = project_spectrum(&props, &contents, ProjectionDirection::Y, |_, _| false)
            .expect("Y Projection");
        for (i, x) in yproj.iter().enumerate() {
            assert_eq!(0.0, *x, "Nonzero value in Y chanel {}", i)
        }
    }
}
#[cfg(test)]
mod recons_contour_tests {
    use super::*;
    use crate::messaging::condition_messages;

    #[test]
    fn err_1() {
        // Contour described is not actually a contour:

        let desc = condition_messages::ConditionProperties {
            cond_name: String::from("junk"),
            type_name: String::from("Not a contour"),
            points: vec![],
            gates: vec![],
            parameters: vec![],
        };
        assert!(reconstitute_contour(desc).is_err());
    }
    #[test]
    fn err_2() {
        // Some how too few points in a thing that claims to be a contour

        let desc = condition_messages::ConditionProperties {
            cond_name: String::from("junk"),
            type_name: String::from("Contour"),
            points: vec![(100.0, 100.0), (200.0, 100.0)],
            gates: vec![],
            parameters: vec![],
        };
        assert!(reconstitute_contour(desc).is_err());
    }
    #[test]
    fn ok_1() {
        let pts = vec![(100.0, 100.0), (200.0, 100.0), (150.0, 150.0)]; // needed for later assertion:
        let desc = condition_messages::ConditionProperties {
            cond_name: String::from("junk"),
            type_name: String::from("Contour"),
            points: pts.clone(),
            gates: vec![],
            parameters: vec![],
        };
        let result = reconstitute_contour(desc);
        assert!(result.is_ok());
        let contour = result.unwrap();

        let contour_points = contour.get_points();
        assert_eq!(pts.len(), contour_points.len());
        for (i, p) in pts.iter().enumerate() {
            assert_eq!(p.0, contour_points[i].x, "X mismatch on point {}", i);
            assert_eq!(p.1, contour_points[i].y, "Y mismatch on point {}", i);
        }
    }
}
// Tests for make_projection_spectrum  Note these will need a
// server to work properly.
#[cfg(test)]
mod make_spectrum_tests {
    use super::*;
    use crate::histogramer;
    use crate::messaging;
    use crate::messaging::{parameter_messages, spectrum_messages};
    use crate::trace;
    use std::sync::mpsc;
    use std::thread;

    fn setup() -> (mpsc::Sender<messaging::Request>, thread::JoinHandle<()>) {
        let (jh, send) = histogramer::start_server(trace::SharedTraceStore::new());
        (send, jh)
    }
    fn teardown(ch: mpsc::Sender<messaging::Request>, jh: thread::JoinHandle<()>) {
        histogramer::stop_server(&ch);
        jh.join().unwrap();
    }

    #[test]
    fn error_1() {
        // The input spectrum is of the wrong type.

        let (ch, jh) = setup();

        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        let data = vec![];
        let desc = spectrum_messages::SpectrumProperties {
            name: String::from("dummy"),
            type_name: String::from("1D"), // not projectable.
            xparams: vec![],
            yparams: vec![],
            xaxis: None,
            yaxis: None,
            gate: None,
        };
        // Either direction is bad:
        assert!(
            make_projection_spectrum(&sapi, "test", &desc, ProjectionDirection::X, data).is_err()
        );
        assert!(
            make_projection_spectrum(&sapi, "test", &desc, ProjectionDirection::Y, vec![]).is_err()
        );

        teardown(ch, jh);
    }
    #[test]
    fn error_2() {
        // Need xaxis spec in description but it's missing (valid type)

        let (ch, jh) = setup();
        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        let desc = spectrum_messages::SpectrumProperties {
            name: String::from("dummy"),
            type_name: String::from("2D"), // valid.
            xparams: vec![],
            yparams: vec![],
            xaxis: None, // must not be none to project x
            yaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            gate: None,
        };
        assert!(
            make_projection_spectrum(&sapi, "test", &desc, ProjectionDirection::X, vec![]).is_err()
        );

        teardown(ch, jh);
    }
    #[test]
    fn error_3() {
        // Need y axis specification to project in y:

        let (ch, jh) = setup();
        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        let desc = spectrum_messages::SpectrumProperties {
            name: String::from("dummy"),
            type_name: String::from("2D"), // valid.
            xparams: vec![],
            yparams: vec![],
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024,
            }),
            yaxis: None, // must not be none to project y
            gate: None,
        };
        assert!(
            make_projection_spectrum(&sapi, "test", &desc, ProjectionDirection::Y, vec![]).is_err()
        );

        teardown(ch, jh);
    }
    // Tests when the input spectrum is Multi2D tests::
    // - spectrum can be created.
    // - it has the right properties:
    // - It can be loaded with the right stuff.

    // Makes the properties and the parameters
    //
    fn make_multi2_properties(
        chan: &mpsc::Sender<messaging::Request>,
    ) -> spectrum_messages::SpectrumProperties {
        let api = parameter_messages::ParameterMessageClient::new(chan);
        for name in vec!["p1", "p2", "p3"] {
            api.create_parameter(name).expect("making parameters");
        }
        spectrum_messages::SpectrumProperties {
            name: String::from("input"),
            type_name: String::from("Multi2D"),
            xparams: vec![String::from("p1"), String::from("p2"), String::from("p3")],
            yparams: vec![],
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1026,
            }),
            yaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 512.0,
                bins: 514,
            }),
            gate: None,
        }
    }

    #[test]
    fn multi2_1() {
        // No error for multi2 spectrum with valid properties.

        let (ch, jh) = setup();

        let properties = make_multi2_properties(&ch);
        let spectrum_api = spectrum_messages::SpectrumMessageClient::new(&ch);

        assert!(make_projection_spectrum(
            &spectrum_api,
            "test1",
            &properties,
            ProjectionDirection::X,
            vec![]
        )
        .is_ok());
        assert!(make_projection_spectrum(
            &spectrum_api,
            "test2",
            &properties,
            ProjectionDirection::Y,
            vec![]
        )
        .is_ok());

        teardown(ch, jh);
    }
    #[test]
    fn multi2_2() {
        // created in server with correct properties x projection.

        let (ch, jh) = setup();

        let properties = make_multi2_properties(&ch);
        let spectrum_api = spectrum_messages::SpectrumMessageClient::new(&ch);

        assert!(make_projection_spectrum(
            &spectrum_api,
            "test1",
            &properties,
            ProjectionDirection::X,
            vec![]
        )
        .is_ok());

        // Get the properties of the created spectruM:

        let created_props = spectrum_api.list_spectra("test1");
        assert!(created_props.is_ok()); // Server must say ok.
        let created_props = created_props.unwrap();
        assert_eq!(1, created_props.len()); // There can be exactly one
        let created_props = created_props[0].clone(); // Extract it's properties

        assert_eq!("test1", created_props.name);
        assert_eq!("Multi1d", created_props.type_name);
        assert_eq!(3, created_props.xparams.len());
        for (i, expected) in vec!["p1", "p2", "p3"].iter().enumerate() {
            assert_eq!(
                *expected, created_props.xparams[i],
                "Param name mismatch: {} {:?}",
                i, created_props.xparams
            );
        }
        assert_eq!(0, created_props.yparams.len());
        assert!(created_props.xaxis.is_some());
        assert_eq!(
            spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1026 // Over/underflow.
            },
            created_props.xaxis.unwrap()
        );
        assert!(created_props.yaxis.is_none());
        assert!(created_props.gate.is_none());

        teardown(ch, jh);
    }
    #[test]
    fn multi2_3() {
        // creatd in server with correct properties y projection.

        let (ch, jh) = setup();

        let properties = make_multi2_properties(&ch);
        let spectrum_api = spectrum_messages::SpectrumMessageClient::new(&ch);

        assert!(make_projection_spectrum(
            &spectrum_api,
            "test1",
            &properties,
            ProjectionDirection::Y,
            vec![]
        )
        .is_ok());

        // Get the properties of the created spectruM:

        let created_props = spectrum_api.list_spectra("test1");
        assert!(created_props.is_ok()); // Server must say ok.
        let created_props = created_props.unwrap();
        assert_eq!(1, created_props.len()); // There can be exactly one
        let created_props = created_props[0].clone(); // Extract it's properties

        assert_eq!("test1", created_props.name);
        assert_eq!("Multi1d", created_props.type_name);
        assert_eq!(3, created_props.xparams.len());
        for (i, expected) in vec!["p1", "p2", "p3"].iter().enumerate() {
            assert_eq!(
                *expected, created_props.xparams[i],
                "Param name mismatch: {} {:?}",
                i, created_props.xparams
            );
        }
        assert_eq!(0, created_props.yparams.len());
        assert!(created_props.xaxis.is_some());
        assert_eq!(
            spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 512.0,
                bins: 514 // Over/underflow.
            },
            created_props.xaxis.unwrap()
        );
        assert!(created_props.yaxis.is_none());
        assert!(created_props.gate.is_none());

        teardown(ch, jh);
    }
    #[test]
    fn multi2_4() {
        let (ch, jh) = setup();

        let properties = make_multi2_properties(&ch);
        let spectrum_api = spectrum_messages::SpectrumMessageClient::new(&ch);
        let mut data = vec![];
        for i in 0..properties.xaxis.unwrap().bins - 1 {
            // +2 for under/overflow.
            data.push(i as f64);
        }
        assert!(make_projection_spectrum(
            &spectrum_api,
            "test1",
            &properties,
            ProjectionDirection::X,
            data.clone()
        )
        .is_ok());

        // The wonky limits get the over/underflow channels.

        let contents = spectrum_api
            .get_contents("test1", -1024.0, 1026.0, -1024.0, 1026.0)
            .expect("Getting spectrum contents");

        // Looks like stuff comes out in order.
        assert_eq!(properties.xaxis.unwrap().bins - 2, contents.len() as u32);
        for i in 0..contents.len() {
            assert_eq!((i + 1) as f64, contents[i].value);
        }
        teardown(ch, jh);
    }
    #[test]
    fn multi2_5() {
        // correct data in y projection

        let (ch, jh) = setup();

        let properties = make_multi2_properties(&ch);
        let spectrum_api = spectrum_messages::SpectrumMessageClient::new(&ch);
        let mut data = vec![];
        for i in 0..properties.yaxis.unwrap().bins - 1 {
            // +2 for under/overflow.
            data.push(i as f64);
        }
        assert!(make_projection_spectrum(
            &spectrum_api,
            "test1",
            &properties,
            ProjectionDirection::X,
            data.clone()
        )
        .is_ok());

        // The wonky limits get the over/underflow channels.

        let contents = spectrum_api
            .get_contents("test1", -1024.0, 1026.0, -1024.0, 1026.0)
            .expect("Getting spectrum contents");

        // Looks like stuff comes out in order.
        assert_eq!(properties.yaxis.unwrap().bins - 2, contents.len() as u32);
        for i in 0..contents.len() {
            assert_eq!((i + 1) as f64, contents[i].value);
        }
        teardown(ch, jh);
    }
    // Tests for projecting a particle gamma spectrum.

    fn make_pgamma_properties(
        chan: &mpsc::Sender<messaging::Request>,
    ) -> spectrum_messages::SpectrumProperties {
        // Make the parameters
        let xparams = vec![
            String::from("xp1"),
            String::from("xp2"),
            String::from("xp3"),
        ];
        let yparams = vec![String::from("yp1"), String::from("yp2")];

        let papi = parameter_messages::ParameterMessageClient::new(&chan);
        for xp in xparams.iter() {
            papi.create_parameter(xp).expect("Making x param");
        }
        for yp in yparams.iter() {
            papi.create_parameter(yp).expect("Making y param");
        }
        // THis must be as gotten back from list_spectra so bins inluce over/under.
        spectrum_messages::SpectrumProperties {
            name: String::from("input"),
            type_name: String::from("PGamma"),
            xparams: xparams.clone(),
            yparams: yparams.clone(),
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1026,
            }),
            yaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 512.0,
                bins: 5124,
            }),
            gate: None,
        }
    }

    #[test]
    fn pgamm_1() {
        // NO error x/y if valid properties.

        let (ch, jh) = setup();
        let props = make_pgamma_properties(&ch);
        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        assert!(
            make_projection_spectrum(&sapi, "test1", &props, ProjectionDirection::X, vec![])
                .is_ok()
        );
        assert!(
            make_projection_spectrum(&sapi, "test2", &props, ProjectionDirection::Y, vec![])
                .is_ok()
        );

        teardown(ch, jh);
    }
    #[test]
    fn pgamm_2() {
        // X has correct properties.
    }
    #[test]
    fn pgamm_3() {
        // Y has correct props.
    }
    #[test]
    fn pgamm_4() {
        // X has correct data.
    }
    #[test]
    fn pgamm_5() {
        // y has correct data
    }
}
