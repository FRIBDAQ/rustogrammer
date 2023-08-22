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
enum ProjectionDirection {
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
