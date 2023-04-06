//! This module implements Rocket handlers for
//! the the /spectcl/gate domain of URLS.
//! The name follows the SpecTcl name for conditions while
//! Rustogrammer knows that there are conditions which are
//! just objects that can be evaluated, as needed for each event
//! which return a true or false value.  
//!
//! A condition can the gate (verb) a spectrum to determine which
//! events are allowed to increment it.
//!
//! A nasty concern is that the condition type names supported
//! by Rustogramer have more useful names like True, And, Cut
//! where those in SpecTcl (and therefore the type-names expected
//! by REST clients) have simpler names like T, F, s, * (slice).
//! it is therefore necessary to map from Rustogramer
//! Gate types to SpecTcl gate types in this domain of URLs.

use rocket::serde::{json::Json, Serialize};
use rocket::State;

use super::*;

use crate::messaging::condition_messages::ConditionMessageClient;
use crate::messaging::parameter_messages::ParameterMessageClient;
