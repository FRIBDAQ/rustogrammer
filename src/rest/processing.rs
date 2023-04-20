//!  This module provides the REST interface to the procesing
//!  thread.  The assumption is that he field _processing_ in the
//!  HistogramState object contains a Mutex wrapped
//!  ProcessingApi object, and the analysis thread has already
//!  been started.
//!  
//! Two mount points are provided:
//!  
//!  *  /attach which provides the attach, detach and list methods.
//!  *  /analyze which provides the start, stop and eventchunk
//! methods.

// Imports:

use rocket::serde::{json::Json, Serialize};
use rocket::State;

use crate::processing::*;

//---------------------------------------------------------------
// The /attach mount point:

//--------------------------------------------------------------
// The /analyze mount point.
//
