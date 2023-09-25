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
        result.resize(n_element, 0.0_f64);
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
    match make_sum_vector(desc, direction) {
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
/// Create the gate appropriate to a projection spectcrum.  See
/// the description of project below for how this is derived.
///
/// ### Parameters:
///   *  gapi- Gate API instance reference.
///   *  dest_name -  destination spectrum name - used to generate
/// the and gate if needed ("_dest_name_projection_gate_")
///   *  source_desc - References the description of the source spectrum.
///   *  contour - If Some the payload is the name of the AOI contour
///   *  snapshot - If true the specturm is a snapshot spectrum.
fn create_projection_gate(
    gapi: &condition_messages::ConditionMessageClient,
    dest_name: &str,
    source_desc: &spectrum_messages::SpectrumProperties,
    contour: Option<String>,
    snapshot: bool,
) -> Option<String> {
    if snapshot {
        gapi.create_false_condition("_snapshot_condition_");
        return Some(String::from("_snapshot_condition_"));
    }
    // If the spectrum has no gate and there's no contour we don't have a
    // gate:

    if source_desc.gate.is_none() && contour.is_none() {
        return None;
    }
    // If there is a contour but no gate the contour rules:

    if contour.is_some() && source_desc.gate.is_none() {
        return Some(contour.clone().unwrap());
    }

    // If there is no contour and a gate, that's the gate:

    if contour.is_none() && source_desc.gate.is_some() {
        return Some(source_desc.gate.clone().unwrap());
    }
    // If there is a contour and a gate we need to make and condition
    // of the two of them.
    //
    if contour.is_some() && source_desc.gate.is_some() {
        let components = vec![source_desc.gate.clone().unwrap(), contour.unwrap()];
        let and_name = format!("_{}_projection_gate_", dest_name);
        gapi.create_and_condition(&and_name, &components);
        return Some(and_name);
    }

    None
}

/// Actually do the projection.
/// This is called by the ReST handler to:
/// *  Figure out the contents of the projected spectrum.
/// *  Create any needed condition(s) to properly gate that spectrum.
/// *  Create the projection spectrum itself.
/// *  Gate the spectrum as needed.
///
/// ### Parameters:
/// *  sapi - spectrum messaging api reference.
/// *  gapi - Condition/gate messaging api reference.
/// *  source - Name of the spectrum to be projected.
/// *  direction - desired direction of projection.
/// *  dest - Name of the resulting spectrum if successful.
/// *  snapshot - if true, the spectrum is gated with a false condition
/// to keep it from incrementing with new data.
/// *  aoi  - If Some() this _must_ be the name of a contour condition
/// the ponts of which are used to restrict the projection only to those
/// channels in the source spectrum that are within the contour.  If None, the
/// entire spectrum is projected.
/// The parameters in the contour are irelevant.
///
/// ### Returns:
///   Result<(), String>:
///   * Ok - nothing useful is returned.
///   * Err  encapsulates a string error message describing why the
/// projection could not be done.
///
/// ### Note:
/// The final gate placed on the spectrum is determined by the state of the
/// _snapshot_ parameter and any gate that is on the source spectrum.
/// *   If snapshot is **true**, it takes precendence over everything and
/// a false condition gates the resulting spectrum so that it will never increment.
/// The false spectrum will be called _snapshot_condition_ (same name as the one
/// used in spectrum I/O to gate snapshot spectra). If necessary it is created.
/// *   If snapshot is **false**, and the source spectrum has no gate, then the
/// resulting spectrum has no gate,
/// *  If snapshot is **false** and the source spectrum has a gate, the same condition
/// gates the resulting spectrum.
/// If there is a region of interest it is anded with any non-snapshot gate.
///
/// The intent of these gating rules is that either the specturm never increments
/// because it's a snapshot (false gate) or it increments in a manner that makes it
/// a faithful projection
///
///  ### TODO:
///
pub fn project(
    sapi: &spectrum_messages::SpectrumMessageClient,
    gapi: &condition_messages::ConditionMessageClient,
    source: &str,
    direction: ProjectionDirection,
    dest: &str,
    snapshot: bool,
    aoi: Option<String>,
) -> Result<(), String> {
    // Ensure the sapi exists and, if there's an aoi contour that as well
    // if so, compute the projection vector and
    // fill in the destination spectrum.

    let source_desc = sapi.list_spectra(source);
    if let Err(s) = source_desc {
        return Err(format!(
            "Could not get source spectrum info from histogram service: {}",
            s
        ));
    }
    let source_desc = source_desc.unwrap();
    if source_desc.len() != 1 {
        return Err(format!("{} does not specify a unique spectrum", source));
    }
    let source_desc = source_desc[0].clone();
    let xlimits = if let Some(xaxis) = source_desc.xaxis {
        (xaxis.low - 10.0, xaxis.high + 10.0)
    } else {
        (0.0, 0.0)
    };
    let ylimits = if let Some(yaxis) = source_desc.yaxis {
        (yaxis.low - 10.0, yaxis.high + 10.)
    } else {
        (0.0, 0.0)
    };
    let contents = sapi.get_contents(source, xlimits.0, xlimits.1, ylimits.0, ylimits.1);
    if let Err(s) = contents {
        return Err(format!("Failed to get spectrum contents: {}", s));
    }
    let contents = contents.unwrap();
    let data = if let Some(roi) = aoi.clone() {
        let cprops = match gapi.list_conditions(&roi) {
            condition_messages::ConditionReply::Error(s) => {
                return Err(format!("Could not get info for ROI {}", s));
            }
            condition_messages::ConditionReply::Listing(p) => p,
            _ => {
                return Err(String::from("Could not get info for ROI"));
            }
        };

        if cprops.len() != 1 {
            return Err(format!("{} does not uniquely identify a condition", roi));
        }
        let contour = condition_messages::reconstitute_contour(cprops[0].clone());
        if let Err(s) = contour {
            return Err(format!("Could not recontitute {} as a contoure {}", roi, s));
        }
        let contour = contour.unwrap();
        project_spectrum(&source_desc, &contents, direction, |x, y| {
            contour.inside(x, y)
        })
    } else {
        project_spectrum(&source_desc, &contents, direction, |_, _| true)
    };
    if let Err(s) = data {
        return Err(format!("Projection failed: {}", s));
    }
    let data = data.unwrap();

    // Now we can create the spectrum and fill it with our hard won projection data:

    if let Err(s) = make_projection_spectrum(sapi, dest, &source_desc, direction, data) {
        return Err(format!("Failed to create projection spectrum: {}", s));
    }

    // Figure out the correct gate:

    if let Some(g) = create_projection_gate(gapi, dest, &source_desc, aoi.clone(), snapshot) {
        sapi.gate_spectrum(dest, &g)
    } else {
        Ok(())
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
            fold: None,
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
            fold: None,
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
            fold: None,
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
            fold: None,
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
            fold: None,
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
            fold: None,
        };
        let v = make_sum_vector(&props, ProjectionDirection::Y)
            .expect("could not make x projection vector");
        assert_eq!(props.yaxis.unwrap().bins as usize, v.len());
    }
}
#[cfg(test)]
mod project_spectrum_tests {
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
            fold: None,
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
            fold: None,
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
            fold: None,
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
            fold: None,
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
            fold: None,
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
            fold: None,
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
            fold: None,
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
// Tests for make_projection_spectrum  Note these will need a
// server to work properly.
#[cfg(test)]
mod make_spectrum_tests {
    use super::*;
    use crate::messaging;
    use crate::messaging::{parameter_messages, spectrum_messages};
    use crate::test::histogramer_common;
    use std::sync::mpsc;
    use std::thread;

