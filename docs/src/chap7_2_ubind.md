# /spectcl/unbind requests

For more information about spectrum binding, see [```/spectcl/sbind```](./chap7_2_sbind.md).

This domain of URIs supports a few methods for unbinding spectra.

* [```/spectcl/unbind/byname```](#spectclunbindbyname) - Unbind given the name of a spectrum.
* [```/spectcl/unbind/byid```](#spectclunbindbyid) - (SpecTcl only) - by spectrum id.
* [```/spectcl/unbind/all```](#spectclunbindall)  - Unbind all spectra.]

## /spectcl/unbind/byname

Give a spectrum name, removes it from spectrum memory.  The spectrum still exists and is incremented, however clients of the shared memory are blind to it.

### Query parameters

* **name** (string)  - mandatory name of the spetrum to unbind.

### Response format detail

A generic response is produced.

#### Sample Responses.

Success:

```json
{
    "status" : "OK",
    "detail" : ""
}
```

Failure 

```json
{
    "status": "Failed to unbind <spectrum-name>",
    "detail": "<reason unbind failed>"
}
```

## /spectcl/unbind/byid

This is only supported in SpecTcl.  Unbinds a spectrum from the shared memory given its spectrum id. 

### Query parameters

* **id** (unsigned) - mandatory parameter that provides the id of the spectrum to unbind.

### Response format detail

Generic response.

#### Sample Responses.

Rustogramer:
```json
{
    "status" : "Unbind by id is not implemented",
    "detail" : "This is not SpecTcl"
}

Spectcl success:

```json
{
    "status" : "OK"
}
```

## /spectcl/unbind/all

Unbinds all bound spectra from shared memory.

### Query parameters

No query parameters are supported.

### Response format detail

A generic response is returned.


#### Sample Responses.

```json
{
    "status" : "OK"
}
```