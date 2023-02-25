//!  PGamma Spectra are useful  in particle gamma
//!  coincidence experiments where both the gamma and
//!  massive particle detectors are arrays.  
//!  The spectra are defined on two indpendent arrays of
//!  parameters, one for the X and one for the Y axis.
//!  When an event can increment the spectrum, all pairs of X/Y
//!  parameters generate increments.  For example:
//!  consider a fully populated event and a Pgamma
//!   histogram with parameters 1,3 on the x axis and 5,7,8 on the y axis, the following
//!   parameter pairs will be used to increment the spectrum:
//!   (1,5), (1,7), (1,8), (3,5), (3,7), (3,8).
//!
//!  As with any spectrum a condition can be applied to gate the increment
//!  of the spectrum.  That is the condition, applied as the gate must
//!  be true for the event to be eligible to increment the spectrum.
//!
//!  Default axis specification are derived indpendently from the
//!  default axis specification fo the X and Y parameter sets.
//!  The algorithm to choose from among the specification is the same
//!  as for all : minimum *_low, maximum of *_high and *_bins.
//!
use super::*;
use ndhistogram::value::Sum;

// This struct defines a parameter for the spectrum:

struct SpectrumParameter {
    name:  String,
    id  :  u32
}

///
/// PGamma is the struct that represents the Particle Gamma Spectrum.
/// In addition to the name and histogram it encapsulates  an array
/// of X and an independent array of Y parameters stored as
/// SpectrumParameter objects:
///
pub struct PGamma {
    applied_gate : SpectrumGate,
    name : String,
    histogram : Hist2D<axis::Uniform, axis::Uniform, Sum>,

    x_params : Vec<SpectrumParameter>,
    y_params : Vec<SpectrumParameter>
}