    fn setup() -> (mpsc::Sender<messaging::Request>, thread::JoinHandle<()>) {
        histogramer_common::setup()
    }
    fn teardown(ch: mpsc::Sender<messaging::Request>, jh: thread::JoinHandle<()>) {
        histogramer_common::teardown(ch, jh);
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
            fold: None,
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
            fold: None,
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
            fold: None,
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
        for name in ["p1", "p2", "p3"] {
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
            fold: None,
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
        for (i, expected) in ["p1", "p2", "p3"].iter().enumerate() {
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
        for (i, expected) in ["p1", "p2", "p3"].iter().enumerate() {
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
        for (i, item) in contents.iter().enumerate() {
            assert_eq!((i + 1) as f64, item.value);
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
        for (i, item) in contents.iter().enumerate() {
            assert_eq!((i + 1) as f64, item.value);
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

        let papi = parameter_messages::ParameterMessageClient::new(chan);
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
                bins: 514,
            }),
            gate: None,
            fold: None,
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

        let (ch, jh) = setup();
        let props = make_pgamma_properties(&ch);
        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        assert!(
            make_projection_spectrum(&sapi, "test1", &props, ProjectionDirection::X, vec![])
                .is_ok()
        );
        let props = sapi
            .list_spectra("test1")
            .expect("Getting spectrum properties");
        assert_eq!(1, props.len());
        let props = props[0].clone();

        // CHeck the wonkiness is that the parameters will be the xparameters
        // repeted once to get the incrementing right:

        assert_eq!("test1", props.name);
        assert_eq!("Multi1d", props.type_name);
        assert_eq!(6, props.xparams.len());
        let expected_xparams = ["xp1", "xp2", "xp3", "xp1", "xp2", "xp3"];
        for (i, p) in expected_xparams.iter().enumerate() {
            assert_eq!(*p, props.xparams[i].clone(), "Mismatch on parm {}", i);
        }
        assert_eq!(0, props.yparams.len());
        assert_eq!(
            spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 1024.0,
                bins: 1026,
            },
            props.xaxis.expect("No x axis specification")
        );
        assert!(props.yaxis.is_none());
        assert!(props.gate.is_none());

        teardown(ch, jh);
    }
    #[test]
    fn pgamm_3() {
        // Y has correct props.

        let (ch, jh) = setup();
        let props = make_pgamma_properties(&ch);
        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        assert!(
            make_projection_spectrum(&sapi, "test1", &props, ProjectionDirection::Y, vec![])
                .is_ok()
        );
        let props = sapi
            .list_spectra("test1")
            .expect("Getting spectrum properties");
        assert_eq!(1, props.len());
        let props = props[0].clone();

        // CHeck the wonkiness is that the parameters will be the xparameters
        // repeted once to get the incrementing right:

        assert_eq!("test1", props.name);
        assert_eq!("Multi1d", props.type_name);
        assert_eq!(6, props.xparams.len());
        let expected_xparams = ["yp1", "yp2", "yp1", "yp2", "yp1", "yp2"];
        for (i, p) in expected_xparams.iter().enumerate() {
            assert_eq!(*p, props.xparams[i].clone(), "Mismatch on parm {}", i);
        }
        assert_eq!(0, props.yparams.len());
        assert_eq!(
            spectrum_messages::AxisSpecification {
                low: 0.0,
                high: 512.0,
                bins: 514,
            },
            props.xaxis.expect("No y axis specification")
        );
        assert!(props.yaxis.is_none());
        assert!(props.gate.is_none());

        teardown(ch, jh);
    }
    #[test]
    fn pgamm_4() {
        // X has correct data.

        let (ch, jh) = setup();
        let props = make_pgamma_properties(&ch);
        // Some data:

        let mut data = vec![];
        for i in 0..props.xaxis.unwrap().bins {
            data.push((i + 10) as f64);
        }

        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        assert!(make_projection_spectrum(
            &sapi,
            "test1",
            &props,
            ProjectionDirection::X,
            data.clone()
        )
        .is_ok());

        let contents = sapi
            .get_contents("test1", -2048.0, 2048.0, -2048.0, 2048.0)
            .expect("Getting spectrum contents");
        // No zeroes so
        assert_eq!(data.len(), contents.len());

        for (i, x) in data.iter().enumerate() {
            assert_eq!(*x, contents[i].value, "Data mismatch at index {}", i)
        }

        teardown(ch, jh);
    }
    #[test]
    fn pgamm_5() {
        // y has correct data

        let (ch, jh) = setup();
        let props = make_pgamma_properties(&ch);
        // Some data:

        let mut data = vec![];
        for i in 0..props.yaxis.unwrap().bins {
            data.push((i + 10) as f64);
        }

        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        assert!(make_projection_spectrum(
            &sapi,
            "test1",
            &props,
            ProjectionDirection::Y,
            data.clone()
        )
        .is_ok());

        let contents = sapi
            .get_contents("test1", -2048.0, 2048.0, -2048.0, 2048.0)
            .expect("Getting spectrum contents");
        // No zeroes so
        assert_eq!(data.len(), contents.len());

        for (i, x) in data.iter().enumerate() {
            assert_eq!(*x, contents[i].value, "Data mismatch at index {}", i)
        }

        teardown(ch, jh);
    }
    // Tests for regular 2d spectra.

    fn make_2d_properties(
        ch: &mpsc::Sender<messaging::Request>,
    ) -> spectrum_messages::SpectrumProperties {
        // Make p1, p2 parameters so the new spectrum can be made:

        let api = parameter_messages::ParameterMessageClient::new(ch);
        api.create_parameter("p1").expect("Making p1");
        api.create_parameter("p2").expect("Making p2");

        spectrum_messages::SpectrumProperties {
            name: String::from("input"),
            type_name: String::from("2D"),
            xparams: vec![String::from("p1")],
            yparams: vec![String::from("p2")],
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
            fold: None,
        }
    }
    #[test]
    fn twod_1() {
        // NO error from attemt to project
        let (ch, jh) = setup();
        let props = make_2d_properties(&ch);
        let api = spectrum_messages::SpectrumMessageClient::new(&ch);

        assert!(
            make_projection_spectrum(&api, "test1", &props, ProjectionDirection::X, vec![]).is_ok()
        );
        assert!(
            make_projection_spectrum(&api, "test2", &props, ProjectionDirection::Y, vec![]).is_ok()
        );

        teardown(ch, jh);
    }
    #[test]
    fn twod_2() {
        // Got x projection properties right.

        let (ch, jh) = setup();
        let props = make_2d_properties(&ch);
        let api = spectrum_messages::SpectrumMessageClient::new(&ch);

        assert!(
            make_projection_spectrum(&api, "test1", &props, ProjectionDirection::X, vec![]).is_ok()
        );

        let props = api
            .list_spectra("test1")
            .expect("unable to get spectrum list");
        assert_eq!(1, props.len());
        let props = props[0].clone();

        assert_eq!(
            spectrum_messages::SpectrumProperties {
                name: String::from("test1"),
                type_name: String::from("1D"),
                xparams: vec![String::from("p1")],
                yparams: vec![],
                xaxis: Some(spectrum_messages::AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1026
                }),
                yaxis: None,
                gate: None,
                fold: None
            },
            props
        );

        teardown(ch, jh);
    }
    #[test]
    fn twod_3() {
        // got y projection properties right.

        let (ch, jh) = setup();
        let props = make_2d_properties(&ch);
        let api = spectrum_messages::SpectrumMessageClient::new(&ch);

        assert!(
            make_projection_spectrum(&api, "test1", &props, ProjectionDirection::Y, vec![]).is_ok()
        );

        let props = api
            .list_spectra("test1")
            .expect("unable to get spectrum list");
        assert_eq!(1, props.len());
        let props = props[0].clone();

        assert_eq!(
            spectrum_messages::SpectrumProperties {
                name: String::from("test1"),
                type_name: String::from("1D"),
                xparams: vec![String::from("p2")],
                yparams: vec![],
                xaxis: Some(spectrum_messages::AxisSpecification {
                    low: 0.0,
                    high: 512.0,
                    bins: 514
                }),
                yaxis: None,
                gate: None,
                fold: None
            },
            props
        );

        teardown(ch, jh);
    }
    #[test]
    fn twod_4() {
        // got x projection contents right

        let (ch, jh) = setup();
        let props = make_2d_properties(&ch);
        let api = spectrum_messages::SpectrumMessageClient::new(&ch);
        let mut data = vec![];
        for i in 0..props.xaxis.unwrap().bins {
            data.push((i + 10) as f64); // So all bins have data.
        }

        make_projection_spectrum(&api, "test", &props, ProjectionDirection::X, data.clone())
            .expect("Making x projection");

        // Get the contents and compare:

        let projection = api
            .get_contents("test", -2048.0, 2048.0, -2048.0, 2048.0)
            .expect("Getting contents");
        assert_eq!(data.len(), projection.len());

        for (i, x) in data.iter().enumerate() {
            assert_eq!(
                *x, projection[i].value,
                "Mismatch at entry: {} {:?}",
                i, projection[i]
            );
        }

        teardown(ch, jh);
    }
    #[test]
    fn twod_5() {
        // got y projection contents right.

        let (ch, jh) = setup();
        let props = make_2d_properties(&ch);
        let api = spectrum_messages::SpectrumMessageClient::new(&ch);
        let mut data = vec![];
        for i in 0..props.yaxis.unwrap().bins {
            data.push((i + 10) as f64); // So all bins have data.
        }

        make_projection_spectrum(&api, "test", &props, ProjectionDirection::Y, data.clone())
            .expect("Making x projection");

        // Get the contents and compare:

        let projection = api
            .get_contents("test", -2048.0, 2048.0, -2048.0, 2048.0)
            .expect("Getting contents");
        assert_eq!(data.len(), projection.len());

        for (i, x) in data.iter().enumerate() {
            assert_eq!(
                *x, projection[i].value,
                "Mismatch at entry: {} {:?}",
                i, projection[i]
            );
        }

        teardown(ch, jh);
    }
    // Test projection of 2d sum spectra.

    fn make_2dsum_properties(
        ch: &mpsc::Sender<messaging::Request>,
    ) -> spectrum_messages::SpectrumProperties {
        // Make some parameters - 2dsums require the same number of x/y parameters.

        let papi = parameter_messages::ParameterMessageClient::new(ch);
        for x in ["x1", "x2", "x3"] {
            papi.create_parameter(x).expect("Making an x parameter");
        }
        for y in ["y1", "y2", "y3"] {
            papi.create_parameter(y).expect("making a y parameter");
        }

        spectrum_messages::SpectrumProperties {
            name: String::from("input"),
            type_name: String::from("2DSum"),
            xparams: vec![String::from("x1"), String::from("x2"), String::from("x3")],
            yparams: vec![String::from("y1"), String::from("y2"), String::from("y3")],
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
            fold: None,
        }
    }
    #[test]
    fn sum2_1() {
        // X/Y projections make no errors.

        let (ch, jh) = setup();
        let desc = make_2dsum_properties(&ch);
        let api = spectrum_messages::SpectrumMessageClient::new(&ch);
        assert!(
            make_projection_spectrum(&api, "test1", &desc, ProjectionDirection::X, vec![]).is_ok()
        );
        assert!(
            make_projection_spectrum(&api, "test2", &desc, ProjectionDirection::Y, vec![]).is_ok()
        );

        teardown(ch, jh);
    }
    #[test]
    fn sum2_2() {
        // X projection has correct properties.

        let (ch, jh) = setup();
        let desc = make_2dsum_properties(&ch);
        let api = spectrum_messages::SpectrumMessageClient::new(&ch);
        assert!(
            make_projection_spectrum(&api, "test1", &desc, ProjectionDirection::X, vec![]).is_ok()
        );

        let props = api
            .list_spectra("test1")
            .expect("failed to get spectrum list");
        assert_eq!(1, props.len());
        let props = props[0].clone();

        assert_eq!(
            spectrum_messages::SpectrumProperties {
                name: String::from("test1"),
                type_name: String::from("Multi1d"),
                xparams: vec![String::from("x1"), String::from("x2"), String::from("x3"),],
                yparams: vec![],
                xaxis: Some(spectrum_messages::AxisSpecification {
                    low: 0.0,
                    high: 1024.0,
                    bins: 1026,
                }),
                yaxis: None,
                gate: None,
                fold: None
            },
            props
        );

        teardown(ch, jh);
    }
    #[test]
    fn sum2_3() {
        // Y projevction has correct properties.

        let (ch, jh) = setup();
        let desc = make_2dsum_properties(&ch);
        let api = spectrum_messages::SpectrumMessageClient::new(&ch);
        assert!(
            make_projection_spectrum(&api, "test1", &desc, ProjectionDirection::Y, vec![]).is_ok()
        );

        let props = api
            .list_spectra("test1")
            .expect("failed to get spectrum list");
        assert_eq!(1, props.len());
        let props = props[0].clone();

        assert_eq!(
            spectrum_messages::SpectrumProperties {
                name: String::from("test1"),
                type_name: String::from("Multi1d"),
                xparams: vec![String::from("y1"), String::from("y2"), String::from("y3"),],
                yparams: vec![],
                xaxis: Some(spectrum_messages::AxisSpecification {
                    low: 0.0,
                    high: 512.0,
                    bins: 514,
                }),
                yaxis: None,
                gate: None,
                fold: None
            },
            props
        );

        teardown(ch, jh);
    }
    #[test]
    fn sum2_4() {
        // Xprojection has correct contents
        let (ch, jh) = setup();
        let desc = make_2dsum_properties(&ch);
        let api = spectrum_messages::SpectrumMessageClient::new(&ch);
        let mut data = vec![];
        for i in 0..desc.xaxis.unwrap().bins {
            data.push((i + 10) as f64);
        }
        assert!(make_projection_spectrum(
            &api,
            "test1",
            &desc,
            ProjectionDirection::X,
            data.clone()
        )
        .is_ok());

        let projection = api
            .get_contents("test1", -2048.0, 2048.0, -2048.0, 2048.0)
            .expect("Getting spectrum contents");
        assert_eq!(data.len(), projection.len());
        for (i, x) in data.iter().enumerate() {
            assert_eq!(
                *x, projection[i].value,
                "comparison failed: {}: {:?}",
                i, projection[i]
            );
        }

        teardown(ch, jh);
    }
    #[test]
    fn sum2_5() {
        // y projectinohas corect contents.
        let (ch, jh) = setup();
        let desc = make_2dsum_properties(&ch);
        let api = spectrum_messages::SpectrumMessageClient::new(&ch);
        let mut data = vec![];
        for i in 0..desc.yaxis.unwrap().bins {
            data.push((i + 10) as f64);
        }
        assert!(make_projection_spectrum(
            &api,
            "test1",
            &desc,
            ProjectionDirection::Y,
            data.clone()
        )
        .is_ok());

        let projection = api
            .get_contents("test1", -2048.0, 2048.0, -2048.0, 2048.0)
            .expect("Getting spectrum contents");
        assert_eq!(data.len(), projection.len());
        for (i, x) in data.iter().enumerate() {
            assert_eq!(
                *x, projection[i].value,
                "comparison failed: {}: {:?}",
                i, projection[i]
            );
        }

        teardown(ch, jh);
    }
}
#[cfg(test)]
mod project_tests {

