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
use crate::sharedmem::binder;
// as with gates we need to map from Rustogramer spectrum
// types to SpecTcl spectrum types.

pub fn rg_sptype_to_spectcl(rg_type: &str) -> String {
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
    xparameters: Vec<String>,
    yparameters: Vec<String>,
    axes: Vec<Axis>,
    xaxis: Option<Axis>,
    yaxis: Option<Axis>,
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
            parameters: d.xparams.clone(),
            xparameters: d.xparams,
            yparameters: d.yparams.clone(),
            axes: Vec::new(),
            xaxis: None,
            yaxis: None,
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
            def.xaxis = Some(Axis {
                low: x.low,
                high: x.high,
                bins: x.bins - 2,
            });
        }
        if let Some(y) = d.yaxis {
            def.axes.push(Axis {
                low: y.low,
                high: y.high,
                bins: y.bins - 2, // Omit over/underflow.
            });
            def.yaxis = Some(Axis {
                low: y.low,
                high: y.high,
                bins: y.bins - 2,
            })
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
/// *   xparameters - the array of x parameter names.
/// *   yparameters - the array of y parameter names.
/// *   axes -- an array of at least one axis definition.  Each element
/// of the array is an object with the fields:
///     - low  - low limit of the axis.
///     - high - high limit of the axis.
///     - bins - the number of bins between [low, high)
/// *  xaxis - If there's an X axis specification (I don't think there is
/// for a summary spectrum), This contains that specification (see axes
/// above for the fields)  If there is no X axis specification this
/// field contains null.
/// *   yaxis - same as xaxis but for the Y axis specification, if any.
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
        Ok(()) => GenericResponse::ok(""),
        Err(msg) => GenericResponse::err(&format!("Failed to delete {}", name), &msg),
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
fn parse_2_axis_defs(axes: &str) -> Result<((f64, f64, u32), (f64, f64, u32)), String> {
    let axis_list = parse_two_element_list(axes);
    if axis_list.is_err() {
        return Err(format!(
            "Failed to break apart axis list: {}",
            axis_list.unwrap_err()
        ));
    }
    let (xaxis_def, yaxis_def) = axis_list.unwrap();

    let xaxis = parse_single_axis_def(&xaxis_def);
    if xaxis.is_err() {
        return Err(format!(
            "Failed to parse X axis definition: {}",
            xaxis.unwrap_err()
        ));
    }
    let (xlow, xhigh, xbins) = xaxis.unwrap();

    let yaxis = parse_single_axis_def(&yaxis_def);
    if yaxis.is_err() {
        return Err(format!(
            "Failed to parse Y axis definition {}",
            yaxis.unwrap_err()
        ));
    }
    let (ylow, yhigh, ybins) = yaxis.unwrap();

    Ok(((xlow, xhigh, xbins), (ylow, yhigh, ybins)))
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
) -> GenericResponse {
    let parsed_params = parse_simple_list(parameters);
    if parsed_params.is_err() {
        return GenericResponse::err(
            "Error parsing 1d spectrum parameter",
            &parsed_params.unwrap_err(),
        );
    }
    let params = parsed_params.unwrap();
    if params.len() != 1 {
        return GenericResponse::err(
            "Eror processing 1d spectrum parameters",
            "Only allowed one parameter",
        );
    }
    let parameter = params[0].clone();
    // Axis parsed as a simple list must be a 3 element list:

    let parsed_axes = parse_axis_def(axes);
    if parsed_axes.is_err() {
        return GenericResponse::err("Invalid axis specification", &parsed_axes.unwrap_err());
    }
    let (low, high, bins) = parsed_axes.unwrap();
    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);

    if let Err(s) = api.create_spectrum_1d(name, &parameter, low, high, bins) {
        GenericResponse::err("Failed to create 1d spectrum", &s)
    } else {
        GenericResponse::ok("")
    }
}
// Make a 2d spectrum
fn make_2d(
    name: &str,
    parameters: &str,
    axes: &str,
    state: &State<HistogramState>,
) -> GenericResponse {
    // need exactly two parameters:

    let parsed_params = parse_simple_list(parameters);
    if parsed_params.is_err() {
        return GenericResponse::err(
            "Failed to parse 2d parameter list",
            &parsed_params.unwrap_err(),
        );
    }
    let params = parsed_params.unwrap();
    if params.len() != 2 {
        return GenericResponse::err(
            "Failed to process parameter list",
            "There must be exactly two parameters for a 2d spectrum",
        );
    }
    let xp = params[0].clone();
    let yp = params[1].clone();

    let axes = parse_2_axis_defs(axes);
    if axes.is_err() {
        return GenericResponse::err("Failed to parse axes definitions", &axes.unwrap_err());
    };
    let ((xlow, xhigh, xbins), (ylow, yhigh, ybins)) = axes.unwrap();

    // Now we can try to make the spectrum:

    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);
    if let Err(s) = api.create_spectrum_2d(name, &xp, &yp, xlow, xhigh, xbins, ylow, yhigh, ybins) {
        GenericResponse::err("Failed to create 2d spectrum", &s)
    } else {
        GenericResponse::ok("")
    }
}
// make a gamma 1 spectrum ( multi1d)

