//!
//!   This module contains code to support making
//!  projections of histograms.
//!  It is intended that this set of functions run outside of the
//!  histogram server and, therefore the data it works with  are
//!  data gotten from message exchanges with that server
//!  defined in crate::messaging::*
//!
//!  

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
    F: FnOnce(f64, f64) -> bool,
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
                let coord = if let ProjectionDirection::X = direction {
                    c.x
                } else {
                    c.y
                };
                let bin = spectrum_messages::coord_to_bin(coord, axis);
                v[bin as usize] += c.value;
            }
            Ok(v)
        }
        Err(s) => Err(s),
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
}