    use super::*;
    use crate::messaging;
    use crate::messaging::{condition_messages, parameter_messages, spectrum_messages};
    use crate::test::histogramer_common;

    use std::sync::mpsc;
    use std::thread;
    // We need to run the histogram server.
    // and have some parameters and a contour and a source spectrum or two.

    fn setup() -> (mpsc::Sender<messaging::Request>, thread::JoinHandle<()>) {
        let (ch, jh) = histogramer_common::setup();
        let papi = parameter_messages::ParameterMessageClient::new(&ch);
        let capi = condition_messages::ConditionMessageClient::new(&ch);
        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);

        for i in 0..10 {
            let name = format!("param.{}", i);
            papi.create_parameter(&name).expect("Making parameter");
        }

        // make a contour on param.0, param.1:

        let points = vec![
            (100.0, 100.0),
            (200.0, 100.0),
            (200.0, 600.0),
            (100.0, 600.0),
        ];
        if let condition_messages::ConditionReply::Error(s) =
            capi.create_contour_condition("contour", 0, 1, &points)
        {
            panic!("Failed to create contour : {}", s);
        }
        // True gate we can put on the spectrum if we need to for testing:

        if let condition_messages::ConditionReply::Error(s) = capi.create_true_condition("true") {
            panic!("Failed to create true condition {}", s);
        }

