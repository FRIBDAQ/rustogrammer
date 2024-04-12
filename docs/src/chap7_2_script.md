# /spectcl/script requests

This URI is only supported by SpecTcl.  It allows the REST interface to inject and execute a Tcl script in the SpecTcl interpreter.  Rustogramer has no command interpreter, Tcl or otherwise and therefore will never support this.

The intended use case is not to inject complex scripts (other than, perhaps via a ```source```, or ```package require``` command), but to send one-liners to SpecTcl. Normally, this would be used to set Tcl variables or invoke application specific commands.


## /spectcl/script

### Query parameters

* **command** (string) - Required.  command string to execute.

### Response format detail

The response generated is a generic response.

#### Sample Responses.

Rustogramer:
```json
{
    "status" :  "Script execution is not supported",
    "detail" : "This is not SpecTcl"
}
```

SpecTcl successful command completion:

```json
{
    "status" : "OK",
    "detail" : "<the result of the command>"
}
```

Failure:
```json
{
    "status" : "ERROR",
    "detail" : "<The result of the command>"
}
```