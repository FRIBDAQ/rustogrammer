# /spectcl/sread requests

Requests that a spectrum contenst file be read.  Note that since it is the server itself that does the read, file paths specified must make sense in the context of that server.  This point is important if the client and server don't have a common view of the file system.  For example, systems that don't share filesystem NFS mounts or a native Windows client talking to a server running in a WSL or other type of virtual machine.

## /spectcl/shared

### Query parameters

* **filename** (string) - Required path to file to be read.  This must make sense in the server.
* **format** (string) - Required.  Format in which the file should be written.   Valid format strings are:
    * ```ascii``` - SpecTcl ASCII format.  This is supported by both SpecTcl and Rustogramer.
    * ```binary``` - SMAUG binary format.  This is a binary format that should be considered deprecated.
    * ```json``` - JavaScript Object Notation.  This is supportd by Rustogramer and SpecTcl after version 5.13-012.  For a description of the JSON see [Format of JSON Spectrum contents files](./chap7_7.md).
* **snapshot** (boolean) - Optional defaults to true.  If true spectra read from file are made as snapshot spectra. This means they will not increment:
    *  In SpecTcl snapshot spectra are spectra that are wrapped in a special container object that refuses to increment the spectrum.
    *  In Rustogramer snapshot spectra are just gated on a special ```False``` gate.
* **replace** (boolean) - Optional defaults to false.  If true, then if a spectrum is read with the same name as an existing spectrum, the existing spectrum is overwitten.  Otherwise a unique spectrum name is generated.
*  **bind** (boolean) - Optional defaults  to true.  If true the spectrum is bound to display shared memory.

### Response format detail

A generic responses is returned.

#### Sample Responses.

Rustogramer success: 
```json
{
    "status" : "OK",
    "detail" : ""
}
```

SpecTcl success:
```json
{
    "status" : "OK"
}
```

Rustogramer fails because the file does not exist:

```json
{
    "status" : "Failed to open input file: /no/such/file",
    "detail" : "No such file or device"
}
```

SpecTcl fails

```json
{
    "status" :  "'sread' command failed",
    "detail" : "<sread command error message>"
}