        // 2-d spectrum named 'test'

        sapi.create_spectrum_2d(
            "test", "param.0", "param.1", 0.0, 1024.0, 512, 0.0, 1024.0, 512,
        )
        .expect("Creating spectrum");

        (ch, jh)
    }
    fn teardown(ch: mpsc::Sender<messaging::Request>, jh: thread::JoinHandle<()>) {
        histogramer_common::teardown(ch, jh);
    }

    fn get_spectrum_info(
        ch: &mpsc::Sender<messaging::Request>,
        name: &str,
    ) -> spectrum_messages::SpectrumProperties {
        let sapi = spectrum_messages::SpectrumMessageClient::new(ch);

        let listing = sapi
            .list_spectra(name)
            .expect("Getting matching spectrum list");
        assert_eq!(1, listing.len());

        listing[0].clone()
    }

    #[test]
    fn make_gate_1() {
        let (ch, jh) = setup();

        let props = get_spectrum_info(&ch, "test");

        let result = create_projection_gate(
            &condition_messages::ConditionMessageClient::new(&ch),
            "proj",
            &props,
            None,
            false,
        );

        assert!(result.is_none()); // Not snap, no condition, -> no condition.

        teardown(ch, jh);
    }
    #[test]
    fn make_gate_2() {
        // snapshot means it's a snapshot:

        let (ch, jh) = setup();

        let props = get_spectrum_info(&ch, "test");

        let result = create_projection_gate(
            &condition_messages::ConditionMessageClient::new(&ch),
            "proj",
            &props,
            None,
            true,
        );
        assert!(result.is_some());

        let result = result.unwrap();
        assert_eq!("_snapshot_condition_", result);

        teardown(ch, jh);
    }
    #[test]
    fn make_gate_3() {
        // Still snapshot if it's true and there's a projection contour:

        let (ch, jh) = setup();

        let props = get_spectrum_info(&ch, "test");

        let result = create_projection_gate(
            &condition_messages::ConditionMessageClient::new(&ch),
            "proj",
            &props,
            Some(String::from("contour")),
            true,
        );
        assert!(result.is_some());

        let result = result.unwrap();
        assert_eq!("_snapshot_condition_", result);

        teardown(ch, jh);
    }
    #[test]
    fn make_gate_4() {
        // still a snapshot if the spectrum is gated:

        let (ch, jh) = setup();
        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        sapi.gate_spectrum("test", "true").expect("gating spectrum");

        let props = get_spectrum_info(&ch, "test");

        let result = create_projection_gate(
            &condition_messages::ConditionMessageClient::new(&ch),
            "proj",
            &props,
            Some(String::from("contour")),
            true,
        );
        assert!(result.is_some());

        let result = result.unwrap();
        assert_eq!("_snapshot_condition_", result);

        teardown(ch, jh);
    }
    #[test]
    fn make_gate_5() {
        // If not a snapshot, there's an ROI contour and nothing else, that's the gate:

        let (ch, jh) = setup();

        let props = get_spectrum_info(&ch, "test");

        let result = create_projection_gate(
            &condition_messages::ConditionMessageClient::new(&ch),
            "proj",
            &props,
            Some(String::from("contour")),
            false,
        );
        assert!(result.is_some());

        let result = result.unwrap();
        assert_eq!("contour", result);

        teardown(ch, jh);
    }
    #[test]
    fn make_gate_6() {
        // not snapshot - if there's a spectrum gate but no contour it's the spectrum
        // gate.

        let (ch, jh) = setup();
        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        sapi.gate_spectrum("test", "true").expect("gating spectrum");

        let props = get_spectrum_info(&ch, "test");

        let result = create_projection_gate(
            &condition_messages::ConditionMessageClient::new(&ch),
            "proj",
            &props,
            None,
            false,
        );
        assert!(result.is_some());

        let result = result.unwrap();
        assert_eq!("true", result);

        teardown(ch, jh);
    }
    #[test]
    fn make_gate_7() {
        // no snapshot but there's a gate and a contour.
        // The result is a new gate: _proj_projection_gate_ which is an
        // and gate with true and contour as components:

        let (ch, jh) = setup();
        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        sapi.gate_spectrum("test", "true").expect("gating spectrum");

        let props = get_spectrum_info(&ch, "test");

        let result = create_projection_gate(
            &condition_messages::ConditionMessageClient::new(&ch),
            "proj",
            &props,
            Some(String::from("contour")),
            false,
        );
        assert!(result.is_some());

        let result = result.unwrap();
        assert_eq!("_proj_projection_gate_", result);

        let capi = condition_messages::ConditionMessageClient::new(&ch);
        if let condition_messages::ConditionReply::Listing(l) = capi.list_conditions(&result) {
            assert_eq!(1, l.len());
            let cond = l[0].clone();
            assert_eq!("And", cond.type_name);
            assert_eq!(2, cond.gates.len());
            assert_eq!("true", cond.gates[0]);
            assert_eq!("contour", cond.gates[1]);
        } else {
            panic!("Failed to get projection gate information");
        }

        teardown(ch, jh);
    }
    // Tests for fn project:
    // Note that we assume that the dependent functions work (they've all been
    // tested). So we really just need to look the logic inside of that function
    ///

