//!  Handlers for the spectcl/spectrum URLs.
//!  These manipulate spectra.  A brief list of the
//!  URI's that are supported:
//!
//! *  /spectcl/spectrum/list - list spectra and their properties.
//! *  /spectcl/spectrum/delete - Deltee a spectrum.
//! *  /spectcl/spectrum/create - create a new spectrum.
//! *  /spectcl/spectrum/contents - Get the contents of a spectrum.
//! *  /spectcl/sspectrum/clear - clear
use rocket::serde::{json::Json, Serialize};
use rocket::State;

use super::*;

use crate::messaging::spectrum_messages::{
    SpectrumMessageClient, SpectrumProperties, SpectrumServerContentsResult,
    SpectrumServerEmptyResult, SpectrumServerListingResult,
};
// as with gates we need to map from Rustogramer spectrum
// types to SpecTcl spectrum types.

fn rg_sptype_to_spectcl(rg_type: &str) -> String {
    match rg_type {
        "1D" => String::from("1"),
        "Multi1d" => String::from("g1"),
        "Multi2d" => String::from("g2"),
        "PGamma" => String::from("gd"),
        "Summary" => String::from("s"),
        "2D" => String::from("2"),
        "2DSum" => String::from("m2"),
        _ => String::from("-unsupported-"),
    }
}
//------------------------------------------------------------
// Stuff we need to list spectra and their properties.

// structures that define the JSON we'll return:

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Axis {
    low: f64,
    high: f64,
    bins: u32,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SpectrumDescription {
    name: String,
    #[serde(rename = "type")]
    spectrum_type: String,
    parameters: Vec<String>,
    axes: Vec<Axis>,
    chantype: String,
    gate: Option<String>,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ListResponse {
    status: String,
    detail: Vec<SpectrumDescription>,
}

// Convert the listing from the message client to a vector
// of spectrum descriptions:

fn list_to_detail(l: Vec<SpectrumProperties>) -> Vec<SpectrumDescription> {
    let mut result = Vec::<SpectrumDescription>::new();
    for mut d in l {
        let mut def = SpectrumDescription {
            name: d.name,
            spectrum_type: rg_sptype_to_spectcl(&d.type_name),
            parameters: d.xparams,
            axes: Vec::<Axis>::new(),
            chantype: String::from("f64"),
            gate: d.gate,
        };
        def.parameters.append(&mut d.yparams);
        if let Some(x) = d.xaxis {
            def.axes.push(Axis {
                low: x.low,
                high: x.high,
                bins: x.bins - 2, // Omit over/underflow
            });
        }
        if let Some(y) = d.yaxis {
            def.axes.push(Axis {
                low: y.low,
                high: y.high,
                bins: y.bins - 2, // Omit over/underflow.
            });
        }

        result.push(def);
    }
    result
}
///
/// List the spectra.  The only query parameter is _filter_ which is an
/// optional parameter that, if provided is a glob pattern that
/// must match a spectrum name for it to be included in the
/// set of listed spectra.  The default value for _filter_ is "*" which
/// matches all names.
///
/// The reply consists of _status_ which, on success is _OK_ and
/// on failure is an error message string.
///
/// On failure the _detail_ field of the resonse is an empty array.
/// On success, _detail_ will be an array that describes all of the
/// spectra that match _filter_ (so this may still be empty).  Each
/// element is a JSON struct that contains:
///
/// *   name -- The name of the matching spectrum.
/// *   type -- the SpecTcl type of the matching spectrum.
//  *   parameters -- an array of paramter names.  For 2-d spectra,
/// the first parameter is the x parameter, the second, the y.
/// note that this can be ambiguous for gd and m2 which have multiple
/// x and y parameters.
/// *   axes -- an array of at least one axis definition.  Each element
/// of the array is an object with the fields:
///     - low  - low limit of the axis.
///     - high - high limit of the axis.
///     - bins - the number of bins between [low, high)
/// *   chantype -- the data type of each channel in the spectrum.
/// in rustogramer this is hardcoded to _f64_
/// *    gate if not _null_ thisi s the name of the conditions that
/// is applied as a gate to the spectrum.
///
/// Note:  SpecTcl and Rustogrammer don't support knowing
/// which parameters are X paramters for PGamma spectra where
/// there can be a different number of x, y parameters
/// for 2dsum spectra, the first half are the X parameters, the
/// second half the y parameters.
///
/// Future enhancement:
#[get("/list?<filter>")]
pub fn list_spectrum(filter: OptionalString, state: &State<HistogramState>) -> Json<ListResponse> {
    let pattern = if let Some(p) = filter {
        p
    } else {
        String::from("*")
    };

    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);

    let response = match api.list_spectra(&pattern) {
        Ok(l) => ListResponse {
            status: String::from("OK"),
            detail: list_to_detail(l),
        },
        Err(s) => ListResponse {
            status: format!("Failed to list spectra: {}", s),
            detail: Vec::<SpectrumDescription>::new(),
        },
    };

    Json(response)
}
//----------------------------------------------------------------
// What's needed to delete a spectrum:

///
/// Handle the delete request.  The only query parameter is _name_
/// the name of the spectrum to delete.  The response on success
/// has a status of *OK* and empty detail.   On failure, the
/// status will be a top level error message like
/// _Failed to delete spectrum xxx_ and the detail will contain a
/// more specific message describing why the delete failed e.g.
/// _Spectrum does not exist_
///
#[get("/delete?<name>")]
pub fn delete_spectrum(name: String, state: &State<HistogramState>) -> Json<GenericResponse> {
    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);

    let response = match api.delete_spectrum(&name) {
        Ok(()) => GenericResponse {
            status: String::from("OK"),
            detail: String::new(),
        },
        Err(msg) => GenericResponse {
            status: format!("Failed to delete {}", name),
            detail: msg,
        },
    };
    Json(response)
}
//-------------------------------------------------------------------
// What's needed to create a spectrum.

