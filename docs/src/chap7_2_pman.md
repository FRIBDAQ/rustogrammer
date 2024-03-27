# /spectcl/pman requests

This domain of URIs only is supported by SpecTcl.  SpecTcl transforms raw event data into parameterized data via a logical *analysis pipeline*.  The pipeline consists of stages called *event processors*.  Each event processor has access to the raw event as well as the unpacked parameters at that stage of the pipeline.  As such, event processors can, not only decode raw data into parameters, but create computed parameters independent of the format of the raw data.

Rustogramer is built to operate on decoded parameter sets rather than raw data so that the process of creating parameters does not have to happen over and over again for each analysis pass.  Therefore analysis pipeline manipulation makes no sense.

In SpecTcl 5.0 and later, commands and APIs were introduced to allow event processors to be incorporated and registered but not, necesarily, made part of the event processing pipeline in use.  The ```pman``` command, describedi n the [```SpecTcl Command Reference```](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html) is the user side of this SpecTcl subsystem. 

The requests in this URI domain provide support for dynamically composing event processing pipelines and selecting the pipeline to be used with the analyzed data.  One very simple use case for this would be to register the filter unpacker and make a pipeline for it while also making a raw event decoding pipeline.  One could then switch between processing raw and filtered data without modifying or switching SpecTcl by selecting the appropriate pipeline for the data set.


The following URIs are supported:


