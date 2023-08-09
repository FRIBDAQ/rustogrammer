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
//! I believe it might be needed for the tree GUI.
//! *   treevariable - See pseudo for the rationale.  Treevariables are
//! supported by the analysis pipeline.

use super::*;
use rocket::serde::{json::Json, Deserialize, Serialize};

//---------------------------------------------------------------
// Pipeline management:

/// Create a pipeline give a name:
#[allow(unused_variables)]
#[get("/create?<name>")]
pub fn pman_create(name: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pipeline management is not implemented",
        "This is not SpecTcl",
    ))
}
/// List pipelines:
#[allow(unused_variables)]
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
#[allow(unused_variables)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct PipelineDescription {
    name: String,
    processors: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ListAllResponse {
    status: String,
    detail: Vec<PipelineDescription>,
}
/// list full pipeline information.

#[allow(unused_variables)]
#[get("/lsall?<pattern>")]
pub fn pman_listall(pattern: OptionalString) -> Json<ListAllResponse> {
    Json(ListAllResponse {
        status: String::from("Pipeline management is not implemented - this is not SpecTcl"),
        detail: vec![],
    })
}
/// List event processors.
#[allow(unused_variables)]
#[get("/lsevp?<pattern>")]
pub fn pman_list_event_processors(pattern: OptionalString) -> Json<StringArrayResponse> {
    Json(StringArrayResponse::new(
        "Pipeline management is not implemented - this is not SpecTcl",
    ))
}
/// Select an event processing pipeline:
#[allow(unused_variables)]
#[get("/use?<name>")]
pub fn pman_choose_pipeline(name: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pipeline management is not implemented",
        "This is not SpecTcl",
    ))
}

/// Add event processor to a pipeline:

#[allow(unused_variables)]
#[get("/add?<pipeline>&<processor>")]
pub fn pman_add_processor(pipeline: String, processor: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pipeline management is not implemented",
        "This is not SpecTcl",
    ))
}
/// Remove an event processor from a pipeline.
#[allow(unused_variables)]
#[get("/rm?<pipeline>&<processor>")]
pub fn pman_rm_processor(pipeline: String, processor: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pipeline management is not implemented",
        "This is not SpecTcl",
    ))
}
/// Clear an event processing pipeline:
#[allow(unused_variables)]
#[get("/clear?<pipeline>")]
pub fn pman_clear(pipeline: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pipeline management is not implemented",
        "This is not SpecTcl",
    ))
}
/// Clone a pipeline:
#[allow(unused_variables)]
#[get("/clone?<source>&<new>")]
pub fn pman_clone(source: String, new: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pipeline management is not implemented",
        "This is not SpecTcl",
    ))
}
//------------------------------------------------------------------
// project:
#[allow(unused_variables)]
#[get("/?<snapshot>&<source>&<newname>&<direction>&<contour>")]
pub fn project(
    snapshot: String,
    source: String,
    newname: String,
    direction: String,
    contour: OptionalString,
) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Projections are not implemented",
        "This is not SpecTcl",
    ))
}
//-----------------------------------------------------------------
// Pseudo parameters.

/// Create a scripted pseudo parameter.
#[allow(unused_variables)]
#[get("/create?<pseudo>&<parameter>&<computation>")]
pub fn pseudo_create(
    pseudo: String,
    parameter: Vec<String>,
    computation: OptionalString,
) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pseudo parameters are not implemented",
        "This is not SpecTcl",
    ))
}
// Description of a pseudo parameter:

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct PseudoDescription {
    name: String,
    parameters: Vec<String>,
    computation: String,
}
// Response to /pseudo/list:

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct PseudoListResponse {
    status: String,
    detail: Vec<PseudoDescription>,
}
/// List pseudos.
#[allow(unused_variables)]
#[get("/list?<pattern>")]
pub fn pseudo_list(pattern: OptionalString) -> Json<PseudoListResponse> {
    Json(PseudoListResponse {
        status: String::from("Psuedo parameters are not implemented - this is not SpecTcl"),
        detail: vec![],
    })
}
/// Delete pseudos
#[allow(unused_variables)]
#[get("/delete?<name>")]
pub fn pseudo_delete(name: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Pseudo parameters are not implemented",
        "This is not SpecTcl",
    ))
}
//-----------------------------------------------------------
// Root tree:

/// Create a root output tree.
#[allow(unused_variables)]
#[get("/create?<tree>&<parameter>&<gate>")]
pub fn roottree_create(
    tree: String,
    parameter: Vec<String>,
    gate: OptionalString,
) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Root Tree output is not supported",
        "This is not SpecTcl",
    ))
}

/// Delete a root output tree.

#[get("/delete?<tree>")]
#[allow(unused_variables)]
pub fn roottree_delete(tree: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Root Tree output is not supported",
        "This is not SpecTcl",
    ))
}