// Tcl list unpacking:
// We're pretty stupid about how this is done.
// We only really support two types of lists:
// - A list with no nested elements.
// - A list with only two elements each a sublist.
//  (for PGamma and 2DSum).
//

fn parse_simple_list(list: &str) -> Result<Vec<String>, String> {
    let list = String::from(list);

    // Simple strings must not have {} embedded:

    if list.contains("{") || list.contains("}") {
        Err(format!("'{}' is not a simple list", list))
    } else {
        let v: Vec<&str> = list.split(' ').collect();
        let mut result = Vec::<String>::new();
        for s in v {
            result.push(String::from(s));
        }
        Ok(result)
    }
}
// Parse a two element sublist each element is a simple list
//

fn parse_two_element_list(list: &str) -> Result<(Vec<String>, Vec<String>), String> {
    let list = String::from(list);

    // Find and parse the first sublist:

    let first_open = list.find('{');
    if first_open.is_none() {
        return Err(format!(
            "'{}' is not a properly formatted 2 element list",
            list
        ));
    }
    let first_open = first_open.unwrap();

    let first_close = list.find('}');
    if first_close.is_none() {
        return Err(format!(
            "'{}' first substring is not properly terminated",
            list
        ));
    }
    let first_close = first_close.unwrap();

    let first_element = parse_simple_list(&list[first_open + 1..first_close]);
    if let Err(msg) = first_element {
        return Err(format!("Parse of first element failed: {}", msg));
    }
    let first_element = first_element.unwrap();

    // Now with the second element:

    let remainder = list.split_at(first_close + 1).1;
    let second_open = remainder.find('{');
    if second_open.is_none() {
        return Err(format!("'{}' cound not find opening of second list", list));
    }
    let second_close = remainder.find('}'); // Seach for the last }
    if second_close.is_none() {
        return Err(format!("'{}' could not find closing of second list", list));
    }
    let second_close = second_close.unwrap();
    let second_open = second_open.unwrap();
    let last_close = remainder.rfind('}').unwrap();
    if second_close != last_close {
        return Err(String::from(
            "The closing } of the second list is not the last }",
        ));
    }
    if second_close < second_open {
        return Err(String::from("Found second close before the second open!!"));
    }
    let second_element = parse_simple_list(&remainder[second_open + 1..second_close]);
    if let Err(msg) = second_element {
        return Err(format!("Parse of second element failed : {}", msg));
    }

    Ok((first_element, second_element.unwrap()))
}
// process a broken down axis def:

fn parse_single_axis_def(axes: &Vec<String>) -> Result<(f64, f64, u32), String> {
    if axes.len() != 3 {
        return Err(String::from("Must have 3 elements"));
    };

    let low = axes[0].parse::<f64>();
    let high = axes[1].parse::<f64>();
    let bins = axes[2].parse::<u32>();

    if low.is_err() || high.is_err() || bins.is_err() {
        return Err(format!(
            "Invalid values  in axis list of {} {} {}",
            axes[0], axes[1], axes[2]
        ));
    }
    let low = low.unwrap();
    let high = high.unwrap();
    let bins = bins.unwrap();

    Ok((low, high, bins))
}
// Process an axis definition.

