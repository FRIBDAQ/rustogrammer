# /spectcl/evbunpack requests

This domain of URIs is only available in SpecTcl.  It works with the dynamic event processing pipeline to configure an event processor that can be used with data that was emitted from the FRIB/NSCLDAQ event builder.   The idea is that you can use the [pipline manager](./chap7_2_pman.md) to create event processing pipelines which you then associated with specific source ids using  this set of URIs.

Operations supported are:

*  [/spectcl/evbunpack/create](#spectclevbunpackcreate) - Creating an event processor with pipeline slots for source ids.
*  [/spectcl/evbunpack/add](#spectclevbunpackadd) - Associate an existing event processing pipeline with an source id.
*  [/spectcl/evbunpack/list](#spectclevbunpacklist) - list the event builder event processors that have been created by this command.


For more information and background, see the **evbunpack** command in the 
[SpecTcl command reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/cmdref/index.html)

## /spectcl/evbunpack/create

Creates a new event unpacker for event built data.  You can think of the unpacker as having a slot for each possible source id. Initially, all slots are empty. 
Note that this operation creates and registers an event processor.  Such event processors can be put into pipelines just like any other event processor.  


### Query parameters

All parameters are mandatory

* ***name**  (string) - name of the event processing pipeline.  This must be unique.
* **frequency** (float) - Clock frequency of the timestamp.  This is used to create event builder diagnostic parameters.  The value of this parameter are in units of floating point MHz.  For examle 16.5  means 16.5MHz.
* **basename** (string) - Provides a basename for the diagnostic parameters.  For more information aobut the diagnostic parameters; see the documentation of ```CEventBuilterEventProcessor``` in the [SpecTcl Programming Reference](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/pgmref/index.html).

### Response format detail

The response is a generic response


#### Sample Responses.

Successful resopnse:

```json
{
    "status": "OK"
}
```
## /spectcl/evbunpack/add


Associates an exiting, registered event processor with a source-id.  Events with fragments that match the source id will invoke that pipeline, passed the fragment's payload.

### Query parameters

All parameters are mandatory.

* **evpname**  (string) - Name of an event processor made via e.g. [/spectcl/evbunpack/create](#spectclevbunpackadd).
* **source** (unsigned) - Source id that will be associated with the next parameter.
* **pipe** (string) - Name of a registered event processor that will be run to process fragments from **source** in each event.  Note this is a badly named parameter.


### Response format detail

The response is a generic response.  On failure, the **status** contains ```evbunpack addprocessor command failed``` with **detail** set to the error message from that command.


#### Sample Responses.

Success:
```json
{
    "status": "OK"
}
```

## /spectcl/evbunpack/list

Returns a list of event processors that are evbunpack event processors.

### Query parameters

* **pattern** (string) - Optional glob pattern that filters out the list to only those names which match the pattern.

### Response format detail

The **detail** is an array of strings.  Each element is the name of an event builder unpacker.

#### Sample Responses.

Success:

```json
{
    "status" : "OK",
    "detail" : [
        "s800",
        "lenda",
        "greta"
    ]
}
```