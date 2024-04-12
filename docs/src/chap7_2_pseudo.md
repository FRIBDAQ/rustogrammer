# /spectcl/psuedo requests

Psuedo parameters are a SpecTcl only object.  A SpecTcl psuedo parameter is a Tcl script that is invoked for each event and may return a new parameter value.  A pseudo parameter depends on a list of other parameters (some of which may also be psuedo parameters as long as they are defined chronogically before used).

Psuedo parameters are not terribly performant.  They are intended to answer what-if experiments which, if successful result in compiled code to produce the computed parameter.

Pseudo parameters are processed after all stages of the event processing pipeline have completed.

See the **psuedo** command documented in the [SpecTcl Command Reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) for more information on psuedo parameters.

The following URIs manipulate pseudo parameters:

* [```/spectcl/pseudo/create```](#spectclpseudocreate) - Create a new pseudo parameter.
* [```/spectcl/pseudo/list```](#spectclpseudolist) - List the properties of pseudo parameters.
* [```/spectcl/pseudo/delete```](#spectclpseudodelete) - Delete an exising pseudo parameter.


## /spectcl/pseudo/create

### Query parameters

* **pseudo** (string) - Mandatory name to give the pseudo parameter.  In addition to provide a name that is used to refer to the pseudo paramater, the actual **proc** name for the computation is derived from its name.
* **parameter** (string) - At least one instance is mandatory.  An instance of **parameter** should appear as a query parameter once for each parameter the computation depends on.
* **computation** (string) -Mandatory. The body of the computation.  You can assume that for each parameter specified by the **parameter** query parameter, there are a pair of variables available to the computation:
   *  The name of the parameter (e.g. ?parameter=george implies a varialbe named ```george```), will contain the value of the parameter for the event being processed when the pseudo code is invoked.
   *  THe name of the parameter with ```isValid``` appended. THe example above implied, that a variable named ```georgeisValid``` is defined. This variable is ```true``` if the parameter has been produced by the proccessing pipline.


### Response format detail

The response is a generic response.

#### Sample Responses.

Rustogramer

```json
{
    "status" : "Pseudo parameters are not implemented",
    "detail" : "This is not SpecTcl"
}
```

SpecTcl success:

```json
{
    "status": "OK"
}
```

SpecTcl failure:

```json
{
    "status": "'pseudo' command failed",
    "detail": "<Error message fromt he pseudo command"
}
```

## /spectcl/pseudo/list

List pseudo parameters and their properties.

### Query parameters

* **pattern** (string) - Optional parameter.  If provided, the names of pseudos included inthe listing must match the pattern.  If not supplied, the pattern defaults to ```*```` which matches everything.

### Response format detail

**detail* is an array containing objects, on object for each listed pseudo parameter.  The attributes of the objects are:

* **name** (string) - name of the pseudo parameter.
* **parameters** (array of strings) - the parameters the pseudo parameter computation depends on.
* **computation** (string) - The computation script.

#### Sample Responses.
From Rustogramer:


```json
{
    "status": "Psuedo parameters are not implemented - this is not SpecTcl",
    "detail": []
}
```
Successful SpecTcl with a parameter ```add12``` that add par1 and par2 together.

```json
{
    "status" :"OK",
    "detail": [
        {
            "name" : "add12",
            "parameters": [
                "par1", 
                "par2"
            ],
            "computation" : " if {$par1isValid && $par2isValid} {
                return [expr {$par1 + $par2}]
            } else {
                return -1000
            }
            "
        }
    ]
}
```
SpecTcl failure:

```json
{
    "status" : "'pseudo -list' command failed",
    "detail" : "<error message from pseudo -list command"
}
```

## /spectcl/pseudo/delete

Deletes an existing Psuedo parameters.

### Query parameters

* **name** - Name of the parameter to delete.


### Response format detail

Response is a Generic Response.

#### Sample Responses.
Rustogramer:
```json
{
    "status" :"Pseudo parameters are not implemented",
    "detail" : "This is not SpecTcl"
}
```

SpecTcl success:

```json
{
    "status" : "OK"
}
```
SpecTcl failed:

```json
{
    "status" : "'pseudo -delete' command failed",
    "detail" : "<error message from pseudo -delete command"
}
```