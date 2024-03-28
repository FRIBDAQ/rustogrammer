# /spectcl/ringformat request

This request allows the client to set the default ringitem version format.  Note that if either SpecTcl or rustogramer encounter a ring format ring item, this is overridden by the contents of that item.


## /spectcl/ringformat

### Query parameters

* **major** (unsigned) - Required.  Major vesion number of the format. 
* **minor** (unsigned) - Required for SpecTcl, ignored by Rustogramer.  The minor version  of the format.  This is ignored by Rustogramer because changing the format of ring items is grounds to increment the major version of NSCLDAQ.



### Response format detail

A generic response is returned.

#### Sample Responses.

Rustogramer success:
```json
{
    "status" : "OK",
    "detail": ""
}
```
SpecTcl success:
```json
{
    "status" : "OK",
}
```


