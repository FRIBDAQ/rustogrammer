# /spectcl/swrite requests

Provides access to the SpecTcl ```swrite``` command to write the contents of spectra to file.  Note that since it is SpecTcl or Rustogramer that is doing the actual write operation, file paths passed to this request must make sense in the filesystem seen by the server program.

## /spectcl/swrite

### Query parameters

* **file**  (string) - Required.  File path of the file in which the spectra are to be written.
* **format** (string) - Required.  Format in which the file should be written.   Valid format strings are:
    * ```ascii``` - SpecTcl ASCII format.  This is supported by both SpecTcl and Rustogramer.
    * ```binary``` - SMAUG binary format.  This is a binary format that should be considered deprecated.
    * ```json``` - JavaScript Object Notation.  This is supportd by Rustogramer and SpecTcl after version 5.13-012.  For a description of the JSON see [Format of JSON Spectrum contents files](./chap7_7.md).
* **spectrum** (string) - Requires at least one.  Each occurance of this query parameters adds a spectrum to the list of spectra that will be written to file.

### Response format detail

Response is a Generic Response object.

#### Sample Responses.

Rustogramer success:
```json
{
    "status" :  "OK",
    "detail" : ""
}
```
SpecTcl success:
```json
{
    "status" :  "OK"
}
```

One possible rustogramer failure, ```jason``` specified for format.:

```json
{
    "status" : "Invalid format type specification:",
    "detail" : "jason"
}
```

One possible SpecTcl failure the underlying ```swrite``` command failed:

```json
{
    "status" : "'swrite' command failed",
    "detail" : "<swrite command error message>"
}