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

use super::*;
use crate::messaging::{spectrum_messages, condition_messages}; // Need for reconstiting contours.
use crate::conditions::twod;

///  This is the payload of a sum.  It's the same for 1-d and 2d integrations:
struct SumElement {
    contents: f64,    // channel contents
    wsum: (f64, f64), // cahnnel contents weighted by x/y positions.
}

pub enum AreaOfInterest {
    Oned {                    // OneD slice of interest
        low: f64,
        high: f64
    },
    Twod (twod::Contour),    // 2d contour of interest.
    All                      // No limits.
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
        spectrum_messages::ChannelType::Underflow | spectrum_messages::ChannelType::Overflow => SumElement {
            contents: 0.0,
            wsum: (0.0, 0.0)
        },
        spectrum_messages::ChannelType::Bin => {
            if match aoi {
                AreaOfInterest::All => true,
                AreaOfInterest::Oned {low, high} => (chan.x >= *low) && (chan.y <= *high),
                AreaOfInterest::Twod (c) => c.inside(chan.x, chan.y)
            } { 
                SumElement {
                    contents: chan.value,
                    wsum: (chan.value*chan.x, chan.value*chan.y)
                }
            } else {
                SumElement {
                    contents: 0.0,
                    wsum: (0.0, 0.0)
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
/// ### Notes:
///   *  The caller can limit the data returned so that fewer channels are examined.
///   *  Only zero or none of limits
pub fn integrate () {}