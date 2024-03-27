# /spectcl/sbind requests

SpecTcl and rustogrramer maintain a shared memory into which spectra can be put.  Such spectra can be accessed by local display programs providing a high speed channel to send histogram data to the displayer.

Spectra placed in shared memory are said to be *bound* to shared memory.  In SpecTcl, there is no cost to binding spectra, the spectrum bins are moved into shared memory and histograming directly occurs in shared memory.  In Rustogramer, the underlying histograming engine does not allow this so channels are periodically copied o that shared memory.

Note that ```sbind``` has its origins in the original SpecTcl where the more natural ```bind``` collides with the Tk ```bind``` command for binding events in display elements to scripts.

The ```/spectcl/sbind``` URI domain has the follwing URIs:

* [```/spectcl/sbind/all```](#spectclsbindall) - Bind all spectra to display memory.
* [```/spectcl/sbind/sbind```](#spectclsbindsbind) - Bind a single spectrum to the display.
* [```/spectcl/sbind/list```](#spectclsbindlist) - List th current bindings.
* [```/spectcl/sbind/set_update```](#spectclsbindset_update) Rustogramer only, specifies the number of seconds between updates to the shared memory.
* [```/spectcl/sbind/get_update```](#spectclsbindget_update) Rustogramer only, returns the shared memory refresh rate.

## /spectcl/sbind/all

Binds all spectra to display memory.

### Query parameters

No paramters are supported.

### Response format detail

A generic response is returned.

#### Sample Responses.

```json
{
    "status": "OK",
    "detail" : ""
}
```

Failure is possible for example, if there is not sufficient free space in the shared memory region to accomodate all of the spectrum channels.  An error return from Rustogramer might look like

```json
{
    "status": "Unable to bind spectrum <aspectrum-name>",
    "detail": "<reason the bind failed>"
}
```


## /spectcl/sbind/sbind

Bind some spectra to the display memory.

### Query parameters

* **spectrum** (string) - Mandatory.  Names a spectrum to bind to the  display memory.  Note that if this query parameter appears more than once, all mentioned spetra will be bound.

### Response format detail

The response is a generic response.

#### Sample Responses.

Success:

```json 
{
    "status": "OK",
    "detail" : ""
}
```

Failure:

```json
{
    "status": "Unable to bind spectrum <aspectrum-name>",
    "detail": "<reason the bind failed>"
}
```

## /spectcl/sbind/list

List the spectrum bindings.

### Query parameters

* **pattern** (string) - Optional glob pattern, only bindings for spectra with names that match the pattern will be listed.   The pattern defaults to ```*``` which matches all spectra.

### Response format detail

The **detail** is a vector of objects with the following attributes:

* **spectrumid** (unsigned)- A number associated with the spectrum (not really useful in most cases).
* **name**  (string) - Name of the spectrum.
* **binding** (unsigned) - The shared memory slot number containing the spectrum's description.

#### Sample Responses.

Success with a single matching spectrum in slot 6:

```json
{
    "status" : "OK",
    "detail" : [
        {
            "spectrumid" : 12,
            "name"       : "a-spectrum",
            "binding"    : 6
        }
    ]
}
```

## /spectcl/sbind/set_update

Available only on Rustogramer.  Provides the refresh period in seconds for the shared memory.  In SpecTcl, since histograms are directly incremented in display memory for bound spectra, this is not needed, however in Rustogramer, spectrum contents in shared memory must be refreshed from their histograms

### Query parameters

* **seconds** (unsigned int) - Mandatory.  Provdes a new update period in seconds.

### Response format detail

A generic response.


#### Sample Responses.

If attempted in SpecTcl you will get a ```404``` error from the server indicating there is no URL match.

Success: 

```json
{
    "status" : "OK",
    "detail" : ""
}
```
## /spectcl/sbind/get_update

Rustogramer only Queries the shared memor refresh period. 

### Query parameters

No query parameters are supported.

### Response format detail

The **detail** attribute is an unsigned integer that is the number of seconds between spectrum contents refreshes.


#### Sample Responses.

```json
{
    "status" : "OK",
    "detail" : 2
}
```

The spectrum  memory is refreshed every ```2``` seconds.