//!
//!  This module contains code that can integrate spectra.
//!  Integration is a matter of selecting a function that
//!  determines if channels in a spectrum should be added to the
//!  sums needed to compute sum, centroid and fwhm given iteration over
//!  the channels in a spectrum.
//!
//!  There only needs to be one integrate function which chooses from
//!  among the following summers:
//!
//!  *  onedrange - Integration 1d from low/high pair.
//!  *  twod      - Integrate 2d over contour object.
//!
//! Each of these takes what iter gave it and returns
//! Option<SumElement> object.
//!  Where None indicates the iteration is not to be included in the integration.
//!  Some  if the integration is included and, in that case,
//!  the payload are the sum elements for the integration.

use crate::conditions::twod;
use crate::messaging::spectrum_messages; // Need for reconstiting contours.

use libm::sqrt;

/// Multiplier from deviance to FWHM under Gaussian assumption:

const GAMMA: f64 = 2.3548200450309493820231386529194; // 2.0_f64 * sqrt(2.0_f64 * 2.0_f64.ln());

///  This is the payload of a sum.  It's the same for 1-d and 2d integrations:
struct SumElement {
    contents: f64,    // channel contents
    wsum: (f64, f64), // cahnnel contents weighted by x/y positions.
}
/// Used to hold the region of interest in which a sum is being done.
#[derive(Clone)]
pub enum AreaOfInterest {
    Oned {
        // OneD slice of interest
        low: f64,
        high: f64,
    },
    Twod(twod::Contour), // 2d contour of interest.
    All,                 // No limits.
}
/// The results of an integration.

#[derive(PartialEq, Debug)]
pub struct Integration {
    pub sum: f64,
    pub centroid: (f64, f64),
    pub fwhm: (f64, f64),
}
// This function handles a single channel returning a SumElement
// Parameters
//  ch : the channel to evaluate.
//  aoi : the area of interest.
// Returns:
//   SumElement - note that if the channel is outside the AOI then
// this is filled with zeroes.
//
// Note that over/underflow bins are never included in the result

fn sum_channel(chan: &spectrum_messages::Channel, aoi: &AreaOfInterest) -> SumElement {
    match chan.chan_type {
        spectrum_messages::ChannelType::Underflow | spectrum_messages::ChannelType::Overflow => {
            SumElement {
                contents: 0.0,
                wsum: (0.0, 0.0),
            }
        }
        spectrum_messages::ChannelType::Bin => {
            if match aoi {
                AreaOfInterest::All => true,
                AreaOfInterest::Oned { low, high } => (chan.x >= *low) && (chan.x <= *high),
                AreaOfInterest::Twod(c) => c.inside(chan.x, chan.y),
            } {
                SumElement {
                    contents: chan.value,
                    wsum: (chan.value * chan.x, chan.value * chan.y),
                }
            } else {
                SumElement {
                    contents: 0.0,
                    wsum: (0.0, 0.0),
                }
            }
        }
    }
}
// COmpute (centroid.x, centroid.y, counts)
fn centroid(
    contents: &spectrum_messages::SpectrumContents,
    aoi: &AreaOfInterest,
) -> (f64, f64, f64) {
    let mut wsums = (0.0_f64, 0.0_f64);
    let mut counts = 0.0_f64;

    for chan in contents {
        let contribution = sum_channel(&chan, aoi);
        counts += contribution.contents;
        wsums.0 += contribution.wsum.0;
        wsums.1 += contribution.wsum.1;
    }
    if counts > 0.0 {
        (wsums.0 / counts, wsums.1 / counts, counts)
    } else {
        (0.0, 0.0, 0.0)
    }
}
// Compute (fwhm.x fwhm.y)
fn fwhm(
    centroid: (f64, f64),
    total_counts: f64,
    contents: &spectrum_messages::SpectrumContents,
    aoi: &AreaOfInterest,
) -> (f64, f64) {
    let mut sqsums = (0.0_f64, 0.0_f64);
    for chan in contents {
        let contribution = sum_channel(&chan, aoi);
        sqsums.0 += contribution.contents * (chan.x - centroid.0) * (chan.x - centroid.0);
        sqsums.1 += contribution.contents * (chan.y - centroid.1) * (chan.y - centroid.1);
    }

    // sqsums _cannot_ be negative so:

    if total_counts > 0.0 {
        (
            sqrt(sqsums.0) * GAMMA / total_counts,
            sqrt(sqsums.1) * GAMMA / total_counts,
        )
    } else {
        (0.0, 0.0)
    }
}

