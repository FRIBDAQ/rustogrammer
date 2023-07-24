//!  Provides the /spectcl/mirror method which, in turn provides a
//! list of all of the mirrors that have been created.
//! This is used by the mirror client API to avoid multiple instances of mirrors
//! in the same host for a single Rustogramer.

use super::*;
use crate::sharedmem::mirror;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;

// Description of a mirror client:
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct MirrorInfo {
    host: String,
    memory: String,
}
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct MirrorResponse {
    status: String,
    detail: Vec<MirrorInfo>,
}

#[get("/")]
pub fn mirror_list() -> Json<MirrorResponse> {
    Json(MirrorResponse {
        status: String::from("Mirroring is not implemented in Rustogramer"),
        detail: vec![],
    })
}
