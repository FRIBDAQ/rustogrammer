# /spectcl/analyze

This family of URIs control data analysis.

* [```/spectcl/analyze/start```](#spectclanalyzestart) Starts analysis
* [```/spectcl/analyze/stop```](#spectclanalyzestop) Stops analysis
* [```/spectcl/analyze/size```](#spectclanalyzesize) Sets the event chunksize for Rustogramer.


## /spectcl/analyze/start

Begins analyzing the attached data source.

### Query parameters

None supported

### Response format detail

Generic response.  SpecTcl always returns an ```OK``` status but Rustogramer has a few possibilities.



## /spectcl/analyze/stop

### Query parameters

None

### Response format detail

Genric response.

#### Sample Responses.

One possible error case is that analysis is not active.  Here's  a SpecTcl return for that:

```json
{
    "status" : "'stop' command failed",
    "detail" : "Run is already halted"
}
```

## /spectcl/analyze/size

Only supported by rustoramer.   Rustogramer is a highly threaded program.  During analysis, a reader thread reads data from the data source passing it on to a histograming thread.  Data communication is via Rust channels.   This URI allows you to set the number of events in a batch sent between the reader and histogramer.


### Query parameters

* **size** - Number of events in a chunk.

### Response format detail

Generic responses.