fn parse_axis_def(axes: &str) -> Result<(f64, f64, u32), String> {
    let parsed_axes = parse_simple_list(axes);
    if parsed_axes.is_err() {
        return Err(parsed_axes.unwrap_err());
    }
    let axes = parsed_axes.unwrap();
    let axis_tuple = parse_single_axis_def(&axes);
    if let Err(s) = axis_tuple {
        return Err(s);
    }
    let axis = axis_tuple.unwrap();
    let low = axis.0;
    let high = axis.1;
    let bins = axis.2;

    Ok((low, high, bins))
}

// Make a 1-d spectrum:
// parameters must be a single parameter name.
// axes must be a single axis specification in the form low high bins
//
fn make_1d(
    name: &str,
    parameters: &str,
    axes: &str,
    state: &State<HistogramState>,
) -> Json<GenericResponse> {
    let parsed_params = parse_simple_list(parameters);
    if parsed_params.is_err() {
        return Json(GenericResponse {
            status: String::from("Error parsing 1d spectrum parameter"),
            detail: parsed_params.unwrap_err(),
        });
    }
    let params = parsed_params.unwrap();
    if params.len() != 1 {
        return Json(GenericResponse {
            status: String::from("Eror processing 1d spectrum parameters"),
            detail: String::from("Only allowed one parameter"),
        });
    }
    let parameter = params[0].clone();
    // Axis parsed as a simple list must be a 3 element list:

    let parsed_axes = parse_axis_def(axes);
    if parsed_axes.is_err() {
        return Json(GenericResponse {
            status: String::from("Invalid axis specification"),
            detail: parsed_axes.unwrap_err(),
        });
    }
    let (low, high, bins) = parsed_axes.unwrap();
    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);

    let r = if let Err(s) = api.create_spectrum_1d(name, &parameter, low, high, bins) {
        GenericResponse {
            status: String::from("Failed to create 1d spectrum"),
            detail: s,
        }
    } else {
        GenericResponse {
            status: String::from("OK"),
            detail: String::from(""),
        }
    };
    Json(r)
}
// Make a 2d spectrum
fn make_2d(
    name: &str,
    parameters: &str,
    axes: &str,
    state: &State<HistogramState>,
) -> Json<GenericResponse> {
    // need exactly two parameters:

    let parsed_params = parse_simple_list(parameters);
    if parsed_params.is_err() {
        return Json(GenericResponse {
            status: String::from("Failed to parse 2d parameter list"),
            detail: parsed_params.unwrap_err(),
        });
    }
    let params = parsed_params.unwrap();
    if params.len() != 2 {
        return Json(GenericResponse {
            status: String::from("Failed to process parameter list"),
            detail: String::from("There must be exactly two parameters for a 2d spectrum"),
        });
    }
    let xp = params[0].clone();
    let yp = params[1].clone();

    let axis_list = parse_two_element_list(axes);
    if axis_list.is_err() {
        return Json(GenericResponse {
            status: String::from("Failed to break apart axis list"),
            detail: axis_list.unwrap_err(),
        });
    }
    let (xaxis_def, yaxis_def) = axis_list.unwrap();

    let xaxis = parse_single_axis_def(&xaxis_def);
    if xaxis.is_err() {
        return Json(GenericResponse {
            status: String::from("Failed to parse x axis definition"),
            detail: xaxis.unwrap_err(),
        });
    }
    let (xlow, xhigh, xbins) = xaxis.unwrap();

    let yaxis = parse_single_axis_def(&yaxis_def);
    if yaxis.is_err() {
        return Json(GenericResponse {
            status: String::from("Failed to parse y axis definition"),
            detail: yaxis.unwrap_err(),
        });
    }
    let (ylow, yhigh, ybins) = yaxis.unwrap();

    // Now we can try to make the spectrum:

    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);
    let result = if let Err(s) =
        api.create_spectrum_2d(name, &xp, &yp, xlow, xhigh, xbins, ylow, yhigh, ybins)
    {
        GenericResponse {
            status: String::from("Failed to create 2d spectrum"),
            detail: s,
        }
    } else {
        GenericResponse {
            status: String::from("OK"),
            detail: String::from(""),
        }
    };

    Json(result)
}
// make a gamma 1 spectrum ( multi1d)

