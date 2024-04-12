# /spectcl/spectrum

Requests from this domain manipulate spectra defined in Rustogramer (and SpecTcl).  The URI's in this domain are:

* [```/spectcl/spectrum/list```](#spectclspectrumlist)  List spectra.
* [```/spectcl/spectrum/delete```](#spectclspectrumdelete) Delete an existing spectrum.
* [```/spectcl/spectrum/create```](#spectclspectrumcreate) Create a new spectrum.
* [```/spectcl/spectrum/contents```](#spectclspectrumcontents) Get the contents (channel values) of a spectrum.
* [```/spectcl/spectrum/zero```](#spectclspectrumzero) Clear the contents of spectra.


## /spectcl/spectrum/list

Lists the properties of one or more spectra.  

### Query parameters

* **filter** (String) - optional parameter to limit the listing to onliy spectra with names that match the pattern specified by this parameter.  The pattern can include any of the bash filesystem matching characters such as ```*``` and ```?```.

### Response format detail

The response **detail** will be an array of spectrum definition structs.  

Fields in the response **detail** are:

* **id** (unsigned int) - Always present. An integer spectrum id.  This has more meaning in SpecTcl than it does for Rustogramer.
* **name** (string) - Always present, the name of the spectrum.
* **type** (string) - always present, the spectrum type.  See the ```spectrum``` command in  the [SpecTcl command reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) for the valid spectrum type strings.
* **parameters** (array of strings)  - Always present.  The list of parameters the spectrum dependson on.  With the exception of ```gd``` spectra, it is possible to determine which parameters are x and which are y.   In SpecTcl 5.14, the following fields were added that rustogramer has always had, to make determination simpler.
* **xparameters** (array of strings) - Always present.  List of x axis parameters.
* **yparameters** (array of strings) - Always present. List of y axis parameters.
* **axes**        (array of axis definitions) - This is always present.  It is usually simple to figure out which axes are which, however **xaxis** and **yaxis** were added for simplicity in SpecTcl 5.14 and were always provided in Rustogramer.  Each axis is a struct that contains the following fields:
    * **low** (float) axis low limit.
    * **high** (float) axis high limit.
    * **bins** (unsigned integer) NUmber of bins on the axis.
* **xaxis** (axis definition)  - X axis specification.
* **yaxis** (axis definition) - Y axis definition.  This is meaningless if there's no meaningful Y axis for the spectrum.  Note that summary spetctra have a Y axis that the user defines and an X axis that is determined by the number of parameters.
* **chantype** (string) - Channel type string.  For SpecTcl see the spectrum command in 
the [SpecTcl command reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) for the valid channel type strings.  Rustogramer adds the channel type ```f64``` which means that channel values are 64 bit floats.
* **gate** (String) - Present when the spectrum is gated (note that in SpecTcl, spectra start out gated on a True gate named ```-TRUE-```).

#### Sample Responses.

Here's what a 1-d spectrum loosk like (SpecTcl):

```json
{
    "status" : "OK",
    "detail" : [{
        "name"        : "raw.00",
        "type"        : "1",
        "parameters"  : ["event.raw.00"],
        "xparameters" : ["event.raw.00"],
        "yparameters" : [],
        "axes"        : [{
            "low"  : 0.000000,
            "high" : 1024.000000,
            "bins" : 1024
        }],
        "xaxis"       : {
            "low"  : 0.000000,
            "high" : 1024.000000,
            "bins" : 1024
        },
        "yaxis"       : {
            "low"  : 0,
            "high" : 0,
            "bins" : 0
        },
        "chantype"    : "long",
        "gate"        : "-TRUE-"
    }]
}
```

For Rustogramer the **gate** and **yaxis** values will be ```null``` and the **chantype** will be ```f64```


Here's a 2d ```word``` spectrum (SpecTcl):
```json
{
    "status" : "OK",
    "detail" : [{
        "name"        : "raw",
        "type"        : "2",
        "parameters"  : ["event.raw.00","event.raw.01"],
        "xparameters" : ["event.raw.00"],
        "yparameters" : ["event.raw.01"],
        "axes"        : [{
            "low"  : 0.000000,
            "high" : 1024.000000,
            "bins" : 512
        },{
            "low"  : 0.000000,
            "high" : 1024.000000,
            "bins" : 512
        }],
        "xaxis"       : {
            "low"  : 0.000000,
            "high" : 1024.000000,
            "bins" : 512
        },
        "yaxis"       : {
            "low"  : 0.000000,
            "high" : 1024.000000,
            "bins" : 512
        },
        "chantype"    : "word",
        "gate"        : "-TRUE-"
    }]
}
```

## /spectcl/spectrum/delete

Allows you to delete an existing spectrum.



### Query parameters

* **name** (string) this mandatory parameter is the name of the spectrum to try to delete.

### Response format detail

The response type is a generic response.


#### Sample Responses.


Successful deleteion (SpecTcl)

```json
{
    "status" : "OK"
}
```
Rustogramer will include an empty **detail** string field.

Attempt to delete a spectrum that does not exist (SpecTcl):

```json
{
    "status" : "not foUNd",
    "detail" : "raw"
}
```

For rustogramer this will look more like:
```json
{
    "status" : "Failed to delete raw",
    "detail" : "Some reason for the failure"
}
```

## /spectcl/spectrum/create

Allows you to create new spectra.

### Query parameters

As this was one of the first URI families to be implemented in SpecTcl, some of the query parameters are a bit oddly formatted.  You will need to understand the format of Tcl lists to understand how the query parameters work.

* **name** Name of the spectrum you want to create.
* **parameters** Tcl Lists of parameters in the form expected by the SpecTcl ```spectrum``` command described in the [SpecTcl command reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html).
* **axes** Tcl lists of axis definitions as described in the ```spectrum``` command section of the he [SpecTcl command reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) 
* **chantype** Channel type (required by SpecTcl, ignored by rustogramer who's channel typ e is alays ```f64```).  For SpecTcl channel types again, see the ```spectrum```command desribed in the he [SpecTcl command reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html).


This is a confusing enough description that I'll give a couple of URI examples for spectrum defintions; a 1d and a 2d spectrum

#### Sample URI for creating a 1d spectrum.

```url
.../spectcl//spectrum/create?name=test&type=1&parameters=event.raw.00&axes={0%201024%201024}
```

Note that the string ```%20``` is the URL escape for an ASCII space character.

#### Sample URI for creating a 2d spectrum

```url
http://localhost:8000/spectcl//spectrum/create?name=test2&type=2&parameters=event.raw.00%20event.raw.01&axes={0%201024%201024}%20{0%201024%201024}
```

Again, the string ```%20``` is the URI escape for an ASCII Space character.



### Response format detail

This request produces a generic response


#### Sample Responses.

Successful **detail** from SpecTcl:
```json
{
    "status" : "OK"
}
```
where Rustogramer will add a **detail** field with an empty string.

Failure for duplicate spectrum name (SpecTcl):

```json
{
    "status" : "command failed",
    "detail" : "Duplicate Key during Dictionary insertion\nKey was:  test2 Id was: -undefined-\n"
}
```
Note that the detail is the same as the error message produced by the SpecTcl ```spectrum``` command.


## /spectcl/spectrum/contents

Retrieves the contents of a spectrum.  SpecTcl only allows the retrieval of the entire spectrum while rustogramer supports returning only the data within a region of intersst.

### Query parameters

* **name** (string) required name of the spectrum to fetch.

If you want to get data only within a region of interest from a 1-d spectrum, you must also add:

*  **xlow** (float) - low limit of the region of interest.
* **xhigh** (float) - high limit of the region of interest.

For 2-d spectra, the region of interest is a rectangle defined by **xlow**, **xhigh** and

* **ylow** (float) - Low limit of the y axis of the region of interest.
* **yhigh** (float) - High limit of the y axis of the region of interest.

### Response format detail

Detail is a struct:

* **statistics*** is a structure that provides information aobut the spectrum over/underflows:
    *  **xunderflow** (unsigned) - the number of underlows in the X direction.
    *  **yoverflow**  (unsigned) - The number of overflows in the x direction.
    *  **yunderflow**  (unsigned) - If present, the number of undeflows in the Y direction.  For rustogramer e.g. 1-d spectrum this will be ``null``. For SpecTcl this will be missing.
    * **yunverflow** (unsigned) - if present, the number of overflows in the y direction.
* **channels** (array of Channel structs) Each element represents a channel with non-zero counts and contains:
    *  **x** (float) - the X bin number of the channel.
    *  **y** (float) - the Y bin number of the channel (meaningless for e.g. 1d spectra). Note that SpecTcl can omit this.
    *  **v** (float) - number of counts in the bin.


Note:  Rustogramer does not (yet) implement gettin gthe Statistics information.  As such, the x under/overflows will always be zero and the y over/underflows will be ```null```

#### Sample Responses.

Empty 1d spectrum from SpecTcl:

```json
{
    "status" : "OK",
    "detail" : {
        "statistics" : {
            "xunderflow" : 0,
            "xoverflow"  : 0
        },
        "channels"   : []
    }
}
```

From Rustogramer, there will also be **yunderflow** and **yoverflow** fields in **statistics** with ```null``` values.

1d spectrum with counts (from SpecTcl) excerpt:

```json
{
    "status" : "OK",
    "detail" : {
        "statistics" : {
            "xunderflow" : 0,
            "xoverflow"  : 1
        },
        "channels"   : [{
            "x" : 52,
            "v" : 1
        },{
            "x" : 61,
            "v" : 1
        },{
            "x" : 66,
            "v" : 1
        },{
            "x" : 75,
            "v" : 1
        },{
            "x" : 77,
            "v" : 1
        },{
            "x" : 79,
            "v" : 1
        }
        ...
        ]
    }
}
```

Excerpt of a 2d spectrum (from SpecTcl):

```json
{
    "status" : "OK",
    "detail" : {
        "statistics" : {
            "xunderflow" : 0,
            "xoverflow"  : 0
        },
        "channels"   : [{
            "x" : 1,
            "v" : 1
        },{
            "x" : 28,
            "v" : 1
        },{
            "x" : 31,
            "v" : 1
        },{
            "x" : 36,
            "v" : 1
        },{
            "x" : 47,
            "v" : 1
        },{
            "x" : 56,
            "v" : 1
        },{
            "x" : 64,
            "v" : 1
        }
        ...
    }
}
```
## /spectcl/spectrum/zero

Allows you to clear one or more spectra.

### Query parameters

* **pattern** optional pattern.  If supplied all spectra that match that *glob* pattern will be cleared.   If not provided the default value of ```*``` clears all spectra.

### Response format detail

The response is a generic response.

#### Sample Responses.

```json
{
    "status" : "OK",
    "detail" : ""
}
```