fn make_gamma1(
    name: &str,
    parameters: &str,
    axes: &str,
    state: &State<HistogramState>,
) -> GenericResponse {
    let parameters = parse_simple_list(parameters);
    if parameters.is_err() {
        return GenericResponse::err("Could not parse parameter list", &parameters.unwrap_err());
    }
    let parameters = parameters.unwrap();

    let axis = parse_axis_def(axes);
    if axis.is_err() {
        return GenericResponse::err("Failed to process axis definition", &axis.unwrap_err());
    }
    let (low, high, bins) = axis.unwrap();

    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);
    if let Err(s) = api.create_spectrum_multi1d(name, &parameters, low, high, bins) {
        GenericResponse::err("Failed to make multi1d spectrum", &s)
    } else {
        GenericResponse::ok("")
    }
}
// Create multi2d - one set of parameters, two axes, however.

fn make_gamma2(
    name: &str,
    parameters: &str,
    axes: &str,
    state: &State<HistogramState>,
) -> GenericResponse {
    let parameters = parse_simple_list(parameters);
    if parameters.is_err() {
        return GenericResponse::err("Could not parse parameter list", &parameters.unwrap_err());
    }
    let parameters = parameters.unwrap(); // Vec of names.

    let axes = parse_2_axis_defs(axes);
    if axes.is_err() {
        return GenericResponse::err("Failed to parse axes definitions", &axes.unwrap_err());
    };
    let ((xlow, xhigh, xbins), (ylow, yhigh, ybins)) = axes.unwrap();

    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);
    if let Err(s) =
        api.create_spectrum_multi2d(name, &parameters, xlow, xhigh, xbins, ylow, yhigh, ybins)
    {
        GenericResponse::err("Failed to create multi2d spectrum", &s)
    } else {
        GenericResponse::ok("")
    }
}
// Make a particle gamma spectrum.
// This has two sets of parameters, x and y each an arbitrary
// length list.  There are 2 axes as well:

fn make_pgamma(
    name: &str,
    parameters: &str,
    axes: &str,
    state: &State<HistogramState>,
) -> GenericResponse {
    // Get the two parameter vectors:

    let parsed_params = parse_two_element_list(parameters);
    if parsed_params.is_err() {
        return GenericResponse::err(
            "Failed to parse parameter list",
            &parsed_params.unwrap_err(),
        );
    }
    let (xparams, yparams) = parsed_params.unwrap();

    // Now the axis specifications:

    let axes = parse_2_axis_defs(axes);
    if axes.is_err() {
        return GenericResponse::err("Failed to parse axes definitions", &axes.unwrap_err());
    };
    let ((xlow, xhigh, xbins), (ylow, yhigh, ybins)) = axes.unwrap();

    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);
    if let Err(s) = api.create_spectrum_pgamma(
        name, &xparams, &yparams, xlow, xhigh, xbins, ylow, yhigh, ybins,
    ) {
        GenericResponse::err("Failed to create pgamma spectrum", &s)
    } else {
        GenericResponse::ok("")
    }
}
// Create a summary spectrum from a single list of parameters
// and a single axis specification.

