//!  Handlers for the spectcl/spectrum URLs.
//!  These manipulate spectra.  A brief list of the
//!  URI's that are supported:
//!
//! *  /spectcl/spectrum/list - list spectra and their properties.
//! *  /spectcl/spectrum/delete - Deltee a spectrum.
//! *  /spectcl/spectrum/create - create a new spectrum.
//! *  /spectcl/spectrum/contents - Get the contents of a spectrum.
//! *  /spectcl/sspectrum/clear - clear
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;

use super::*;

use crate::messaging::spectrum_messages::{SpectrumMessageClient, SpectrumProperties};
use crate::sharedmem::binder;
/// as with gates we need to map from Rustogramer spectrum
/// types to SpecTcl spectrum types.

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
/// Convert SpecTcl data type to Rustogramer:
pub fn spectcl_sptype_to_rustogramer(sptype: &str) -> Result<String, String> {
    match sptype {
        "1" => Ok(String::from("1D")),
        "g1" => Ok(String::from("Mutlti1d")),
        "g2" => Ok(String::from("Multi2d")),
        "gd" => Ok(String::from("PGamma")),
        "s" => Ok(String::from("Summary")),
        "2" => Ok(String::from("")),
        "m2" => Ok(String::from("2DSum")),
        _ => Err(format!("Unsupported SpecTcl spectrum type {}", sptype)),
    }
}
//------------------------------------------------------------
// Stuff we need to list spectra and their properties.

// structures that define the JSON we'll return:

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Axis {
    low: f64,
    high: f64,
    bins: u32,
}
#[derive(Serialize, Deserialize, Clone)]
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