/// Integrate a spectrum within a region of interest.
/// For 1-d spectra the region of interest is supplied by a low/high pair.
/// for 2-d spectra a contour describes the region of interest.
/// Note that to integrate the entire spectrum:
/// *  1-d just use limits that include the entire range of the axis.
/// *  Construct a rectangular contour that spans the entire range of both axes.
///
/// ### Parameters:
///   *  contents - spectrum contents gotten from the histogram server.
///   *  aoi - an area of interest that defines the region of integration.
///
/// ### Returns:
///   Instanceo of an Integration.
/// ### Notes:
///   *  The caller can limit the data returned so that fewer channels are examined.
///   *  Only zero or none of limits
///   * This takes two runs over the data but is less likely to overflow.
pub fn integrate(
    contents: &spectrum_messages::SpectrumContents,
    aoi: AreaOfInterest,
) -> Integration {
    let (cx, cy, counts) = centroid(contents, &aoi);
    let width = fwhm((cx, cy), counts, contents, &aoi);

    Integration {
        sum: counts,
        centroid: (cx, cy),
        fwhm: width,
    }
}

#[cfg(test)]
mod test_utilities {
    use crate::conditions::twod::{Contour, Point};

    // Utility to make an nice diamond for contour tests:
    //

    pub fn make_contour() -> Contour {
        Contour::new(
            0,
            1,
            vec![
                Point::new(100.0, 0.0),
                Point::new(150.0, 50.0),
                Point::new(100.0, 100.0),
                Point::new(50.0, 50.0),
            ],
        )
        .unwrap()
    }
}

#[cfg(test)]
mod integrate_channel_tests {
    use super::*;
    use crate::messaging::spectrum_messages::{Channel, ChannelType};

    #[test]
    fn none_1() {
        // No restriction regular bin - contributes:

        let chan = Channel {
            chan_type: ChannelType::Bin,
            x: 10.0,
            y: 20.0,
            bin: 0,
            value: 100.0,
        };
        let aoi = AreaOfInterest::All;

        let value = sum_channel(&chan, &aoi);
        assert_eq!(100.0, value.contents);
        assert_eq!(100.0 * 10.0, value.wsum.0);
        assert_eq!(100.0 * 20.0, value.wsum.1);
    }
    #[test]
    fn none_2() {
        // Never include overflow:

        let chan = Channel {
            chan_type: ChannelType::Overflow,
            x: 10.0,
            y: 20.0,
            bin: 0,
            value: 100.0,
        };
        let aoi = AreaOfInterest::All;

        let value = sum_channel(&chan, &aoi);

        assert_eq!(0.0, value.contents);
        assert_eq!((0.0, 0.0), value.wsum);
    }
    #[test]
    fn none_3() {
        // never include underflows:

        let chan = Channel {
            chan_type: ChannelType::Underflow,
            x: 10.0,
            y: 20.0,
            bin: 0,
            value: 100.0,
        };
        let aoi = AreaOfInterest::All;

        let value = sum_channel(&chan, &aoi);

        assert_eq!(0.0, value.contents);
        assert_eq!((0.0, 0.0), value.wsum);
    }
    #[test]
    fn cut_1() {
        // Limits inside for 1d.

        let chan = Channel {
            chan_type: ChannelType::Bin,
            x: 100.0,
            y: 0.0,
            bin: 0,
            value: 120.0,
        };
        let aoi = AreaOfInterest::Oned {
            low: 10.0,
            high: 200.0,
        };

        let value = sum_channel(&chan, &aoi);
        assert_eq!(120.0, value.contents);
        assert_eq!((100.0 * 120.0, 0.0), value.wsum);
    }
    #[test]
    fn cut_2() {
        // Limits lower than cut:

        let chan = Channel {
            chan_type: ChannelType::Bin,
            x: 100.0,
            y: 0.0,
            bin: 0,
            value: 120.0,
        };
        let aoi = AreaOfInterest::Oned {
            low: 101.0,
            high: 200.0,
        };

        let value = sum_channel(&chan, &aoi);
        assert_eq!(0.0, value.contents);
        assert_eq!((0.0, 0.0), value.wsum);
    }
    #[test]
    fn cut_3() {
        // Limits higher than cut.

        let chan = Channel {
            chan_type: ChannelType::Bin,
            x: 100.0,
            y: 0.0,
            bin: 0,
            value: 120.0,
        };
        let aoi = AreaOfInterest::Oned {
            low: 10.0,
            high: 99.0,
        };

        let value = sum_channel(&chan, &aoi);
        assert_eq!(0.0, value.contents);
        assert_eq!((0.0, 0.0), value.wsum);
    }

