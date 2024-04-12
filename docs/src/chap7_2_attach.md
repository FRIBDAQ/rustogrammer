# /spectcl/attach

This set of URIs manipulates the attachment of a data source to the server.  The following URIs are provided:

* [```/spectcl/attach/attach```](#spectclattachattach) attaches a data source to the server. Note that any previously attached source is detached.
* [```/spectcl/attach/list```](#spectclattachlist) describes the data source attached tot he server.
* [```/spectcl/attach/detach```](#spectclattachdetach) detaches the current data source.




## /spectcl/attach/attach

Attaches a new data source to the server.  The server detaches any previously attached data source.

### Query parameters

* **type**  Type of data source to attach.  This can be one of:
    *  ```pipe``` (only supported by SpecTcl) data comes from a program started on the other end of a pipe.  The program must emit data to ```stdout```
    * ```file``` (supported by both)  data is read from a file.
* **source** Specifies the data source.  This depends on the data source type:
    * ```pipe``` A string containing the program and its arguments.  For example suppose you are attaching gzcat to uncompress a file named ./events.gz  this would be ```gzcat ./events.gz```
    * ```file``` Path to the file to attach e.g. ```./run-0000-00.evt```
* **size** optional size of reads done from the data source.  This defaults to ```8192``` if not provided.   Rustogramer ignores this but SpecTcl honors it.

### Response format detail

A Generic response is returned.


#### Sample Responses.
Success:

```json
{
    "status" : "OK",
    "detail" : ""
    
}
```

Failure:
```json
{
    "status" : "attach command failed",
    "detail" : "No such file or directory"
}
```

## /spectcl/attach/list

Queries what is attached to the server.

### Query parameters

No query parameters are supported/required

### Response format detail

A generic repsonse.  This always has **status**=```OK```

#### Sample Responses.

Attached to a file:

```json
{
    "status" : "OK",
    "detail" : "File: run-0001-00.evt"
}
```

## /spectcl/attach/detach

This method is only supported by Rustogramer.  It detaches the data source.

### Query parameters

None supported.

### Response format detail

A generic response.

#### Sample Responses.
Success:

```json
{
    "status" : "OK",
    "detail" : ""
    
}
```