fn make_summary(
    name: &str,
    parameters: &str,
    axes: &str,
    state: &State<HistogramState>,
) -> GenericResponse {
    let parameters = parse_simple_list(parameters);
    if parameters.is_err() {
        return GenericResponse::err(
            "Failed to parse the parameter list",
            &parameters.unwrap_err(),
        );
    }
    let parameters = parameters.unwrap(); // Vec<String> now.

    let axes = parse_axis_def(axes);
    if axes.is_err() {
        return GenericResponse::err("Failed to process axis definition", &axes.unwrap_err());
    }
    let (low, high, bins) = axes.unwrap();

    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);
    if let Err(s) = api.create_spectrum_summary(name, &parameters, low, high, bins) {
        GenericResponse::err("Failed to create spectrum", &s)
    } else {
        GenericResponse::ok("")
    }
}
// Create a 2d sum spectrum.  There must be two parameter lists
// and two axes.  We let the server sort out that the two parameter
// lists must also be the same length.
fn make_2dsum(
    name: &str,
    parameters: &str,
    axes: &str,
    state: &State<HistogramState>,
) -> GenericResponse {
    let parameters = parse_two_element_list(parameters);
    if parameters.is_err() {
        return GenericResponse::err(
            "Failed to parse the parameter list(s)",
            &parameters.unwrap_err(),
        );
    }
    let (xpars, ypars) = parameters.unwrap(); // both Vec<String>

    let axes = parse_2_axis_defs(axes);
    if axes.is_err() {
        return GenericResponse::err("Failed to parse axes definitions", &axes.unwrap_err());
    }
    let ((xlow, xhigh, xbins), (ylow, yhigh, ybins)) = axes.unwrap();

    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);
    if let Err(s) =
        api.create_spectrum_2dsum(name, &xpars, &ypars, xlow, xhigh, xbins, ylow, yhigh, ybins)
    {
        GenericResponse::err("Failed to create 2d sum spectrum", &s)
    } else {
        GenericResponse::ok("")
    }
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
    Json(match type_name.as_str() {
        "1" => make_1d(&name, &parameters, &axes, state),
        "2" => make_2d(&name, &parameters, &axes, state),
        "g1" => make_gamma1(&name, &parameters, &axes, state),
        "g2" => make_gamma2(&name, &parameters, &axes, state),
        "gd" => make_pgamma(&name, &parameters, &axes, state),
        "s" => make_summary(&name, &parameters, &axes, state),
        "m2" => make_2dsum(&name, &parameters, &axes, state),
        _ => GenericResponse::err(
            "Unsupported spectrum type",
            &format!("Bad type was '{}'", type_name),
        ),
    })
}
//------------------------------------------------------------------
// Stuff needed to get the contents of a spectrum.

