# parameter requests

The base URI for these, after the ```protocol//host:port``` stuff is ```/spectcl/parameter```
Several operations are supported on parameters:

*  [```/list```](#spectclparameterlist) - lists the parameters and their properties.
*  [```/edit```](#spectclparameteredit) - Modifies the metadata associated with a parameter.
*  [```/promote```](#spectclparameterpromote) - Promote a raw parameter to a tree parameter.
*  [```/create```](#spectclparametercreate) - Create a new parameter (Rustogramer only)
*  [```/listnew```](#spectclparameterlistnew) - lists parameters that have modified metadata.
*  ```/check``` - Checks the modified state of parameters.
*  ```/uncheck``` - Turns off the modified state of parameters.
*  ```/version``` - Provides version information about the capabilities of the parameter system (earlier versions of the SpecTcl ```treeparameter``` did not support the ```-create``` operation).


## /spectcl/parameter/list

Lists the parameters that are defined along with their metadata

### Query parameters

* ```filter``` (optional) if provided the value of this query parameter is a patter that must be matched by the parameter name in order for it to appear in the response.  The filter string can include filesystem matching wild-card characters (e.g. ```*``` or ```.```).  If the ```filter``` query parameter is not supplied, it will default to ```*``` which will match all parameters.

### Reponse format detail

The **detail** field of the response is a possibly empty array of parameter descriptions.  Each parameter description is, itself, a struct.  It is not an error for the filter string not to match any parameters.  That case results in an ```OK``` status with an empty array as the **detail**

Each parameter description is a struct with the following keys:

*  **name** - Name of the parameter being described.
* **id** - An integer id that is assigned to the parameter.  The id is used in the histograming engine to specify the parameter.
* **bins** - Suggested binning for axes that are defined on the parameter.
* **low**  - Suggested low limit for axes that are defined on the parameter.
* **hi**   - Suggested high limit for axes that are defined on the parameter.
* **units** - Units of measure for the parameter (these are for documentation purposes only).
* **description** - (Rustogramer only) A description that documents the parameter purpose.  In SpecTcl, this field is missing.

#### Sample Responses.
  A single parameter matches  the filter.

  ```json
  {
    "status" : "OK",
    "detail" : [{
        "name"        : "event.sum",
        "id"          : 10,
        "bins"        : 100,
        "low"         : 1,
        "hi"          : 100,
        "units"       : "arbitrary",
        "description" : "Sum over the arraay event.raw.nn"
    }]
}
  ```

  Here is what you will get if your filter does not match any parameters
  ```json
{
    "status" : "OK",
    "detail" : []
}
```
Note how the **detail** field is just an empty array.

## /spectcl/parameter/edit

This request lets you modify the metadata associated with a parameter.

### Query parameters

* **name** (Required string) - Name of the parameter to modify.
* **low** (Optional float) New value for the suggested axis low limit.
* **high** (Optional float) New value for the suggested axis high limit.
* **bins** (Optional unsigned integer) New value for the suggested axis binning.
* **units** (Optional string) New value for the parameters units of measure.
* **description** (Optional string -ignored by SpeTcl) - A description that documents the purpose of the parameter

### Reponse format detail

* Rustogramer returns a generic response.  SpecTcl, returns only a **status** on success, but the detail field is present on error.  Rustogramer always returns a **detail** field and on success it is an empty string.

#### Sample Responses.

Successful return (Rustogramer):

```json
{
    "status" : "OK",
    "detail" : ""
}

```

Successful return (SpecTcl)

```json
{
    "status" : "OK"
}
```

Failure return for both Rustogramer and SpecTcl

```json
{
    "status" : "not found",
    "detail" : "event.raw"
}
```

## /spectcl/parameter/promote

In SpecTcl, there is a distinction between parameters with metadata (tree parameters) and parameters without metadata (raw parameters). In Rustogramer all parameters have metadata.  For
* Rustogramer - this is equivalent to the [/spectcl/parameter/edit](#spectclparameteredit) operation.
* SpecTcl - this makes a treeparameter from a raw parameter.

### Query parameters

* **name** (required String) - Name of the parameter to promote.
* **bins** (required for SpecTcl, optional for Rustogramer unsigned int) - The recommended bins for the promoted parameter.
* **low** (required for SpecTcl, optional for Rustogramer float) - The recommended low value for the promoted parameter.
* **high** (required for SpecTcl, optional for Rustogramer float) - The new high value for the promoted parameter.
* **units** (optional string) - Units of measure string for the promoted parameter.
* **description** (Rustogramer only String) - New desciption for the parameter

### Reponse format detail

The response is a Generic response.

#### Sample Responses.

Note again that SpecTcl may omit the **detail** field on success:

```json
{
    "status" : "already treeparameter"
}
```

Here's an error respons:

```json
{
    "status" : "already treeparameter",
    "detail" : "event.raw.00"
}
```

Here the **detail** is used to indicate which parameter was involved in the error

## /spectcl/parameter/create

Creatse a new tree parameter in SpecTcl or an ordinary parameter in Rustogramer.  The parameter must not yet exist.

Note that with SpecTcl this is a front end to the ```treeparameter -create```  command.

### Query parameters

* **name** (Required string) - Name of the parameter to create.
* **low** (Required for SpecTcl optional for Rustogramer float) - parameter's low limit metadata.
* **high** (Required for SpecTcl optional for Rustogramer float) - parameter's high limit metadata.
* **units** (Optional string) - parameter's units of measure metadata (defaults to "" for SpecTcl)
* **description** (Optional Rustogramer only string) desription metadata for the parameter.

### Reponse format detail

Generic response where again, SpecTcl might omit the **detail** field on success.  THe **detail** filed is used on failure to supply additional information for the  failure reason.

#### Sample Responses.

Successful creation:

```json
{
    "status" : "OK"
}
```

Failure in Spectcl:

```json
{
    "status" : "'treeparameter -create' failed: ",
    "detail" : "Could not parse  as type double\nUsage:\n     treeparameter -list ?pattern?\n     treeparameter -listnew\n     treeparameter -set name bins low high inc units\n     treeparameter -setinc name inc\n     treeparameter -setbins name bins\n     treeparameter -setunit name units\n     treeparameter -setlimits name low high\n     treeparameter -check name\n     treeparameter -uncheck name\n     treeparameter -create  name low high bins units\n     treeparameter -version"
}
```

Note how the **detail** field just contains the error message directly from the ```treeparameter -create``` command.

## /spectcl/parameter/listnew

### Query parameters

None.

### Reponse format detail

**detail** is an array of names of parameters that wre created since the start of the run. Note that this is really only implemented in SpecTcl.  In Rustogramer, this will be an empty array.

#### Sample Responses.

SpecTcl response:

```json
{
    "status" : "OK",
    "detail" : ["george"]
}
```

The parameter **george** was created.
