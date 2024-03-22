# /spectcl/gate requests

The /spectcl/gate domain or URIs provides the ability to manipulate gates (rustogramer conditions).


URIs include:

* [```/spectcl/gate/list```](#spectclgatelist) - lists defined conditions.
* [```/spectcl/gate/delete```](#spectclgatedelete) - Delets a condition
* [```/spectcl/gate/edit```](#spectclgateedit) - Create or modify a condition.

## /spectcl/gate/list

Returns a list of gates with names that match an optional pattern.

### Query parameters

* **pattern** (optional string) If not supplied this defaults to ```*``` which matches all names.  The pattern can use any filesystem wild-card characters and patterns.

### Response format detail

The **detail** of the response is a bit complex.   It consists of an array of structs.  Some struct fields are gate type dependent and, for SpecTcl unecessary fields are not present while for rustogramer all fields are present but the unecessary fields have the value ```null```

Each struct has the following fields.

* **name** (String) - Always present.  This is the name of the condition/gate
* **type** (String) - Always present.  The gate type string. See the [SpecTcl command reference for ``gate``](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) for the possible values  and meanings of this string.
* **gates** (Array of Strings) - Present only for compound conditions/gates (e.g. *and* *or* an d *not*).
* **parameters** (Array of Strings) - Present only for conditions/gates that depend on parameters (for example, and not limitied to *slice* or *contour*).  This is a list of the parameter names the condition/gate depends on.
* **points** (Array of Point structs ) - Present only for  conditions/gates that represent geometric shapes in two dimensional parameter space (for example, but not limited to *slice* or *contour*).  These are the points that make up the shape.  Each point, itself, is a struct made up of.
    * **x** - (float) the x coordinate of the point.
    * **y** - (float) the y coordinate of the point. 
* **low** - (float) Present only for conditions/gates that are a one-dimensional slice in parameter space.  This is the low limit of that slice.
* **high** - (float) Present only for conditions/gates that are a one-dimensional slice in parameter space.  This is the high limit of that slice.


#### Sample Responses.


Here is a response that shows pretty much all of the gate struct types (from SpecTcl so unused fields are omitted - had this come from Rustogramer, unused fields would be ```null```):

```json
{
    "status" : "OK",
    "detail" : [{
        "name"       : "acontour",
        "type"       : "c",
        "parameters" : ["event.raw.00","event.raw.01"],
        "points"     : [{
            "x" : 398.316437,
            "y" : 458.697357
        },{
            "x" : 206.994919,
            "y" : 138.077942
        },{
            "x" : 647.967651,
            "y" : 77.811142
        },{
            "x" : 845.511047,
            "y" : 302.003662
        },{
            "x" : 710.186035,
            "y" : 550.302917
        }]
    },{
        "name"  : "anand",
        "type"  : "*",
        "gates" : ["acontour","anot"]
    },{
        "name"  : "anot",
        "type"  : "-",
        "gates" : ["aslice"]
    },{
        "name"       : "aslice",
        "type"       : "s",
        "parameters" : ["event.raw.00"],
        "low"        : 330.595703,
        "high"       : 674.400635
    }]
}
```

*  ```acontour``` is a contour and therefore has **points**.  Note that the points are in parameter space defined by X=```event.raw.00``` and Y=```event.raw.01```
*  ```aslice```  is a slice gate and therefore has **low** and **high**
*  ```anot``` is a not gate and therefore has **gates** with a single gate name, the name of the gate it negates.
*  ```anand``` is an And gate which also has ***gates***, in this case both ```acontour``` and ```anot``` must be true (the inverse of ```aslice```) for the condition to be true.

Another important note;  The order in which the conditions are listed should not be assumed.  While SpecTcl will, in general list the conditions alphabetically by name, Rustogramer will list them at random.  In particular this means that, in order to reconstruct the gates, they must be re-ordered in dependency order (you can't make ```anand``` until ```acontour```  and ```anot``` have been defined and you can't make ```anot``` until ```aslice``` is defined).




## /spectcl/gate/delete

Deletes a condition.  Note  that while rustogramer actually delete conditions, SpecTcl modifies them into False conditions.

### Query parameters

* **name** (String) this mandatory parameter is the name of the condition to delete.

### Response format detail

A generic response.

#### Sample Responses.

Successful condition deletion:

```json
{
    "status" : "OK"
}

```

Where Rustogramer's response will include an empty **detail** field.
  Note that since SpecTcl just replaces deleted gates with a False gate it is legal to delete a "deleted" gate.  That is an error in Rustogramer, however.


  Attempting to delete a nonexistent gate ```anando``` generates the following in SPecTcl

```json
{
    "status" : "not found",
    "detail" : "anando"
}
```
In rustogramer you'll get:

```json
{
    "status" : "Failed to delete condition anando",
    "detail" : "anando"
}
```

## /spectcl/gate/edit

Creates a new condition/gate or edits an existing one.  These two operations are functionalyly identical.  If the condition specified in the query parameters for this request already exists, it is replaced.  If not, it is created.

Note that condition replacement is dynamic.  Spectra that gave this condition applied to them as gates have their gating modified to reflect the new condition definition on the next event processed.

### Query parameters


* **name** (String) - Mandatory specifies the name of the condition/gate being edited.
* **type** (String) - mandatory specifies the type of condition/gate being edited.  See the [SpecTcl command reference for ``gate``](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) for the possible values  and meanings of this string.
* **gate** (Multiple String) - This is required for conditions that depend on other conditions.  It should be presenet once for each dependent condition. For example:<br/>
```.../spectcl/gate/edit?name=anand&type=*&gate=g1&gate=g2&gate=g3```<br/>
is how to specify an and gate named ```anand``` that depends on the gates ```g1```, ```g2``` and ```g3```
* **xparameter** (String) - Mandatory for two dimensional geometric shape gates in parameter space. The parameter on the X axis of the condition/gates space.
* **yparameter** (String) - Mandatory for two dimensional geometric shape gates in parameter space. The parameter on the Y axis of the condition/gates space.
* **parameter** (String) - Mandaatory for slice (```s``` type) and for conditions with multiple unorderd parameters, for example gamma slices (```gs```) or gamma contours (```gc```).  This can be specified as many times as needed to supply all parameters. For examle the gamma contour depending on p1, p2, p3 would be something like:<br'>
```.../spectdl/gate/edit?name=gamma-contour&type=gc&parameter=p1&parameter=p2&parameter=p3...```
*  **xcoord** (float) - mandatory for 2d geometric gates (e.g. contours ```c```).  This is the X-coordinate of a gate point.   specify this as many times as needed.  To specify an ordered set of x-coordinates.
* **ycoord** (float) - mandatory for 2d geometric gates (e.g. contours ```c```).  This is the X-coordinate of a gate point.   specify this as many times as needed.  To specify an ordered set of x-coordinates.  Here, for example, is a definition of a contour that is a right triangle:<br/>
```
.../spectcl/gate/edit?name=contour&type=c&xparameter=p1&yparameter=p2&xcoord=100&ycoord=100&xcoord=200&ycoord=100&xcoord=200&ycoord=200
```
* **low** (float) - mandatory for slice like gates (e.g. ``s``` and ```gs```); The  low limit of the condition.  
* **high** (float) - mandatory for slice like gates; the high limit of the conditions.
* **value** (integer) - For SpecTcl mask gates, this is the mask value.

### Response format detail

The response is a generic response.

#### Sample Responses.

Successful gate creation in SpecTcl:

```json
{
    "status" : "OK"
}
```

Rustogramer will include an empty **detail** field as well.

Failed gate creation - **type** parameter omitted (SpecTcl):

```json
{
    "status" : "missing Parameter",
    "detail" : "type"
}
```

Note that the REST server in Rustogramer does some pre-processing and will fail to match a URI that does not include both a **name** and a **type**

However detailed error messages will be in the **status** field for rustogramer.  Suppose, for example, you try to create a not codition without supplying a dependent gate:

```json
{
    "status" : "Not conditions can have at most one dependent condition",
    "detail" : ""
}
```

Will be returned.