    #[test]
    fn contour_1() {
        // point is inside a contour

        let chan = Channel {
            chan_type: ChannelType::Bin,
            x: 100.0, // On the midline of the contour.
            y: 50.0,
            bin: 0,
            value: 100.0,
        };
        let aoi = AreaOfInterest::Twod(test_utilities::make_contour());

        let value = sum_channel(&chan, &aoi);
        assert_eq!(100.0, value.contents);
        assert_eq!((100.0 * 100.0, 100.0 * 50.0), value.wsum);
    }
    #[test]
    fn contour_2() {
        // Out at upper quad.

        let chan = Channel {
            chan_type: ChannelType::Bin,
            x: 50.0,
            y: 75.0,
            bin: 0,
            value: 100.0,
        };
        let aoi = AreaOfInterest::Twod(test_utilities::make_contour());

        let value = sum_channel(&chan, &aoi);
        assert_eq!(0.0, value.contents);
        assert_eq!((0.0, 0.0), value.wsum);
    }
}
#[cfg(test)]
mod integration_tests {
    use super::test_utilities::make_contour;
    use super::*;
    use crate::messaging::spectrum_messages::{Channel, ChannelType, SpectrumContents};

    // Make a 1d spike at where that's got how_high counts:

    fn make_spike_1d(x: f64, how_high: f64) -> SpectrumContents {
        vec![Channel {
            chan_type: ChannelType::Bin,
            x,
            y: 0.0,
            bin: 0,
            value: how_high,
        }]
    }
    fn make_spike_2d(x: f64, y: f64, how_high: f64) -> SpectrumContents {
        vec![Channel {
            chan_type: ChannelType::Bin,
            x,
            y,
            bin: 0,
            value: how_high,
        }]
    }

