//! Because of the very nature of Rustogramer there are
//! REST interfaces that cannot be implemented.  
//! These are collected into this file.
//!  
//!  They include, in no particular order:
//!
//! *   Mirroring of the shared memory - that is actually
//! Scheduled for version 0.2
//! *   pipeline management - There is no analysis pipeline in Rustogramer,
//! the analysis pipeline as concieved of for SpecTcl is external and
//! provide Rustogramer with pre-decoded data.
//! *   projection - I'm not that sure about when/how to do that. Arguably,
//! This is something that could be done in a displayer.   It could also
//! Be argued that a projection is really just a 1-d spectrum the user could
//! define which, may or may not be gated on another spectrum.  After all,
//! that's how they are implemented in SpecTcl.
//! *   psuedo - Any computed parameters are done in the external analysis
//! pipeline and, therefore are not supported in Rustogramer.  Note
//! that the psuedo feature of SpecTcl itself is seldom used, more normally
//! people add this to compiled code.
//! *   roottree - Root tree creation is something that should be done
//! by other parts of the analysis pipeline.   Not Rustogramer.
//! *   script - There is on command language to script.
//! *   trace  - This is scheduled for implementation, if needed, in version 0.2
//! I believe it might be needed for the tree GUI.
//! *   treevariable - See pseudo for the rationale.  Treevariables are
//! supported by the analysis pipeline.

use super::*;
use rocket::serde::{json::Json, Serialize};

//------------------------------------------------------------
// Mirroring

// Description of a mirror client:
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct MirrorInfo {
    host: String,
    memory: String,
}
#[derive(Serialize)]
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