fn make_gamma1(
    name: &str,
    parameters: &str,
    axes: &str,
    state: &State<HistogramState>,
) -> Json<GenericResponse> {
    let parameters = parse_simple_list(parameters);
    if parameters.is_err() {
        return Json(GenericResponse {
            status: String::from("Could not parse parameter list"),
            detail: parameters.unwrap_err(),
        });
    }
    let parameters = parameters.unwrap();

    let axis = parse_axis_def(axes);
    if axis.is_err() {
        return Json(GenericResponse {
            status: String::from("Failed to process axis definition"),
            detail: axis.unwrap_err(),
        });
    }
    let (low, high, bins) = axis.unwrap();

    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);
    let response = if let Err(s) = api.create_spectrum_multi1d(name, &parameters, low, high, bins) {
        GenericResponse {
            status: String::from("Failed to make multi1d spectrum"),
            detail: s,
        }
    } else {
        GenericResponse {
            status: String::from("OK"),
            detail: String::from(""),
        }
    };
    Json(response)
}
// Create multi2d - one set of parameters, two axes, however.

fn make_gamma2(
    name: &str,
    parameters: &str,
    axes: &str,
    state: &State<HistogramState>,
) -> Json<GenericResponse> {
    let parameters = parse_simple_list(parameters);
    if parameters.is_err() {
        return Json(GenericResponse {
            status: String::from("Could not parse parameter list"),
            detail: parameters.unwrap_err(),
        });
    }
    let parameters = parameters.unwrap(); // Vec of names.

    let axis_list = parse_two_element_list(axes);
    if axis_list.is_err() {
        return Json(GenericResponse {
            status: String::from("Failed to break apart axis list"),
            detail: axis_list.unwrap_err(),
        });
    }
    let (xaxis_def, yaxis_def) = axis_list.unwrap();

    let xaxis = parse_single_axis_def(&xaxis_def);
    if xaxis.is_err() {
        return Json(GenericResponse {
            status: String::from("Failed to parse x axis definition"),
            detail: xaxis.unwrap_err(),
        });
    }
    let (xlow, xhigh, xbins) = xaxis.unwrap();

    let yaxis = parse_single_axis_def(&yaxis_def);
    if yaxis.is_err() {
        return Json(GenericResponse {
            status: String::from("Failed to parse y axis definition"),
            detail: yaxis.unwrap_err(),
        });
    }
    let (ylow, yhigh, ybins) = yaxis.unwrap();

    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);
    let result = if let Err(s) =
        api.create_spectrum_multi2d(name, &parameters, xlow, xhigh, xbins, ylow, yhigh, ybins)
    {
        GenericResponse {
            status: String::from("Failed to create multi2d spectrum"),
            detail: s,
        }
    } else {
        GenericResponse {
            status: String::from("OK"),
            detail: String::from(""),
        }
    };

    Json(result)
}

/// For the spectra that Rustogramer supports, only some subset of the
/// The query parameters are needed.  Specifically:
///
/// *  name  - name of the spectrum being created.
/// *  type  - Type of the spectrum being created (in SpecTcl type names).
/// *  parameters - Tcl list formatted version of the parameter names
/// Tcl list format is required since for 2DSum an PGamma
/// spectra we need to make a distinction between X and Y parameters.
/// In that case, the list is a two elements sub-list where the first
/// element is a list of the X parameters and the second a list of
/// the y parameters. e.g.
/// ?parameters={{a b c} {d e f g}}  for a PGamma spectrum
/// provide the x parameters as a,b,c and the y parameters as d,e,f,g.
/// *   axes one or two axis specifications in Tcl list format e.g.
/// {low high bins}
///
/// SpecTcl REST defines _chantype_ which we ignore because
/// all our spectra are f64 (double).
///
/// The SpecTcl REST supports defining projection spectra which
/// Rustogrammer does not have. These have _roi_ and _direction_
/// which define a region of interest contour/band and a projection direction
/// We ignore those parameters.
///
/// Return:   This is a GenericResponse where on success,
/// _status_ = *OK* and _detail_ is empty.
/// If there's an error _status_ is the top level error message and
/// _detail_ provides more information about the error.
///
#[get("/create?<name>&<type>&<parameters>&<axes>")]
pub fn create_spectrum(
    name: String,
    r#type: String,
    parameters: String,
    axes: String,
    state: &State<HistogramState>,
) -> Json<GenericResponse> {
    let type_name = r#type; // Don't want raw names like that.
    match type_name.as_str() {
        "1" => {
            // Make 1d need:
            return make_1d(&name, &parameters, &axes, state);
        }
        "2" => {
            return make_2d(&name, &parameters, &axes, state);
        }
        "g1" => {
            return make_gamma1(&name, &parameters, &axes, state);
        }
        "g2" => {
            //
            return make_gamma2(&name, &parameters, &axes, state);
        }
        "gd" => {
            // Make PGamma
        }
        "s" => {
            // Make summary spectrum.
        }
        "m2" => {
            // Make 2dsum
        }
        _ => {
            // unsupported type.
        }
    }
    Json(GenericResponse {
        status: String::from("OK"),
        detail: String::new(),
    })
}

