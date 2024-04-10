# Python REST reference

Rustogramer also provides a Python ReST client.  This is an object oriented wrapper for the ReST requests supported by Rustogramer and SpecTcl.  In fact, the 
[GUI](./chapter_4.md) uses this ReST client library.


This section provides:

* Recipies for [Importing the ReST client](#importing-the-client) on Linux and windows.
* [Reference for the RustogramerException](#rustogramerexception-reference) exception class.
* [Reference for the rustogramer](#rustogramer-client-reference) client class.


## Importing the client

The issue to consider for being able to import the Python ReST is how to set up the import path given the installation base of the Rustogrammer package.  This is because the winddows and linux installer install these in different subdirectories.  Here is sample code that should work in both Windows and Linux to locate and import both the [RustogramerException](#rustogramerexception-reference) exception class and the [client class](#rustogramer-client-reference):


The code below assumes the environment variable RG_ROOT contains the top level installation directory for rustogramer.

```python
import os
import sys

linux_subdir   = "/share/restclients/Python"

rg_root = os.getenv("RG_ROOT")                  # 1
if rg_root is None:
    print("You must define the environment variable 'RG_ROOT'")
    exit()

if os.name == "nt":                             # 2
    import_path = os.path.join(rg_root, 'restclients', 'python')
elif os.name == "posix":
    import_path = os.path.join(rg_root, 'share', 'restclients', 'python')
else:
    print("Unsupported platform: ", os.name)

sys.path.append(import_path)                  # 3

from rustogramer_client import RustogramerException, rustogramer  # 4


```
The numbers in the explanatory text below refer to the numbered comments in the code fragment above.

1. This code fetches the definition of the environment variable ```RG_ROOT``` which is the top-level installation directory for Rustogramer.
2. Depending on the operating system platform, ```nt``` for windows and ```posix``` for unix/linux systems, the correct full import path is computed as the variable ```import_path```
3. The correct import path is added to the import library search list.
4. The rustogramer_client library elements are imported into the script.

## RustogramerException Reference

If an error is detected performing a transaction with the server, the rustogramer client will 
raise a ```RustogramerException```  this method is dervived from ```Exception```.  It includes an implemenation of the ```str``` method which allows it to be printable.  For example:

```python
< Code from the previous section to import the libraries: >

client = rustogramer({"host":"localhost", "port":8000})
try:
    version = client.get_version()
    ...
except RustogramerException as e:
    print("Interaction with the server failed:" e)
    exit(-1)
```

## Rustogramer Client Reference

The ```rustogramer_client.rustogramer``` class is the client for rustogramer's ReST interface.  Instantiating it provides a client object.  Invoking the methods of that object results in transactions.  Failing transactions raise a [RustogramerException](#rustogramerexception-reference) which, if not caught results in program termination.

* ```debug```The rustogramer class provides this class variable to turn on debugging.  This is initialized to ```False``` if set to be True, the class will output the URIs of the requests it makes. For example

```python
< stuff needed to import rustogramer >
rustogramer.debug = True    # I want debugging output.
```

Below we describe the clent methods.  Note that all methods also have docstrings in the code so you can interactively get information about them from the Python ```help``` function e.g.:

```
help(rustogramer.condition_list)
Help on function condition_list in module rustogramer_client:

condition_list(self, pattern='*')
    Returns a list of defined conitions.  Conditions returned must
    have names that match the optional pattern parameter which is a glob pattern.
    If the pattern is omitted, it defaults to "*" which matches all gates.
```

### __init__ (constructor)
#### Description 
Constructs a new instance of the client object.  Note that the connection to the server is not tested.  Only performing actions on the server result in connections to the server as ReST is a single transaction protocol at that level.

#### Parameters
*  ```connection``` (dict)- This is a dict that decribes how the connection to the server will be done.  The keys determine how the connection is done and where:
    *  **host** (string) - Required - The host on which the server is running. This can be the DNS name of the system or a dotted IP address.
    * **port** (unsigned integer) - If using explicit port numbers the value of this key shoulid be the port number.
    * **service** (string) - if using NSCLDAQ service lookups, this is the name of the service.  In that case, **port** should not be present and **pmanport** must be provided.
    * **pmanport** (unsigned integer) - the port on which the NSCLDAQ port manager is listening. If service lookup is being used, this must be present. Normally, this will have the value ```30000```
    * **user** (string) - If using NSLCDAQ service lookups and a user other than the user you are running under registered **service** this should be the username of the user that did.

#### Returns

An instance of a ```rustogramer``` class.  Methods on this object can be called to perform operations with the server.  In general, those operations will return a dict that has keys **status** and **detail**  note that if **status** was not ```OK``` a ```RustogramerException``` will be raised. The useful information will be in the value of the **detail** key.

### apply_gate
#### Description
Applies a gate to one or more spectra.  The gate and spectrum must, of course already be defined.
#### Parameters
* *gate_name*  (string)- Name of the gate to apply.
* *spectrum_name* (string or iterable of strings) - If a single string, this is the name of the one spectrum to which *gate_name* will be applied.  If an iterable of strings, this will be e.g. a list of the names of the spectra to which the gate will be applied.
#### Returns
 The **detail** key of the the returned dict will have nothing.

### apply_list
#### Description
   Returns a list of gate applications.
#### Parameters
* *pattern* (Optional string defaults to ```*```) - A pattern that spectrum names must match to be inclded in the list.

#### Returns
The **detail** key of the returned dict is an iterable that contains dicts with the following keys:

* **spectrum** (string)- name of a spectrum.
* **gate**  (string)- Name of the gate applied to that spectrum.

### ungate_spectrum
#### Description

Remove any gate from one or more spectra.

#### Parameters
* names (string or iterable of strings) - If a single string, the spectrum with that name will be ungated.  If an iterable, all of the named spectra in the iterable will be ungated.

#### Returns

**detail** has nothing useful.


### get_chan
#### Description

Get the value of a spectrum channel.

#### Parameters
* *name* (string) - name of the specturm.
* *x*    (number) - X channel.
* *y*    (number, optional) - Y channel, only required if the spectrum has two axes.

#### Returns

**detail** contains a number  which is the number of counts in the specified bin of the spectrum.

### set_chan
#### Description
Sets the contents of a spectrum bin to the desired value.


#### Parameters
* *name* (string) - name of the specturm.
* *x*    (number) - X channel.
* *value* (number) - counts to set in the desired channel
* *y*    (number, optional) - Y channel, only required if the spectrum has two axes.


#### Returns

**detail** contains nothing useful.


### attach_source
#### Description
Attach a data source for analysis.  Note:

*  If a data source is attached it may be detached even if this fails.  
*  Once a data source is attached, analysis must be explicitly started.
*  Rustogramer only supports file data sources while SpecTcl supports file and pipe data sources.  See Parameters below.

#### Parameters

*  *type* (string) the type of data source.  This can be either ```file``` or ```pipe```. 
*  *source* (string) the source for that type:
    *  If the source is ```file``` this must be the path to that file in the context of the server.
    *  If the source is ```pipe``` this must be the program invocation line to run on the other end of the pipe. Note that:
        * PATH is in the context of the server.  
        * The program will not have a shell. 
        * The program must emit data in the format expected by the server to its stdout as that will be connected to the write end of the pipe while the server will be connected to the read end.
* *format* (optional string) - THe format of data produced by the source.  This can be one of:
    * ```ring``` - the default if not supplied.  Data comes from NSCLDAQ ring buffer based systems (NSCLDAQ 10 and later).
    * ```nscl``` - Fixed size  buffers in NSCLDAQ 8 or earlier format.  Only supported by SpecTcl.
    * ```jumbo``` - Fixed sized buffers in NSCLDAQ 8 or later with sizes that can be larger than 64Kbytes. Only supported by SpecTcl.
    * ```filter```  - XDR Filter format. Only supported by SpecTcl.
* *size* (optional unsigned) - Size of the reads done on the data source.  For fixed size block formats (```nscl```, ```jumbo``` and ```filter```), this must be the size of the block in the data.  E.g. for ```nscl``` and ```filter``` this must be ```8192```.  For ```ring``` this can be anything as ring items are properly assembled across block boundaries.  THis is actually ignored by Rustogramer which reads one ring item at a time.


#### Returns
Nothing useful in **detail**

### attach_show
#### Description
Describes what the attached data source is.
#### Parameters
None
#### Returns
**detail** is a string that contains a connection description string.  For example, for a  file data source, this will be something like ```File: <path to filename>``` while for a pipe:
```Pipe: <full program invocation string>```

### detach_source
#### Description
Detaches the current data source.  What this means depends on the server.  Rustogramer does support being detached from a data source while SpecTcl does not, therefore this is implemented by attaching SpecTcl to the file ```/dev/nulll```
#### Parameters
None
#### Returns
**detail** is nothing useful.

### start_analysis
#### Description
Start analyzing data from the current data source. 

 SpecTcl is initially attached to a test data source which supplies ring items that contains fixed size test data.  When "detached", SpecTcl is actually attached to ```/dev/null``` and therefore SpecTcl will immediately see an end file.

Rustogramer,  will return an error if the program is not attached to anything.
#### Parameters
None
#### Returns
**detail** is not useful.

### stop_analysis
#### Description
Stops analysis from any current data source.  If analysis is not active an error is returned.
#### Parameters
None
#### Returns
**detail** as nothing useful.

### set_batch_size
#### Description
Rustogramer only.  The input thread in Rustogramer reads a ring item at a time until a *batch* of ring items have been read. At that point, the entire batch of ring item data are submitted to the histograming thread for processing.

This allows the number of events in a batch to be set.  Larger values are more efficient, but the histogram updates will have higher latencies.  Smaller values, reduce the latency but are lesss efficient.
#### Parameters
* *num_events*   Number of events in a batch.
#### Returns
**detail** contains nothing useful.


### evbunpack_create
#### Description
Creat an event built data unpacker.  This is only supported by SpecTcl and is part of the dynamic event processing pipeline subsystem.  An eventbuilt data unpacker is an event processor that can assign event processors to handle data for fragments from each expected source id.  Unhandled source ids are simply skipped.

#### Parameters
* *name*  (string) - name by which the event processor will be referred.
* *frequency* (float) - The event building clock in MHz.  This is used to produce diagnostic parameters.
* *basename* (string) - The base name from which the diagnostic parameters will be created.  For example, if *basename* is ```evb_diag``` the timestamp in seconds for each event will be called. ```evb_diag.run_time```

#### Returns
**detail** will contain nothing useful.

### evbunpack_add
#### Description
Register an event processor to handle data from a source id.  If one has been registered previously it is replaced.   It is legal to register the same event processor to handle more than one source (though it is up to the processor to know how to use the source id to determine which parameters each source should generate).  Only supported by SpecTcl
#### Parameters
* *name*  (string)  -name of the event built event procesor.
* *source_id* (unsigned) - Source id on which to register.
* *processor_name* (string) - name of the evnt processor that will handle fragments with the *source_id* specifiedl
#### Returns
*detail* has nothing useful.

### evbunpack_list
#### Description
List the event builder unpackers. Only supported by SpecTcl
#### Parameters
None
#### Returns
**detail** is an iterable collection of strings.  Each string the name of an event built data unpacker created via e.g. *evbunpack_create*.

### request_exit
#### Description
As the server to exit.  Currently this is only supported by Rustogramer.  After returning the response to the request, the server will do an orderly shutdown.
#### Parameters
None
#### Returns
**detail** has nothing useful

### filter_new
#### Description
Create a new filter.  This is only implemented in SpecTcl.
#### Parameters
* *name* (string) - name of the filter.
* *gate* (string) - Gate which will select events that are written by the filter.
* *parameters* (iterable string collection) - Names of the parameters that will be written for each event that makes *gate* true.

#### Returns
**detail** is nothing useful.

### filter_delete
#### Description
Destroys a filter.  Once deleted a filter will no longer writte data.  This is only implemented in SpecTcl.
#### Parameters
* *name* (string) - Name of the filter to delete.
#### Returns
**detail** contains nothing useful.

### filter_enable
#### Description
Turns on a filter. A filter can be enabled if it is associated with an output file.  Enabling a filter means that it will write events to the output file beginning with the next evnt that satisfied its gate. Only supported by SpecTcl

#### Parameters
* *name* (string)- name of the filter to enable.
#### Returns
**detail** has nothing useful.

### filter_disable
#### Description
Turns of a filter.  The filter will no longer write data to its output file until it is once more enabled. Only supported by SpecTcl
#### Parameters
* *name* (string) - name of the filter.
#### Returns
**detail** has nothing useful.

### filter_regate
#### Description
Changes the gate that is used to select events the filter will write. Only supported by SpecTcl
#### Parameters
* *name* (string)- name of the filter.
* *gate_name* (string) - Name of the new gate for the filter.
#### Returns
**detail** contains nothing useful.

### filter_setfile
#### Description
Sets the output file for the filter.  Since filters are written by the server, the filename/path must make sense in the server's context. Only supported by SpecTcl

#### Parameters
* *name* (string) - name of the filter.
* *path* (string) - Path to the file to write, in the context of the server.
#### Returns
**detail** contains nothing useful.

### filter_list
#### Description
List the properties of filters. Only supported by SpecTcl
#### Parameters
None
#### Returns
An iterable that contains dictionaries with the following keys:

* **name** (string) - name of the filter.
* **gate** (string) - name of the gate that determines which events are written to the filter file.
* **file** (string) - Path to the file the filter will write.  
* **parameters** (iterable of strings) - Names of parameters that are being written each event.
* **enabled** (string) - If the filter is enabled, this will be ```enabled``` if not, ```disabled```.
* **format** (string) - format of the filter file (e.g. ```xdr```).

### fit_create
#### Description
Creates a new fit object.  SpecTcl only.
#### Parameters
* *name* (string)- name of the fit object
* *spectrum* (string)  - name of the spectrum on which the fit is defined.
* *low*, *high* (floats) - Limits over which the fit will be performed.
* *type* (string) - type of fit to do.  See he documentation of the fit command in the
[SpecTcl command referend](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html
#### Returns
**detail** contains nothing useful.


### fit_update
#### Description 
Recompute set of fits. SpecTcl only.
#### Parameters
* *pattern* (string) - Fits must have names that match the pattern to be updated.  If not provided this defatults to ```*``` which matches all strings.
#### Returns

A list of dicts. Each dict describves a fit an dcontains the following keys:

* **name** (string)- Name of the fit.
* **spectrum** (string)- name of the spectrum on which the fit is computed.
* **type** (string) - type of the fit.
* **low**, **high** (floats) - Limits of the fit.
* **parameters** (dict) - Fit parameters.  The keys depend on the fit type, however fits should provide a **chisquare**  which would hold the goodness of the fit.

### fit_delete
#### Description
Delete a fit (SpecTcl only).
#### Parameters
* *name* (string)- name of the fit to delete.
#### Returns
**detail** has nothing useful.

### fit_proc
#### Description
Returns a Tcl proc that can be evaluated at any point to evaluate the fit (SpecTcl Only).
#### Parameters
* *name* (string) - Name of the fit.
#### Returns

**detail** contains a Tcl procedure which takes a single parameter as an argument.  When called the proc will evaluate the fit at the point passed in.   The proc evaulates the fit only as of the most recent update and is not dynamic (that is if you upate a fit again, you should re-fetch the proc).

### fold_apply
#### Description
Apply a fold to a spectrum.  Folded spectra must be gamma spectra.

#### Parameters
* *fold* (string) - name of the gate used to fold the gamma spectrum.
* *spectrum* (string) - name of the spectrum to fold.

#### Returns
**detail** contains nothing.

### fold_list
#### Description
List the properties of folds that match the pattern.
#### Parameters
* *pattern* (string) - optional pattern that folded spectra must macth to be listed.
#### Returns
**detail** consists of an iterable of dicts.  The dicts have the following keys:
**spectrum** (string) - name of a folded spectrum.
**gate** (string)  - name of the gate folding the spectrum.

### fold_remove
#### Description
Unfolds a previously folded spectrum.
#### Parameters
* *spectrum* (string) - name of the spectrum to unfold.
#### Returns
**detail** returns nothing interesting.


### condition_list
#### Description
Lists the properties of conditions (gates) that have names that match a pattern.

#### Parameters
* *pattern* (string) - Names of conditions listed must match this optional pattern. If not supplied, this defaults to ```*``` which matches all strings.

#### Returns
An iterable containing dicts with the following keys.

* **name** (string) - name of the condition.
* **type** (string) - contidion type. See the ```gate``` command in the [SpecTcl command Reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) for valid type strings.
* **gates** (iterable) - Iterable containing strings which are the gates this condition depends on if it is a compound gate.
* **parameters** (iterable) - Iterable containing strings which are the parameters this condition depends on if it is a primitive gate.
* **points** (iterable) - Iterable containing dicts if the condition is a 2-d geometric shape.  Each dict contains **x** - the X coordinate and **y** the Y coordinate of a point which are floats.  These points define the acceptance region for the condition in accordance with the condition type.
* **low**, **high** (floats) - the low and high limits of the acceptance region of a condition that represents a 1D acceptance region.

### condition_delete
#### Description
Deletes a condition.
#### Parameters
* *name* (string) - name of the gate to delete.
#### Returns
Nothing.



Note that if the server is SpecTcl only the appropriate keys may be present.

### condition_make_true
#### Description
Creates a condition that is always true.
#### Parameters
* *name* (string) - Name of the new condition to create. If there is already a condition with this name it is replaced.
#### Returns
Nothing

### condition_make_false
#### Description
Create a condition that is never true (always false).
#### Parameters
* *name* (string) -Name of the new condition.
#### Returns
Nothing

### conditio_make_not
#### Description
Makes a condition that is the logical inverse of the dependent codition.
#### Parameters
* *name* (string)  - Name of the condition being made.
* *negated* (string) - The name of the condition that will be negated to make this new condition.
#### Returns
Nothing.

### condition_make_or
#### Description
Create an or compound gate.  The condition is true when any component condition is true.

#### Parameters
* *name* (string) - name of the condition.
* *components* (iterable) - Iterable of the condition names that are evaluated to evaluate this condition.
#### Returns
None

### condition_make_and
#### Description
Create an and condition.  This condition is true only if *all* of its component conditions are also otrue.
#### Parameters
* *name* (string) name of the condition.
* *components* (iterable) - Iterable of the condition names that are evaulated.
#### Returns
None




### condition_make_slice
#### Description
Create a 1-d slice gate on a parameter.
#### Parameters
* *name* (string) - name of the slice.
* *parameter* (string) - Parameter that must be inside the slice to make the condition true.
* *low* *high* (floats) - The low and high limits of the slice's region of acceptance.
#### Returns
Nothing

### condition_make_contour
#### Description
Create a closed contour condition that is rue whenver the pair of parameters define a point that is inside the contour.  Insidedness is evaluated using the odd crossing rule:  A point is inside the contour if a line drawn in any direction crosses an odd number of figure boundaries. Note that zero is even for this evaluation.
#### Parameters

* *name* (string)  - name of the contour codition.
* *xparameter* (string) -name of the parameter that will provide the x coordinate of the point.
* *yparameter* (string) _ name of the parameter that will provide the Y coordinate of a point.
*  *coords* (itarable) - Iterable whose values are dicts that have the keys **x** and **y** with values that are floats that are the X and Y coordinates of the figure respectively.
#### Returns
Nothing.

### condition_make_band
#### Description
Crate a band condition.  A band is defined by an ordered set of points in a 2d parameter space.  The condition is true for points that are below the polyline defined by the points.  If the polyline backtracks the higher of the two line segments in a region defines the band.
#### Parameters
* *name* (string) - Name of the band condition.
* *xparameter* (string) - name of the parameter that contributes the x coordinate of the event.
* *yparameter* (string) -name of the parameter that contributes the y coordinate of the vent.
* *coords* (interable) - iterable of dicts that contain the keys:  **x** and **y** which provide x and y floating point coordinates for the band's polyline.
#### Returns
Nothing

### condition_make_gamma_slice
#### Description
Create a gamma slice. A gamma slice is like a slice, however there are an unbounded number of parameters. The slice is true if any of them make the slice true.  You can, therefore, think of a gamma slice as  the or of identical slices on all of the paramters in the gamma slice.  These slices are also useful as folds.
#### Parameters
* *name* (string) - Name of the conditionn.
* *parameters* (iterable) - each iteration produces a string that is the name of a parameter the condition is checked against.
* *low*, *high* (floats) - the limits that define the acceptance region for the condition.

#### Returns
nothing

### condition _make_gamma_contour
#### Description
Creates a gamma contour on a set of parameters.  A gamma contouur is like the OR of identical contours defined on all pairs of parmeters as both X and Y parameters.
#### Parameters
* *name* (string) - name of the condition.
* *parameters* (iterable) - Contains the names of the parameters the contour will be evaluated on.
* *points* (iterable) - Contains the points as dicts with the keys **x** and **y** where each coordinate in the point is a floating point value.
#### Returns
Nothing

### condition_make_gamma_band
#### Description
Same as a gamma contour, however the ponts define a band not a contour.
#### Parameters
* *name* (string) - name of the condition.
* *parameters* (iterable) - Contains the names of the parameters the band will be evaluated on.
* *points* (iterable) - Contains the points as dicts with the keys **x** and **y** where each coordinate in the point is a floating point value.
#### Returns
Nothing


### condition_make_mask_equal
#### Description
Makes a condition that is true if the parameter taken as an integer is identical to the mask.
#### Parameters
* *name* (string) - name of the condition.
* *parameter* (string) - parameter to evaluate the condition.
* *value* (unsigned) - Integer value of the mask.k
#### Returns
Nothing


### condition_make_mask_and
#### Description
Makes a condition that is true if the parameter taken as an integer is identical to the mask.
#### Parameters
* *name* (string) - name of the condition.
* *parameter* (string) - parameter to evaluate the condition.
* *value* (unsigned) - Integer value of the maskk
#### Returns
Nothing

### condition_make_mask_equal
#### Description
Makes a condition that is true if the parameter taken as an integer that when bitwise *and*ed with the parameter is identical to the mask.
#### Parameters
* *name* (string) - name of the condition.
* *parameter* (string) - parameter to evaluate the condition.
* *value* (unsigned) - Integer value of the mask.k
#### Returns
Nothing


### condition_make_mask_nand
#### Description
Makes a condition that is true if the parameter taken as an integer is equal to the bitwise inverse of the mask.
#### Parameters
* *name* (string) - name of the condition.
* *parameter* (string) - parameter to evaluate the condition.
* *value* (unsigned) - Integer value of the mask.k
#### Returns
Nothing

### get_statistics
#### Description
Return spectrum overflow/underflow statistics.
#### Parameters
* *pattern* (string) - Optional pattern. Spectra with names that match the pattern are returned.  Note that if the pattern is omitted, it defaults to ```*``` which matches all names.
#### Returns
**detail** contains an iterable containing dicts that provide the statistics for spectra.   Each dict has the following keys:

* **name** (string) - Spectrum name
* **underflows** (iterable) - 1 or 2 element iterable with integers that are the number of underflows for first the X axis.
* **overflows** (iterable) - 1 or 2 element iterable with integers that are the numbe rof overflows for first the X axis and then the Y axis.

Note that for Rustogramer, both elements are always present, but the second one is always 0 for spectra with only one axis. SpecTcl omits the second axis if it does not exist.

By underflow and overflow, we mean the number of events that would have been to the left or below the histogram origin (underflow) or to the right or above the end of the axis.


### integrate_1d
#### Description
Integrate a 1-d spectrum.  Note that this method does not directly support integrating a slice condition.  To do that, you must fetch the slice definition and extract its limits.

#### Parameters
* *spectrum* (string) - name of the spectrum which must have only one axis.
* *low* (float) - low cut off for the integration.
* *high* (flost) - high cut off for the integration.


#### Returns

**detail** is a dict containing the keys:

* **centroid**  - The centroid of the integration. For Rustogramer this is an iterable containing one element while for SpecTcl it is a float.  See below.
* **fwhm** - The full width at half maximum under gaussian line shape assumptions.  Same type as centroid
* **counts** (unsigned) - total number of counts in the region of integration.

To unpack **centroid** and **fwhm** the function below is useful:

```python
def get_value(value):
    try:
        for v in value:
            return v
    except TypeError:
        return value

```

If ```value``` is iterable, this method returns the first element of the iteration, otherwise it just returns the value.  This function can be used to extract data from a 1d integration as shown below;

```python
...

result = client.integrate_1d(sname, low, high)
centroid = get_value(result['detail']['centroid'])
fwhm     = get_value(result['detail']['fwhm'])
counts   = result['detail']['counts']
...
```

### integrate_2d
#### Description
Performs an integration of a spectrum with 2 axes.  Note that this method does not support integration within a contour. To do that you will need to fetch the definition of the contour and supply its coordinates to ```integrate_2d```
#### Parameters
* *spectrum* (string)- Name of the specrum to integrate.
* *coords* (iterable) - The coordinates of integration.  Each element is a dict that has the keys **X** and **y** which are the x and y coordinates of a contour point respectively.

#### Returns
**detail** contains a dict with the keys:

* **centroid**  (iterable)- Two items. The centroid of the integration. The first element is the X coordinate of the centroid, the second element is the Y coordinate of the centroid.
* **fwhm** - The full width at half maximum under gaussian line shape assumptions.  Same type as centroid
* **counts** (unsigned) - total number of counts in the region of integration.


### parameter_list
#### Description
Describes tree parameters and their metadata.  Not that rustogramer considers all parameters to be tree parameters.  This is not true for SpecTcl
#### Parameters

* *pattern* (string) - Optional glob pattern.  The listing is  limited to parameters with names that match the pattern.  If not
supplied, this defaults to ```*``` which matches anything.

#### Returns
**detail** is an iterable containing dicts.  Each dict describes a parameter and has the following keys:

* **name** (string) - name of the parameter.
* **id** (unsigned) - integer assigned to the parameter.  This value is used by the histogramer functions in both SpecTcl and Rustogramer, and is not generally relevant.
* **bins** (unsigned > 0) - Number of bins recommended for spectrum axes on this parameter.
* **low**, **high** (floats) - Recommended low and high limits for axes on this parameter.
* **units** (string) - documents the parameter's units of measure.
* **description** (string) - Rustogramer only.  Reserved for future use in which it will be a description of the parameter for documentation purposes.

### parameter_version
#### Description
Return the tree parameter version.  Differing versions of the treee parameter subsystem have somewhat different capabilities.  This returns a version string that gives the tree parameter version of  the server.
#### Parameters
None
#### Returns
**detail** is a version string e.g. "2.1"


### parameter_create
#### Description
Create a new parameter with metadata.  Note that the metadata are passed as a dict where only the keys for the desired metadata need be supplied.
#### Parameters
* *name* (string) - name of the parameter. Cannot be the name of an existing parameter.
* *poperties* (dict) - Dict of the desired metadata.  You only need to supply keys for metadata for which you want to override the defaults.  The defaults are chosen to be close to SpecTcl/treeGUI default metadata for axes.  The following keys in this dict are used (if present) to set metadata
    * **low** (float) - Suggested low axis limit.  Defaults to 0.0 if not provided.
    * **high** (float) - Suggested high axis limit.  Defaults to 100.0 if not provided.
    * **bins** (unsigned > 0) - Suggested axis binning. Defaults to 100 if not provided.
    * **units** (string) - Units of measure. Defaults to "" if not provided.
    * **description** (string) - Rustogramer only.  A description that documents the purpose of the parameter.  Defaults to "" if not provided.
#### Returns
Nothing useful in **detail** on success.

### parameter_modify
#### Description
Modify the metadata for an existing parameter.  
#### Parameters
* *name* (string) - name of the parameter.
* *properties* (dict) - Properties to modify.  See [parameter_create](#parameter_create) for a description of this parameter.

#### Returns
**detail** has nothing useful.

### parameter_promote
#### Description
Given a raw parameter promotes it to a tree parameter.  This is only meaningful in SpecTcl as all Rustogramer parameters are tree parameters.
#### Parameters
* *name* (string) - Name of the parameter.
* *properties* (dict) - Metadata for the parameter.  See [parameter_create](#parameter_create) for a description of the metadata keys.
#### Returns
Nothing useful.

### parameter_check
#### Description
Returns the check flag for a parameter.  If a parameter's metadata has bee modified, the check flag is set.  This is so that when saving state one can limit the parameter state saved to only those parameters whose definitions have changed at run-time.

See also [parameter_uncheck](#parameter_uncheck)

#### Parameters
* *name* -name of the parameter to fetch the check flag for.
#### Returns

**detail** is an integer that is non-zero if the check fla was set.

### parameter_uncheck
#### Description
Unsets the parameter's check flag.  See [parameter_check](#parameter_check) for a description of this flag.
#### Parameters
* *name* (string) - name of the parameter to modify.
#### Returns
Nothing useful.

### rawparameter_create
#### Description
Create a raw parameter.  This is really different from [parameter_create](#parameter_create) only for SpecTcl.  Creates a parameter definition and metadata for a parameter that is *not* a tree parameter.  For SpecTcl, this is an important distinction because these parameters:
*  Are invisible to [parameter_list](#parameter_list), however see [rawparameter_list_by_name](#rawparameter_list_by_name) below.
*  Cannot be bound via a ```CTreeParameter``` object but only accessed programmatically via an index in ```CEvent```.


#### Parameters
* *name* (string) - Name of the parameter to create, must not be used.
* *properties* (dict) - properties that contain the proposed metadata for the parameter.  If a key is not provided, that metadata will not be defined for the parameter.
    * **number** (unsigned) - parameter id - this is required.
    * **resolution** (unsigned) - This is recommended for parameters that are raw digitizer values.  It represents the number of bits in the digitizer and implies setting the **low** and **high** metadata below.
    * **low** (float) - recommended low limit for axes on this parameter.
    * **high** (float) - recommended high limit for axes on this parameter.
    * **units** (string) - Units of measure.

Note that you should use either **resolution** *or* **low** and **high** but no both.

#### Returns
Nothing useful.

### rawparamter_list_byname
#### Description
Given a pattern, list the raw parameters that match that pattrern and their properties.
#### Parameters
* *pattern* (string) - Optional glob pattern that filters the returned to only the parameters that match the pattern.  If omitted, this defaults to ```*``` which maches everything.

#### Returns
**detail** is an iterable that contains dicts.  The dicts have the following keys.  Keys are only present in SpecTcl if the corresponding metadata was provided for the parameter.  In Rustogramer, the missing keys are there but have the ```null``` value.

* **name** (string) - name of the parameter.
* **id** (unsigned) - Parmaeter id (set with the **number** metadata).
* **resolution**(unsigned > 0) -  Only present if the **resolution** metadata was set.
* **low**, **high** (floats) - recommended axis limits for this parameter.
* **units** (string) - Units of measure.



### rawparameter_list_byid
#### Description
List the properties of a parameter given it sid.
#### Parameters
* *id*  The parameter id of the desired paramter.
#### Returns
Same as for [rawparameter_list_byname](#rawparamter_list_byname).



### ringformat_set
#### Description
This should be used in conjunction with the attach method to specify a default ringbuffer format.  Prior to starting analysis.  If unspecified, the format is determined by SpecTcl in the following way:

*  If a ```RING_FORMAT``` item is encountered, it sets the data format.
*  If the ring version was specified but no ```RING_FORMAT``` was specified, that ring version will be used.
*  IF all else the ring format will default:
    *  Prior to SpecTcl 5.14 to 10.0
    *  With SpecTcl 5.14 and later to 11.0

#### Parameters
* *major* (unsigned) -Major version number.
#### Returns
Nothing useful

### ringformat_get
#### Description
Rustogramer only - queries the default ring format.
#### Parameters
None
#### Returns
**detail** is a dict that has the keys

* **major** (unsigned) - major version of the ring format.
* **minor** (unsigned) - minor version of the ring format (always ```0```).

### sbind_all
#### Description
Attempts to bind all spectra to the display shared memory.  This can only fail if either:
*  There are more spectra than there are spectrum description headers.
*  The channel soup part of the display shared memory  is not large ennough to accomodate all of the spectra.
#### Parameters
None
#### Returns
None.

### sbind_spectra
#### Description
Bind selected spectra to shared memory.
#### Parameters
* *spectra* (iterable) - Iterable of spectrum names that should be bound.
#### Returns
None.

### sbind_list
#### Description
List bound spectra and their bindings for spectra with names that match a pattern.
#### Parameters
* *pattern* (string) - Optional glob pattern. Only bound spectra with names that match *pattern* are listed. Note that if this is omitted the pattern defaults to ```*``` which matchs everything.
#### Returns
**detail** is an iterable containing dicts.  The dicts have the following keys:
* **spectrumid** (unsigned) - Useless integer.
* **name**  (string) - name of the spectrum.
* **binding** (unsigned) - Spectrum descriptor slot number that was assigned to the spectrum in the display shared memory.

### sbind_set_update_period
#### Description
Rustogramer only.  SpecTcl spectra are incremented directly into shared memory.  The histogram package used by rustogramer does not support this.  Therefore, it is necessary to periodically update the shared memory contents.  This method sets the time between these updates.
#### Parameters
* *seconds* (unsigned > 0) - number of seconds between updates.
#### Returns
None

### sbind_get_update_period
#### Description
Rustogramer only. Return the spectrum shared memory refresh peiord.
See [sbind_set_update_period](#sbind_set_update_period) for a description of this value.  
#### Parameters
None
#### Returns
**detail** is an unsigned integer number of seconds between shared memory refreshes.

### unbind_by_names
#### Description
Removes a set of spectra from display shared memory given their names.  This is the preferred method.
#### Parameters
* *names* - iterable containing the names of the spectra to unbind.
#### Returns
Nothing useful.

### unbind_by_ids
#### Description
Unbinds a set of spectra from the display shared memory given their spctrum is.  It is preferred to use
[unbind_by_names](#unbind_by_names).
#### Parameters
* *ids* (iterable) - iterable containing the ids of the spectra to unbind.  
#### Returns
Nothing useful.

### unbind_all
#### Description
Remove all spectra from the display shared memory.
#### Parameters
none
#### Returns
none..

### shmem_getkey
#### Description
Returns the display shared memory identification.
#### Parameters
None
#### Returns
**detail** is a string.  The string has one of the following forms:

* Four character string - this is the SYSV shared memory key value.
* The text ```sysv:``` followed by a four character string. The four character string is the SYSV shared memory key.  The shared memory can be mapped using ```shmget(2)``` to return the shared memory id followed by ```shmat(2)``` to do the actual mapping.
* The text ```file:``` followed by a string.  The string is the path to a file which can be mapped using mmap(2).
* The text ```posix:``` folllowed by a  string. The string is the name of a posix shared memory region that can be mapped via ```shm_open```


### shmem_getsize
#### Description
Return the number of bytes in the spectrum shared memory
#### Parameters
None
#### Returns
**detail** is an unsigned total number of bytes (specctrum header storage and channel sopu) in the Display shared memory. This can be used for the shared memory size parameter required by all of the mapping methods

### shmem_getvariables
#### Description
Return some SpecTcl variables or their Rustogramer equivalets.
#### Parameters
None
#### Returns
**detail** is a dict containing.

*  **DisplayMegabytes** (unsigned)  - The number of 1024*1024 bytes in the shared memory
spectrum pool.
*  **OnlineState** (boolean) - True if connected to an online data source.
*  **EventListSize** (unsigned > 0) - Number of events in each processing batch.
*  **ParameterCount** (unsigned) - Number of parameters in the initial flattened
event.
*  **SpecTclHome** (String) - the top-level directory of the installation tree.
*  **LastSequence** (unsigned) - Sequence number of the most recently processed
data
*  **RunNumber** (unsigned) - run number of the run being processed.
*  **RunState** (string) - "Active" if processing is underway, "Inactive" if not.
*  **DisplayType** (string) - Type of integrated displayer started by the program
for Rustogramer this is always "None". 
*  **BuffersAnalyzed** (unsigned) - Number of items that have been analyzed.  For
SpecTcl (not Rustogramer), this taken with LastSequence allows a rough
computation of the fraction of data analyzed online.  Note that
Rustogramer always analyzes offline (well there are ways but....).
*  **RunTitle** (string) - Title string of the most recent run (being) analyzed.

### spectrum_list
#### Description
List the properties of selected spectra.
#### Parameters
* *pattern* (string) - Optional glob pattern.  Only the spectra with names that match the patern will be included in the listing. If omitted, the pattern defaults to ```*``` which matches everything.
#### Returns
**detail** is an iterable that contains maps.  Each map describes one matching spectrum and contains the following keys:

* **id** (unsigned) - integer identifier for the spectrum.  This is not that useful and, in most cases should be ignored.
* **name** (string) - Name of the spectrum.  This *will* match the *pattern* parameter.
* **type** (string) - The specturm type;  see the spectrum command in the [SpecTcl command reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) for information about the values thie key might take.
* **parameters** (iterable) - Containing strings that are the names of of the parameters the spectrum depends on.  In general you should care more about the **xparamters** and **yparameters** below.
* **xparameters** (iterable) - contains the names of the x parameters for a spectrum.  For gamma summary spectra a comman delimiter is between the parameters for each x-bin.
* **yparameters** (iterable) - contains the name sof the y parameters for a spectrum.  In SpecTcl this is only present when the spectrum has y parameters.
* **axes** - Iterable of axis definitions. Each axis definition is a dict with the keys:
    * **low** (float) - axis low limit.
    * **high** (float) - axis high limit.
    * **bins** (unsigned integer) axis high limit.
* **xaxis** (dict) - X axis definition.
* **yaxis** (dict) - Y axis definition.
* **chantype** (string) - The data type for each channel. This can be one of:
    * **f64** - 64 bit floating point (Rustogramer).
    * **long** - 32 bit unsigned integer (SpecTcl).
    * **short** - 16 bit unsigned integer (SpecTcl).
    * **byte** - 8 bit unsigned integer (SpecTcl).
* **gate** (string) - The gate applied to the spectrum if any.

### spectrum_delete
#### Description
  Delete a specctrum.
#### Parameters
* *name* (string) - name of the spectrum to delete.
#### Returns
none

### spectrum_create1d
#### Description
Create a simple 1-d spectrum.  This is a spectrum of type ```1```.
#### Parameters
* *name* (string) - name of the spectrum.  Must be unique amongst spectra.
* *parameter* (string) - name of the parameter that will be histogramed (x axis).
* *low*, *high* (floats) - X axis low and high limits.
* *bins* (unsigned > 0) - number of bins on the x axis.
* *chantype* (string) - Optional channel type specication that defaults to ```f64``` if not supplied.  Note this is only legal for Rustogramer, if SpecTcl is the server, you must explicitly provide a channel type.  Valid channel types are:
    * ```f64``` -  64 bit floating point. This is only valid for Rustogramer.
    * ```long``` - 32 bit unsigned integer.  This is only valid for SpecTcl.
    * ```word``` - 16 bit unsigned integer.   This is only valid for SpecTcl.
    * ```byte``` - 8 bit unsigned integer.  This is only valid for SpecTcl.

#### Returns
Nothing

### spectrum_create2d
#### Description
Create a simple 2-d specturm.  This is  a spectrum of type ```2```.  These spectra have an x and a y parameter.  If both are present and any gate is true, the x and y parameters define a location in the spectrum that translates to the bin that is located.
#### Parameters
* *name* (string) - name of the spectrum.
* *xparam* (string) - Name of the parameter on the x axis.
* *yparam* (string) - name of the parameter on the y axis.
* *xlow*, *xhigh (float) - Low and high limits of the x axis.
* *xbins* (unsigned > 0) - Number of bins on the x axis.
* *ylow*, *yhigh (float) - Low and high limits of the y axis.
* *ybins* (unsigned > 0) - Number of bins on the Y axis.
* *chantype* (string) - Channel type specification see [spectrum_create1d](#spectrum_create1d) for a description of this argument.

#### Returns
Nothing

### spectrum_createg1
#### Description
Creates a multiply incremented 1-d spectrum, also called a 1d gamma spectrum.   This is SpecTcl type ```g1```.  There are an arbitrary number of parameters associated with this spectrum.  If the gate is true, the histogram is incremented once for each spectrum parameter present in the event.  If the spectrum is folded, the increment is once for every parameter not involved in the fold condition.  

#### Parameters
* *name* (string) - name of the new spectrum.
* *parameters* (iterable) - iterable of strings containing the names of the parameters to histogram.
* *xlow*, *xhigh* (float) - low and high x axis limits.  The y axis is the counts axis.
* *bins* (unsigned > 0) - Number of bins on the X axis.
* *chantype* (string) - data type for bins.  See [spectrum_create1d](#spectrum_create1d) for more information about this parameter.
#### Returns
Nothing
### spectrum_createg2
#### Description
Create a multiply incremented 2-d spectrum of type ```g2```.  These spectra have an arbitrary number of parameters (at least two).  Each time the spectrum's gate is true, the spectrum is incremented at the bins defined by all unorderd pairs of parameters present in the event. A simple example of what I mean but un-ordered pairs, suppose I've defined this spectrum on parameters ```p1``` and ```p2``` and both a present in the event, Increments will happen at the points defined by (```p1```, ```p2```) *and*  (```p2```, ```p1```).

If the spectrum is folded, then this increment is for all pairs of parameters that are *not* involved in the gate.
#### Parameters
* *name* (string) - name of the spectrum.
* *parameters* (iterable) - Iterable of strings.  Each element is the name of a spectrum parameter.
* *xlow*, *xhigh (float) - Low and high limits of the x axis.
* *xbins* (unsigned > 0) - Number of bins on the x axis.
* *ylow*, *yhigh (float) - Low and high limits of the y axis.
* *ybins* (unsigned > 0) - Number of bins on the Y axis.
* *chantype* (string) - Channel type specification see [spectrum_create1d](#spectrum_create1d) for a description of this argument.

#### Returns
Nothing.


### spectrum_creategd
#### Description
Creates a 2-d multiply incremented of type ```gd``` this is most often used as a particle-gamma coincidence spectrum.  The spectrum has a set of x parameters and a set of y parameters.  For events where the gate is true, it is incremented for each pair of x and y parameters present in the event.

Suppose, for example, the x parameters are ```x1```, ```x2```, ```x3```, and the Y parameters are ```y1``` and ```y2```.  For an event that has ```x1``` and ```x3```, and ```y2```, increments will happen at the points defined by (```x1```, ```y2```) and (```x3```, ```y2```).
#### Parameters
* *name* (string) - name of the spectrum.
* *xparameters* (iterable) - containing strings that are the names of the x parameters.
* *yparameters* (iterable) - containing strings that are the names of the y parameters.
* *xlow*, *xhigh (float) - Low and high limits of the x axis.
* *xbins* (unsigned > 0) - Number of bins on the x axis.
* *ylow*, *yhigh (float) - Low and high limits of the y axis.
* *ybins* (unsigned > 0) - Number of bins on the Y axis.
* *chantype* (string) - Channel type specification see [spectrum_create1d](#spectrum_create1d) for a description of this argument.

#### Returns
Nothing.

### spectrum_createsummary
#### Description
Creates a spectrum of type ```s```, a summary spectrum. A summary spectrum is a special type of 2-d spectrum.  It has several parameters.  The 1d spectrum of each parameter is allocated an x axis bin and incremeented on the y axis of that bin.  Suppose, for example, the parameters are ```p1,p1,p3,p4,p5```; the X axis will have 5 bins. The y axis, will be specified by this method.

If an event makes the gate for that axis true and has parameters ```p1, p3, p5``` there will be increments on (0, ```p1```), (2, ```p3```) and (4, ```p5```).    This spectrum type is normally used to visualize the health and, if desired, the gain matching of elements of a lage detector array.

#### Parameters
* *name* (string)  - name of the spectrum.
* *parameters* (iterable) - Each element of the iterable is a string, parameter name.  The first element is assigned to x axis bin 0, the second to x axis bin 1 and so on.
* *low*, *high* (float) - Y axis low and high limits.
* *bins* (unsigned > 0) - number of Y axis bins.  The number of x axis bins is ```len(parameters)```.
* *chantype* (string) - Channel type specification see [spectrum_create1d](#spectrum_create1d) for a description of this argument.

Note that it is the Y axis that is specified.  The X axis is determined by the *parameter* argument and is defined as between 0 and ```len(parameters)``` with ```len(parameters)``` bins.
#### Returns
Nothing.

### spectrum_create2dsum
#### Description
Creates a spectrum that is essentially a 2d sum spectrum (type ```m2```).  The spectrum has an equal number of x and y parameters.  For each X parameter there is a corresponding y parameter.  If the gate is true, then all pairs of corresponding parameters in the event cause an increment.

Suppose, for example, we have x parameters (```x1,x2,x3,x4,x5```) and y parameters (```y1,y2,y3,y4,y5```).  Suppose the event has parameters (```x1,x3,x5, y1,y4,y5```).  There will be increments only for (```x1,y1```) and (```x5, y5```). The spectrum type comes from the fact that it is the sum of the 2d spectra for each corresponding x/y pair of parameters.  In our example, the spectrum is the sum of 2d spectra defined on ```(x1,y1), (x2, y2), (x3, y3), (x4,y4), (x5,y5)```.
#### Parameters
* *name* (string) - name of the spectrum.
* *xparameters* (iterable) - containing strings that are the names of the x parameters.
* *yparameters* (iterable) - containing strings that are the names of the y parameters.
* *xlow*, *xhigh (float) - Low and high limits of the x axis.
* *xbins* (unsigned > 0) - Number of bins on the x axis.
* *ylow*, *yhigh (float) - Low and high limits of the y axis.
* *ybins* (unsigned > 0) - Number of bins on the Y axis.
* *chantype* (string) - Channel type specification see [spectrum_create1d](#spectrum_create1d) for a description of this argument.
#### Returns
Nothing

### spectrum_createstripchart
#### Description
Only available in SpecTcl (```S```).  A strip chart spectrum is a special type of 1d spectrum defined on two parameters, a *time* and *value*  for each event that has the time and value parameters, an X channel is computed from the time.  If the time is out of the axis bounds, the spectrum, contents and axis are shifted to bring the time back into the bounds.  The bin defined by the time is incremented by the value parameter.

Suppose, for example, the axis is defined as 0.0 to 1000.0 with 1000 bins.  An event with time 50 and value 100 will result in bin number 50 incremented by 100.  If the time were 1020, the spectrum would be shifted by at least 21 bins to the left in order to accomodate that time. 

The effect, for monotonic time parameters is that of a strip chart recorder.  Note that shifts can be in either direction.  For example, you might have a time parameter that is zeroed at the beginning of each run. In that case, the spectrum will be shifted to the right rather than the left if needed.
#### Parameters
* *name* (string) -  spectrum name.
* *time* (string) - The time parameter name.
* *vertical* (string) -the value parameter name.
* *low*, *high*, (floats) - initial X axis limits.
* *bins* (unsigned > 0) - the number of x axis bins.  This remains invariant as the spectrum shifts.
* *chantype* (string) - Channel type specification see [spectrum_create1d](#spectrum_create1d) for a description of this argument.

#### Returns
Nothing.

### spectrum_createbitmask
#### Description
Create a bitmask spectrum (```b```).  The parameter for this spectrum type is taken as an integer.  The spectrum is incremented one for each bit set in that mask.  

#### Parameters
* *name* (string) - name of the spectrum.
* *parameter* (string) - name of the parameter to be histogramed.
* *bits* (unsigned > 0) - The number of bits in  the parameter.  The axis is then defined with a low of 0, a high if *bits* with *bits* bins.
* *chantype* (string) - Channel type specification see [spectrum_create1d](#spectrum_create1d) for a description of this argument.
#### Returns
Nothing.

### spectrum_creategammasummary
#### Description
Create a *gamma summary* spectrum (type ```gs```).  This spectrum can be thought of as a summary spectrum where each X axis bin is a ```g1``` spectrum on the y axis.  
#### Parameters
* *name* (string) - name of the spectrum.
* *parameters* (iterable) - Each iteration returns an iterable containing the parameters for an x bin.
* *ylow*, *yhigh* (floats) - Y axis low/high limits.
* *ybins* (unsigned > 0) - Number of y axis bins. 
* *chantype* (string) - Channel type specification see [spectrum_create1d](#spectrum_create1d) for a description of this argument.
#### Returns
None.















