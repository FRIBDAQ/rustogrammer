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
    sum: f64,
    centroid: (f64, f64),
    fwhm: (f64, f64),
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
pub fn integrate(
    contents: &spectrum_messages::SpectrumContents,
    aoi: AreaOfInterest,
) -> Integration {
    let mut result = Integration {
        sum: 0.0,
        centroid: (0.0, 0.0),
        fwhm: (0.0, 0.0),
    };
    let mut counts: f64 = 0.0;
    let mut wsum: (f64, f64) = (0.0, 0.0);
    let mut wsumsq = (0.0_f64, 0.0_f64);
    for chan in contents {
        let contribution = sum_channel(&chan, &aoi);
        counts += contribution.contents;
        wsum.0 += contribution.wsum.0;
        wsum.1 += contribution.wsum.1;

        wsumsq.0 += wsum.0 * wsum.0;
        wsumsq.1 + -wsum.1 * wsum.1;
    }

    // It's possible for the ROI to be empty in which case we can't do the
    // divisions:

    if counts != 0.0 {
        let centroid = (wsum.0 / counts, wsum.1 / counts);

        let variance = (wsumsq.0 - wsum.0 * wsum.0, wsumsq.1 - wsum.1 * wsum.1);

        let variance = (
            if variance.0 > 0.0 {
                sqrt(wsumsq.0) / counts
            } else {
                0.0
            },
            if variance.1 > 0.0 {
                sqrt(wsumsq.1) / counts
            } else {
                0.0
            },
        );

        result.sum = counts;
        result.centroid = (centroid.0, centroid.1);
        result.fwhm = (variance.0 * GAMMA, variance.1 * GAMMA);
    }

    result
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
    use super::*;
    use crate::messaging::spectrum_messages::{Channel, ChannelType, SpectrumContents};

    // Make a 1d spike at where that's got how_high counts:

    fn make_spike_1d(x: f64, how_high: f64) -> SpectrumContents {
        vec![Channel {
            chan_type: ChannelType::Bin,
            x: x,
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
        let spike2 = make_spike_1d(110.0, 250.0);
        contents.push(spike2[0]);
        let result = integrate(&contents, AreaOfInterest::All);
        assert_eq!(500.0, result.sum);
        assert_eq!((105.0, 0.0), result.centroid);

        let sumsq = (100.0_f64 * 250.0_f64).powi(2) + (110.0_f64 * 250.0_f64).powi(2);
        let variance = sumsq - (100.0_f64 * 250.0_f64 + 110.0_f64 * 250.0_f64).powi(2);
        let fwhm = (sqrt(variance) / 500.0) * GAMMA;
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
            }, result
        );
    }
}