    #[test]
    fn empty_1() {
        // Empty spectrum returns 0's:

        let contents: SpectrumContents = vec![];
        let result = integrate(&contents, AreaOfInterest::All);

        assert_eq!(
            Integration {
                sum: 0.0,
                centroid: (0.0, 0.0),
                fwhm: (0.0, 0.0)
            },
            result
        );
    }
    #[test]
    fn spike1_1() {
        // 1d spke for all:

        let contents = make_spike_1d(100.0, 250.0);
        let result = integrate(&contents, AreaOfInterest::All);

        assert_eq!(
            Integration {
                sum: 250.0,
                centroid: (100.0, 0.0),
                fwhm: (0.0, 0.0)
            },
            result
        );
    }
    #[test]
    fn spike1_2() {
        // The AOI is a slice but the spike is inside the slice:

        let contents = make_spike_1d(100.0, 250.0);
        let result = integrate(
            &contents,
            AreaOfInterest::Oned {
                low: 50.0,
                high: 150.0,
            },
        );

        assert_eq!(
            Integration {
                sum: 250.0,
                centroid: (100.0, 0.0),
                fwhm: (0.0, 0.0)
            },
            result
        );
    }
    #[test]
    fn spike1_3() {
        // AOI is a slice with the spike outside the slice:

        let contents = make_spike_1d(100.0, 250.0);
        let result = integrate(
            &contents,
            AreaOfInterest::Oned {
                low: 150.0,
                high: 250.0,
            },
        );

        assert_eq!(
            Integration {
                sum: 0.0,
                centroid: (0.0, 0.0),
                fwhm: (0.0, 0.0)
            },
            result
        );
    }
    #[test]
    fn oned() {
        // Only slightly more interesting data

        let mut contents = make_spike_1d(100.0, 250.0);
        let spike2 = make_spike_1d(110.0, 200.0);
        contents.push(spike2[0]);
        let result = integrate(&contents, AreaOfInterest::All);
        assert_eq!(450.0, result.sum);
        let csbc = (100.0 * 250.0 + 110.0 * 200.0) / 450.0;
        assert_eq!((csbc, 0.0), result.centroid);

        let sqsum =
            250.0 * (100.0 - csbc) * (100.0 - csbc) + 200.0 * (110.0 - csbc) * (110.0 - csbc);
        let fwhm = sqrt(sqsum) * GAMMA / 450.0;

        assert_eq!((fwhm, 0.0), result.fwhm);
    }

    // 2-d integrations

    #[test]
    fn spike2_1() {
        // single spike - AOI is all:

        let contents = make_spike_2d(100.0, 200.0, 400.0);
        let result = integrate(&contents, AreaOfInterest::All);

        assert_eq!(
            Integration {
                sum: 400.0,
                centroid: (100.0, 200.0),
                fwhm: (0.0, 0.0)
            },
            result
        );
    }
    #[test]
    fn spike2_2() {
        // single spike inside contour AOI

        let contents = make_spike_2d(100.0, 50.0, 1234.0);
        let result = integrate(&contents, AreaOfInterest::Twod(make_contour()));

        assert_eq!(
            Integration {
                sum: 1234.0,
                centroid: (100.0, 50.0),
                fwhm: (0.0, 0.0)
            },
            result
        )
    }
    #[test]
    fn spike2_3() {
        // Single spike outside the AOI:

        let contents = make_spike_2d(150.0, 60.0, 1243.0); // Right and above.
        let result = integrate(&contents, AreaOfInterest::Twod(make_contour()));
        assert_eq!(
            Integration {
                sum: 0.0,
                centroid: (0.0, 0.0),
                fwhm: (0.0, 0.0)
            },
            result
        );
    }
    #[test]
    fn twod_1() {
        // A couple of spikes:

        let mut contents = make_spike_2d(100.0, 60.0, 100.0);
        let other_spike = make_spike_2d(120.0, 70.0, 150.0);
        contents.push(other_spike[0]);
        let result = integrate(&contents, AreaOfInterest::All);

        // X centroid and fwhm:

        let cx: f64 = (100.0 * 100.0 + 120.0 * 150.0) / 250.0;
        let var: f64 = 100.0 * (100.0 - cx).powi(2) + 150.0 * (120.0 - cx).powi(2);
        let fwhmx = GAMMA * sqrt(var) / 250.0;

        assert_eq!(250.0, result.sum);
        assert_eq!(cx, result.centroid.0);
        assert_eq!(fwhmx, result.fwhm.0);

        // ... and in the y direction:

        let cy: f64 = (100.0 * 60.0 + 150.0 * 70.0) / 250.0;
        let var: f64 = 100.0 * (60.0 - cy).powi(2) + 150.0 * (70.0 - cy).powi(2);
        let fwhmy = GAMMA * sqrt(var) / 250.0;
        assert_eq!(cy, result.centroid.1);
        assert_eq!(fwhmy, result.fwhm.1);
    }
}
