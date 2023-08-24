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

    let resulting_axis = match direction {
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

    // For most cases this is true:

    let params = match direction {
        ProjectionDirection::X => desc.xparams.clone(),
        ProjectionDirection::Y => desc.yparams.clone(),
    };

    // What we do depends on both the spectrum type and direction.
    // Would be nice figure that out all in one swoop but sadly not

    let status = match desc.type_name.as_str() {
        "Multi2D" => {
            // Multi-1d spectrum for all parameters:
            let mut params = desc.xparams.clone();
            params.append(&mut desc.yparams.clone()); // Clone since append consumers.
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
            for i in 0..n {
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
    use crate::conditions::twod;
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
    use crate::messaging::spectrum_messages;
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
            type_name: String::from("1D"),   // not projectable.
            xparams: vec![],
            yparams: vec![],
            xaxis: None,
            yaxis: None,
            gate: None
        };
        // Either direction is bad:
        assert!(make_projection_spectrum(&sapi, "test", &desc, ProjectionDirection::X, data).is_err());
        assert!(make_projection_spectrum(&sapi, "test", &desc, ProjectionDirection::Y, vec![]).is_err());

        teardown(ch, jh);
    }
    #[test]
    fn error_2() {
        // Need xaxis spec in description but it's missing (valid type)

        let (ch, jh) = setup();
        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        let desc = spectrum_messages::SpectrumProperties {
            name: String::from("dummy"),
            type_name: String::from("2D"),   // valid.
            xparams: vec![],
            yparams: vec![],
            xaxis: None,                    // must not be none to project x
            yaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024
            }),
            gate: None
        };
        assert!(make_projection_spectrum(&sapi, "test", &desc, ProjectionDirection::X, vec![]).is_err());

        teardown(ch, jh);
    }
    #[test]
    fn error_3() {
        // Need y axis specification to project in y:

        let (ch, jh) = setup();
        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        let desc = spectrum_messages::SpectrumProperties {
            name: String::from("dummy"),
            type_name: String::from("2D"),   // valid.
            xparams: vec![],
            yparams: vec![],
            xaxis: Some(spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1024
            }),
            yaxis: None,                    // must not be none to project y
            gate: None
        };
        assert!(make_projection_spectrum(&sapi, "test", &desc, ProjectionDirection::Y, vec![]).is_err());

        teardown(ch, jh);
    }
}
