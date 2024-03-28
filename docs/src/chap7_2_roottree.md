# /spectcl/rootree requests

This URI domain is only available in SpecTcl.  It supports access to SpecTcl's ability to make root trees from the parameters created by  its event processing pipeline.   Since Rustogramer does not create root trees, this is not meaningful and making requests to these URIs for Rustogramer will result in a Generic response of the form:

```json
{
    "status": "Root Tree output is not supported",
    "detail": "This is not SpecTcl"
}
```

unless otherwise noted (e.g. for reponses from SpecTcl that are not generic responses).

SpecTcl, supports the following URIs:


* [```/spectcl/roottree/create```](#spectclroottreecreate) - Makes a new root tree.
* [```/spectcl/roottree/delete```](#spectclroottreedelete) - Delete an existing root tree.
* [```/spectcl/roottree/list```](#spectclroottreelist) - List root trees and their properties.


For more information about root tree support in SpecTcl, see the ```roottree``` command in the 
[SpecTcl Command Reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html).

## /spectcl/roottree/create

Create a new root tree object.

### Query parameters

* **name** (string) - Required. Name of the new tree being created.  Must be unique.
* **parameter** (string) - At least one required. Each instance of the **parameter** query paramater provides a glob pattern.  Parameters in the event which match the pattern are included in the output tree.
* **gate** (string) - Optional.  If provided the root tree will only output events that satisfy the specified gate.  Note that:
    *   If no gate is specified all events are written.
    *   Changes to the gate dynamically affect the roottree output.
    *   The point above means that if you delete the gate, the root tree will not output events as in SpecTcl a deleted gate is the same as a ```False``` gate.

### Response format detail

**detail** is a generic response.

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
    "status" : "'roottree create' command failed",
    "detail":  "<root tree create error message>"
}
```

## /spectcl/roottree/delete

Delete an existing root tree object.

### Query parameters

*  **tree** (string) - Required.  The name of the tree to delete.

### Response format detail

The response is a generic response.

#### Sample Responses.

Success:
```json
{
    "status": "OK"
}
```

Failure: 
```json
{
    "status" : "'roottree delete' command failed",
    "detail" : "<error message from roottree delete command>"
}
```
## /spectcl/roottree/list

Lists the properties of root trees.

### Query parameters

* **Pattern** (string) - Optional. If provided, only the root trees with names that match the glob pattern are included in the list.  If not provided, the pattern defaults to ```*``` which matches all names.

### Response format detail

**detail** is an array of objects.  Each object describes one root tree and has the following attributes:

* **name** (string) - name of the tree.
* **params** (array of strings) - names of parameters that will be written to the tree.
* **gate** (string) - name of the tree's gate.  If the tree does not have a gate, this will be an empty string.


#### Sample Responses.

Since the **detail** is not a string, the Rustogramer return object looks like this:

```json
{
    "status" : "Root tree output is not implemented - this is not SpecTcl",
    "detail" : []
}
```

This shape is compatible with what's expected by SpecTcl clients.

SpecTcl success with one matching tree:
```json
{
    "status" : "OK", 
    "detail" :[
        {
            "name" : "atree",
            "params": [
                "event.raw.00", "event.raw.01",
                "event.raw.02", "event.raw.03",
                "event.raw.04", "event.raw.05"
            ],
            "gate": "tree-gate"
        }
    ]
}
```

SpecTcl failure is a generic response:

```json
{
    "status" : "'roottree list' command failed",
    "detail" : "<roottree list error message>"
}
```