# Tcl REST reference

Rustogramer installs a Tcl REST API in the /share/restclients/Tcl subdirectory of the installation tree Linux and in the restclients\Tcl directory on windows. To use the packages in that directory add it to the Tcl Library search path.  Two packages are provided:

*  [SpecTclRESTClient](#low-level-package) - a low level obejct oriented interface.
*  [SpecTclRestCommand](#spectcl-command-simulation) Simulation of SpecTcl commands using the low level REST client software.

There two ways to get the appropriate subdirectory added to your package search path.  The first is to define
TCLLIBPATH e.g. suppose the installation directory was /usr/opt/rustogramer/1.0.0:
On linux:

```bash
TCLLIBPATH=/usr/opt/rustogramer/1.0.0/share/restclients/Tcl tclsh
```

Rusn tclsh with the library added and 

```cmd
TCLLIBPATH=/usr/opt/rustogramer/1.0.0/restclients/Tcl tclsh
```

does so on windows as well.

As second method, is to **lappend** the script package directory to the **auto_path** global variable, which contains the search path.  Suppose, to prevent pollution of the TCLLIBPATH environment variable (which could have additional package directory trees needed by your application), you instead define the TCLREST environment variable to point to the package directory:

```tcl
lappend auto_path $::env(TCLREST)
```

Will work on both Linux and Windows to add that path to the package search path because the global variable ```env``` is an array whose keys are environment variable names and values the values of those environment variables.

## Low Level Package

The low level pacakge provides an object oriented approach to interacting with the SpecTcl server.  
Scripts that use this should

*  Instantiate a ```SpecTclRestClient``` object.
*  Make requests of that object.


### Construction

Construction looks like:

```tcl
set client [SpecTclRestClient name ?-host host-spec? ?-port port-spec? ?-debug bool?]
```

Where:

*  *name* - is the name of the object.  If you use the special name ```%AUTO%``` a unique name will be used.
*  *-host* - if provided is the host (IP address or DNS name) of the host running the server. This defaults to ```localhost```
*  *-port* - If provided is the port number on which SpecTcl or Rustogramer is listening for ReST connections.  Both programs output the value of this to stdout early in their startup.
This defaults to ```8000``` which is the default Rustogramer ReST port.
*  *-debug*  - Expects a boolean value which, if provided and true enabled debugging output to stdout showing both the requests and response received.  Defaults to false.


### $client applyGate

```
$client applyGate agate spectra
```

Apply a gate to one or more spectra.

### Parameters

* agate - Name of a condition/gate.
* spectra - List of spectra to apply the gate to.

### Description

Applies the gate to the list of spectra.

### Example:

```tcl
$client applyGate slice [list spec1 spec2 spec3]

### $client applyList

```tcl
$client applylist ?pattern?
```

### Parameters

* *pattern* - optional glob pattern against which the spectrum names must match to be included.

### Description

List the gates for all spectra that match the pattern. If not supplied, the pattern defaults to ```*```` matching all spectra.

### Returns

A list of dicts.  Each dict has the keys:

* **gate** - name of a gate.
* **spectrum** - The name of the spectrum the gate is applied to.

### $client attachSource

```Tcl
$client attachSource type source-spec ?size? ?format?
```

### Parameters

*  type - The source type.  This can be either ```pipe``` or ```file```.
*  source-spec - Source specification. For file data sources this is the path to the file. For pipe data sources this is the full command string needed to run the program.
*  size - Size of reads in bytes that will be done (defaults to 8192).
*  format- Data format.  Defaults to ```ring```

See the Spectcl attach command in the documentation.

### Description

Attaches a data source to the server.  Note that for file data sources, and  command names, those must be visible in the context of the server.  The data source is not active when attached.  Data analysis must first be started.

### Returns

Nothing.

### $client attachList

```tcl
$client attachList
```

### Parameters
None


### Returns
Returns a string that identifies the current data source.

### $client sbindAll

```tcl
$client sbindAll
```

### Parameters
None
### Description

Binds all spectra to the shared display memory.

### Returns
None

### $client sbindSpectra

```
$client sbindSpectra spectra
```
### Parameters

* *spectra*  - A list of spectra to bind.

### Description

Binds the spectra in the list provided to display memory e.g.

```tcl
$client sbindSpectra [list spec1 spec2 spec3]
```

### Returns
None
### $client sbindList

```
$client sbindList ?pattern?
```

### Parameters

* *pattern* Optional pattern. Only spectra that match the pattern will be included in the listing. If omitted, ```*``` is used which matches all spectra.

### Description

Returns a list of dicts that reflect the display bindings for spectra that match a glob pattern.

### Returns

List of dicts that have the keys:
* spectrumid - a numeric id assigned to the spectrum.
* name - name of the spectrum.
* binding - Binding slot number for the spectrm.

Note that unbound spectra are omitted from the list, even if their name matchs the pattern.

### $client fitCreate

```tcl
$client fitCreate name spectruml ow high type
```

### Parameters

*  *name* - unique name to assign to the fit.
*  *spectrum* - Name of the one dimensional spectrumon which to define the fit.
* *low* - low channel limit over which the fit is performed.
* *high* - high channel limit over which the fit is performed.
* *ftype* - fit type.  This can be a built in fit type or a fit type that was added by the application.

### Description

Creates a new fit object.  This unconditionally fails on Rustogramer as it does not support internsl fitting.

### Returns
None

### $client fitUpdate

```tcl
$client fitUpdate ?pattern?
```

### Parameters

* *pattern* - optional glob pattern. Only fits with names that match the pattern are updated. If not supplied, defaults to ```*``` which matches everything.

### Description

Matching fits are recomputed on the current data in their spectra.  As data accumulate into histograms it is important to update the fit parameters to prevent them from getting out of date.

### Returns
Nothing.

### $client fitDelete

```tcl
$client fit delete fit-name
```
### Parameters

* fit-name - Name of the fit to delete.

### Description

Deletes the named fit.

### Returns
None

### $client fitList

```tcl
$client fitList ?pattern?
```

### Parameters

* *pattern* - Optional glob pattern. Only fits with names that match the pattern are included in the list.  If not rovided, the pattern defaults to ```*``` which matches everything.

### Description

Returns a list of the properties of all fits that match the pattern.

### Returns

Tcl list of dicts.  Each dict in the  list decribes a fit with the following keys:

* *name* - Name of the fit.
* *spectrum* - Name of the spectrum the fit is defined on.
* *type*  - Type of fit.
* *low*, *high*  Limits of the fit in bins.
*  *parameters*  Dict containing fit parameters computed by the most recent update request.  THe keys in this dict will depend on the fit type, however all fits should provide a **chisquare** key to asess the goodness of the fit.  See the [Fit ReST request](./chap7_2_fit.md) for more information.

### $client fitProc

```tcl
$client fitProc name
```

### Parameters

* *name* - name of the fit.

### Description

Returns a Tcl proc definition that can compute the fit at any point on the spectrum.  The fit is parameterized by a floating point position on the spectrum x axis.  It evaluates and returns the value of the model function given the parameterization of the fit as of its last update.

### Returns

The text of the fit proc.  The proc name will be ```fitline``` and will be paramterized by a position on the X axis.

### $client foldApply

```tcl
$clent foldApply gate spectrum
```

### Parameters
* *gate* - name of the gate to use as  a fold.
* *spectra* - List of spectra to apply8 the fold to 

For example:

```tcl
$client foldApply afold [list s1 s2 s3 s4]
```
### Description

Given a gate and a Tcl list of spectra, applies the gate as a fold to the spectrum.

### Returns

None

### $client foldList

```tcl
$client foldList ?pattern?
```
### Parameters

* pattern - Optional glob pattern.  Only spectra with names that match the pattern will be listed.  If the pattern parameter is omitted, ```*``` is used which matches everything.

### Description

Lists the properties of folds that match a pattern.  

### Returns

A list of dicts. Each dict contains the keys:

*  *spectrum*  - name of the folded spectrum.
*  *gate* - Name of the gate/condition used to do the folding.

### $client foldRemove

```tcl
$client foldRemove spectrum
```
### Parameters

* *spectrum* - name of a spectrum to be unfolded.

### Description

Removes any fold from a spectrum.

### Returns
Nothing.

### $client channelGet

```tcl
$client channelGet spectrum xchan ?ychan?
```

### Parameters

* *spectrum* - name of the spectrum.
* *xchan*   - X channel coordinate to fetch.
* *ychan*   - only required for spectra with two axes, this is the Y channel coordinate to fetch.


### Description
Fetches the value of a spectrum bin idendified by its bin coordinates.

### Returns

Integer number of counts in the specified bin.

### $client channelSet
```tcl
$client channelSet spectrum value xchannel ?ychannel?
```

### Parameters
* *spectrum* - name of the spectrum.
* *value*   - Value to load into the channel
* *xchan*   - X channel coordinate to fetch.
* *ychan*   - only required for spectra with two axes, this is the Y channel coordinate to load

### Description

Loads a channel in a spectrum with a specified value.

### Returns
None.

### $client spectrumClear

```tcl
$client spectrum clear pattern
```

### Parameters
* *pattern* Required glob pattern.  Spectra which match this pattern are cleared.  There is no default, See, however [spectrumClearAll](#spectrumClearAll) below.  Making pattern required supports clearing individual spectra.

Examples:

```tcl
$client spectrumClear george;  # Only clear spectrum named "george"
$client spectrumClear event.*; # Clear spectra with names beginning "event."
```

### Description

Clears the contents of spectra that match the required glob pattern parameter.

### Returns
None

### $client spectrumClearAll

```tcl
$client spectrumClearAll
```

### Parameters

None

### Description


Clear the contents of all spectra.

### Returns
None



### $client spectrumProject

Make a projection spectrum.

### Parameters
* *old* - Name of the spectrum being projected (must exist).
* *new* - Name of the spectrum to create (must *not* exist).
* *direction* - The string ```x``` or ```y``` indicating the projection direction.  ```x``` Means project down onto the x axis.
* *snapshot* non-zero value if the spectrum created should be a snapshot spectum.  If zero the projection will be a snapshot.
*   *contour* (optional) - if provided must be the name of a contour that is displayable on *old*.  Only the region within this contour is projected.  If the spectrum is not a snapshot, the contour is applied to the *new* spectrum so that the region of projection remains the contour.


### Description

Creates a projection spectrum.  Note that in the Tcl API at this time, projections are always bound into display memory.

### Returns
None.

### $client spectrumStatistics

Obtain over and underflow statistics spectra.

### Parameters

* *pattern* (optional) - Spectra whose names match the pattern are returned.   If not supplied, the  pattern defaults to ```*``` which matches all spectrum names.


### Description

Requests a count of the x/y under and overflow counts.  Underflow counts are incremented when an increment point would be to the left, or below of the axis (x underflow if left of the y axis, y undeflow if below the x axis).  Overflows are when the increment point would be to the right  or above an axis end point.

### Returns

A list of dicts.  Each dict provides the under/overflow counts for one spectrum and has the following keys:

* **name** - name of the spectrum being described.
* **underflows** - list of underflow counts (one element for 1d spectra and 2 elements for 2d spectra).  Note that rustogramer provides both elements unconditionally so you must know something about the underlying spectrum to interpret the result. 
If a second list element is meaningful, the first element is the number of x underfows, the second the y undeflows.
* **overflows** - list of overflow counts.

### $client treeparameterCreate

Create a new paraemeter with tree parameter metadata.

### Parameters

* *name* - name of the new parameter.  This must not already be a parameter name.
* *low* - Low limit metadata for the parameter.
* *high*  - High limit metadata for the parameter.
* *units* - (optional) Units of measure for the parameter, if not supplied, defaults to an empty string.


### Description

Creates a new parameter and provides it with tree parameter metadata.

### Returns
None

### $client treeparameterList

Lists the tree parameters.

### Parameters
* *filter* (optional) - Lists the properties of all parameters with names that match the glob filter string.  If the optional *filter* is not supplied it defaults to ```*``` which matches all parameter names.

### Description

Produces a list of all parameters and their tree parameter metadata.  Note that if the server is SpecTcl only parameters that are explitly tree parameters can be included in the list.

### Returns
A list of dicts.  One per parameter that matches the *filter*.  Each dict describes a parameter with the following keys:

* **name** - name of the parameter.
* **id**   - Id of the parameter.  In Rustogramer this is assigned. In SpecTcl, it can be either explicitly defined by the user or assigned by the tree parameter subsystem.
* **bins** - Number of bins recommended for axes on this parameter.
* **low** - Recommended low limit for  axes on this parameter.
* **hi** - recommended high limit for axes on this parameter.
* **units** - Units of measure of the parameter.
* **description** - This is only available for Rustogrmer and reserved for a future use when this might be a long form description of the parameter's purpose.


### $client treeparameterListNew

Lists the tree paramters that were created by users during the program run. Only SpecTcl produces useful information here.

### Parameters

None

### Description

In SpecTcl the ```treeparameter -create``` command provides the ability to create tree parameters on the fly.  It may be desirable to save these tree parameter definitions to file.  Rustogramer, however, can define tree parameters from the event parameter files.  As such it does not really support this but returns as if no parameters were created.

### Returns

A list of the names of created parameters.

### $client treeparameterSet

Sets the tree parameter metadata for a parameter.

### Parameters

* *name* - Name of the parameter
* *bins* - Suggested number of bins for an axis on this parameter.
* *low*  - Suggested low limit for axes on this parameter.
* *high*  - Suggested high limit for axes on this parameter.
* *units* (optional) - Units of measure for the parameter. Defaults to an empty string

### Description

Modifies the metadata for a treee parameter. This modifies all metadata. To modify selected parts of the metaata, you can first list the parameter for example

```tcl
# Modify only the bins metadata (to 100) for the parameter geore.
# For simplicity assume george exists.

set metadata [lindex [$client treeparameterList george] 0]
$client treeparameterSet [dict get $metadata name]  \
    100, [dict get $metadata low] \
    [dict get $metadata hi] \ 
    [dict get $metadata units]

```

see the convienience method below, however which can do this sort of thing in a production quality way for you.

### Returns

None

### $client treeparameterSetInc 

Sets the width of chanels for a tree parmeter's suggested axes.

### Parameters

* *name* - name of an existing tree parameter.
* *width* - desired bin width.

### Description

Using the high/low metadata for a parameter, computes a new value for the bins metadata so that the bin width will be the *width* parameter.

### Returns

None

### $client treeparameterSetBins

Set the number of bins metadata fo a tree parameter.

### Parameters
* *name* - tree parameter name.
* *bins* - desired bins metadata

### Description

For a given tree parameter, sets its bins metadata only.

### Returns
None

### $client treeparameterSetUnits

Sets new units metadata for a parameter.

### Parameters

* *name* - name of the parameter.
* *units* - New units of measure. Must not be an empty string.

### Description
Sets a new units of measure metadata for a given treeparameter.

### Returns
None

### $client treeparameterSetLimits

Sets the suggested axis limits for a tree parameter.

### Parameters

* *name* - name of the tree parameter.
* *low* -  new low limit metadata.
* *high* - new high limit metadata.

### Returns
None

### $client treeparameterCheck

Fetch the modified flag for the tree parameter.

### Parameters

* *name* - name of the parameter.

### Description

Tree parameters have a modification flag.  When metadata are changed, this flag is set.  The intent is that applications can use this to determine if saving a tree parameter  is needed for state recovery.  If the modifiation flag is not set, in general; the parameter need not be saved.

### Returns
None

### treeparameterUncheck
Reset the modified flag.

### Parameters
* *name* - name of the tree parameter.

### Description
Unsets the changed flag of the tree parmaeter (see) [treeparameterCheck](#client-treeparametercheck) above.

### Returns 
None

### $client treeparameterVersion

Get the verison string

### Parameters
none

### Description
 Not all of the tree parameter capabilities are implemented in all versions.   THis method returns the tree parameter version string.

### Returns

Tree parameter subsystem version string. In general this will be in the form M.m where ```M``` is a major version and ```m``` is the minor version.

### $client treevariableList

Lists the tree variables and their properties.

### Parameters

None


### Returns

A list of dicts.  Each dict describes a single treevariable using the following keys:

* **name** - name of the variable.
* **value** - Current variable value.
* **units** - Variable's units of measure.

### $client treevariableSet

Set new value and metadata:

### Parameters

* *name* -  name of the variable to modify.
* *value*  - New value for the variable.
* *units* - New units for the variable.


### Description

Sets value and metadata for the treevariable.  Sadly the only way to just set the value is to first get its units:

```tcl

proc setValue {client name value} {
    set listing [$client treevariableList]
    foreach item $listing {
        if {$name eq [$dict get $item name]} {
            set units [dict get $item units]
            $client treevariableSet $name $value $units
            return
        }
    }
    
    error "No such tree variable: $name"
}
```

### Returns
None

### $client treevariableCheck

Check the state of the variable's changed flag.

### Parameters

* *name* - name of the variable to check.k

### Description

Tree variables have an associated changed flag.  When either the value or units of measure are changed, this flag is set, and cannot be reset.  This method determines the value of that flag.

The normal use of the flag is to selectively save treevariables in configuration files rather than saving all of them.  This saves time and disk space for large configurations.

### Returns

Non zero if the change flag is true.

### $client treevariableSetChanged

Sets the changed flag.

### Parameters

* *name* - tree variable name.

### Description

Sets the changed flag to true.

### Returns
None.

### $client treevariableFireTraces

Fire traces associated with a tree variable.

### Parameters

* *pattern* (optional) - Only variables with names that match this glob pattern have their traces fired.  If not supplied, the pattern used is ```*``` which matches everything.

### Description

Tcl tree variables are mapped to C++ variables.  The ```treevariableSet``` method changes this underlying C++ variable.  When this is done, traces that might be set on that variable (e.g. by Tk because the variable is a -textvariable for a label) are not fired as Tcl knows nothing of the modification.
  
This method fires write traces for all of the tree variables with names that match the pattern allowing Tcl scripts to become aware of the changes.

### Returns
None

### $client filterCreate

Create an event filter (SpecTcl).

### Parameters

* *name* - name  of the new filter.
* *gate* - Gate that determines which events make it through the filter to its output file.
* *parameters* - List of parameters that will be written to the output file.

### Description

SpecTcl filters allow the rapid re-analysis of data subsets.  Data are subsetted by parameters (only some parameters need be written to a filter) and category (only events that make a gate true are written to a filter).  Filter event files, like parameter files are self-describing and do not need to be decoded by user code.  As the decode process is often the most time expensive part of running SpecTcl, analyzing a filter file, even one that contains the entire data-set is significantly faster than analyzing a raw data file.

This operation creates a new filter defining its name, the condition which must be met to write an event to the filter and a list of parameters that will be written.  Once created a filter must still be [associated with an output file](#client-filterfile) and [enabled](#client-filterenable)for it to write data.

### Returns
None

### $client filterDelete

Delete a filter.

### Parameters

* *name* - name of the filter to delete.

### Description

Deletes a filter.  If the filter is active (enabled and associated with a file), the data are flushed to file and the file closed.

### Returns
None

### $client filterEnable

Enable a filter.

### Parameters

* *name* - Name of the filter.

### Description

Enables a filter to write data.  This is only legal if the filter is already associated with a file.

### Returns
None

### $client filterDisable

Disables a filter

### Parameters

* *name* - name of the filter.

### Description
Disables an filter from writing data.  The filter's pending data are flushed to file and the file closed.  Note that re-enabling the filter will append data.

### Returns
None

### $client filterRegate

Apply a new gate to the filter.

### Parameters

* *name* - Name of the filter.
* *gate* - name of the condition that will gate the filter.  If data analysis are active and the filter is enabled, the gate takes effect with the next event the filter sees.

### Description

Changes the gate that is used to determine which events are filtered into the file.

### Returns
None

### $client filterFile

Associate an output file with the filter.

### Parameters

* *name* - name of the filter.
* *path* - Path to the file to create.

### Description

Changes or sets the file to which the filter will output data.

### Returns
None

### $client filterList

List filters and their properties.

### Parameters

* *pattern* (optional) - Optional pattern.  Filter names must match this glob pattern to be inclued in the listing.  Note that if the pattern is not supplied, it defaults to ```*``` which matches everything.

### Description

Returns a list of filters and their properties for filters with names that match a glob pattern.

### Returns
A list of dicts.  Each dict describes a filter and contains the keys:

* **name** - name of the filter.
* **gate** - Name of the filter's gate.
* **file** - Path to the filter file (this is valid in the context of the server). Empty string if the filter is not yet associated with a file.
* **parameters** - List of parameters that are written to the filter on events that make **gate** true.
* **enabled** - The text ```enabled``` if the filter is enabled.  If the filters is not enabled, the value ```disabled``` is returned.
* **format** - The filter format string.  The only built in format is ```xdr``` however other formats may be added since filter formats are extensible.


### $client filterFormmat
Set the filter output file format.

### Parameters

* *name* - filter name.
* *format* - String selecting the format e.g. ```xdr```

### Description

Selects the format of the filter file.   This must be done when the filter is disabled (not writing data).

### Returns
None

### $client gateList

Lists conditions that are defined.

### Parameters

* *pattern* (optional) - Only gates with names that match this glob pattern are included in the listing. If omitted, ```*```  is used for the pattern which matches everything.

### Description

Lists gates and their properties for the subset of gates that match the glot pattern

### Returns

A list of dicts.  Each dict describes a gate. Note that from Rustogramer, all dict keys are always present but must be ignored or are ```null``` if not relevant to the gate type. SpecTcl may omit dict keys for gate types for which they are not relevant.

* **name** - always present; the gate name
* **type** - always present, the gate type.  See the ```gate``` command in the [SpecTcl command reference](http://https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) for the set of supported types.  Note that Rustogramer gate types are a subset of those supported by SpecTcl.
* **gates** - List of names of gates this gate depends on (e.g. for a ```*``` gate).
* **parameters** - List of names of parameters the gate depends on (e.g. for a ```gs``` gate).
* **points** - List of dicts that describe the points that make up 2-d geometric gates. Each dict contains **x** and **y** keys for the x and y coordinates of the point respectively.
* **low** - Low limit for 1d geometric gates (e.g. ```s``` gates).
* **high** - High limit for 1d geometric gates.


### $client gateDelete

Delete a gate.

### Parameters

* *name* - name of the gate to delete.

### Description

Deletes a gate.  This means different things on SpecTcl vs. Rutogramer:

In SpecTcl, a deleted gate becomes a False gate and is treated accordingly in e.g. compound gates and gated spectra.

In Rustogrammer, gates are actually deleted and
*  The deleted gate is treated as always false in compound gates that depended on it.
*  Spectra that were gated directly on the gate are ungated.

### Returns
Nothing

### gateCreateSimple1D

Create a 1d geometrical gate.

### Parameters

* *name* - name of the gate.
* *gatetype* - type of the gate.
* *parameters* - parameters the gate depends on.
* *low*, *high* - gate limits.

### Description

Creates a slice-like gate.  A slice like gate can currently be either a slice (type ```s```) or gamma slice (type ```gs```) and the caller will get an error if *gatetype* is any other gate type.
Slice-like gates are characterized by a low and high limit that define a region of interest in parameter space within which the gate is considered true.

### Returns
None

### $client gateCreateSimple2D

Create a 2d geometric gate.

### Parameters

* *name* name of the new gate.
* *gatetype* type of gate (see the Description below).
* *xparameters* - List of x parameters.
* *yparameters* - List of y parameters
* *xcoords*    -  list of X coordinates of the points.
* *ycoords*    - list of y coordinates of the points.

### Description

Creates a gate that is a 2-d geometric figure.  There are two types of figures;
* Contours; which are closed regions for which the interior is considered accepted.
* Bands; which are polylines for which below the line is considered accepted.

Different gate types will require different handling of the parameters:

* ```b``` and ```c``` gatse require a single x and a sinle y parameters.
* Gamma gates (```gc``` and ```gb```) require all parameters the gate is checked on to be a list in the *xparameters* parameter.  For these gates *yparameters* are ignored.

### Returns
None

### $client gateCreateMask

Creat a bitmask gate.

### Parameters
* *name* - name of the new gate.
* *gatetype* - Type of the new gate.  See Description.
* *parameter*  - parameter the gate is checked on.
* *mask* - the bit mask.

### Description

Creates a bitmask gate.  Ther are three types of bitmask gates:  ```am```, ```em``` and ```nm```. See the [SpecTcl Command Reference](http://https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) descrption of the ```gate``` command for a description of these gate types.

### Returns
None.

### $client createCompound

Creates a compound gate.

### Parameters

* *name* Gate name.
* *gatetype* Type of gate being made.
* *gates* dependent gate names.

### Description

Creates a gate that depends on other gates.  These are ```*```, ```+````, ```-``` and ```c2band```.
  For a ```-``` gate, only one gate name can be in *gate*.  For a ```g2band``` gate, there must be two dependent gates and they must both be ```b``` gates.

  The ```c2band``` gate takes two bands and joins the first points together as wel as the last to define a contour.

### Returns
None

### $client integrate

### Parameters

*  *name* - name of the spectrum to integrate.
*  *roi* region of interest in which to integrate.  See Description below.

### Description

Performs a 1d or 2d integration of a spectrum within a region of interest.   The *roi* parameter must be one of the following:
*  A dict containing the key **gate** gate name - in which case it must be a slice-like gate for 1d integrations and a contour-like gate for 2ds.
*  A dict containing keys **low** and **high** which are the limits of integration for a 1d integration.
*   A dict containing the keys **xcoords** and **ycoords** in which case these define the x and y coordinates of a contour-like area of interest.

For example

```tcl
$client integrate aspectrum [dict create gate acontour]
$client integrate oned [dict create low 100 high 200]
$client integrate apectrum [dict create xcoords [list 100 200 200] ycoords [list 100 100 400]]
```

### Returns

A dict containing the keys:
* **centroid** - one or two element list with the centroid (coorinates).
* **fwhm** - one or two element list with the FWHM under gaussian peak shape assumptions.
* **counts** - total counts within the AOI.

### $client parameterNew 

Create a new raw parameter and its metadata.

### Parameters
* *name* - name of the new parameter (must not exist)
* *id*   - identifying integer (must not exist).
* *metadata* - dict containing the metadata.

### Description
Note in rustogramer there is no distinction between a treeparameter and a raw parameter.  In SpecTcl, for historic reasons there is.  In SpecTcl, parameters have limited metadata while tree parameters in SpecTcl and Rustogramer have full metadata.

The *id** binds  the parameter to a specific array-like object index in which that parameter should be unpacked.  In SpecTcl this allows user code to explicitly set ```CEvent``` elements in Rustogramer, there are only parameter name/id correspondences and you are better using the tree parameter create as it will assign and id.

The *metadata* is a dict (possibly empty if you do not require/desire metadata).  The possible dict keys are:

* **resolution** - number of bits of resolution the parameter (assumed to be a raw digitizer has).  If the value is *m*, the parameter low is 0 and high 2^m - 1.
*  **low** - low value metadata.  This cannot be used with **resolution**
* **high** - high value metadat.  This cannot be used with **resolution**
** units** - Units of measure metadata.

### Returns

None.

### $client parameterDelete

Deletes a raw parameter.

### Parameters

* *name* - name of the parameter to delete.
* *id*  - id of the parameter to delete.

### Description

Delete a parameter by naem or id.  Only one should be supplied for example:

```tcl
$client parameterDelete george;   # Delete parameter george by name
$client parameterDelete {} 123;   # Delete parameter no. 123 by id.

```

### Returns
None


### $client parameterList 

Lists raw parameter and their metadata.

### Parameters
* *pattern*  - Glob pattern matched against the names to determine which are listied. Defaults to ```*``` which matches everything.
* *id*  - Id of single parameter to list.

### Description
Either the pattern should be provided or the id or neither. Here is an exhaustive list of examples:

```tcl
$client parameterList event*;   # lists all params beginning with "event"
$client parameterList {} 1234;  # Lists parameter id 1234
$client parameterList;          # Lists all parameters.
```

### Returns

List of dicts.  Each dict describes a parameter. Dicts may have the following keys:

*  **name** name of the parameter.
* **id** id of the parameter.
* **resolution** bits of resolution (if that was set or omitted if not).
* **low** low limit if set or omitted if not.
* **high** High limit if set or emitted if not.
* **units** Units of measure of the parameter.  Only present if supplied to the parameter.

### $client pseudoCreate

Creates a pseudo parameter. Pseudo parameter are only supported by SpecTcl.

### Parameters

* *name* - name of the new pseudo parameter
* *parameters* - Parameters that are required to compute the pseudo.
* *body*  - Body of the Tcl proc to use to compute the parameters.  See the ```pseudo``` command in the [SpecTcl Command Reference](http://https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) for a description of this. 

### Description

Creates a new psueod parameter that is computed via the script  in *body* and depends on the *parameters* for its computation.  Only SpecTcl supports pseudo parameters computed via Tcl scripts.

Note SpecTcl does not ensure that all *parameters* are present in the event as it is possible the computation may not always need them all.

### Returns
Nothing

### $client pseudoList

List pseudo parameters and their definitions.

### Parameters

* *pattern* - An optional glob pattern that psuedo names must match to be included in the list.  If not supplied the pattern ```*``` is used which matches everything.

### Description

Returns a listing of pseudo parameters and their properties. The pseudos with name matching *pattern* are returned.

### Returns

List of dicts. Each dict describes a pseudo parameter and has the keys:

* **name** - name of the pseudo.
* **parameters** - list of parameters used by the pseudo. 
* **computation** - A Tcl body that computes the pseudo parameter value for each event.  See the ```pseudo``` command in the [SpecTcl Command Reference](http://https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) for a description of this. 


### $client pseudoDelete

Delete an existing pseudo parameter.

### Parameters

@param *name* - name of the pseudo to delete.

### Description

Deletes a pseudo parameter.  While the parameter will no longer be computed, its definition (use of name and id) will remain.

### Returns
None

### $client sread

Read spectrum from file.

### Parameters

* *filename* - Path, in the context of the server, to the file to read.
* *options*  - Dict describing the options for the read:
    *  **format** Format of the file (defaults to ASCII).
    *  **snapshot** True if the spectrum read is a snapshot (true by default).
    *  **replace** If true any existing spectrum with the same name is replaed (false by default).
    *  **bind** If true, the spectrum read will be bound into display memory (true by default).

### Description

Reads a spectrum from a spectrum definition file.  Valid values for **format** are:
*  ```ascii``` - SpecTcl ASCII format.
*  ```json``` - Rustogramer JSON format (Supported by SpecTcl beginning with 5.13-014)
*  ```binary``` - Deprecated VMS/SMAUG format (SpecTcl only).

Note that while the ```sread``` command has the ability to read several spectra serially from a file (see ```sread``` in the [SpecTcl Command Reference](http://https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html)) the lack of shared state between ReST client anb server for this makes it impossible for ReST clients to do so.

### Returns
Nothing

### $client ringformat

Set the ring format for event file processing.

### Parameters

* *major* - major version of NSCLDAQ that took the data in the source.
* *minor* - minor version of NSCLDAQ that took the data in the source. 

### Description

Data taken by NSCLDAQ are in ring item format, however, there are several payload formats.  The format of the ring item depends, largely on the major version of the NSCLDAQ that read it.
This method sets the ring format.  Note tha the existence of a ring format item in the data itself can override this.

### Returns
None.

### $client scontents

Get the contents of  a spectrum.

### Parameters

* *name* - name of the spectrum.

### Description

Returns information about the spectrum statistics and the non zero channels.  For large 2-axis histograms this can be time/bandwidth expensive both due to the number of spectrum bins to look at in the server and the amount of data that might be returned in a dense spectrum.  If you really need to access spectrum contents at high bandwidt, you should look into the mirroring API as that provides much better bandwidth and much lower latency.

### Returns

A dict is returned. Note, for Rustogramer, all keys are present while for SpecTcl, the keys that don't make sense for the spectrum type (e.g. **yoverflow** for 1d spectrum types) can be omitted.
The dict can have the following keys:

* **xoverflow**  Number of overflow counts on the X axis.
* **xunderflow** Number of underflow counts on the X axis.
* **yoverflow**  Number of overflow counts for the Y axis of a 2d spectrum type.
* **yunderflow** Number of underflow counts for the Y axis of a 2d spectrum type.
* **channels**   List of bin content information dicts.  There will only be entries for bins that have non-zero counts.  Each dict has the keys:
    * **x** - the X bin number.
    * **y** - the Y bins number for 2d spectrum types.  Omitted in SpecTcl for 1d types and shold be ignored for 1d types in Rustogramer.
    * **v** - The value of the bin (number of counts).


The **channels** spectrum dicts have shortened keys to somewhat decrease the bandwidth requirements.  SpecTcl may also send its return value with ```deflate``` Content-Encoding as well to reduce the bandwidth requirements.

### $client shmkey

Return the server's shared memory identifier.

### Parameters
None

### Description
Returns the display shared memory identifier for the server. 

### Returns
A string with any of the following forms:

*  Four characters with no ```:``` This is an SYSV shared memory key.
*  A string that begins with ```sysv:``` and has four more characters; a SYSV shared memory key.
*  A string that begins with ```file:``` the remainder of the string is the path to a file that can be memory mapped to get access to the shared memory.
* A string that begins with ```posix:``` the remainder of the string is a POSIX shared memory identifier that can be accessed via ```shm_open```.


### $client shmemsize

Get display memory size.

### Parameters
None

### Description
Obtains the size of the shared display memory in bytes.


### Returns

Size of shared memory in bytes.

### $client shmupdate_set 

Rustogramer only - sets the shared memory update period.

### Parameters

* *seconds*  - minimum number of seconds between shared memory updates.

### Description

Rustogramer's display bound histograms are not directly incremented in shared memory, unlike SpecTcl.  This returns the update period in seconds.  Note that this is the minimum time between updates as load may stretch it out.;


### Returns
none


### $client shmupdate_get
get the shared memory update period (Rustogramer only).

### Parameters

None

### Description

Returns the shared memory update period.  See also [shm_set](#client-shmupdate_set).

### Returns
Number of seconds between updates of the rustogramer shared display memory.

### $client spectrumList

List spectrum properties.

### Parameters

* *pattern* - optional glob pattern. If provided, to be included in the resonse a spectrum's name must match the pattern. If not provided, the matching pattern defaults to ```*``` which matches all spectra.

### Description

Produces a list of the properties spectra  defined in the server.  Note that spectra do not have to be bound to display memory to be listed.  The *pattern* optional parameter allows the list to be filtered by names that match a glob pattern.

### Returns
List of dicts.  Each dict describes a spectrum.   In rustogramer all keys of the dict are returned but some may have empty values, if they are not relevantt o that type of spectrum.  In SpecTcl, irrelevant keys may be omitted.

Dict keys are:

* **id** - Integer id of the spectrum.  This is not really all that useful.
* **name** - Name of the spectrum.
* **type** - Spectrum types.  See the [SpecTcl command reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) description of the ```spectrum``` command for a list of the valid spectrum type codes.
* **parameter** - List of parameter required by the spectrum.  These are the same as you might provide to a ```spectrum -create``` command to SpecTcl
* **axes** - List of axis definitions.  Each dict has the keys **low**, **high**, and **bins**.
* **chantype**  - Data type of channels in the spectrum.  THis can be one of:
    * *f64* - Rustogramer only - channels are 64 bit floating values.
    * *long* - SpecTcl only, channels contain a 32 bit unsigned integer.
    * *short* - SpecTcl only, channels contain a 16 bit unsigned integer.
    * *byte*  - SpecTcl only, channels contain an 8 bit unsiged integer,
* **gate** - If the spectrum is gated, the name of the gate.

### $client spectrumCreate

Get the server to make a new specttrum.

### Parameters

* *name*  - Name of the new spectrum.  This must not be the name of an existing spectcrum.
* *type* - Spectrum type.  See the [SpecTcl command reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) description of the ```spectrum``` command for a list of the valid spectrum type codes you can use here.
* *parameters*  - List of parameters the spectrum uses. This is in the form used by the SpecTcl ```spectrum -create``` command.
* *axes* List of spectrum axes.  With the exception of summary spectrum types, where this is a Y axis specification, there is always an X axis specification and, if the spectrum is a 2d type, a second axis in the list which is a Y axis specification.
* *opts* - A dict of options.  Not all options are required.
    *  *chantype*  Channel data type of the spectrum.  In SpecTcl, this defaults to *long* but can be *word* or *byte*.  In rustogramer, the channel type is unconditionally *f64*.
    *  *direction* - Needed for *2dmproj* spectrum types.  This is the projection direction.
    *  *roigate* - Optional for *2dmproj* spectrum types.  This can be an contour displayable on an underlying *m2* spectrum type within which the projection is done.

Note that this allows the creation of a *2dmproj* spectrum without the initial creation of the underlying *m2* spectrum which is projected.   If this is done, you'll get a new spectrum with no initial counts but which is incremented as new data come in and which, would be a faithful representation of a projection of a virtual underlying *m2* spectrum.

### Description

Creates a new spectrum.  Note that this also allows the creatino of a *2dmproj* spectrum without requiring a source spectrum from which to make the initial projection.

Spectra created will have no counts but will increment as new data arrive.

### Returns
None

### $client spectrumDelete

Delete a spectrum.

### Parameters
* *name* - name of the spectrum to delete.

### Description
* Deletes a spectrum.  If the spectrum is bound to display memory, resources it used in the display shared memory (the description header and channel soup section it used) are freed for re-use. 

### Returns
None

### $client unbindByName
Undbind spectra from display shared memory given their names.
### Parameters
* *names* - List of names of the spectra to unbind.

### Description

Removes spectra from display shared memory given their names.  This releases all resources used by them in shared memory. This includes its header description and the chunk of the channel soup it consumed.

### Returns
None.

### $client unbindById

Unbinds spectra from display shared memory given their ids.

### Parameters
* *ids* - List of spectrum ids to unbind.

### Description

Removes spectra from display shared memory given their ids.  This releases all resources used by them in shared memory. This includes its header description and the chunk of the channel soup it consumed.

```unbindByName``` should be preferred to this.


### Returns
None.

### $client unbindAll
Unbind all spectra from display shared memory

### Parameters

None

### Returns
None

### $client ungate

Removes gating conditions from spectra.

### Parameters
* *names* - list of names of spectra to ungate.

### Description

For each spectrum name in *names* any gating condition is removed from that spectrum.  

### Returns
None

### $client version

Get server version information.

### Parameters
None

### Description
Returns version information and, possibly program name, for the server. Note that only older SpecTcl does not return the pogram name and therefore the lack of a program name implies a SpecTcl server.


### Returns

A dict containing the following keys:

* **major** - program major version number.
* **minor** - program minor version number.
* **editlevel** - programe edit/patch-level version number.
* **program_name** - may be omitted by older SpecTcl's. The name of the program. For now this can be the strings:
    * ```Rustogramer```
    * ```SpecTcl```

### $client swrite

Write spectra and contents to file.

### Parameters
* *filename* - name of the file to write to.  This path must make sense in the context of the server program.
* *spectra* - list of names of spectra to write to file.
* *format* - Format specifier. This can be:
    * *ascii*  - Simple ASCII format.
    * *json*   - Json ASCII format. was added to SpecTcl in 5.13-014.
    * *binary* - Legacy VMS/SMAUG format which onlyi SpecTcl can write.


### Description

Writes the list of spectra to file in the selected *format*. Note that it is the server that does the actual write and therefore *filename* must make sense in its context rather than the client's.  This point is important in environments where the client and server don't share the same filesystem mappings. A simple FRIB example might be the client running in a container with a different set of ```--bind``` options that the container the server is running in.

### Returns
None

### $client start

Start analysis.

### Parameters
None

### Description
Once an event source is specified for the server, it is still necessary to start analyzing data from that source.  This method asks the server to start analyzing data from it current source.

### Returns
None

### $client stop
Stop analysis.

### Parameters
None

### Description
Stops analyzing data from the current data source. Note the data source is _not_ detached.  This can be problematic for blocking data sources that are adaptors for an online system.  Specifically, stopping analysis in the midst of a run can result in back-pressure flow control that eventually works its way back to the data source halting acquisition.

For NSCLDAQ see the ```--non-blocking``` option in the ringselector application to avoid this problem.  This is decribed in the [NSCLDAQ Documentation](https://docs.nscl.msu.edu/daq/newsite/nscldaq-11.3/index.html).  See the ```1daq``` section of that table of contents and click on the ```ringselector``` link.

### Returns
None.

### $client rootTreeCreate

Create a root tree.

### Parameters

* *name* - name of the root tree.
* *parameterPatterns* - list of glob patterns that specify the parameters that will be booked into the tree.
* *gate* Optional patern.  Specifies a gate that will determine which events are added to the tree.  The default is an empty string which applies a True gate.

### Description
Roottrees are a bit like SpecTcl filters.  They too are only supported by SpecTcl.  Root trees  are file backed Root data files.  The tree created in that file by a root tree is a selected set of decoded parameters for each event.   The selection criterion is an ordinary SpecTcl gate.

Unlike filters, which are only part of the event sink pipeline; RootTrees have feet both in the event processing pipeline and the event sink pipeline.  The event processing pipeline parts are responsible for opening/closing new files as new runs are encountered an the event sink pipeline is responsible for booking tree leaves for each event that satisfies the tree's gate.

### Returns
None

### $client rootTreeDelete
Deletes a root tree

### Parameters
* *name* - name of the tree to delete.

### Description
Flushes and closes the file associated with a root tree and then destroys the tree releasing all resources associated with it.

### Returns
None

### $client rootTreeList
Lists root trees and their properties.

### Parameters
* *pattern*  optional glob pattern.  If provided, only trees with names that match the pattern are included int the list.  If omitted, the pattern is treated as ```*``` which matches everything.

### Description
Lists the properties of all root trees that match the *pattern*.

### Returns
A list of dicts that have the following keys:

* **tree** - name of of the tree.
* **parameters** - list of the parameter patterns that are booked into the tree.
* **gate** - Name of the gate that determines which event are booked into the tree.

### $client pmanCreate

Make an analyss pipeline (SpecTcl only).

### Parameters

* *name* - name of the new analysis pipeline.  This must not be the name of an existing pipeline.  The pipeline will, intially, have no event processors.

### Description
Creates a new, empty event processing pipeline.  A feature of SpecTcl 5 and later is that in addition to the initial, compiled in event analysis pipeline, applications can register event processors, create additional pipelines and switch them in and out as required.

One use case.  An event analysis pipeline consisting of the filter decoder processing element could be created then made current to analyze filter files.  Other use cases might be to have a sigle SpecTcl with event processors registered for all of the detector systems you use with the analysis pipeline composed at startup from them, depending on the actual experiment.

Note that the SpecTcl plug-in capability can also be used to dynamically load and register event processors and pipelines at run time.  See the [SpecTcl programming guide](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/pgmguide/index.html) chatper 11.

### Returns
None

### $client pmanList

List the names of all of the event processing pipelines (SpecTcl Only).

### Parameters
* *pattern* - optional pattern.  If supplied, this is a glob pattern. Only names matching the patter will be returned.  If not supplied, the matching pattern defaults o ```*``` which matches everything.

### Description
Lists the names (only) of all of the registered event processing pipelines.  Pipelines can be registered programmatically or at run time via [pmanCreate](#client-pmancreate), or in user C++ code.


### Returns
A Tcl list whose elements are the event processing piplines.  While the current pipeline manager dictionary will spit out the names in alphabetical order, you should not rely on that or any other order.

### $pmanCurrent
List the information about the current event procesing pipeline (SpecTcl only).

### Parameters
None

### Description
The current event processing pipeline is the one that raw events are dispatched to by SpecTcl to be turned into parameters.  This returns details about the current processing pipeline.  Note that the even processing pipeline created in the classical ```MySpecTclApp::CreateAnalysisPipeline``` method is called ```default```

### Returns
A dict that contains:
*  **name** - the name of the current pipeline. The initial pipeline, unless changed by the user's MySpecTclApp implementation is ```default```
* **processors** -Tcl  list of the event names of the event processors in the pipeline.  These names are in registration order and, therefore reflect the order in which they will be invoked to process an event. 


### $client pmanListAll
List all information about event procesing pipelines (SpecTcl only).

### Parameters
* *pattern* - optional glob pattern.  If supplied the names of pipelines must match the pattern to be included in the listing.  IF not provided the pattern matched against is ```*``` which matches everything.

### Description
Provides detailed information about all event processors that match *pattern*.  

### Returns
A Tcl list of Tcl dicts. Each dict contains the following keys:
* **name** - name of the event processing pipeline.
* **processors** - Tcl list of the names of the event processors in the pipeline.  The order of this list will be the order in which processors were added to the pipeline which, in turn, is the order in which the pipeline processors are called.

### $client pmanListEventProcessors

Lists the registered event processors (SpecTcl only).

### Parameters
* *pattern* - Optional glob pattern.  If supplied, processor names must match the pattern to be included.   If omitted, the match pattern defaults to ```*``` which matches everything.

### Description
Returns a list of the event processors that have been registered with the dynamic pipeline subsystem.  These are processors that can be added to event processing pipelines.  As a side note, an event processor, can be registered to more than one pipeline.  

### Returns
Tcl list of event processor names.  You should not rely on these to be in any order.

### $client pmanUse

Select the current event processing pipeline (SpecTcl only)

### Parameters
* *pipeline* - name of the event  processing pipeline to make current.

### Description
Select *pipeline* to be the current event processing pipeline.  The current event processing pipeline is the one that SpecTcl will hand events to for processing into parameters.  There can only be one current event processing pipeline.  If this succeeds, the previous current pipeline is available for future use (to be made current) but is not invoked for events.

### Returns
None

### $client pmanAdd
Adds an event processor to an event processing pipeline (SpecTcl only).

### Parameters
* *pipeline* - name of the event processing pipeline.
* *processor* - name of the event processor to add.

### Description

Event processing pipelines are made up of an ordered list of event processors.  This method appends an event processor to the end of the list of event processors that make up a pipeline.  If or when the event processor  is made current, this implies tha the event processor will be invoked to process events fromt he data source.

### Returns
None.

### $client pmanRemove

Removes an event processor from an pipeline (SpecTcl only).

### Parameters
* *pipeline* - name of the pipeline to be edited.
* *processor* - name of the event processor to remove from the pipeline.


### $client mirror

Return a list of the mirrors that are currently being served.

### Parameters

* *pattern* - Optional, if supplied this is a glob pattern.  The mirrors listed must have hosts that match *pattern*. If *pattern* is omitted, it defaults to ```*```

### Description

Both SpecTcl and Rustogramer can serve their display shared memories via a mirror server.  On linux the clients are smart enough to know if a specific host is already mirroring to a local shared memory and just attach to the mirror.

This service provides a list of the mirrors active on the host to support exactly that sort of intelligence.

### Returns

A list of dicts that describe the active mirror clients.  Each dict contains the following keys:

* **host** - A host which is mirroring.
* **memory** - Identifies the shared memory local to the **host** in which the mirror is being maintained.  This has the forms:
    *  Four characters - the key to a SYSV shared memory segment.
    *  ```sysv:`` followed by four characters, the four characters are a SYSV memory key.
    *  ```posix:``` followed by a string.  The string is A POSIX shared memory file.
    *  ```file:``` followed by a stsring.  The string is a path to a file that is the backing store for the shared memory.  A ```mmap(2)``` of that file will, if permissions allow, gain access to that memory.

### Description
Removes the named event processor from the named event processinug pipeline.


### Returns
None.

### $client pmanClear
Removes all event processors from a pipeline (SpecTcl only).

### Parameters
* *pipeline* - name of the pipeline to clear

### Description
Removes all event processors fromt he named pipeline.  Once this is done, if the pipeline were current, it would do nothing.

### Returns
None

### $client pmanClone
Create a copy of an existing pipeline (SpecTcl only).

### Parameters
* *source* - existing pipeline to clone.
* *new* - Name of the new pipeline to create.

### Description
  You can think of this request as a way of using an existing event processing pipeline as a starting point for the create of a new pipeline.  
  
  For example, suppose you have a pipeline named
  ```raw``` that has event processors  that create raw parameters from the detector data in the event.  Suppose further that you have event processors that create computed parameters from the raw parameters, and that during setup, you need to understand how to set treevariables to properly parameterize *those* event processors.  You could make a new pipeline named ```raw+computed``` by cloning ```raw``` and adding the computational event processors to ```raw+computed```.  

  You could, during setup, make ```raw``` current and then, once the treevariables are set, make ```raw+computed``` current at which time SpecTcl will populate the computed values.
### Returns
None

### $client evbCreate
Create an event builder event processor (SpecTcl only).

### Parameters
* *name* - name of the new event processor.  This must be unique amongst event processors.
* *frequency* - The timestamp clock frequency in floating point MHz.  This is used to create some time evolution diagnostic parameters.
* *basename* - Textual base name for the diagnostic parameters.

### Description
To understand the event builder interfaces, see the [SpecTcl program reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/pgmref/index.html) description of ```CEventBuilderEventProcessor``` as these are the objects this set of interfaces manipulate.  See also the ```evbunpack``` command in the [SpecTcl command reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) as that's the underlying command that these requests will invoke.

This request creates a new event processor and registers it. The event processor itself is a ```CEventBuilderEventProcesssor```.  That processor understands how to iterate over the fragments in an event and dispatch to event processors registered to handle each expected source id.
The new event processor has no registered source id handlers when created. 

The idea is that you can use this to unpack raw data from event built data, as e.g. the first element of some event processing pipeline that is made current.

### Returns
Nothing.

### $client evbAdd

Register an event processor to handle a source id for a an event builder unpacker (SpecTcl only)

### Parameters

* *name*  - name of the event build event processor created with [evbCreate](#client-evbcreate) above.
* *source* - Integer source id the processor will handle.
* *processor* - Name of an event processor that will handle that source id.

### Description
The event builder unpackers managed by this API subset have an event processor registered for each source id that is expected from the raw data.   Note that for hierarchical event building, nothing stops you from using another event built event processor for the *processor* parameter.

This method associates an event processor, *processor* with fragments from the source id *source*.  If there already was one for that source, it is no longer associated with that source.
It is important to note that in spite of some mis-naming and inconsistencies in the source-code, *processor* is a processor not a pipeline.  This is usually not a problem because:

*  Usually a single processor can make the raw data for the pipeline and additional processors after the event built data processor can compute from the resulting raw unpacking.
*  It is pretty trivial to have an event processor that, itself, implements a pipeline of event processors which could be registered if needed.

### Returns
None

### $client evbList

Lists the event builder event processors (SpecTcl only).

### Parameters
* *pattern* - Optional glob pattern which the name of an evb event processor must match to be listed.  If omitted the matching pattern defaults to ```*``` which matches everything.

### Description
Lists the event builder event processors (Those reated via [evbCreate](#client-evbcreate)) with names that match the pattern.

### Returns
List of strings that are the evb event processor names.

### $client command

Execute a Tcl command in the server's interpreter (SpecTcl only)

### Parameters
* *script* - The script to execute.

### Description

Executes a script in the server's interpreter.  

### Returns
The result of the command.

### $client getVars

Return informational variables.

### Parameters
None

### Description
SpecTcl holds some useful information in Tcl variables.  Rustogramer has muh of the same information available (in Rust data/variables).  This method returns these variables.

### Returns
A dict with the following keys:


* **Displaymegabytes** (unsigned) - Megabytes of shared memory spectrum storage.
* **OnlineState** (bool) - set ``true`` by some SpecTcl scripts that use ```attach -pipe``` to attach to the online DAQ system.  Rustogramer sets this to ```false```
* **EventListSize** - The size of the event batch.  For SpecTcl this is the number of decoded events sent on each histogramming operation. For Rustogramer, the number of event ring items sent to the histogram thread in each operation.
* **ParameterCount** (unsigned/string)- In SpecTcl, this is the initial size used for ```CEvent``` objects, while for Rusgtogramer this is the value "-undefined-"
* **SpecTclHome** (string) - SpecTcl - the top level of the installation directory tree. for Rustogramer, this is the directory in which the executable was installed.
* **LastSequence** (unsigned/string) - Number of ring items processed in the most recent run for SpecTcl, for Rustogramer, this is "--undefined-"
* **RunNumber** (unsigned/string) - for SpecTcl, this is the run number of the most recently seen state change ring item.  For rustogramer this is "-undefined-"
* **RunState** (int/string) - For SpecTcl this is nonzero if analysis is active or zero if not.  For Rustogramer this is "-undefined-".
* **DisplayType** (string) - For SpecTcl this identifies the type of the displayer, e.g. ```qtpy```.  Rustogramer has no integrated displayer so it always returns ```None``` to be consistent with headless SpecTcl.
* **BuffersAnalyzed** (unsigned/string) - The total number of ring items analyzed.  For SpecTcl, taken with **LastSequence** the fraction of events analyzed can be computed.  Rustogramer returns "-undefined-"
* **RunTitle** (string) - Title from the most recent state change item for SpecTcl, "-undefined-" for rustohgramer.

The following statistics attributes are present in SpecTcl but not in Rustogramer:

* **Statistics(EventsRejectedThisRun)** (unsigned) - Number of eevents for which the event processing pipeline returned ```kfFALSE``` in this run.
* **Statistics(RunsAnalyzed)** - Number of times a ```BEGIN_RUN``` ring item was seen when analyzing data.
* **Statistics(EventsAnalyzed)** - Number of events analyzed.
* **Statistics(EventsAccepted)** - Number of events for which the event processing pipline returned ```kfTRUE```
* **Statistics(EventsAnalyzedThisRun)** - Number of events analyzed in the current run.
* **Statistics(EventsRejected)** - Total number of events for which the event processing pipeline returned ```kfFALSE```.
* **Statistics(EventsAcceptedThisRun)** - Number of  events in this run for which the event processing pipeline retunrned ```kfTRUE```

### $client traceEstablish

Establish an interest in obtaining changes to the parameter, spectrum, bindings and gate dictionaries.

### Parameters

* *retention* - minimum retention time for queued trace data.  Note that traces may be retained longer because trace data queues are only pruned when new trace data can be queued.

### Description

Traces in SpecTcl support notifying scripts of events that might require action in e.g. user interfaces.  They are a mechanism to avoid having applications poll for full spectrum, gate, parameter and display shared memory bindings to understand how to update their models of what is going on in SpecTcl (e.g. Tk displays).  ReST interfaces are inherently unable to directly implement the sorts of server notifications that tracing requires.

Therefore, tracing in Rustogramer and SpecTcl works as follows:

1.  A client registers interest in trace data by invoking the traceEstablish request.
2.  Periodically, the client polls for new traces by invoking the [traceFetch](#client-tracefetch) request.
3.  When no longer interested in trace data (e.g. on exit) the client performs a [traceDone](#client-tracedone) request.

Clients that have established an interest in traces are given a token to use when polling for traces and when declaring they are done with traces.  In order to prevent the queued trace data from growing without bound if a client never does a traceDone request or  just does not perform traceFetch requests, the ReST server associates a *retention time* with each client/token.  When new trace data arrives, any queued trace data older than a client's retention time is removed fromt he queue.

Therefore the client should set *retention* to a value significantly larger than it intends to poll for new traces.

### Returns

An integer trace token that should be used to identify iteself when performing
[tracFetch](#client-tracefetch) and [traceDone](#client-tracedone) requests.

Note it is possible for a cilent to outlast a server.  When that happens, the trace token will be invalid and an attemp to do a traceFetch will fail.   What to do at that point depends on the client.  It could re-establish its concept of the server's state and do another traceEstablish or, more usually exit.


### $client traceDone

Mark no longer interested in trace data.

### Parameters
* *token*  - The client token returned from [traceEstablish](#client-traceestablish).

### Description
Indicates the client is no longer interseted in polling for trace data.  The *token* passed in will no longer be valid on return.

### Returns
None

### $client traceFetch
Fetch client trace data

### Parameters
* *token* - the token gotten from [traceEstablish](#client-traceestablish).

### Description

Returns any trace data since the last invokation of traceFetch on this token.  Note that if the time since the last poll was longer than the retention period specified on [traceEstablish](#client-traceestablish) some traces may be lost.

### Returns

A dict containing the following keys:

* **parameter** - array of parameter trace strings.
* **spectrum** - array of spectrum trace strings.
* **gate** - array of gate trace strings.
* **binding** - array of display bindings trace strings.

The value of each element of a trace array is a string the form of the string is:
```
operation target
```

where ```target``` is the name of the object that was affected by the operation (e.g. for spectrum traces a spectrum name)

Valid trace operations for all but **binding** traces are:

*  ```add``` - the object was added.
* ```changed``` - the object was changed (really you'll only see this in gate traces).
* ```delete``` - the named object was deleted.

Bindings traces have these operations:

*  ```add``` the named spectrum was bound into display shared memory.
*  ```remove``` the named spetrum was unbound from display shared memory.

## SpecTcl Command Simulation

Applications that are meant to run locally in the SpecTcl interpreter can also be easily ported to run over the ReST server using the SpecTcl command simulation package. This means that those peograms can also control Rustogramer *if* they stick to the set of commands for which there are functioning Rustogramer ReST services.

This section:
*  Shows how to [start up an application that use  SpecTcl command simulation](#how-to-get-started-with-an-existing-application) and
*  [Describes the set of supported commands](#support-for-spectcl-commands) as well as which ones are not supported by Rustogramer.

### How to get started with an existing application.

This section presents a pair of sample framing scripts that show how you can use the SpecTcl command simulator to wrap existing applications.

Both scripts assume 
*  There is an environment variabla named ```RG_ROOT``` with a value that is the installation directory of Rustogramer. 
*  There is an environment variable named ```RG_HOST``` with a value that is the name of the host on which the server is runing.
*  There is an environment variable named ```RG_REST``` which has the ReST port on which the server is listening for requests.

#### Wrapping an application using ```source```

This wrapping assumes there is a Tcl script whose path is in the environment variable ```CLIENT_SCRIPT``` It
*  Sets up the path to the SpecTclRestCommand package and includes it.
*  Sets up the package to talk to the correct host and port.
*  Starts the application's script using ```source```  In that case, note that the ```argv``` argument list is available to the application

```tcl

#  The package directory depends on the os:
set package_dir $::env(RG_ROOT)
set os $tcl_platform(platform);   # "windows" for windows. unix for linux e.g.

if {$os eq "windows"} {
    set package_dir [file join $package_dir restclients tcl]
} elseif {$os eq "unix"} {
    set package_dir [file join $package_dir share restclients Tcl]
} else {
    error "Unsupported operating system platform:  $os"
}

# Now we can load the package:

lappend auto_path $package_dir
package require SpecTclRestCommand

#  Set up the package:

set host $::env(RG_HOST)
set port $::env(RG_REST)
set debug 0;           # 1 if you want debugging output.


SpecTclCommand::initialize $host $port $debug
maintainVariables 1

#  Start the application:

set application $::env(CLIENT_SCRIPT)
source $application
```


#### Wrapping an application using ```package require```

This wrapping assumes that
*  The application is encapsulated in a Tcl package and that loading the package will start the application.
*  The application's package directory is in the envirionment variable ```APP_DIR```
*  The application's package name is ```APP_PACKAGE```

much of the code below is identical to that of the [previous example](#wrapping-an-application-using-source).

```tcl
#  The package directory depends on the os:
set package_dir $::env(RG_ROOT)
set os $tcl_platform(platform);   # "windows" for windows. unix for linux e.g.

if {$os eq "windows"} {
    set package_dir [file join $package_dir restclients tcl]
} elseif {$os eq "unix"} {
    set package_dir [file join $package_dir share restclients Tcl]
} else {
    error "Unsupported operating system platform:  $os"
}

# Now we can load the package:

lappend auto_path $package_dir
package require SpecTclRestCommand

#  Set up the package:

set host $::env(RG_HOST)
set port $::env(RG_REST)
set debug 0;           # 1 if you want debugging output.

SpecTclCommand::initialize $host $port $debug
maintainVariables 1

#  Start the application:

lappend auto_path $::env(APP_DIR)
package require $::env(APP_PACKAGE)

```

### Support for SpecTcl commands:

If the column labeled ```rustogramer support``` in the table below is empty, full support is available.  Parenthesized notes to the right of a row refer to numbered elements of the list below the table.

```
+--------------------+-----------------+---------------------+
| Command            | Supported       | rustogramer support |
+====================+=================+=====================+
| apply              | Yes             |                     |
| attach             | Yes             | only -file          |
| sbind              | Yes             |                     |
| fit                | Yes             | not supported       |
| fold               | Yes             |                     |
+--------------------+-----------------+---------------------+
| channel            | Yes             |                     |
| clear              | Yes             |                     |
| project            | Yes             |                     |
| specstats          | Yes             |                     |
| treeparameter      | Yes             |                     |
+--------------------+-----------------+---------------------+
| treevariable       | Yes             | not supported       |
| filter             | Yes             | not supported       |
| gate               | Yes             | only rg gate types  |
| integrate          | Yes             |                     |
| parameter          | Yes             |                     |
+--------------------+-----------------+---------------------+
| pseudo             | Yes             | not supported       |
| sread              | Yes             | all but binary fmt  | (1)
| ringformat         | Yes             |                     |
| scontents          | Yes             |                     |
| shmemkey           | Yes             |                     |
+--------------------+-----------------+---------------------+
| spectrum           | Yes             | only rg spec types  |
| unbind             | Yes             |                     |
| ungate             | Yes             |                     |
| version            | Yes             |                     |
| swrite             | Yes             | all but binary fmt  | (1)
+--------------------+-----------------+---------------------+
| start              | Yes             |                     |
| stop               | Yes             |                     |
| roottree           | Yes             | Not supported       |
| pman               | Yes             | Not supported       |
| evbunpack          | Yes             | not supported       |
+--------------------+-----------------+---------------------+
| mirror             | Yes             |                     |
| shmemsize          | Yes             |                     |
| rootexec           | No              |                     | (2)
| isRemote           | Yes             |                     |
| tape               | No              |                     | (3)
+--------------------+-----------------+---------------------+
| ungate             | Yes             |                     |
+--------------------+-----------------+---------------------+

```
Notes:
1.  The sread command over the ReST interface does not support doing an sread from a file descriptor that was opened by the client side script.
2.  See the execCommand proc however to get SpecTcl to do that.
3.  This command is deprecated in SpecTcl.

The proc ```maintainVariables``` fetches the current values of the spectcl variables.
This requires an event loop such as Tk applications have or the ```vwait``` command runs for the duration of the ```vwait```
If you want to be sure that you have the current values of the SpecTcl variables;  invoke 
```updateVariables```.  

#### Traces 

The SpecTcl command simulator doe support all tracing.  The first time traces re requrested, the package informs the ReST server of its interest in traces.  It then starts an re-triggered timed proc that fetches and dispatches any traces.  All of this requires an event loop which you get in a Tk application and for the duration of a ```vwait```command.