//------------------------------------------------------------------
// Tcl List parsing is worthy of testing.

#[cfg(test)]
mod list_parse_tests {
    use super::*;

    #[test]
    fn simple_1() {
        let list = "this is a test";
        let parsed = parse_simple_list(list);
        assert!(parsed.is_ok());

        assert_eq!(
            vec![
                String::from("this"),
                String::from("is"),
                String::from("a"),
                String::from("test")
            ],
            parsed.unwrap()
        );
    }
    #[test]
    fn simple_2() {
        // Something with a { in it is not a simple list:

        let list = "this is {not a simple list";
        let parsed = parse_simple_list(list);
        assert!(parsed.is_err());
    }
    #[test]
    fn simple_3() {
        // something with a } in it is not a simple list:

        let list = "this is not a simple} list";
        let parsed = parse_simple_list(list);
        assert!(parsed.is_err());
    }
    // Test for two element list sof the form {simple-list}{simple list}
    // or {Simple-list}<whitespace>{simple-list}
    //
    #[test]
    fn two_1() {
        // Two 1 element simple lists.

        let list = "{element1}{element2}";
        let parsed = parse_two_element_list(list);
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        let l1 = parsed.0;
        let l2 = parsed.1;

        assert_eq!(vec![String::from("element1")], l1);
        assert_eq!(vec![String::from("element2")], l2);
    }
    #[test]
    fn two_2() {
        //  whitespace between the lists:

        let list = "{element1} {element2}";
        let parsed = parse_two_element_list(list);
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        let l1 = parsed.0;
        let l2 = parsed.1;

        assert_eq!(vec![String::from("element1")], l1);
        assert_eq!(vec![String::from("element2")], l2);
    }
    #[test]
    fn two_3() {
        // First list is mulit-element

        let list = "{e1 e2 e3} {e1}";
        let parsed = parse_two_element_list(list);
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        let l1 = parsed.0;
        let l2 = parsed.1;

        assert_eq!(
            vec![String::from("e1"), String::from("e2"), String::from("e3"),],
            l1
        );
        assert_eq!(vec![String::from("e1")], l2);
    }
    #[test]
    fn two_4() {
        // second list has multiples:

        let list = "{e1} {e2 e3 e4}";
        let parsed = parse_two_element_list(list);
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        let l1 = parsed.0;
        let l2 = parsed.1;

        assert_eq!(vec![String::from("e1")], l1);
        assert_eq!(
            vec![String::from("e2"), String::from("e3"), String::from("e4")],
            l2
        );
    }
    // Errors detectable by twolist parsing
    #[test]
    fn two_5() {
        // no open bracket:

        let list = "e1 e2 e3"; // Really a simple list
        let parsed = parse_two_element_list(list);
        assert!(parsed.is_err());
    }
    #[test]
    fn two_6() {
        // open but no close:

        let list = "{e1 e2 e3";
        let parsed = parse_two_element_list(list);
        assert!(parsed.is_err());
    }
    #[test]
    fn two_7() {
        //first list properly delimted but no second {

        let list = "{e1 e2 e3} a b c";
        let parsed = parse_two_element_list(list);
        assert!(parsed.is_err());
    }
    #[test]
    fn two_8() {
        // first list properly delimited but second list only opened:

        let list = "{1 2 3} {a b c";
        let parsed = parse_two_element_list(list);
        assert!(parsed.is_err());
    }
    #[test]
    fn two_9() {
        // extra open in list 1 - simple parse of the sublist
        // will fail.

        let list = "{1 2 { 3} {a b c}";
        let parsed = parse_two_element_list(list);
        assert!(parsed.is_err());
    }
    #[test]
    fn two_10() {
        // extra open in list 2
        let list = "{1 2 3} {a b { c}";
        let parsed = parse_two_element_list(list);
        assert!(parsed.is_err());
    }
    #[test]
    fn two_11() {
        // extra close in list 1?
        let list = "{1 2 } 3}  {a b c}";
        let parsed = parse_two_element_list(list);
        assert!(parsed.is_err());
    }
    #[test]
    fn two_12() {
        // extra close in list2?!?
        let list = "{1 2 3} {a b} c}";
        let parsed = parse_two_element_list(list);
        assert!(parsed.is_err());
    }
}
