# /spectcl/version requests

Provides information about the version and program. 


## /spectcl/version

In SpecTcl, version strings are of the form M.m-eee where M is called the *major* version, m the *minor* version and eee the *edit level*.  In rustogramer version strings are of the form M.m.e  

While the version strings differ in format, the fields present are the same.  

### Query parameters

none

### Response format detail

**detail** is a struct.  It has the following attributes:

* **major**  (unsigned) - the major version number of the program.
* **minor**  (unsigned) - the minor version number of the program.
* **editlevel** (unsigned) - the edit level of the program.
* **program_name** (string) - This is always present from Rustogramer and contains the string: ```Rustogramer```. It is only present in SpecTcl versions later than 5.14-013 when it contains the string ```SpecTcl```. Therefore the server program is 
    *  Rustogramer if **program_name** is present and contains ```Rustogramer```
    * SpecTcl if **program_name** is no present or is present and contains ```SpecTcl```


#### Sample Responses.

Rustogramer Version 1.1.0:

```json 
{
    "status" : "OK",
    "detail" : {
        "major" :1,
        "minor" :1, 
        "editlevel" : 0,
        "program_name" : "Rustogramer"
    }
}
```

SpecTcl 5.14-015:

```json
{
    "status" : "OK",
    "detail" : {
        "major" :5,
        "minor" :14, 
        "editlevel" : 15,
        "program_name" : "SpecTcl"
    }
}
```
SpecTcl 5.14-001; Note that **program_name** is missing from **detail**
```json
{
    "status" : "OK",
    "detail" : {
        "major" :5,
        "minor" :14, 
        "editlevel" : 1,
    }
}
```
