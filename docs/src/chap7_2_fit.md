# /spectcl/fit requests

This is only avaialble with SpecTcl.  SpecTcl supports fitting regions of interest on 1-d spectra.  The actual fit function used can be extended, however ```linear``` and ```gaussian```.  Note that ```gaussian``` performs a gaussian fit on a constant background.

See the [SpecTcl command reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) for information about the ```fit``` command.  The set of fits is extensible by the user.  See the section "Extending the set of SpecTcl fit types in the [SpecTcl programming guide](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/pgmguide/index.html)

The ```/spectcl/fit``` domain of URIs provides the following operations:

* [```/spectcl/fit/create```](#spectclfitcreate) - create a new SpecTcl fit object.
* [```/spectcl/fit/update```](#spectclfitupdate) - Computes fit parameter values on current histogram data.
* [```/spectcl/fit/delete```](#spectclfitdelete) - Delete a spectcl fit object.
* [```/spectcl/fit/list```](#spectclfitlist) - List one or more fits providing each fit's current function parameterization.
* [```/spectcl/fit/proc```](#specclfitproc) - Returns a Tcl proc that can be used to evaluate the fit at any point.

## /spectcl/fit/create

Creates a new fit object. Fit object, once created, can be updated, which causes them to compute/recompute their parameterizations, which can the be fetched via the list operation.

### Query parameters

* **name** (string) - mandatory name to associate with the fit object.
* **spectrum** (string) - mandatory name of a spectrum that only has an X paramter axis (e.g. a 1d or gamma 1d spectrum).
* **low** (unsigned integer) - mandatory low limit in channels of the region of interest.
* **high** (unsigned integer) - mandatory high limit in channels of the region of interest.
* **type** (string)  - Fit type string, e.g. ```gaussian```

### Response format detail

The response is a Generic response.

#### Sample Responses.

Success:
```json
{
    "status" : "OK"
}
```


Failure:

```json
{
    "status" : "'fit'command failed: ",
    "detail" : "<error message from the fit command>"
}
```

## /spectcl/fit/update

Performs fits an updates the paramterization of the fit functionss stored in the update.  This computes the parameterization of the fit functions on current data.  Prior to its first update, the parameterization is whatever values the author of that fit type chose and, in general, is not meaningful.


### Query parameters

* **pattern** (string) - Glob pattern.  Only fits with names that match the pattern are pdated.

### Response format detail

Response is a generic response.


#### Sample Responses.

On Success:

```json
{
    "status" : "OK"
}
```

On Failure:

```json
 {
    "status": "'fit'command failed: ",
    "detail": "<Error message from the fit command>"

 }

## /spectcl/fit/delete

Deletes a single named fit object.

### Query parameters

* **name** - name of the fit to delete.

### Response format detail

The response is a generic response.

#### Sample Responses.

On Success:

```json
{
    "status" : "OK"
}
```


On Failure:

```json
{
    "status" : "'fit'command failed: ",
    "detail" : "<Error returned from the fit command>"
}
```

## /spectcl/fit/list

Lists a set of fits.  When fits are listed the fit function parameters as of the most recent ```/update``` are also listed.


### Query parameters

* **pattern** (string) - (optional)  Restricts the listed fits to only those that match the glob pattern provided.

### Response format detail

The **detail** is an array of objects.  Each object describes a fit that was done and contains the fields:

* **name** (string) - Name of the fit being described.
* **spectrum** (string) name of the spectrum the fit is defined on
* **type** (string) - fit type (e.g. ```gaussian```).
* **low** (unsigned) - Low bin of the area of interest.
* **high** (unsigned) - High bin of the area of interest.
* **parameters** (Object) - the shape of this object depends on the type of fit.  Fields of this object are the most recently computed fit parameters.  See the ```fit``` command in the [SpecTcl command reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) for the fields for each of the built-int fit types.  The fields provided by user written fits depend on the author of the fit type support.  All fit types *should* provide a **chisquare** field which holds the goodness of fit.


#### Sample Responses.

Successful list where a single gaussian fit is matched to **pattern**:

```json
{
    "status" : "OK",
    "detail" : [
        {
            "name" : "agaussianfit",
            "spectrum" : "aspectrum",
            "type" : "gaussian",
            "low" : 100,
            "high" : 300,
            "parameters" :  {
               "baseline" : 127,
               "height"   : 1234.7,
               "centroid" : 207.6,
               "sigma"    : 12.7,
               "chisquare" : 15.766
            }
        }
    ]
}
```

Note the ```parameters``` are just pulled out of the air and do not relflect any actual fit.

## spectcl/fit/proc

Given a fit, provides a Tcl proc that can be given channel numbers (floating point) and return to value of the fit at that channel.

### Query parameters

* **name** - name of the fit.

### Response format detail
 A generic response is produced however **detail** is the text of a Tcl proc.




#### Sample Responses.

Suppose we have a linear fit, what you might get back is:

```json
{
    "status" : "OK",
    "detail" : "proc fitline x {   \nset slope 2.7\nset offset 362.6\nreturn [expr {$x*$slope+$offset}]\n}"
}
```