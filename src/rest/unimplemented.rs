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

//---------------------------------------------------------------
// Pipeline management:

/// Create a pipeline give a name:
#[get("/create?<name>")]
pub fn pman_create(name: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pipline management is not implemented",
        "This is not SpecTcl",
    ))
}
/// List pipelines:
#[get("/list?<pattern>")]
pub fn pman_list(pattern: OptionalString) -> Json<StringArrayResponse> {
    Json(StringArrayResponse::new(
        "Pipeline managment is not implemented - this is not SpecTcl",
    ))
}
/// Name of current pipeline:
#[get("/current")]
pub fn pman_current() -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pipeline management is not implemented",
        "This is not SpecTcl",
    ))
}
// listall
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct PipelineDescription {
    name: String,
    processors: Vec<String>,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ListAllResponse {
    status: String,
    detail: Vec<PipelineDescription>,
}
/// list full pipeline information.

#[get("/lsall?<pattern>")]
pub fn pman_listall(pattern: OptionalString) -> Json<ListAllResponse> {
    Json(ListAllResponse {
        status: String::from("Pipeline management is not implemented - this is not SpecTcl"),
        detail: vec![],
    })
}
/// List event processors.
#[get("/lsevp?<pattern>")]
pub fn pman_list_event_processors(pattern: OptionalString) -> Json<StringArrayResponse> {
    Json(StringArrayResponse::new(
        "Pipeline management is not implemented - this is not SpecTcl",
    ))
}
/// Select an event processing pipeline:
#[get("/use?<name>")]
pub fn pman_choose_pipeline(name: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pipeline management is not implemented",
        "This is not SpecTcl",
    ))
}

/// Add event processor to a pipeline:

#[get("/add?<pipeline>&<processor>")]
pub fn pman_add_processor(pipeline: String, processor: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pipeline management is not implemented",
        "This is not SpecTcl",
    ))
}
/// Remove an event processor from a pipeline.
#[get("/rm?<pipeline>&<processor>")]
pub fn pman_rm_processor(pipeline: String, processor: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pipeline management is not implemented",
        "This is not SpecTcl",
    ))
}
/// Clear an event processing pipeline:

#[get("/clear?<pipeline>")]
pub fn pman_clear(pipeline: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pipeline management is not implemented",
        "This is not SpecTcl",
    ))
}
/// Clone a pipeline:
#[get("/clone?<source>&<new>")]
pub fn pman_clone(source: String, new: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pipeline management is not implemented",
        "This is not SpecTcl",
    ))
}
