//!  Provides the version domain.  In our case, we get the
//!  Version from the Cargo manifest.  This is in the form
//!  a.b.c where, in keeping with the versioning system in
//!  NSCLDAQ  and SpecTcl, we treat a as the major version, b as the
//!  minor version and C as the edit level.
//!
//!  We also add the package name to the restult so that
//!  clients can differentiate us from SpecTcl.
//!

use rocket::serde::{json::Json, Serialize};
use std::env;

///  This is the detail returned to the client:
///
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct VersionDetail {
    major: u32,
    minor: u32,
    editlevel: u32,
    program_name: String,
}

/// The full result that's turned into JSON for the client:
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct VersionResponse {
    status: String,
    detail: VersionDetail,
}

/// Returns the package version as a JSON VersionResponse.
/// These are all fished out of environment variables set in the
/// program image by Cargo when it builds the Rustogramer
///
/// ### Environment variables
/// * CARGO_PKG_VERSION_MAJOR -- The major version of the program.
/// * CARGO_PKG_VERSION_MINOR -- The minor version of the program.
/// * CARGO_PKG_VERSION_PATCH -- The edit level of the program.
/// * CARGO_PKG_NAME -- The name of the program.
///
/// ### Returns:
/// *  Json serialized VersionResponse.  The only possible
/// failures are an inability to get and, in the case of version elements,
/// convert the environment variables to integers.  In that case
/// Status wil be _Unable to get the program version_ and the
/// major, minoir, editlevel fields of the detail will be
/// indeterminate values with the program_name defaulting to _Rustogramer_
///
#[get("/")]
pub fn get_version() -> Json<VersionResponse> {
    // initialize for failure:
    let mut result = VersionResponse {
        status: String::from("Unable to get the program version"),
        detail: VersionDetail {
            major: 0,
            minor: 0,
            editlevel: 0,
            program_name: String::from("Rustogramer"),
        },
    };

    let major = env::var("CARGO_PKG_VERSION_MAJOR");
    if let Err(_) = major {
        return Json(result);
    } else {
        if let Ok(m) = major.unwrap().parse::<u32>() {
            result.detail.major = m;
        } else {
            return Json(result);
        }
    }

    let minor = env::var("CARGO_PKG_VERSION_MINOR");
    if let Err(_) = minor {
        return Json(result);
    } else {
        if let Ok(m) = minor.unwrap().parse::<u32>() {
            result.detail.minor = m;
        } else {
            return Json(result);
        }
    }

    let edit = env::var("CARGO_PKG_VERSION_PATCH");
    if let Err(_) = edit {
        return Json(result);
    } else {
        if let Ok(e) = edit.unwrap().parse::<u32>() {
            result.detail.editlevel = e;
        } else {
            return Json(result);
        }
    }

    let name = env::var("CARGO_PKG_NAME");
    if let Ok(n) = name {
        result.detail.program_name = n.clone();
        result.status = String::from("OK");
    }
    Json(result)
}