/// Each channel value looks like this:

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Channel {
    xchan: f64,
    ychan: f64,
    value: f64,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ContentsResponse {
    status: String,
    detail: Vec<Channel>,
}

// Determine if a spectrum type has 2 axes:

fn has_y_axis(stype: &str) -> bool {
    match stype {
        "1D" | "Multi1d" | "Summary" => false,
        "2D" | "Multi2d" | "PGamma" | "2DSum" => true,
        _ => false,
    }
}

///
/// Get the contents of a spectrum.  Note that
/// The request parameters are:
///
/// *  name (required) - the name of the spectrum to fetch
/// *  xlow (optional) - the low x limit of the chunk of the spectrum to get
/// *  xhigh(optional) - the high x limit of the chunk of the spectrum to get.
/// *  ylow (optional) - The low y limit of the chunk of the spectrum to get.
/// *  yhigh (optional) - The high y limit of the chunk of the spectrum to get.
///
/// If a limit is not supplied it is defaulted to the
/// appropriate axis limit.  This implies that we will fetch the
/// spectrum definition before doing much else.
///
/// Note - the ability to describe a region of interest
/// within which we want the contents is new with Rustogramer.
///
///
#[get("/contents?<name>&<xlow>&<xhigh>&<ylow>&<yhigh>")]
pub fn get_contents(
    name: String,
    xlow: Option<f64>,
    xhigh: Option<f64>,
    ylow: Option<f64>,
    yhigh: Option<f64>,
    state: &State<HistogramState>,
) -> Json<ContentsResponse> {
    // First get the description of the spectrum to set the
    // default ROI to the entire spectrum:

    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);
    let list = api.list_spectra(&name);
    if let Err(s) = list {
        return Json(ContentsResponse {
            status: format!("Failed to fetch info for {} : {}", name, s),
            detail: vec![],
        });
    }
    let list = list.unwrap();
    if list.len() != 1 {
        return Json(ContentsResponse {
            status: format!(
                "Failed to fetch info for {} no such spectrum or ambiguous name",
                name,
            ),
            detail: vec![],
        });
    }
    let description = list[0].clone();
    let xaxis = description.xaxis.unwrap();
    let (mut x_low, mut x_high) = (xaxis.low, xaxis.high);
    let (mut y_low, mut y_high) = if has_y_axis(&description.type_name) {
        let yaxis = description.yaxis.unwrap();
        (yaxis.low, yaxis.high)
    } else {
        (0.0, 0.0)
    };
    if let Some(xl) = xlow {
        x_low = xl;
    }
    if let Some(xh) = xhigh {
        x_high = xh;
    }
    if let Some(yl) = ylow {
        y_low = yl;
    }
    if let Some(yh) = yhigh {
        y_high = yh;
    }

    // Fetch the region of interest:

    let contents = api.get_contents(&name, x_low, x_high, y_low, y_high);
    let result = if let Err(s) = contents {
        ContentsResponse {
            status: format!("Failed to get spectrum contents: {}", s),
            detail: vec![],
        }
    } else {
        let mut reply = ContentsResponse {
            status: String::from("OK"),
            detail: vec![],
        };
        let contents = contents.unwrap();
        for c in contents {
            reply.detail.push(Channel {
                xchan: c.x,
                ychan: c.y,
                value: c.value,
            });
        }
        reply
    };

    Json(result)
}
//--------------------------------------------------------------
// What's needed to clear a set of spectra.

///
/// Handle requests to clear one or more spectra.
/// Query parameters:
///
/// * pattern - if supplied is a glob pattern that specifies the
/// set of spectra to clear.  Only spectra with names matching the pattern
/// will be cleared.  If not supplied this defaults to
/// _*_ which matches all spectra.
///
/// Note, in general, a spectrum name is a valid glob pattern allowing
/// the client to clear a single spectrum.
///
#[get("/clear?<pattern>")]
pub fn clear_spectra(
    pattern: Option<String>,
    state: &State<HistogramState>,
) -> Json<GenericResponse> {
    let mut pat = String::from("*");
    if let Some(p) = pattern {
        pat = p;
    }
    let api = SpectrumMessageClient::new(&state.inner().state.lock().unwrap().1);
    let reply = if let Err(s) = api.clear_spectra(&pat) {
        GenericResponse::err(&format!("Failed to clear spectra matching '{}'", pat), &s)
    } else {
        // also need to clear the shared memory copies of the bound
        // spectra:

        let bind_api = binder::BindingApi::new(&state.inner().binder.lock().unwrap().0);
        if let Err(s) = bind_api.clear_spectra(&pat) {
            GenericResponse::err("Failed to clear bound spectra: ", &s)
        } else {
            GenericResponse::ok("")
        }
    };

    Json(reply)
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