* [```/spectcl/pman/create```](#spectclpmancreate) - Create a new, empty, event processing pipeline.
* [```/spectcl/pman/ls```](#spectclpmanls) - List just the names of the event procesing pipelines currently defined.
* [```/spectcl/pman/current```](#spectclpmancurrent) - Return the  currently selected pipeline.
* [```/spectcl/pman/lsall```](#spectclpmanlsall) - List processing pipelines and the event processors in them.
* [```/spectcl/pman/lsevp```](#spectclpmanlsevp) - Lists the names of the event processors.
* [```/spectcl/pman/use```](#spectclpmanuse) - Select the current event processing pipeline.
* [```/spectcl/pman/add```](#spectclpmanadd) - Add an event processor to the end of an event processing pipeline.
* [```/spectcl/pman/rm```](#spectclpmanrm) - Remove an event processor from a pipeline.
* [```/spectcl/pman/clear```](#spectclpmanclear) - Remove all event processors from a pipe.
* [```/spectcl/pman/clone```](#spectclpmanclone) - Create a duplicat of an existing pipeline.

## /spectcl/pman/create

SpecTcl only - create a new event processing pipeline.  The pipeline will have no event processors.

### Query parameters

* **name** (string) - Name of the processor to create.

### Response format detail

Generic response

#### Sample Responses.

From rustogramer:

```json
{
    "status": "Pipeline management is not implemented",
    "detail": "This is not SpecTcl",
}
```

From SpecTcl success:

```json
{
    "status": "OK"
}
```

From SpecTcl failure: j

```json
{
    "status" :  "'pman mk' command failed",
    "detail" : "<Error message from pman mk command>"

}
```

## /spectcl/pman/ls

Lists just the names of the pipelines. To get more information, see  [/spectcl/pman/lsall](#spectclpmanlsall).

### Query parameters

* **pattern** (string) - Optional glob pattern.  Only pipeline names that match that pattern will be listed. If not supplied, the pattern defaults to ```*``` which matches everthing.

### Response format detail

**detail** is an array of strings.  Each string is the name of a pipeline.


#### Sample Responses.

Rustogramer:
```json
{
    "status" : "Pipeline managment is not implemented - this is not SpecTcl",
    "detail": []
}
```

SpecTcl success:

```json
{
    "status" : "OK", 
    "detail" : [
        "raw-to-parameters",
        "filter"
    ]
}
```

SpecTcl failure gives a generic response:

```json
{
    "status" : "'pman ls' command failed",
    "detail" : "<Error message from the pman ls command>"
}
```

## /spectcl/pman/current

Provide information about the currently selected event processor.

### Query parameters

No query parameters are supported.

### Response format detail

**detail** is an object with attributes:

* **name** (string) - pipeline name.
* **processors** (array of strings) - Names of the processors in the current pipeline.  Note that the array element order is the same as the pipeline order.

#### Sample Responses.

Rustogramer (Generic response):

```json
{
    "status" : "Pipeline management is not implemented",
    "detail" : "This is not SpecTcl",
}
```

SpecTcl success:

```json
{
    "status" "OK",
    "detail" {
        "name" : "raw-to-parameters", 
        "processors" : [
            "subsystem-1",
            "subsystem-2",
            "correlations",
            "computed-parameters"
        ]
    }
}
```

SpecTcl failure (Generic response):

```json
{
    "status" : "'pman current' command failed",
    "detail" : "<Error message from pman current command>"
}

```

## /spectcl/pman/lsall

Provide detailed listings of event processing pipelines.

### Query parameters

* **pattern** (string) - Optional glob pattern.  The event processors listed must have names that match the pattern.  If not provided, pattern defaults to ```*``` which matches everything.

### Response format detail

The **detail** is an array of objects.  Each object has the attributes:

* **name**  (string) - pipeline name.
* **processors** (array of strings) - Names of the event processors in the pipeline in the order in which they will be called.

#### Sample Responses.

Rustogramer
```json
{
    "status" : "Pipeline management is not implemented - this is not SpecTcl",
    "detail" : []
}


SpecTcl success:

```json
{
    "status" : "OK", 
    "detail" : [
        {
            "name" : "raw",
            "processors" : [
                "subsystem-1",
                "subsystem-2",
                "correlations",
                "computed-parameters"
            ]
        },
        {
            "name" : "filter",
            "processors": [
                "filter-unpacker"
            ]
        }
    ]
}
```

## /spectcl/pman/lsevp

List the names of event processors.

### Query parameters

* **pattern** (string) - Optional glob pattern.  Only event processors that match the pattern will be listed.

### Response format detail

**detail** is an array of strings that are the names of event processors.

#### Sample Responses.

Rustogramer
```json
{
    "status" :  "Pipeline management is not implemented - this is not SpecTcl", 
    "detail" : []
}
```

Success from SpecTcl:
```json
{
    "status" : "OK",
    "detail" :  [
        "subsystem-1",
        "subsystem-2",
        "correlations",
        "computed-parameters",
        "filter-unpacker"
    ]
}
```
Failure from SpecTcl (generic response):

```json
{
    "status" : "'pman ls-evp' command failed",
    "detail" : "<error message from pman ls-evp command>"
}
```
## /spectcl/pman/use 

Select the current event pipeline

### Query parameters

* **name** (string) - Name of the event processing pipeline to make current.

### Response format detail

Generic response.


#### Sample Responses.

Rustogramer:

```json
{
    "status" : "Pipeline management is not implemented",
    "detail" : "This is not SpecTcl"
}
```

SpecTcl success:

```json
{
    "status" : "OK"
}
```

SpecTcl Failure:

```json
{
    "status" : "'pman use' command failed",
    "detail" : "<error message from pman use>"
}
```

## /spectcl/pman/add

Adds a new event processor to an event processing pipeline.  The new processor is added to the end of the pipeline.
Note that if the pipeline being edited is current the effect on event processing is immediate.


### Query parameters

* **pipeline** (string) - Mandatory name of the pipeline to be edited.
* **processor** (string) - Mandatory name of the event processor to append to the pipeline.  Note that a processor can be part of more than one pipeline of the application requires it.

### Response format detail

Generic response.


#### Sample Responses.

Rustogramer:

```json
{
    "status": "Pipeline management is not implemented",
    "detail": "This is not SpecTcl"
}
```

SpecTcl success:

```json
{
    "status" : "OK"
}
```

SpecTcl Failure:

```json
{
    "status" : "pman 'add' command failed",
    "detail" : "<error message from pman add command>"
}
```
## /spectcl/pman/rm 

Remove an event processor from a pipeline.  If the pipeline is currently in use, the effects on event processing are immediate.

### Query parameters

* **pipeline** (string) - mandatory name of the pipeline to modify.
* **processor** (string) - mandatory name of the event processor to remove from the pipeline.

### Response format detail

The response is a generic response.


#### Sample Responses.

Rustogramer: 

```json
{
    "status": "Pipeline management is not implemented",
    "detail": "This is not SpecTcl"
}
```

SpecTcl success:

```json
{
    "status" : "OK"
}
```

SpecTcl Failure:

```json
{
    "status" : "'pman rm' command failed",
    "detail" : "<error message returned by pman rm command>"
}
```

## /spectcl/pman/clear

Removes all of the processors from an event processing pipeline.  If the pipeline is currently in use, the effect is immediate and could be disaastrous.

### Query parameters

* **pipeline** (string) - Mandatory name of the pipeline to clear.

### Response format detail

Generates a generic response.


#### Sample Responses.
Rustogramer: 

```json
{
    "status": "Pipeline management is not implemented",
    "detail": "This is not SpecTcl"
}
```

SpecTcl success:

```json
{
    "status" : "OK"
}
```

SpecTcl Failure:

```json
{
    "status" : "'pman clear' command failed",
    "detail" : "<error message returned by pman clear command>"
}
```

## /spectcl/pman/clone 

Sometimes it's useful to take an exising event processing pipeline as a starting point for a new one.  The clone request creates a new event processing pipeline that is a duplicate of an existing one.

### Query parameters

* **source** (string) - Mandatory name of an existing pipeline to clone.
* **new** (string) - Name of a new pipeline to create that will be a duplicate  of the source.

### Response format detail

A generic response is produced.

#### Sample Responses.


Rustogramer: 

```json
{
    "status": "Pipeline management is not implemented",
    "detail": "This is not SpecTcl"
}
```

SpecTcl success:

```json
{
    "status" : "OK"
}
```

SpecTcl Failure:

```json
{
    "status" : "'pman clone' command failed",
    "detail" : "<error message returned by pman clone command>"
}
```
