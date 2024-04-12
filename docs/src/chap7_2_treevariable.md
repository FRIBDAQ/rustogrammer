# /spectcl/treevariable requests

This is only supported by SpecTcl, as Rustogramer does not support tree variables.

Almost all requests directed at Rustogramer for this domain produce Generic Responses that are:

```json
{
    "status" :"Tree variables are not implemented",
    "detail" : "This is not SpecTcl"
}
```
Other return types are noted in the individual request documentation.

The domain provides the following URIs.

* [```/spectcl/treevariable/list```](#spectcltreevariablelist) - List tree variables and their properties.
* [```/spectcl/treevariable/set```](#spectcltreevariableset) - Set new value and units for a tree variable.
* [```/spectcl/treevariable/check```](#spectcltreevariablecheck) - See if the changed flag is set for the tree variable.
* [```/spectcl/treevariable/setchanged```](#spectcltreevariablesetchanged) - Set the changed flag for a treevariable.
* [```/spectcl/treevariable/firetraces```](#spectcltreevariablefiretraces) - Fire the traces for a set of tree variables, allowin scripts and UI elements that care about the variable to know about changes.

 In SpecTcl, tree variables are bound to Tcl variables as linked variables.  the set operation changes the value of the linked variable.  In general, for scripts which have traces set o the variable, traces must be explicitly fired (```/spectcl/treevariablefiretraces```) for those traces to execute.  Tk GUi elements that bind to variables will, behind the scenes, establish traces and therefore traces must be fired for those elements to update visually.  The units metadata are kept separate from the Tcl interpreter and is only known to it through the ```treevariable -list``` command.

 The purpose of the changed flag is to keep track of which variables have values different from those compiled into SpecTcl.  This allows software that saves the SpecTcl state to selectively save only the changed treevariables.



## /spectcl/treevariable/list

Lists the priperties of all treevariables.  Note that there is no way to selectively list the treevariables (e.g. with a pattern query parameter.)

### Query parameters

None supported.

### Response format detail

The **detail** is an array of objects.  Each object describes  tree variable and has the following attributes.

* **name** (string) - name of the tree variable being described.
* **value** (float) - Value of the variable.  This will be correct whether traces have been fired or not.
* **units** (string) - Units of measure metadata.

#### Sample Responses.

To maintain the shape of the response detail Rustogramer's response is:

```json
{
    "status" : "Tree variables are not implemented.  This is not SpecTcl",
    "detail" : []
}
```
Here's a SpecTcl return with one treevariable:

```json
{
    "status" : "OK",
    "detail" : [
        {
            "name" : "avarialbe",
            "value" : 3.14159265359,
            "units" : "radians/half-pie"
        }
    ]
}
```
Failure (I'm not sure I see how this can ever happen but...):
```json
{
    "status" :  "'treevariable -list' failed: ",
    "detail" : "<error message from treevariable -list>"
}
```

## /spectcl/treevariable/set

Sets the value and units of measure metadata of a treevariable.  Note that for historical reasons, both must be set.

### Query parameters

* **name** (string) - Required. Name of the treevariable to modify.
* **value** (float) - Required.  New value for the tree variable.
* **units** (string) - Required.  New units of measure for the variable. 

### Response format detail

Generic response.

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
    "status" : "'treevariable -set' failed",
    "detail" : "<treevariable -set error message"
}
```
## /spectcl/treevariable/check

Return the value of the check flag for a tree variable.  The check flag is non-zero if, at any time during the SpecTcl run, the treevariable was modified.

### Query parameters

* **name** (string) - Required. Name of the tree variable being queried.

### Response format detail

On success, **detail** containss an integer that is zero if the change flag was not set and non-zero if it was.

#### Sample Responses.

Change flag not set:
```json
{
    "status" : "OK"
    "detail" : 0
}
```

Error:
```json
{
    "status" : "'treevariable -check' failed",
    "detail" : "<treevariable -check error message>"
}
```

Note: Prior to 5.13-012, the error return mistakenly had a **status** of ```OK```

## /spectcl/treevariable/setchanged

Set a treevariable changed flag.  The changed flag is a latched boolean that is initialized ```false``` but is set to ```true``` by, e.g. this request, when a value is changed.

### Query parameters

* **name** (string) -  Required. Name of the treevariable whose changed flag will be set.

### Response format detail

Generic response.

#### Sample Responses.
Success:

```json
{
    "status"  : " OK"
}
```
Failure:
```json
{
    "status" :  "'treevariable -setchanged' command failed",
    "detail" : "<treevariable -setchanged' error message" 
}
```
Note: Prior to 5.13-012, the error return mistakenly had a **status** of ```OK```

## /spectcl/treevariable/firetraces

Fire traces associated with a set of tree variable.s

### Query parameters

* **pattern** (string) - Optional. The traces associated with all treevariables with names matching the pattern are fired.  If the pattern is omitted, ```*``` is matched, which matches everything.

### Response format detail

Generic response


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
    "status" : "'treevariable -firetraces failed: ", 
    "detail" : "treevariable -firetraces error message> "
}
```