#[derive(Serialize, Deserialize)]
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
pub fn list_spectrum(
    filter: OptionalString,
    state: &State<SharedHistogramChannel>,
) -> Json<ListResponse> {
    let pattern = if let Some(p) = filter {
        p
    } else {
        String::from("*")
    };

    let api = SpectrumMessageClient::new(&state.inner().lock().unwrap());

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
pub fn delete_spectrum(
    name: String,
    state: &State<SharedHistogramChannel>,
) -> Json<GenericResponse> {
    let api = SpectrumMessageClient::new(&state.inner().lock().unwrap());

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

    if list.contains('{') || list.contains('}') {
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

type ParsedAxis = (f64, f64, u32);

fn parse_axis_def(axes: &str) -> Result<ParsedAxis, String> {
    let axes = parse_simple_list(axes)?;
    let axis_tuple = parse_single_axis_def(&axes)?;

    let axis = axis_tuple;
    let low = axis.0;
    let high = axis.1;
    let bins = axis.2;

    Ok((low, high, bins))
}

fn parse_2_axis_defs(axes: &str) -> Result<(ParsedAxis, ParsedAxis), String> {
    let axis_list = parse_two_element_list(axes);
    if let Err(s) = axis_list {
        return Err(format!("Failed to break apart axis list: {}", &s));
    }
    let (xaxis_def, yaxis_def) = axis_list.unwrap();

    let xaxis = parse_single_axis_def(&xaxis_def);
    if let Err(s) = xaxis {
        return Err(format!("Failed to parse X axis definition: {}", &s));
    }
    let (xlow, xhigh, xbins) = xaxis.unwrap();

    let yaxis = parse_single_axis_def(&yaxis_def);
    if let Err(s) = yaxis {
        return Err(format!("Failed to parse Y axis definition {}", &s));
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
    state: &State<SharedHistogramChannel>,
) -> GenericResponse {
    let parsed_params = parse_simple_list(parameters);
    if let Err(s) = parsed_params {
        return GenericResponse::err("Error parsing 1d spectrum parameter", &s);
    }
    let params = parsed_params.unwrap();
    if params.len() != 1 {
        return GenericResponse::err(
            "Error processing 1d spectrum parameters",
            "Only allowed one parameter",
        );
    }
    let parameter = params[0].clone();
    // Axis parsed as a simple list must be a 3 element list:

    let parsed_axes = parse_axis_def(axes);
    if let Err(s) = parsed_axes {
        return GenericResponse::err("Invalid axis specification", &s);
    }
    let (low, high, bins) = parsed_axes.unwrap();
    let api = SpectrumMessageClient::new(&state.inner().lock().unwrap());

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
    state: &State<SharedHistogramChannel>,
) -> GenericResponse {
    // need exactly two parameters:

    let parsed_params = parse_simple_list(parameters);
    if let Err(s) = parsed_params {
        return GenericResponse::err("Failed to parse 2d parameter list", &s);
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
    if let Err(s) = axes {
        return GenericResponse::err("Failed to parse axes definitions", &s);
    };
    let ((xlow, xhigh, xbins), (ylow, yhigh, ybins)) = axes.unwrap();

    // Now we can try to make the spectrum:

    let api = SpectrumMessageClient::new(&state.inner().lock().unwrap());
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
    state: &State<SharedHistogramChannel>,
) -> GenericResponse {
    let parameters = parse_simple_list(parameters);
    if let Err(s) = parameters {
        return GenericResponse::err("Could not parse parameter list", &s);
    }
    let parameters = parameters.unwrap();

    let axis = parse_axis_def(axes);
    if let Err(s) = axis {
        return GenericResponse::err("Failed to process axis definition", &s);
    }
    let (low, high, bins) = axis.unwrap();

    let api = SpectrumMessageClient::new(&state.inner().lock().unwrap());
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
    state: &State<SharedHistogramChannel>,
) -> GenericResponse {
    let parameters = match parse_simple_list(parameters) {
        Err(s) => {
            return GenericResponse::err("Could not parse parameter list", &s);
        }
        Ok(p) => p,
    };

    let ((xlow, xhigh, xbins), (ylow, yhigh, ybins)) = match parse_2_axis_defs(axes) {
        Err(s) => {
            return GenericResponse::err("Failed to parse axes definitions", &s);
        }
        Ok(a) => a,
    };

    let api = SpectrumMessageClient::new(&state.inner().lock().unwrap());

    match api.create_spectrum_multi2d(name, &parameters, xlow, xhigh, xbins, ylow, yhigh, ybins) {
        Ok(()) => GenericResponse::ok(""),
        Err(s) => GenericResponse::err("Failed to create multi2d spectrum", &s),
    }
}
// Make a particle gamma spectrum.
// This has two sets of parameters, x and y each an arbitrary
// length list.  There are 2 axes as well:

fn make_pgamma(
    name: &str,
    parameters: &str,
    axes: &str,
    state: &State<SharedHistogramChannel>,
) -> GenericResponse {
    // Get the two parameter vectors:

    let parsed_params = parse_two_element_list(parameters);
    if let Err(s) = parsed_params {
        return GenericResponse::err("Failed to parse parameter list", &s);
    }
    let (xparams, yparams) = parsed_params.unwrap();

    // Now the axis specifications:

    let axes = parse_2_axis_defs(axes);
    if let Err(s) = axes {
        return GenericResponse::err("Failed to parse axes definitions", &s);
    };
    let ((xlow, xhigh, xbins), (ylow, yhigh, ybins)) = axes.unwrap();

    let api = SpectrumMessageClient::new(&state.inner().lock().unwrap());
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
    state: &State<SharedHistogramChannel>,
) -> GenericResponse {
    let parameters = parse_simple_list(parameters);
    if let Err(s) = parameters {
        return GenericResponse::err("Failed to parse the parameter list", &s);
    }
    let parameters = parameters.unwrap(); // Vec<String> now.

    let axes = parse_axis_def(axes);
    if let Err(s) = axes {
        return GenericResponse::err("Failed to process axis definition", &s);
    }
    let (low, high, bins) = axes.unwrap();

    let api = SpectrumMessageClient::new(&state.inner().lock().unwrap());
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
    state: &State<SharedHistogramChannel>,
) -> GenericResponse {
    let parameters = parse_two_element_list(parameters);
    if let Err(s) = parameters {
        return GenericResponse::err("Failed to parse the parameter list(s)", &s);
    }
    let (xpars, ypars) = parameters.unwrap(); // both Vec<String>

    let axes = parse_2_axis_defs(axes);
    if let Err(s) = axes {
        return GenericResponse::err("Failed to parse axes definitions", &s);
    }
    let ((xlow, xhigh, xbins), (ylow, yhigh, ybins)) = axes.unwrap();

    let api = SpectrumMessageClient::new(&state.inner().lock().unwrap());
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
/// **Note**, however, that copies bound in shared memory will
/// have type long (u32) for all spectra as that's supported by
/// Xamine and f64 is not.
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
    state: &State<SharedHistogramChannel>,
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

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Channel {
    xchan: f64,
    ychan: f64,
    value: f64,
}
#[derive(Serialize, Deserialize)]
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
    state: &State<SharedHistogramChannel>,
) -> Json<ContentsResponse> {
    // First get the description of the spectrum to set the
    // default ROI to the entire spectrum:

    let api = SpectrumMessageClient::new(&state.inner().lock().unwrap());
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

    let (mut x_low, mut x_high) = if let Some(x) = description.xaxis {
        (x.low, x.high)
    } else {
        (0.0, 0.0)
    };
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
    hg: &State<SharedHistogramChannel>,
    state: &State<SharedBinderChannel>,
) -> Json<GenericResponse> {
    let mut pat = String::from("*");
    if let Some(p) = pattern {
        pat = p;
    }
    let api = SpectrumMessageClient::new(&hg.inner().lock().unwrap());
    let reply = if let Err(s) = api.clear_spectra(&pat) {
        GenericResponse::err(&format!("Failed to clear spectra matching '{}'", pat), &s)
    } else {
        // also need to clear the shared memory copies of the bound
        // spectra:

        let bind_api = binder::BindingApi::new(&state.inner().lock().unwrap());
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

#[cfg(test)]
mod spectrum_tests {
    use super::*;
    use crate::messaging;
    use crate::messaging::{condition_messages, parameter_messages, spectrum_messages};
    use crate::parameters::EventParameter;
    use crate::processing;
    use crate::sharedmem::binder;
    use crate::test::rest_common;

    use rocket;
    use rocket::local::blocking::Client;
    use rocket::Build;
    use rocket::Rocket;

    use std::sync::mpsc;

    fn make_some_test_objects(
        sapi: &spectrum_messages::SpectrumMessageClient,
        papi: &parameter_messages::ParameterMessageClient,
    ) {
        // Some parameters:

        for i in 0..10 {
            papi.create_parameter(&(format!("parameter.{}", i)))
                .expect("Creating a parameters");
        }

        // Some spectra: One of each type.

        sapi.create_spectrum_1d("oned", "parameter.0", 0.0, 1024.0, 512)
            .expect("oned");
        sapi.create_spectrum_multi1d(
            "m1d",
            &[
                String::from("parameter.0"),
                String::from("parameter.1"),
                String::from("parameter.2"),
                String::from("parameter.3"),
                String::from("parameter.4"),
                String::from("parameter.5"),
            ],
            0.0,
            1024.0,
            512,
        )
        .expect("m1d");
        sapi.create_spectrum_multi2d(
            "m2d",
            &[
                String::from("parameter.0"),
                String::from("parameter.1"),
                String::from("parameter.2"),
                String::from("parameter.3"),
                String::from("parameter.4"),
                String::from("parameter.5"),
            ],
            0.0,
            1024.0,
            256,
            0.0,
            1024.0,
            256,
        )
        .expect("m2d");
        sapi.create_spectrum_pgamma(
            "pgamma",
            &[
                String::from("parameter.0"),
                String::from("parameter.1"),
                String::from("parameter.2"),
                String::from("parameter.3"),
                String::from("parameter.4"),
                String::from("parameter.5"),
            ],
            &[
                String::from("parameter.4"),
                String::from("parameter.5"),
                String::from("parameter.6"),
                String::from("parameter.7"),
                String::from("parameter.8"),
                String::from("parameter.9"),
            ],
            0.0,
            1024.0,
            256,
            0.0,
            1024.0,
            256,
        )
        .expect("pgamma");
        sapi.create_spectrum_summary(
            "summary",
            &[
                String::from("parameter.0"),
                String::from("parameter.1"),
                String::from("parameter.2"),
                String::from("parameter.3"),
                String::from("parameter.4"),
                String::from("parameter.5"),
                String::from("parameter.6"),
                String::from("parameter.7"),
                String::from("parameter.8"),
                String::from("parameter.9"),
            ],
            0.0,
            1024.0,
            256,
        )
        .expect("summary");
        sapi.create_spectrum_2d(
            "twod",
            "parameter.0",
            "parameter.1",
            0.0,
            1024.0,
            256,
            0.0,
            1024.0,
            256,
        )
        .expect("twod");
        sapi.create_spectrum_2dsum(
            "2dsum",
            &[String::from("parameter.0"), String::from("parameter.1")],
            &[String::from("parameter.2"), String::from("parameter.3")],
            0.0,
            1024.0,
            256,
            0.0,
            1024.0,
            256,
        )
        .expect("2dsum");
    }

    fn setup() -> Rocket<Build> {
        // Note we have two domains here because of the SpecTcl
        // divsion between tree parameters and raw parameters.

        let rocket = rest_common::setup().mount(
            "/",
            routes![
                list_spectrum,
                delete_spectrum,
                create_spectrum,
                get_contents,
                clear_spectra,
            ],
        );
        //  Get the histogram sender channel from the state, instantiate
        // a parameter and histogram api and invoke make_some_test_objects
        // to set up the common test environment specific to these tests:

        let hg_api = spectrum_messages::SpectrumMessageClient::new(
            &rocket
                .state::<SharedHistogramChannel>()
                .expect("getting State")
                .lock()
                .unwrap()
                .clone(),
        );
        let par_api = parameter_messages::ParameterMessageClient::new(
            &rocket
                .state::<SharedHistogramChannel>()
                .expect("Getting state")
                .lock()
                .unwrap()
                .clone(),
        );
        make_some_test_objects(&hg_api, &par_api);

        //

        rocket
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
        // Unfiltered all spectra made by make_some_test_objects get listed:
        let rocket = setup();
        let (chan, papi, binder_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/list");
        let reply = req
            .dispatch()
            .into_json::<ListResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(7, reply.detail.len());

        // The order of the spectra is unpredictable so we
        // first sort by name:

        let mut spectrum_info = reply.detail.clone();
        spectrum_info.sort_by(|a, b| a.name.cmp(&b.name));

        // first is 2dsum:

        let info = &spectrum_info[0];
        assert_eq!("2dsum", info.name);
        assert_eq!("m2", info.spectrum_type); // SpecTcl type.
        assert_eq!("f64", info.chantype);
        assert!(info.gate.is_none());
        assert_eq!(
            vec![String::from("parameter.0"), String::from("parameter.1")].len(),
            info.xparameters.len()
        );
        for (i, s) in [String::from("parameter.0"), String::from("parameter.1")]
            .iter()
            .enumerate()
        {
            assert_eq!(s.as_str(), info.xparameters[i]);
        }
        assert_eq!(
            vec![String::from("parameter.2"), String::from("parameter.3")].len(),
            info.yparameters.len()
        );
        for (i, s) in [String::from("parameter.2"), String::from("parameter.3")]
            .iter()
            .enumerate()
        {
            assert_eq!(s.as_str(), info.yparameters[i]);
        }
        assert!(info.xaxis.is_some());
        let xaxis = info.xaxis.clone().unwrap();
        assert_eq!(0.0, xaxis.low);
        assert_eq!(1024.0, xaxis.high);
        assert_eq!(256, xaxis.bins);

        assert!(info.yaxis.is_some());
        let yaxis = &info.yaxis.clone().unwrap();
        assert_eq!(0.0, yaxis.low);
        assert_eq!(1024.0, yaxis.high);
        assert_eq!(256, yaxis.bins);

        // m1d is next alphabetically - an m2 spectrum.

        let info = &spectrum_info[1];
        assert_eq!("m1d", info.name);
        assert_eq!("g1", info.spectrum_type);
        assert_eq!("f64", info.chantype);
        assert!(info.gate.is_none());
        let sbparams = vec![
            String::from("parameter.0"),
            String::from("parameter.1"),
            String::from("parameter.2"),
            String::from("parameter.3"),
            String::from("parameter.4"),
            String::from("parameter.5"),
        ];
        assert_eq!(sbparams.len(), info.parameters.len());
        for (i, s) in sbparams.iter().enumerate() {
            assert_eq!(s.as_str(), info.parameters[i]);
        }
        assert!(info.xaxis.is_some());
        let xaxis = info.xaxis.clone().unwrap();
        assert_eq!(0.0, xaxis.low);
        assert_eq!(1024.0, xaxis.high);
        assert_eq!(512, xaxis.bins);
        assert!(info.yaxis.is_none());

        // next is m2d - a g2 spectrum.

        let info = &spectrum_info[2];
        assert_eq!("m2d", info.name);
        assert_eq!("g2", info.spectrum_type);
        assert_eq!("f64", info.chantype);
        assert!(info.gate.is_none());
        let sbparams = vec![
            String::from("parameter.0"),
            String::from("parameter.1"),
            String::from("parameter.2"),
            String::from("parameter.3"),
            String::from("parameter.4"),
            String::from("parameter.5"),
        ];
        assert_eq!(sbparams.len(), info.parameters.len());
        for (i, s) in sbparams.iter().enumerate() {
            assert_eq!(s.as_str(), info.parameters[i]);
        }
        assert!(info.xaxis.is_some());
        let xaxis = info.xaxis.clone().unwrap();
        assert_eq!(0.0, xaxis.low);
        assert_eq!(1024.0, xaxis.high);
        assert_eq!(256, xaxis.bins);

        assert!(info.yaxis.is_some());
        let yaxis = info.yaxis.clone().unwrap();
        assert_eq!(0.0, yaxis.low);
        assert_eq!(1024.0, yaxis.high);
        assert_eq!(256, yaxis.bins);

        // Next is oned - "1" spectrum.

        let info = &spectrum_info[3];
        assert_eq!("oned", info.name);
        assert_eq!("1", info.spectrum_type);
        assert_eq!("f64", info.chantype);
        assert!(info.gate.is_none());
        assert_eq!(1, info.parameters.len());
        assert_eq!("parameter.0", info.parameters[0]);
        assert!(info.xaxis.is_some());
        assert!(info.yaxis.is_none());
        let xaxis = info.xaxis.clone().unwrap();
        assert_eq!(0.0, xaxis.low);
        assert_eq!(1024.0, xaxis.high);
        assert_eq!(512, xaxis.bins);

        // next pgamma I think (type gd).

        let info = &spectrum_info[4];
        assert_eq!("pgamma", info.name);
        assert_eq!("gd", info.spectrum_type);
        assert_eq!("f64", info.chantype);
        assert!(info.gate.is_none());
        assert_eq!(6, info.xparameters.len());
        let xparams = vec![
            String::from("parameter.0"),
            String::from("parameter.1"),
            String::from("parameter.2"),
            String::from("parameter.3"),
            String::from("parameter.4"),
            String::from("parameter.5"),
        ];
        for (i, s) in xparams.iter().enumerate() {
            assert_eq!(s.as_str(), info.xparameters[i]);
        }
        assert_eq!(6, info.yparameters.len());
        let yparams = vec![
            String::from("parameter.4"),
            String::from("parameter.5"),
            String::from("parameter.6"),
            String::from("parameter.7"),
            String::from("parameter.8"),
            String::from("parameter.9"),
        ];
        for (i, s) in yparams.iter().enumerate() {
            assert_eq!(s.as_str(), info.yparameters[i]);
        }
        assert!(info.xaxis.is_some());
        let xaxis = info.xaxis.clone().unwrap();
        assert_eq!(0.0, xaxis.low);
        assert_eq!(1024.0, xaxis.high);
        assert_eq!(256, xaxis.bins);
        assert!(info.yaxis.is_some());
        let yaxis = info.yaxis.clone().unwrap();
        assert_eq!(0.0, yaxis.low);
        assert_eq!(1024.0, yaxis.high);
        assert_eq!(256, yaxis.bins);

        // Next is summary:

        let info = &spectrum_info[5];
        assert_eq!("summary", info.name);
        assert_eq!("s", info.spectrum_type);
        assert_eq!("f64", info.chantype);
        assert!(info.gate.is_none());
        assert_eq!(10, info.parameters.len());
        let params = vec![
            String::from("parameter.0"),
            String::from("parameter.1"),
            String::from("parameter.2"),
            String::from("parameter.3"),
            String::from("parameter.4"),
            String::from("parameter.5"),
            String::from("parameter.6"),
            String::from("parameter.7"),
            String::from("parameter.8"),
            String::from("parameter.9"),
        ];
        for (i, s) in params.iter().enumerate() {
            assert_eq!(s.as_str(), info.parameters[i]);
        }
        assert!(info.yaxis.is_some());
        let yaxis = info.yaxis.clone().unwrap();
        assert_eq!(0.0, yaxis.low);
        assert_eq!(1024.0, yaxis.high);
        assert_eq!(256, yaxis.bins);
        assert!(info.xaxis.is_some());
        assert_eq!(0.0, info.xaxis.clone().unwrap().low);
        assert_eq!(10.0, info.xaxis.clone().unwrap().high);
        assert_eq!(10, info.xaxis.clone().unwrap().bins);

        // Twod is last:

        let info = &spectrum_info[6];
        assert_eq!("twod", info.name);
        assert_eq!("2", info.spectrum_type);
        assert_eq!("f64", info.chantype);
        assert!(info.gate.is_none());
        assert_eq!(1, info.xparameters.len());
        assert_eq!("parameter.0", info.xparameters[0]);
        assert_eq!(1, info.yparameters.len());
        assert_eq!("parameter.1", info.yparameters[0]);
        assert!(info.xaxis.is_some());
        let xaxis = info.xaxis.clone().unwrap();
        assert_eq!(0.0, xaxis.low);
        assert_eq!(1024.0, xaxis.high);
        assert_eq!(256, xaxis.bins);

        assert!(info.yaxis.is_some());
        let yaxis = info.yaxis.clone().unwrap();
        assert_eq!(0.0, yaxis.low);
        assert_eq!(1024.0, yaxis.high);
        assert_eq!(256, yaxis.bins);

        // Close out the test
        teardown(chan, &papi, &binder_api);
    }
    #[test]
    fn list_2() {
        // filtered list.
        let rocket = setup();
        let (chan, papi, binder_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("making client");
        let req = client.get("/list?filter=t*");
        let reply = req
            .dispatch()
            .into_json::<ListResponse>()
            .expect("Parsing json");

        assert_eq!("OK", reply.status);
        assert_eq!(1, reply.detail.len());
        let info = &reply.detail[0];
        assert_eq!("twod", info.name);
        assert_eq!("2", info.spectrum_type);
        assert_eq!("f64", info.chantype);
        assert!(info.gate.is_none());
        assert_eq!(1, info.xparameters.len());
        assert_eq!("parameter.0", info.xparameters[0]);
        assert_eq!(1, info.yparameters.len());
        assert_eq!("parameter.1", info.yparameters[0]);
        assert!(info.xaxis.is_some());
        let xaxis = info.xaxis.clone().unwrap();
        assert_eq!(0.0, xaxis.low);
        assert_eq!(1024.0, xaxis.high);
        assert_eq!(256, xaxis.bins);

        assert!(info.yaxis.is_some());
        let yaxis = info.yaxis.clone().unwrap();
        assert_eq!(0.0, yaxis.low);
        assert_eq!(1024.0, yaxis.high);
        assert_eq!(256, yaxis.bins);

        teardown(chan, &papi, &binder_api);
    }
    #[test]
    fn list_3() {
        // Make a spectrm gated and check that the list shows this:

        let rocket = setup();
        let (chan, papi, binder_api) = getstate(&rocket);

        let capi = condition_messages::ConditionMessageClient::new(&chan);
        capi.create_true_condition("Acondition");
        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        sapi.gate_spectrum("twod", "Acondition")
            .expect("Gating spectrum");

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/list?filter=twod");
        let reply = req
            .dispatch()
            .into_json::<ListResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(1, reply.detail.len());
        assert!(reply.detail[0].gate.is_some());
        assert_eq!("Acondition", reply.detail[0].gate.clone().unwrap());

        teardown(chan, &papi, &binder_api);
    }
    #[test]
    fn delete_1() {
        // delete an existing spectrum.

        let rocket = setup();
        let (chan, papi, binder_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/delete?name=summary");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing json");

        assert_eq!("OK", reply.status);

        teardown(chan, &papi, &binder_api);
    }
    #[test]
    fn delete_2() {
        // delete a nonexistenf spectrum is an error:

        let rocket = setup();
        let (chan, papi, binder_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/delete?name=nosuch");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing json");

        assert_eq!("Failed to delete nosuch", reply.status);

        teardown(chan, &papi, &binder_api);
    }
    // Test spectrum creation.  We'll use ReST to create the test spectrum
    // and the API to see if it was correctly made.

    #[test]
    fn create1d_1() {
        // Correct creation of a 1d spectrum:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/create?name=test&type=1&parameters=parameter.0&axes=-1%201%20512");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Creating 1d spectrum");

        assert_eq!("OK", reply.status);

        let hapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let listing = hapi.list_spectra("test").expect("listing spectra");

        assert_eq!(1, listing.len());
        let info = &listing[0];
        assert_eq!("test", info.name);
        assert_eq!("1D", info.type_name); // Native type.
        assert_eq!(1, info.xparams.len());
        assert_eq!("parameter.0", info.xparams[0]);
        assert_eq!(0, info.yparams.len());
        assert!(info.xaxis.is_some());
        let x = info.xaxis.unwrap();
        assert_eq!(-1.0, x.low);
        assert_eq!(1.0, x.high);
        assert_eq!(514, x.bins); // underflow and overflow.
        assert!(info.yaxis.is_none());
        assert!(info.gate.is_none());

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create1d_2() {
        // invalid parameter:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/create?name=test&type=1&parameters=parameter.00&axes=-1%201%20512");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Creating 1d spectrum");

        assert_eq!("Failed to create 1d spectrum", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create1d_3() {
        // can only have one parameters:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client
            .get("/create?name=test&type=1&parameters=parameter.0%20parameter.1&axes=-1%201%20512");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Creating 1d spectrum");

        assert_eq!("Error processing 1d spectrum parameters", reply.status);
        assert_eq!("Only allowed one parameter", reply.detail);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create1d_4() {
        // invalid axis specification:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/create?name=test&type=1&parameters=parameter.0&axes=-1%201");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Creating 1d spectrum");

        assert_eq!("Invalid axis specification", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create2d_1() {
        // Create a valid 2d spectrum.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Createing client");
        let req = client.get("/create?name=test&type=2&parameters=parameter.0%20parameter.1&axes={0%20100%20100}%20{-1%201%20100}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Decoding JSON");

        assert_eq!("OK", reply.status);

        let hapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let listing = hapi.list_spectra("test").expect("Listing with API");
        assert_eq!(1, listing.len());
        let info = &listing[0];

        assert_eq!("test", info.name);
        assert_eq!("2D", info.type_name); // native type.
        assert_eq!(1, info.xparams.len());
        assert_eq!("parameter.0", info.xparams[0]);
        assert_eq!(1, info.yparams.len());
        assert_eq!("parameter.1", info.yparams[0]);
        assert!(info.xaxis.is_some());
        assert!(info.yaxis.is_some());
        assert!(info.gate.is_none());
        let x = info.xaxis.unwrap();
        assert_eq!(0.0, x.low);
        assert_eq!(100.0, x.high);
        assert_eq!(102, x.bins);
        let y = info.yaxis.unwrap();
        assert_eq!(-1.0, y.low);
        assert_eq!(1.0, y.high);
        assert_eq!(102, y.bins);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create2d_2() {
        // must only be 2 parameters

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Createing client");
        let req = client.get("/create?name=test&type=2&parameters=parameter.0%20parameter.1%20parameter.2&axes={0%20100%20100}%20{-1%201%20100}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Decoding JSON");

        assert_eq!("Failed to process parameter list", reply.status);
        assert_eq!(
            "There must be exactly two parameters for a 2d spectrum",
            reply.detail
        );

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create2d_3() {
        // badly formed parameter list:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Createing client");
        let req = client.get("/create?name=test&type=2&parameters={parameter.0%20parameter.1&axes={0%20100%20100}%20{-1%201%20100}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Decoding JSON");

        assert_eq!("Failed to parse 2d parameter list", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create2d_4() {
        // bad X parameter

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Createing client");
        let req = client.get("/create?name=test&type=2&parameters=parameter.10%20parameter.1%&axes={0%20100%20100}%20{-1%201%20100}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Decoding JSON");

        assert_eq!("Failed to create 2d spectrum", reply.status);
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create2d_5() {
        // bad y parameter.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Createing client");
        let req = client.get("/create?name=test&type=2&parameters=parameter.0%20parameter.11%&axes={0%20100%20100}%20{-1%201%20100}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Decoding JSON");

        assert_eq!("Failed to create 2d spectrum", reply.status);
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create2d_6() {
        // only one axis specification.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Createing client");
        let req = client.get(
            "/create?name=test&type=2&parameters=parameter.0%20parameter.1%&axes=0%20100%20100",
        );
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Decoding JSON");

        assert_eq!("Failed to parse axes definitions", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create2d_7() {
        // Parse error for axis defs:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=2&parameters=parameter.0%20parameter.1&axes={0%20100%20100}%20{-1%201%20100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Decoding JSON");

        assert_eq!("Failed to parse axes definitions", reply.status);
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn createg1_1() {
        // successful creation of a Multi1D (g1 in SpecTcl notation).

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=g1&parameters=parameter.0%20parameter.1%20parameter.2%20parameter.3&axes=0%20100%20100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing JSON");

        assert_eq!("OK", reply.status);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let list = sapi.list_spectra("test").expect("API listing of spectrum");
        assert_eq!(1, list.len());
        let info = &list[0];
        assert_eq!("test", info.name);
        assert_eq!("Multi1d", info.type_name);
        assert_eq!(4, info.xparams.len());
        assert_eq!(0, info.yparams.len());
        let params = ["parameter.0", "parameter.1", "parameter.2", "parameter.3"];
        for (i, s) in params.iter().enumerate() {
            assert_eq!(*s, info.xparams[i]);
        }

        assert!(info.xaxis.is_some());
        assert!(info.yaxis.is_none());
        assert!(info.gate.is_none());

        let xaxis = info.xaxis.unwrap();
        assert_eq!(0.0, xaxis.low);
        assert_eq!(100.0, xaxis.high);
        assert_eq!(102, xaxis.bins);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn createg1_2() {
        // Need only one axis.
        // A white box note:  The sam code is used to parse
        // axes for all spectrum types so between the 1d and 2d
        // tests, parse error reporting has been done.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=g1&parameters=parameter.0%20parameter.1%20parameter.2%20parameter.3&axes={0%20100%20100}%20{0%20100%20100}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing JSON");

        assert_eq!("Failed to process axis definition", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn createg1_3() {
        // all parameters must be defined:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=g1&parameters=parameter.0%20parameter.1%20parameter.2%20parameter.13&axes=0%20100%20100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing JSON");

        assert_eq!("Failed to make multi1d spectrum", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn createg2_1() {
        // succesfully create a Multi2d (g2 in SpecTcl parlance).

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=g2&parameters=parameter.0%20parameter.1%20parameter.2%20parameter.3&axes={0%20100%20100}%20{0%20100%20100}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing JSON");

        assert_eq!("OK", reply.status);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let list = sapi.list_spectra("test").expect("API listing of spectrum");
        assert_eq!(1, list.len());
        let info = &list[0];
        assert_eq!("test", info.name);
        assert_eq!("Multi2d", info.type_name);
        assert_eq!(4, info.xparams.len());
        assert_eq!(0, info.yparams.len());
        assert!(info.xaxis.is_some());
        assert!(info.yaxis.is_some());
        assert!(info.gate.is_none());

        let params = ["parameter.0", "parameter.1", "parameter.2", "parameter.3"];
        for (i, s) in params.iter().enumerate() {
            assert_eq!(*s, info.xparams[i]);
        }

        let x = info.xaxis.unwrap();
        assert_eq!(0.0, x.low);
        assert_eq!(100.0, x.high);
        assert_eq!(102, x.bins);

        let y = info.yaxis.unwrap();
        assert_eq!(0.0, y.low);
        assert_eq!(100.0, y.high);
        assert_eq!(102, y.bins);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn createg2_2() {
        // all parameters must be defined:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=g2&parameters=parameter.0%20parameter.1%20parameter.2%20parameter.13&axes={0%20100%20100}%20{0%20100%20100}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing JSON");
        assert_eq!("Failed to create multi2d spectrum", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn createg2_3() {
        // Need 2 axes:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=g2&parameters=parameter.0%20parameter.1%20parameter.2%20parameter.3&axes=0%20100%20100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("parsing JSON");
        assert_eq!("Failed to parse axes definitions", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn creategd_1() {
        // Successful creation of PGamma  spectrum (gd in SpecTcl).

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=gd&parameters={parameter.0%20parameter.1%20parameter.2}%20{parameter.3%20parameter.4}&axes={0%20100%20100}%20{-1%201%20200}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let listing = sapi.list_spectra("test").expect("Listing spectra with API");
        assert_eq!(1, listing.len());
        let info = &listing[0];
        assert_eq!("test", info.name);
        assert_eq!("PGamma", info.type_name);
        assert_eq!(3, info.xparams.len());
        assert_eq!(2, info.yparams.len());
        assert!(info.xaxis.is_some());
        assert!(info.yaxis.is_some());
        assert!(info.gate.is_none());

        let xpars = ["parameter.0", "parameter.1", "parameter.2"];
        for (i, s) in xpars.iter().enumerate() {
            assert_eq!(*s, info.xparams[i]);
        }
        let ypars = ["parameter.3", "parameter.4"];
        for (i, s) in ypars.iter().enumerate() {
            assert_eq!(*s, info.yparams[i]);
        }
        let x = info.xaxis.unwrap();
        assert_eq!(0.0, x.low);
        assert_eq!(100.0, x.high);
        assert_eq!(102, x.bins);

        let y = info.yaxis.unwrap();
        assert_eq!(-1.0, y.low);
        assert_eq!(1.0, y.high);
        assert_eq!(202, y.bins);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn creategd_2() {
        // All params must be defined. for x
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=gd&parameters={parameter.0%20parameter.1%20parameter.12}%20{parameter.3%20parameter.4}&axes={0%20100%20100}%20{-1%201%20200}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Failed to create pgamma spectrum", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn creategd_3() {
        // All params must be defined for y

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=gd&parameters={parameter.0%20parameter.1%20parameter.2}%20{parameter.3%20parameter.14}&axes={0%20100%20100}%20{-1%201%20200}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Failed to create pgamma spectrum", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn creategd_4() {
        // need two parameter lists.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=gd&parameters=parameter.0%20parameter.1%20parameter.2&axes={0%20100%20100}%20{-1%201%20200}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Failed to parse parameter list", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn creategd_5() {
        // Need two axes.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=gd&parameters={parameter.0%20parameter.1%20parameter.2}%20{parameter.3%20parameter.4}&axes=0%20100%20100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Failed to parse axes definitions", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn createsummary_1() {
        // Create a valid summary spectrum.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=s&parameters=parameter.0%20parameter.1%20parameter.2%20parameter.3&axes=-1%201%20100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let listing = sapi
            .list_spectra("test")
            .expect("Using API to list spectra");
        assert_eq!(1, listing.len());
        let info = &listing[0];
        assert_eq!("test", info.name);
        assert_eq!("Summary", info.type_name);
        assert_eq!(4, info.xparams.len());
        assert_eq!(0, info.yparams.len());
        assert!(info.xaxis.is_some());
        assert_eq!(0.0, info.xaxis.unwrap().low);
        assert_eq!(4.0, info.xaxis.unwrap().high);
        assert_eq!(6, info.xaxis.unwrap().bins);

        assert!(info.yaxis.is_some());
        assert!(info.gate.is_none());

        let pars = ["parameter.0", "parameter.1", "parameter.2", "parameter.3"];
        for (i, p) in pars.iter().enumerate() {
            assert_eq!(*p, info.xparams[i]);
        }
        let y = info.yaxis.unwrap();
        assert_eq!(-1.0, y.low);
        assert_eq!(1.0, y.high);
        assert_eq!(102, y.bins);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn createsummary_2() {
        // All parameters must be defined.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=s&parameters=xparameter.0%20parameter.1%20parameter.2%20parameter.3&axes=-1%201%20100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Failed to create spectrum", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn createsummary_3() {
        // Only one parameter list allowed.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=s&parameters={parameter.0%20parameter.1%20parameter.2%20parameter.3}%20{parameter.4}&axes=-1%201%20100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Failed to parse the parameter list", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn creatsummary_4() {
        // only one axis list allowed.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Creating client");
        let req = client.get("/create?name=test&type=s&parameters=parameter.0%20parameter.1%20parameter.2%20parameter.3&axes={-1%201%20100}%20{0.0%201.0%2050}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Failed to process axis definition", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create2dsum_1() {
        // Correctly create a 2DSum (m2) spectrum.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/create?name=test&type=m2&parameters={parameter.0%20parameter.1%20parameter.2%20parameter.3}%20{parameter.4%20parameter.5%20parameter.6%20parameter.7}&axes={0.0%2010.0%20100}%20{-1.0%201.0%20250}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        let list = sapi
            .list_spectra("test")
            .expect("Using API to get spectrum info");
        assert_eq!(1, list.len());
        let info = &list[0];
        assert_eq!("test", info.name);
        assert_eq!("2DSum", info.type_name);
        assert_eq!(4, info.xparams.len());
        assert_eq!(4, info.yparams.len());
        assert!(info.xaxis.is_some());
        assert!(info.yaxis.is_some());
        assert!(info.gate.is_none());
        let xpars = ["parameter.0", "parameter.1", "parameter.2", "parameter.3"];
        let ypars = ["parameter.4", "parameter.5", "parameter.6", "parameter.7"];
        // this loop takes advantage of the fact the param lists are same lengths.
        for (i, s) in xpars.iter().enumerate() {
            assert_eq!(*s, info.xparams[i]);
            assert_eq!(ypars[i], info.yparams[i]);
        }

        let x = info.xaxis.unwrap();
        assert_eq!(0.0, x.low);
        assert_eq!(10.0, x.high);
        assert_eq!(102, x.bins);

        let y = info.yaxis.unwrap();
        assert_eq!(-1.0, y.low);
        assert_eq!(1.0, y.high);
        assert_eq!(252, y.bins);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn creates2dsum_2() {
        // two parameter lists are required.
        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/create?name=test&type=m2&parameters=parameter.0%20parameter.1%20parameter.2%20parameter.3&axes={0.0%2010.0%20100}%20{-1.0%201.0%20250}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Failed to parse the parameter list(s)", reply.status);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create2dsum_3() {
        // all x parameters must be defined.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/create?name=test&type=m2&parameters={xparameter.0%20parameter.1%20parameter.2%20parameter.3}%20{parameter.4%20parameter.5%20parameter.6%20parameter.7}&axes={0.0%2010.0%20100}%20{-1.0%201.0%20250}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Failed to create 2d sum spectrum", reply.status);
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create2dsum_4() {
        // All y parameters must be defined.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/create?name=test&type=m2&parameters={parameter.0%20parameter.1%20parameter.2%20parameter.3}%20{parameter.4%20parameter.5%20parameter.6%20parameter.70}&axes={0.0%2010.0%20100}%20{-1.0%201.0%20250}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Failed to create 2d sum spectrum", reply.status);
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create2dsum_5() {
        // X/Y parameters must be the same length.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/create?name=test&type=m2&parameters={parameter.0%20parameter.1%20parameter.2%20parameter.3}%20{parameter.4%20parameter.5%20parameter.6}&axes={0.0%2010.0%20100}%20{-1.0%201.0%20250}");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");
        assert_eq!("Failed to create 2d sum spectrum", reply.status);
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn create2dsum_6() {
        // Must have two axis lists.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/create?name=test&type=m2&parameters={parameter.0%20parameter.1%20parameter.2%20parameter.3}%20{parameter.4%20parameter.5%20parameter.6%20parameter.7}&axes=0.0%2010.0%20100");
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("Failed to parse axes definitions", reply.status);
        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn get_1() {
        // Initially, none of the test spectra have any data:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/contents?name=oned&xlow=0.0&xhigh=1024.0");
        let reply = req
            .dispatch()
            .into_json::<ContentsResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(0, reply.detail.len());

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn get_2() {
        // put a count in channel 256 (512.0) first.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        // Make the event/event vector to send to the histogramer:

        let p = EventParameter::new(1, 512.0);
        let e = vec![p];
        let events = vec![e];

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        sapi.process_events(&events).expect("Providing events");

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/contents?name=oned&xlow=0.0&xhigh=1024.0");
        let reply = req
            .dispatch()
            .into_json::<ContentsResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(1, reply.detail.len());
        assert_eq!(512.0, reply.detail[0].xchan);
        assert_eq!(1.0, reply.detail[0].value);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn get_3() {
        // set AOI so that we don't see the count:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        // Make the event/event vector to send to the histogramer:

        let p = EventParameter::new(1, 512.0);
        let e = vec![p];
        let events = vec![e];

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        sapi.process_events(&events).expect("Providing events");

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/contents?name=oned&xlow=514.0&xhigh=1024.0");
        let reply = req
            .dispatch()
            .into_json::<ContentsResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status);
        assert_eq!(0, reply.detail.len());

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn get_4() {
        // put a count in the twod spectrum.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        // Make the event/event vector to send to the histogramer:

        let p1 = EventParameter::new(1, 512.0);
        let p2 = EventParameter::new(2, 256.0); // so xchan/ychan differ.
        let e = vec![p1, p2];
        let events = vec![e];

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        sapi.process_events(&events).expect("Providing events");

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/contents?name=twod&xlow=0.0&xhigh=1024.0&ylow=0.0&yhigh=1024.0");
        let reply = req
            .dispatch()
            .into_json::<ContentsResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", reply.status);
        assert_eq!(1, reply.detail.len());

        assert_eq!(512.0, reply.detail[0].xchan);
        assert_eq!(256.0, reply.detail[0].ychan);
        assert_eq!(1.0, reply.detail[0].value);

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn get_5() {
        // count outsidef of ROI

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        // Make the event/event vector to send to the histogramer:

        let p1 = EventParameter::new(1, 512.0);
        let p2 = EventParameter::new(2, 256.0); // so xchan/ychan differ.
        let e = vec![p1, p2];
        let events = vec![e];

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        sapi.process_events(&events).expect("Providing events");

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/contents?name=twod&xlow=0.0&xhigh=1024.0&ylow=258.0&yhigh=1024.0");
        let reply = req
            .dispatch()
            .into_json::<ContentsResponse>()
            .expect("Parsing JSON");
        assert_eq!("OK", reply.status);
        assert_eq!(0, reply.detail.len());

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn get_6() {
        // No such spectrum.

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        let client = Client::untracked(rocket).expect("Making client");
        let req = client.get("/contents?name=twodd&xlow=0.0&xhigh=1024.0&ylow=258.0&yhigh=1024.0");
        let reply = req
            .dispatch()
            .into_json::<ContentsResponse>()
            .expect("Parsing JSON");

        assert_eq!(
            "Failed to fetch info for twodd no such spectrum or ambiguous name",
            reply.status
        );

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn clear_1() {
        // Clear all spectra:

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        // Make the event/event vector to send to the histogramer:

        let p1 = EventParameter::new(1, 512.0);
        let p2 = EventParameter::new(2, 256.0); // so xchan/ychan differ.
        let e = vec![p1, p2];
        let events = vec![e];

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        sapi.process_events(&events).expect("Providing events");

        let client = Client::untracked(rocket).expect("Rocket client");
        let req = client.get("/clear"); // no pattern means *
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status, "{}", reply.detail);

        // all of the spectra should have no counts:

        let spectra = vec!["oned", "m1d", "m2d", "pgamma", "summary", "twod", "2dsum"];
        for s in spectra {
            let data = sapi
                .get_contents(s, -1024.0, 1024.0, -1024.0, 1024.0)
                .expect("Get contents");
            assert_eq!(0, data.len(), "{} has counts", s);
        }

        teardown(chan, &papi, &bind_api);
    }
    #[test]
    fn clear_2() {
        // Clear only m1d

        let rocket = setup();
        let (chan, papi, bind_api) = getstate(&rocket);

        // Make the event/event vector to send to the histogramer:

        let p1 = EventParameter::new(1, 512.0);
        let p2 = EventParameter::new(2, 256.0); // so xchan/ychan differ.
        let e = vec![p1, p2];
        let events = vec![e];

        let sapi = spectrum_messages::SpectrumMessageClient::new(&chan);
        sapi.process_events(&events).expect("Providing events");

        let client = Client::untracked(rocket).expect("Rocket client");
        let req = client.get("/clear?pattern=m1d"); // no pattern means *
        let reply = req
            .dispatch()
            .into_json::<GenericResponse>()
            .expect("Parsing JSON");

        assert_eq!("OK", reply.status, "{}", reply.detail);

        // m1 should be cleared...everyone else has counts (I think).

        let spectra = vec![
            ("m1d", 0),
            ("oned", 1),
            ("pgamma", 0),
            ("summary", 2),
            ("twod", 1),
            ("2dsum", 0),
        ];
        for s in spectra {
            let data = sapi
                .get_contents(s.0, -1024.0, 1024.0, -1024.0, 1024.0)
                .expect("Get contents");
            assert_eq!(s.1, data.len(), "{} has count mismatch", s.0);
        }

        teardown(chan, &papi, &bind_api);
    }
}
