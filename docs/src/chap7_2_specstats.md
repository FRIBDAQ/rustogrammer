# /spectcl/specstats requests

Returns statistics about the underflows and overflows for spectra.

## /spectcl/specstats

### Query parameters

* **pattern** (string) - Optional pattern.  Only spectra whose names match the glob pattern are included in the listing.  The pattern defaults to ```*``` matching all names if not provided in the request.

### Response format detail

**detail** is an array of structs, one for each spectrum that matches the pattern.  Each struct has the following attributes:

* **name** (string) name of the spectrum.
* **undeflows** (array of u32) - two element array of number of underflows.  The first element are X axis overflows the second, Y axis overflows.
* **overflows** (array of u32) - two element array of number of underflows.  The first element are X axis overflows the second, Y axis overflows.

Note that SpecTcl, for one dimensional spectrim types will have a one element array for both **underflows** and **overflows** rustogramer will unconditionally use 2 element arrays but the second element of the array should be ignored for one dimensional spectrum types.

#### Sample Responses.

Rustogramer  a single 1-d spectrum matches:

```json
{
    "status" : "OK", 
    "detail" : [
        {
            "name" : "1-d-spectrum",
            "underflows" : [12, 0],
            "overflows":   [732, 0]
        }
    ]
}
```
Same result for SpecTcl:
```json
{
    "status" : "OK", 
    "detail" : [
        {
            "name" : "1-d-spectrum",
            "underflows" : [12],
            "overflows":   [732]
        }
    ]
}
```

Both Rustogramer an SpecTcl 2-d spectrum matches:

```json
{
    "status" : "OK", 
    "detail" : [
        {
            "name" : "2-d-spectrum",
            "underflows" : [12, 5],
            "overflows":   [732, 0]
        }
    ]
}
```

