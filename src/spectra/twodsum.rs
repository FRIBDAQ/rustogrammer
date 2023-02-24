//!  A two d sum spectrum is exactly that. A spectrum that would
//!  be the sum of several two d spectra with the same gate applied.
//!
//!  The spectrum is defined over an arbitrary set of x/y parameter
//!  pairs.  If the applied gate is satisfied, the spectrum is incremented
//!  for each of those pairs which have a value in the event.
//!
//!  Suppose, for example, the spectrum is defined on the following (x,y)
//!  pairs: (1,2), (3,4), (5,6).  And an event has contents:
//!   1=100, 2=100, 3=500, 5=600, 6=700.  The channels for:
//!  (100,200), and (600,700) will be incremented (4 is not present
//!  so no increment for the pair (3,4) will be done).
//!
//!  As with all spectra a gate can be applied to the spectrum.
//!  If one is, increments only occur if the evaluation of that
//!  gate returns true for the event.
//!
use super::*;
use ndhistogram::value::Sum;