// Description of a root tree:

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RootTreeDescription {
    tree: String,
    params: Vec<String>,
    gate: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RootTreeListResponse {
    status: String,
    detail: Vec<RootTreeDescription>,
}

/// List the root trees:
#[allow(unused_variables)]
#[get("/list?<pattern>")]
pub fn roottree_list(pattern: OptionalString) -> Json<RootTreeListResponse> {
    Json(RootTreeListResponse {
        status: String::from("Root tree output is not implemented - this is not SpecTcl"),
        detail: vec![],
    })
}
//----------------------------------------------------------------
// Script.
#[allow(unused_variables)]
#[get("/?<command>")]
pub fn script_execute(command: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Script execution is not supported",
        "This is not SpecTcl",
    ))
}
//---------------------------------------------------------------------
// tree variables.

// What we get per tree variable in a listing:

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct TreeVariable {
    name: String,
    value: f64,
    units: String,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct TreeVariableListResponse {
    status: String,
    detail: Vec<TreeVariable>,
}
/// List tree variables.
#[get("/list")]
pub fn treevariable_list() -> Json<TreeVariableListResponse> {
    Json(TreeVariableListResponse {
        status: String::from("Tree variables are not implemented.  This is not SpecTcl"),
        detail: vec![],
    })
}

/// Set a new value for a tree variabls
#[allow(unused_variables)]
#[get("/set?<name>&<value>&<units>")]
pub fn treevariable_set(name: String, value: f64, units: OptionalString) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Tree variables are not implemented",
        "This is not SpecTcl",
    ))
}
/// Get changed flag.
#[allow(unused_variables)]
#[get("/check?<name>")]
pub fn treevariable_check(name: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Tree variables are not implemented",
        "This is not SpecTcl",
    ))
}
/// Set changed flag
#[allow(unused_variables)]
#[get("/setchanged?<name>")]
pub fn treevariable_set_changed(name: String) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Tree variables are not implemented",
        "This is not SpecTcl",
    ))
}
/// Fire changed traces:
#[allow(unused_variables)]
#[get("/firetraces?<pattern>")]
pub fn treevariable_fire_traces(pattern: OptionalString) -> Json<GenericResponse> {
    Json(GenericResponse::err(
        "Tree variables are not implemented",
        "This is not SpecTcl",
    ))
}

#[cfg(test)]
mod pipeline_tests {
    use super::*;
    use crate::messaging;
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;

