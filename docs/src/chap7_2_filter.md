# /spectcl/filter requests

SpecTcl filters output a reduced data set given an input set.  The output set is self-descsribing and can contain a limited parameters set as well as events that only make a specific gate true.

This domain of URIs is only supported by SpecTcl.  If attempted with Rustogramer, Generic responses of the form:

```json
{
    "status" : "/spectcl/filter/<specific> is not implemented",
    "detail" : "This is not SpecTcl"
}
```

Are returned where ```<specific>``` is the specific request and is one of:

* [```new```](#spectclfilternew) - Which SpecTcl uses to create a new filter.
* [```delete```](#spectclfilterdelete) - which delets an existing filter.
* [```enable```](#spectclfilterenable) - which enables an existing filter to write it's subset of data for future events.
* [```disable```](#spectclfilterdisable) - which disables an existing filter so that it will no longher write events.
* [```regate```](#spectclfilterregate) - which associates a different gate with an existing filter, changing the subset of events that will be written by the filter (when enabled).
* [```file```](#spectclfilterfile) - Which specifies a file on which filtered data will be written.
* [```list```](#spectclfilterlist) - which lists filters and their properties.
* [```format```](#spectclfilterformat) - which specifies an output format for a filter.


This family of URIs is a front end to the SpecTcl **filter** command documented in the
[SpecTcl command reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html)

## /spectcl/filter/new

Creates a new filter.  The filter is not associated with a file and cannot be enabled until it is.

### Query parameters

* **name** (string) - Mandatory Name to be given to the new filter.
* **gate** (string) - name of a gate that will select the events the filter will write whne it is enabled.
* **parameter** (string) - In general this occurs several times, once for each parameter you wish written by the filter. 

For example:

```
.../spectcl/filter/create?name=afilter&gate=alpha&parameter=alpha.energy&parameter=alpha.theta&parameter=alphas.phi&parameter=total.energy
```

Attempts to create a filter named ```afilter``` that will write events that make ```alpha``` true and will write the parameters
```alpha.energy```, ```alpha.theta```, ```alpha.phi``` and ```total.energy```

### Response format detail

The response is a generic respones.

#### Sample Responses.

Succesful request:

```json
{
    "status" : "ok"
}
```

Request that is missing a gate:

```json
{
    "status" : "Missing required query parameter: ",
    "detail" : "gate"
}
```

## /spectcl/filter/delete

Deletes a filter. Any open filter file is flushed and closed.

### Query parameters

* **name** (string) - Mandatory name of a filter to delete.

### Response format detail

Rsponses are generic responses.

#### Sample Responses.

Successful completion:

```json
{
    "status" : "OK"
}
```

Failure:

```json
{
    "status" :  "'filter -delete' command failed",
    "detail" : "<error message from the fileter -delete command>"
}

## /spectcl/filter/enable

Enable a filter.   Note that the filter must have a file associated with it for this to succeed.  Filters are created in the disabled state.  Once enabled, on subsequent events that make their gates true, they will write filtered data to file.

### Query parameters

* **name** (string) - mandtory filter name.

### Response format detail

Response is a generic response.


#### Sample Responses.

Success:
```json
{
    "status" : "OK"
}
```

Falure form:

```json
{
    "status" : "'filter -enable' command failed" 
    "detail" : "<Error message from SpecTcl filter commnand>"
}
```

## /spectcl/filter/enable

Enables a filter to write events.  Once a file has been associated with a filter it can be enabled to write events to that file. See also [/disable](#spectclfilterdisable)

### Query parameters

* **name** (string) - mandatory parameter - the name of the filter to enable.

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
    "status" : "'filter command' command failed" ,
    "detail" : "<error message from filter command>"
}
```
## /spectcl/filter/disable

THe filter specified flushes any buffered data to its output file; and no longer writes data unless it is later enabled without changing the output file.

### Query parameters

* **name** (string) - mandatory parameter specifies the filter.

### Response format detail

Generic format.


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
    "status" : "'filter -disable' command failed" ,
    "detail" : "<error message from filter command>"
}
```
## /spectcl/filter/regate

Changes the gate that determines which event are written to the filter.  Note as well that the filter will also dynamically reflects edits to its gate.

### Query parameters

* **name** (string) - mandatory parameter that specifies the filter to modify.
* **gate** (string) - mandatory parameter that specifies a new gate for the filter. While odd, it is not an error to specify the current gate.

### Response format detail

Generic reply.

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
    "status" : "'filter -regate' command failed" ,
    "detail" : "<error message from filter command>"
}
```
## /spectcl/filter/file

Sets the filter output file.  Note that any existing file is first closed.

### Query parameters

* **name** (string) - mandatory name of the filter.
* **file** (string) - mandatory path to the new output file:
    *  **file** is interpreted by SpecTcl an therefore must be a valid file path in the context of the server.
    *  If **file** exists, it will be ovewritten.
    *  A file must have been specified for a filter for it to be legally enabled.

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
    "status" : "'filter -file' command failed" ,
    "detail" : "<error message from filter command>"
}
```
## /spectcl/filter/list

Lists filters and their properties.

### Query parameters

* **pattern** (string) - Optional glob pattern. Only filters with names that match **pattern** will be included in the listing.  If omitted the pattern defaults to ```*``` which matches all filters.

### Response format detail

**detail** is an array of objects. The objects have the followig fields:

* **name** (string) - Name of the filter being desribed.
* **gate** (string) - Name of the gate applied to the filter.
* **file** (string) - File to which the filter writes its events. This could be an empty string if the filters is not yet associated with a file.
* **parameters** (array of strings) - Name of the parameters written to the filter for each event it writes.
* **enabled** (string) - Either ```enabled``` or ```disabled``` depending on the filter enabled status.
* **format** (string) - The format with which the filter is written. See [format](#spectclfilterformat) for more information about this.

#### Sample Responses.

Success - with a single filter: 

```json
{
    "status" : "OK",
    "detail" : [
        {
            "name" : "afilter",
            "gate" : "agate",
            "file" : "/home/ron/filterile.flt",
            "parameters" : [
                "param1",
                "param2",
                "param3"
            ],
            "enabled": "enabled",
            "format" : "xdr"
        }
    ]
}
```
This can only fail if **pattern** is an illegal glob pattern.


## /spectcl/filter/format

Sets the format of the filter output. By default this is ```xdr```, which is the built in filter file format.  The set of filter file formats can be extended.  This is described in the section ```Extending SpecTcl's filter file format``` in the [SpecTcl Programming Guide](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/pgmguide/index.html).

The format of the built in ```xdr``` filter format [is described here](https://docs.nscl.msu.edu/daq/spectcl/Programming/filterread.htm). Scroll down to the section ```Structure of a Filtered event file.```

### Query parameters

* **name** (string) - mandatory name of the filter to modify.
* **format** (string) - mandatory format selector.

### Response format detail

This is a Generic response.

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
    "status" : "'filter -format' command failed" ,
    "detail" : "<error message from filter command>"
}
```

