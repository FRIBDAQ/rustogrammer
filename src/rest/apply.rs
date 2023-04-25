//!  This module implements the spectcl/apply domain of the RST server.
//!  we support two requests:
//!
//! *  apply (/spectcl/apply/apply) which applies a gate
//! to a spectrum so that the spectrum can only be incremented for
//! events which make that gate true.
//! *  list - lists spectra with the gates that are applied to them
//! Note that this request accepts a pattern to filter the set
//! of gates that are listed.
//!

use crate::messaging::spectrum_messages;
use rocket::serde::{json::Json, Serialize};
use rocket::State;