    fn setup() -> Rocket<Build> {
        rest_common::setup().mount(
            "/",
            routes![
                pman_create,
                pman_list,
                pman_current,
                pman_listall,
                pman_list_event_processors,
                pman_choose_pipeline,
                pman_add_processor,
                pman_rm_processor,
                pman_clear,
                pman_clone,
            ],
        )
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
    ) {
        rest_common::get_state(r)
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        rest_common::teardown(c, p, b);
    }
    #[test]
    fn create_1() {
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket test client");
        let req = client.get("/create?name=junk");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Pipeline management is not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn list_1() {
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket test client");
        let req = client.get("/list?pattern=junk");
        let reply = req
            .dispatch()
            .into_json::<StringArrayResponse>()
            .expect("Parsing JSON");

        assert_eq!(
            "Pipeline managment is not implemented - this is not SpecTcl",
            reply.status
        );
        assert_eq!(0, reply.detail.len());

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn current_1() {
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket test client");
        let req = client.get("/current");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Pipeline management is not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn listall_1() {
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket test client");
        let req = client.get("/lsall?pattern=*");
        let reply = req
            .dispatch()
            .into_json::<ListAllResponse>()
            .expect("Parsing JSON");

        assert_eq!(
            "Pipeline management is not implemented - this is not SpecTcl",
            reply.status
        );
        assert_eq!(0, reply.detail.len());

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn lsevp_1() {
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket test client");
        let req = client.get("/lsevp?pattern=*");
        let reply = req
            .dispatch()
            .into_json::<StringArrayResponse>()
            .expect("Parsing JSON");

        assert_eq!(
            "Pipeline management is not implemented - this is not SpecTcl",
            reply.status
        );
        assert_eq!(0, reply.detail.len());

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn use_1() {
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket test client");
        let req = client.get("/use?name=junk");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Pipeline management is not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn add_1() {
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket test client");
        let req = client.get("/add?pipeline=pipe&processor=proc");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Pipeline management is not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn rm_1() {
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket test client");
        let req = client.get("/rm?pipeline=pipe&processor=proc");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Pipeline management is not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn clear_1() {
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket test client");
        let req = client.get("/clear?pipeline=pipe");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Pipeline management is not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn clone_1() {
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket test client");
        let req = client.get("/clone?source=src&new=new");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Pipeline management is not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bind_api);
    }
}
#[cfg(test)]
mod project_tests {
    use super::*;
    use crate::messaging;
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;
    
    fn setup() -> Rocket<Build> {
        rest_common::setup().mount("/", routes![project])
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
    ) {
        rest_common::get_state(r)
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        rest_common::teardown(c, p, b);
    }
    #[test]
    fn project_1() {
        let rocket = setup();
        let (chan, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/?snapshot=yes&source=dummy&newname=newspec&direction=x");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Projections are not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bapi)
    }
}
#[cfg(test)]
mod pseudo_test {
    use super::*;
    use crate::messaging;
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;
    
    fn setup() -> Rocket<Build> {
        rest_common::setup().mount("/", routes![pseudo_create, pseudo_list, pseudo_delete])
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
    ) {
        rest_common::get_state(r)
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        rest_common::teardown(c, p, b);
    }
    #[test]
    fn create_1() {
        let rocket = setup();
        let (chan, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("creating client");
        let req = client.get("/create?pseudo=p&parameter=a&parameter=b&computation={$a+$b}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Pseudo parameters are not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bapi);
    }
    #[test]
    fn list_1() {
        let rocket = setup();
        let (chan, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("creating client");
        let req = client.get("/list?pattern=*");
        let reply = req
            .dispatch()
            .into_json::<PseudoListResponse>()
            .expect("Parsing JSON");

        assert_eq!(
            "Psuedo parameters are not implemented - this is not SpecTcl",
            reply.status
        );
        assert_eq!(0, reply.detail.len());

        teardown(chan, &papi, &bapi);
    }
    #[test]
    fn delete_1() {
        let rocket = setup();
        let (chan, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("creating client");
        let req = client.get("/delete?name=dummy");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Pseudo parameters are not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bapi);
    }
}
#[cfg(test)]
mod roottree_tests {
    use super::*;
    use crate::messaging;
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;
    
    fn setup() -> Rocket<Build> {
        rest_common::setup().mount(
            "/",
            routes![roottree_create, roottree_delete, roottree_list],
        )
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
    ) {
        rest_common::get_state(r)
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        rest_common::teardown(c, p, b);
    }
    #[test]
    fn create_1() {
        let rocket = setup();
        let (chan, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket client");
        let req = client.get("/create?tree=treename&parameter=p1&parmeter=p2");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Root Tree output is not supported", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bapi);
    }
    #[test]
    fn delete_1() {
        let rocket = setup();
        let (chan, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket client");
        let req = client.get("/delete?tree=treename");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Root Tree output is not supported", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bapi);
    }
    #[test]
    fn list_1() {
        let rocket = setup();
        let (chan, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making rocket client");
        let req = client.get("/list?pattern=*");
        let reply = req
            .dispatch()
            .into_json::<RootTreeListResponse>()
            .expect("Parsing JSON");

        assert_eq!(
            "Root tree output is not implemented - this is not SpecTcl",
            reply.status
        );
        assert_eq!(0, reply.detail.len());

        teardown(chan, &papi, &bapi);
    }
}
#[cfg(test)]
mod script_tests {
    use super::*;
    use crate::messaging;
    use crate::test::rest_common;
    
    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    fn setup() -> Rocket<Build> {
        rest_common::setup().mount("/", routes![script_execute])
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
    ) {
        rest_common::get_state(r)
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        rest_common::teardown(c, p, b);
    }
    #[test]
    fn execute_1() {
        let rocket = setup();
        let (chan, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making rocket client");
        let req = client.get("/?command=something");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Script execution is not supported", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bapi);
    }
}
#[cfg(test)]
mod treevar_tests {
    use super::*;
    use crate::messaging;
    use crate::test::rest_common;
    
    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    fn setup() -> Rocket<Build> {
        rest_common::setup().mount(
            "/",
            routes![
                treevariable_list,
                treevariable_set,
                treevariable_check,
                treevariable_set_changed,
                treevariable_fire_traces
            ],
        )
    }
    fn getstate(
        r: &Rocket<Build>,
    ) -> (
        mpsc::Sender<messaging::Request>,
        processing::ProcessingApi,
        binder::BindingApi,
    ) {
        rest_common::get_state(r)
    }
    fn teardown(
        c: mpsc::Sender<messaging::Request>,
        p: &processing::ProcessingApi,
        b: &binder::BindingApi,
    ) {
        rest_common::teardown(c, p, b);
    }
    #[test]
    fn list_1() {
        let rocket = setup();
        let (chan, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/list");
        let reply = req
            .dispatch()
            .into_json::<TreeVariableListResponse>()
            .expect("Parsing JSON");

        assert_eq!(
            "Tree variables are not implemented.  This is not SpecTcl",
            reply.status
        );
        assert_eq!(0, reply.detail.len());

        teardown(chan, &papi, &bapi);
    }
    #[test]
    fn set_1() {
        let rocket = setup();
        let (chan, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/set?name=dummy&value=2.54&units=cm%2Fin");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Tree variables are not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bapi);
    }
    #[test]
    fn check_1() {
        let rocket = setup();
        let (chan, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/check?name=dummy");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Tree variables are not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bapi);
    }
    #[test]
    fn setchanged_1() {
        let rocket = setup();
        let (chan, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/setchanged?name=dummy");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Tree variables are not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bapi);
    }
    #[test]
    fn firetraces_1() {
        let rocket = setup();
        let (chan, papi, bapi) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/firetraces?pattern=*");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Tree variables are not implemented", reply.status);
        assert_eq!("This is not SpecTcl", reply.detail);

        teardown(chan, &papi, &bapi);
    }
}
