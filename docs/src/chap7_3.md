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


## SpecTcl Command Simulation



