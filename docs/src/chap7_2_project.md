# /spectcl/project requests

The ```/spectcl/project``` URI crates a new spectrum by projecting a 2-d spectrum (e.g.  ```2``` or ```g2``` ...) onto one of its axes.  Optionally the projection can be inside an area of interest specified by a contour.  The new spectrum can either be a snapshot spectrum, in which case it is never incremented after being created, or an ordinary spectcrum, in which case it will be incremented if possible.

Snapshot Spectra are handled differently betweeen SpecTcl and Rustogramer.  SpecTcl snapshot spectra are 1-d spectra that are wrapped in a container that prevents them from being incremented.  Rustogramer snapshot spectra are created by gating them on a ```False``` gate.  This also implies that a snapshot spectrum, in Rustogramer can be turned into an ordinary spectrum by ungating it, while a SpecTcl snapshot cannot.

## /spectcl/snapshot



### Query parameters

*  **source** (string)  - Mandatory name of the spectrum to project.
*  **newname** (string) - Mandatory name of the new spectrom to create.
*  **snapshot** (boolean) - Mandatory, if true a snapshot will be created. For SpecTcl any boolean Tcl value can be used.  For Rustogramer;
    * True values are any of ```Yes```, ```yes```, ```True``` or ```true```
    * False values are any of ```No```, ```no```, ```False``` or ```false```
* **direction** (string) - Mandatory direction selector indicating which direction the projectionis onto. One of:
    *  Onto the X axis if ```X``` or ```x```
    *  Onto the Y axis if ```Y``` oe ```y```
* **contour** (string) - Optional.  If supplied this must be a contour that is displayable on the spectrum and the projection will be inside the contour.  If the resulting spectrum is not a snapshot, it will be gated on the contour.  Thus if the contour is modified after the projection, the manner in which the spectrum is incremented will no longer be faithful to the original projection.
* **bind** (boolean) - Optional.  If supplied and ```false``` the new spectrum is not bound into display memory. If not supplied or ```true``` it is. 



### Response format detail

A generic response is produced.


#### Sample Responses.

Success (Rustogramer)

```json
{
    "status" : "OK",
    "detail" : ""
}
```

Failure from Rustogramer:
```json
{
    "status" : "Could not bind projected spectrum",
    "detail" : "<reason the projection failed>"
}
```

Failure from Spectcl

```json
{
    "status" : "'project' command failed: ",
    "detail" : "<error message from the project command>"
}
```