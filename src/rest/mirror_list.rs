//!  Provides the /spectcl/mirror method which, in turn provides a
//! list of all of the mirrors that have been created.
//! This is used by the mirror client API to avoid multiple instances of mirrors
//! in the same host for a single Rustogramer.

use crate::sharedmem::mirror;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;

// Description of a mirror client:
#[derive(Serialize, Deserialize, Clone)]
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
pub fn mirror_list(state: &State<mirror::SharedMirrorDirectory>) -> Json<MirrorResponse> {
    let mut result = MirrorResponse {
        status: String::from("OK"),
        detail: Vec::new(),
    };
    for entry in state.inner().lock().unwrap().iter() {
        result.detail.push(MirrorInfo {
            host: entry.host(),
            memory: entry.key(),
        });
    }
    Json(result)
}

#[cfg(test)]
mod mirror_list_tests {
    // Note that we can test without actually setting up the
    // whole infrastructure...we can make test data directly into the
    // mirror directory we pass into the test server.
    //
    use super::*;
    use crate::sharedmem::mirror;
    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::{Arc, Mutex};

    fn setup() -> Rocket<Build> {
        let state: mirror::SharedMirrorDirectory = Arc::new(Mutex::new(mirror::Directory::new()));

        rocket::build()
            .manage(state)
            .mount("/", routes![mirror_list])
    }
    fn get_directory(r: &Rocket<Build>) -> mirror::SharedMirrorDirectory {
        r.state::<mirror::SharedMirrorDirectory>()
            .expect("Valid state")
            .clone()
    }

    #[test]
    fn list_1() {
        // Nothing in the directory:

        let rocket = setup();

        let client = Client::untracked(rocket).expect("Making server");
        let req = client.get("/");
        let reply = req
            .dispatch()
            .into_json::<MirrorResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(0, reply.detail.len());
    }
    #[test]
    fn list_2() {
        // Put a single item in the directory:

        let rocket = setup();
        let dir = get_directory(&rocket);
        dir.lock().unwrap().add("some-host", "some_key").expect("Adding item");

        let client = Client::untracked(rocket).expect("Making server");
        let req = client.get("/");
        let reply = req
            .dispatch()
            .into_json::<MirrorResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(1, reply.detail.len());
        assert_eq!("some-host", reply.detail[0].host);
        assert_eq!("some_key", reply.detail[0].memory);
    }
    #[test]
    fn list_3() {
        //  Put in a few entries. they all should be listed:

        let rocket = setup();
        let dir = get_directory(&rocket);

        let hosts = vec!["host1", "host2", "host3"]; // alpha oredered hosts.
        let mems = vec!["memory1", "memory2", "memory3"];
        assert_eq!(hosts.len(), mems.len()); // defensive
        for (i, h) in hosts.iter().enumerate() {
            dir.lock().unwrap().add(h, mems[i]).expect("adding item");
        }

        let client = Client::untracked(rocket).expect("Making server");
        let req = client.get("/");
        let reply = req
            .dispatch()
            .into_json::<MirrorResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(hosts.len(), reply.detail.len());

        // The orederis not defined so we'll sort by host and see how that looks:

        let mut items = reply.detail.clone();
        items.sort_by(|a, b| a.host.cmp(&b.host));

        for (i, _) in items.iter().enumerate() {
            assert_eq!(hosts[i], items[i].host, "Failed on item: {}", i);
            assert_eq!(mems[i], items[i].memory, "Failed on item; {}", i);
        }
    }
}