    #[test]
    fn project_1() {
        let (ch, jh) = setup();
        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        let gapi = condition_messages::ConditionMessageClient::new(&ch);

        // Put some data into "test" to project.  We're projecting on x/ no contour:
        // put a horizontal line of data in the test spectrum:

        let mut contents = Vec::<spectrum_messages::Channel>::new();
        for i in 0..512 {
            contents.push(spectrum_messages::Channel {
                chan_type: spectrum_messages::ChannelType::Bin,
                x: (i * 2) as f64,
                y: 512.0,
                bin: 0,
                value: (i + 10) as f64,
            });
        }
        sapi.fill_spectrum("test", contents)
            .expect("Filling 'test' spectrum");

        // Projecting that onto X should give just the channels  we put in.

        project(
            &sapi,
            &gapi,
            "test",
            ProjectionDirection::X,
            "proj",
            false,
            None,
        )
        .expect("Failed to project");

        // Ensure we have created the spectrum and it has the right contents:

        let desc = sapi.list_spectra("proj").expect("Getting spectrum list");
        assert_eq!(1, desc.len());
        let desc = desc[0].clone();

        assert_eq!("1D", desc.type_name);
        assert_eq!(1, desc.xparams.len());
        assert_eq!("param.0", desc.xparams[0]);
        assert_eq!(0, desc.yparams.len());
        assert!(desc.xaxis.is_some());
        let xaxis = desc.xaxis.unwrap();
        assert_eq!(0.0, xaxis.low);
        assert_eq!(1024.0, xaxis.high);
        assert_eq!(514, xaxis.bins); // over/underflow chans.
        assert!(desc.yaxis.is_none());
        assert!(desc.gate.is_none());

        // CHeck the contents:

        let contents = sapi
            .get_contents("proj", -1024.0, 1024.0, -1024.0, 1024.0)
            .expect("get spectrum contents");
        assert_eq!(512, contents.len(), "Contents are: {:?}", contents);

        // Assuming the contents come out in channel order:

        for (i, c) in contents.iter().enumerate() {
            assert_eq!(
                spectrum_messages::Channel {
                    chan_type: spectrum_messages::ChannelType::Bin,
                    x: (i * 2) as f64,
                    y: 0.0,
                    bin: i + 1, // +1 for underflow channel
                    value: (i + 10) as f64,
                },
                *c,
                "Mismatch at {}: {:?}",
                i,
                c
            );
        }

        teardown(ch, jh);
    }
    #[test]
    fn project_2() {
        // Same as above with same data - should get a sum of the
        // line projected over to the Y axis.

        let (ch, jh) = setup();
        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        let gapi = condition_messages::ConditionMessageClient::new(&ch);

        // Put some data into "test" to project.  We're projecting on x/ no contour:
        // put a horizontal line of data in the test spectrum:

        let mut contents = Vec::<spectrum_messages::Channel>::new();
        let mut sum = 0.0; // lazy way.
        for i in 0..512 {
            let value = (i + 10) as f64;
            contents.push(spectrum_messages::Channel {
                chan_type: spectrum_messages::ChannelType::Bin,
                x: (i * 2) as f64,
                y: 512.0,
                bin: 0,
                value,
            });
            sum += value;
        }
        sapi.fill_spectrum("test", contents)
            .expect("Filling 'test' spectrum");

        // Projecting that onto X should give just the channels  we put in.

        project(
            &sapi,
            &gapi,
            "test",
            ProjectionDirection::Y,
            "proj",
            false,
            None,
        )
        .expect("Failed to project");

        // Ensure we have created the spectrum and it has the right contents:

        let desc = sapi.list_spectra("proj").expect("Getting spectrum list");
        assert_eq!(1, desc.len());
        let desc = desc[0].clone();

        assert_eq!("1D", desc.type_name);
        assert_eq!(1, desc.xparams.len());
        assert_eq!("param.1", desc.xparams[0]);
        assert_eq!(0, desc.yparams.len());
        assert!(desc.xaxis.is_some());
        let xaxis = desc.xaxis.unwrap();
        assert_eq!(0.0, xaxis.low);
        assert_eq!(1024.0, xaxis.high);
        assert_eq!(514, xaxis.bins); // over/underflow chans.
        assert!(desc.yaxis.is_none());
        assert!(desc.gate.is_none());

        // CHeck the contents:

        let contents = sapi
            .get_contents("proj", -1024.0, 1024.0, -1024.0, 1024.0)
            .expect("get spectrum contents");
        assert_eq!(1, contents.len());

        assert_eq!(
            spectrum_messages::Channel {
                chan_type: spectrum_messages::ChannelType::Bin,
                x: 512.0,
                y: 0.0,
                bin: 257,
                value: sum
            },
            contents[0],
        );

        teardown(ch, jh);
    }
    #[test]
    fn project_3() {
        // X projection within a simple contour:

        let (ch, jh) = setup();

        // Same data:

        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        let gapi = condition_messages::ConditionMessageClient::new(&ch);

        // Put some data into "test" to project.  We're projecting on x/ no contour:
        // put a horizontal line of data in the test spectrum:

        let mut contents = Vec::<spectrum_messages::Channel>::new();
        for i in 0..512 {
            contents.push(spectrum_messages::Channel {
                chan_type: spectrum_messages::ChannelType::Bin,
                x: (i * 2) as f64,
                y: 512.0,
                bin: 0,
                value: (i + 10) as f64,
            });
        }
        sapi.fill_spectrum("test", contents)
            .expect("Filling 'test' spectrum");

        project(
            &sapi,
            &gapi,
            "test",
            ProjectionDirection::X,
            "proj",
            false,
            Some(String::from("contour")),
        )
        .expect("Projecting");

        // Ensure we have created the spectrum and it has the right contents:

        let desc = sapi.list_spectra("proj").expect("Getting spectrum list");
        assert_eq!(1, desc.len());
        let desc = desc[0].clone();

        assert_eq!("1D", desc.type_name);
        assert_eq!(1, desc.xparams.len());
        assert_eq!("param.0", desc.xparams[0]);
        assert_eq!(0, desc.yparams.len());
        assert!(desc.xaxis.is_some());
        let xaxis = desc.xaxis.unwrap();
        assert_eq!(0.0, xaxis.low);
        assert_eq!(1024.0, xaxis.high);
        assert_eq!(514, xaxis.bins); // over/underflow chans.
        assert!(desc.yaxis.is_none());
        assert!(desc.gate.is_some());
        assert_eq!("contour", desc.gate.unwrap());

        let data = sapi
            .get_contents("proj", -1024.0, 1024.0, -1024.0, 1024.0)
            .expect("Getting contents");
        assert_eq!(50, data.len(), "Size mismatch: {:?}", data);
        let x0 = 102.0; // start point of the ROI.
        for (i, d) in data.iter().enumerate() {
            let x = x0 + (i * 2) as f64;
            let bin = (x / 2.0) as usize;
            assert_eq!(
                spectrum_messages::Channel {
                    chan_type: spectrum_messages::ChannelType::Bin,
                    x,
                    y: 0.0,
                    bin: bin + 1,
                    value: (bin + 10) as f64
                },
                *d
            )
        }
        teardown(ch, jh);
    }
    #[test]
    fn project_4() {
        // Project within a contour in y...

        let (ch, jh) = setup();

        // Same data:

        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        let gapi = condition_messages::ConditionMessageClient::new(&ch);

        // Put some data into "test" to project.  We're projecting on x/ no contour:
        // put a horizontal line of data in the test spectrum:

        let mut contents = Vec::<spectrum_messages::Channel>::new();
        let mut sum = 0.0;
        for i in 0..512 {
            let value = (i + 10) as f64;
            let x = (i * 2) as f64;
            if x > 100.0 && x <= 200.0 {
                sum += value;
            }
            contents.push(spectrum_messages::Channel {
                chan_type: spectrum_messages::ChannelType::Bin,
                x,
                y: 512.0,
                bin: 0,
                value,
            });
        }
        sapi.fill_spectrum("test", contents)
            .expect("Filling 'test' spectrum");

        project(
            &sapi,
            &gapi,
            "test",
            ProjectionDirection::Y,
            "proj",
            false,
            Some(String::from("contour")),
        )
        .expect("Projecting");

        // Ensure we have created the spectrum and it has the right contents:

        let desc = sapi.list_spectra("proj").expect("Getting spectrum list");
        assert_eq!(1, desc.len());
        let desc = desc[0].clone();

        assert_eq!("1D", desc.type_name);
        assert_eq!(1, desc.xparams.len());
        assert_eq!("param.1", desc.xparams[0]);
        assert_eq!(0, desc.yparams.len());
        assert!(desc.xaxis.is_some());
        let xaxis = desc.xaxis.unwrap();
        assert_eq!(0.0, xaxis.low);
        assert_eq!(1024.0, xaxis.high);
        assert_eq!(514, xaxis.bins); // over/underflow chans.
        assert!(desc.yaxis.is_none());
        assert!(desc.gate.is_some());
        assert_eq!("contour", desc.gate.unwrap());

        let data = sapi
            .get_contents("proj", -1024.0, 1024.0, -1024.0, 1024.0)
            .expect("Getting contents");
        assert_eq!(1, data.len(), "Size mismatch: {:?}", data);
        assert_eq!(
            spectrum_messages::Channel {
                chan_type: spectrum_messages::ChannelType::Bin,
                x: 512.0,
                y: 0.0,
                bin: (512.0 / 2.0) as usize + 1,
                value: sum
            },
            data[0]
        );

        teardown(ch, jh);
    }
    #[test]
    fn project_5() {
        // X projection within a contour as for project_3 but
        // the source spectrum is also gated. Should not change
        // the outcome other than the gate on the final spectrum and
        // its characteristics.

        // Project within a contour in y...

        let (ch, jh) = setup();

        // Same data:

        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        let gapi = condition_messages::ConditionMessageClient::new(&ch);

        // gate the test spectrum:

        sapi.gate_spectrum("test", "true").expect("Gating spectrum");

        // Put some data into "test" to project.  We're projecting on x/ no contour:
        // put a horizontal line of data in the test spectrum:

        let mut contents = Vec::<spectrum_messages::Channel>::new();

        for i in 0..512 {
            let value = (i + 10) as f64;
            let x = (i * 2) as f64;

            contents.push(spectrum_messages::Channel {
                chan_type: spectrum_messages::ChannelType::Bin,
                x,
                y: 512.0,
                bin: 0,
                value,
            });
        }
        sapi.fill_spectrum("test", contents)
            .expect("Filling 'test' spectrum");

        project(
            &sapi,
            &gapi,
            "test",
            ProjectionDirection::X,
            "proj",
            false,
            Some(String::from("contour")),
        )
        .expect("Projecting");

        // Ensure we have created the spectrum and it has the right contents:

        let desc = sapi.list_spectra("proj").expect("Getting spectrum list");
        assert_eq!(1, desc.len());
        let desc = desc[0].clone();

        assert_eq!("1D", desc.type_name);
        assert_eq!(1, desc.xparams.len());
        assert_eq!("param.0", desc.xparams[0]);
        assert_eq!(0, desc.yparams.len());
        assert!(desc.xaxis.is_some());
        let xaxis = desc.xaxis.clone().unwrap();
        assert_eq!(0.0, xaxis.low);
        assert_eq!(1024.0, xaxis.high);
        assert_eq!(514, xaxis.bins); // over/underflow chans.
        assert!(desc.yaxis.is_none());
        assert!(desc.gate.is_some());
        assert_eq!("_proj_projection_gate_", desc.gate.unwrap());

        let data = sapi
            .get_contents("proj", -1024.0, 1024.0, -1024.0, 1024.0)
            .expect("Getting contents");
        assert_eq!(50, data.len(), "Size mismatch: {:?}", data);
        let x0 = 102.0; // start point of the ROI.
        for (i, d) in data.iter().enumerate() {
            let x = x0 + (i * 2) as f64;
            let bin = (x / 2.0) as usize;
            assert_eq!(
                spectrum_messages::Channel {
                    chan_type: spectrum_messages::ChannelType::Bin,
                    x,
                    y: 0.0,
                    bin: bin + 1,
                    value: (bin + 10) as f64
                },
                *d
            );
        }

        // See that the gate is correct:

        match gapi.list_conditions("_proj_projection_gate_") {
            condition_messages::ConditionReply::Error(s) => assert!(false, "{}", s),
            condition_messages::ConditionReply::Listing(v) => {
                assert_eq!(1, v.len());
                let gate = v[0].clone();
                assert_eq!(
                    condition_messages::ConditionProperties {
                        cond_name: String::from("_proj_projection_gate_"),
                        type_name: String::from("And"),
                        points: vec![],
                        gates: vec![String::from("true"), String::from("contour")],
                        parameters: vec![]
                    },
                    gate
                );
            }
            _ => panic!("Unexpected return type from gate list"),
        };

        teardown(ch, jh);
    }
    #[test]
    fn project_6() {
        // y projection of a gated spectrum:

        let (ch, jh) = setup();

        // Same data:

        let sapi = spectrum_messages::SpectrumMessageClient::new(&ch);
        let gapi = condition_messages::ConditionMessageClient::new(&ch);

        sapi.gate_spectrum("test", "true").expect("Gating spectrum");

        // Put some data into "test" to project.  We're projecting on x/ no contour:
        // put a horizontal line of data in the test spectrum:

        let mut contents = Vec::<spectrum_messages::Channel>::new();
        let mut sum = 0.0;
        for i in 0..512 {
            let value = (i + 10) as f64;
            let x = (i * 2) as f64;
            if x > 100.0 && x <= 200.0 {
                sum += value;
            }
            contents.push(spectrum_messages::Channel {
                chan_type: spectrum_messages::ChannelType::Bin,
                x,
                y: 512.0,
                bin: 0,
                value,
            });
        }
        sapi.fill_spectrum("test", contents)
            .expect("Filling 'test' spectrum");

        project(
            &sapi,
            &gapi,
            "test",
            ProjectionDirection::Y,
            "proj",
            false,
            Some(String::from("contour")),
        )
        .expect("Projecting");

        // Ensure we have created the spectrum and it has the right contents:

        let desc = sapi.list_spectra("proj").expect("Getting spectrum list");
        assert_eq!(1, desc.len());
        let desc = desc[0].clone();

        assert_eq!("1D", desc.type_name);
        assert_eq!(1, desc.xparams.len());
        assert_eq!("param.1", desc.xparams[0]);
        assert_eq!(0, desc.yparams.len());
        assert!(desc.xaxis.is_some());
        let xaxis = desc.xaxis.unwrap();
        assert_eq!(0.0, xaxis.low);
        assert_eq!(1024.0, xaxis.high);
        assert_eq!(514, xaxis.bins); // over/underflow chans.
        assert!(desc.yaxis.is_none());
        assert!(desc.gate.is_some());
        assert_eq!("_proj_projection_gate_", desc.gate.unwrap());

        let data = sapi
            .get_contents("proj", -1024.0, 1024.0, -1024.0, 1024.0)
            .expect("Getting contents");
        assert_eq!(1, data.len(), "Size mismatch: {:?}", data);
        assert_eq!(
            spectrum_messages::Channel {
                chan_type: spectrum_messages::ChannelType::Bin,
                x: 512.0,
                y: 0.0,
                bin: (512.0 / 2.0) as usize + 1,
                value: sum
            },
            data[0]
        );

        match gapi.list_conditions("_proj_projection_gate_") {
            condition_messages::ConditionReply::Error(s) => assert!(false, "{}", s),
            condition_messages::ConditionReply::Listing(v) => {
                assert_eq!(1, v.len());
                let gate = v[0].clone();
                assert_eq!(
                    condition_messages::ConditionProperties {
                        cond_name: String::from("_proj_projection_gate_"),
                        type_name: String::from("And"),
                        points: vec![],
                        gates: vec![String::from("true"), String::from("contour")],
                        parameters: vec![]
                    },
                    gate
                );
            }
            _ => panic!("Unexpected return type from gate list"),
        }
        teardown(ch, jh);
    }
}
