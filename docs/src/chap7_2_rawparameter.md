# /spectcl/rawparameter requests


For Rustogramer, the requests in this URI domain map to similar requests in the
[/spectcl/parameter](./chap7_2_parameter.md) domain.  For SpecTcl, however for historical reasons, there is a difference between a raw parameter and a tree parameter.  

Originally tree parameters were  a contributed package written by Daniel Bazin.  Later, due to its utility, tree parameters were incorporated into supported SpecTcl code.  However a distinction does exist, internally between tree parameters and the original raw parameters, which may not be mapped to tree paramters. 

In SpecTcl, therefor, these URIs manipulate raw parameters without affecting any tree parameters that might be bound to them.

The requests include:

* [```/spectcl/rawparameter/new```](#spectclrawparameternew) (Rustogramer maps this to [```/spectcl/parameter/create```](./chap7_2_parameter.md#spectclparametercreate)).  SpecTcl creates a new raw parameter.
* [```/spectcl/rawparameter/delete```](#spectclrawparameterdelete) (Rustogramer implements this).  This deletes an existing (raw) parameter. 
* [```/spectcl/rawparameter/list```](#spectclrawparameterlist) (Rustogramer maps this to [```spectcl/parameter/list```](./chap7_2_parameter.md#spectclparameterlist))

## /spectcl/rawparameter/new

Creates a new raw parameter.  Note that in Rustogramer, this is forwarded to the handler for [```/spectcl/parameter/new```](./chap7_2_parameter.md#spectclparametercreate). Refer to that documentation.  This section documents how this URI is implemented for SpecTcl.

### Query parameters

* ***name*** (required string) - Name of the parameter to be created.  The value of this parameter *must* not be the name of an existing parameter,
* **number** (required in SpecTcl  unsigned integer) - The parameter id to be assigned to the parameter.
* **resolution** (optional unsigned integer) - Number of bits of resolution in the metadata for the raw parameter.  
* **low**, **high** (optional floats) -  Low and high limits for parameter values.
* **units** (optional string) - Units of measure metadata.

### Response format detail

On success, this just has **status** with the value ```OK``` .  On failure
***detail*** provides more information about the error.

#### Sample Responses.

Successful completion:

```json
{
    "status" : "OK"
}
```

Attempt to redefine a parameter:

```json
{
    "status" : "'parameter -new' command failed",
    "detail" : "Duplicate Key during Dictionary insertion\nKey was:  event.raw.00 Id was: -undefined-\n"
}
```

*detail* is the error message from ```parameter -new```  in this case it indicates that ```event.raw.00``` already exists and the request is attemptig to find it.

## /spectcl/rawparameter/delete

Deletes a raw parameter.

### Query parameters
One of the following parameters must be present, but not both.

* name (string) - Name of the parameter to delete.
* id (unsigned integer) - Number of the parameter to delete

### Response format detail
 The response is a generic response, whith SpecTcl omitting **detail** if the operation succeded.

#### Sample Responses.
Successful request:

```json
{
    "status" : "OK"
}
```
 
Delete a nonexistent parameter:

```json
{
    "status" : "'parameter -delete' command failed",
    "detail" : "Failed search of dictionary by Key string\nKey was:  aaaa Id was: -undefined-\n"
}
```

## /spectcl/rawparameter/list

Lists the raw parameters with names matching a pattern or a specific id.

### Query parameters

One of the following are required

* pattern (string) pattern used to match the parameter names that will be listed. The pattern can contain any filename matching wild cards supported by the shell.
*  id (unsigned integer) Number of the paramter to list.

### Response format detail

On success, **detail**  is an array of structs.  Each struct has the fields:

* **name** - name of the parameter being described.
* **id**  - Id of the parameter.
* **resolution** - Only present if the raw parameter has a resolution set. The integer resolution.
* **low** **high** - Only present if the raw parameter has low/high limits.  These are the floating point low and high limits.
* **units** - Only present if the raw parameter has units of measure. This is the units string.

#### Sample Responses.

Successful return with one match ```event.sum```

```json
{ 
    "status" : "OK", 
    "detail" : [
        { 
            "name" : "event.sum", 
            "id" : 10, 
            "units" : "arbitrary" 
        }
    ] 
}
```