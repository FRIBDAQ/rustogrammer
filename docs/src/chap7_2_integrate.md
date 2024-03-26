
# /spectcl/integrate requests

This request allows you to integrate regions of interest on 1-d or 2-d spectra.



## /spectcl/integrate

Request the integraion. The region of interest can be specfied:

*  1-d spectrum :
    *  as a low/high pair of floats.
    *  as a slice condition.
*  2-d spectrum :
    * As a set of x/y points that are closed to form a contour-like area of interest (insidedness is computed in the same way as it is for contours).
    * As a contour condition/gate.


### Query parameters

At least one set of the optional parameters that specify a region of interest must be present in the query parameters.

* **spectrum** (string) - Required - name of the spectrum to integrate.
* **gate** (string) - Optional Name of gate whose interior is integrated.
* For 1-d spectra only providing explicit limits:
    * **low** (float) - Low limit of region of interest.
    * **high** (float) - High limit of region of interest.
* For 2-d spectra only, providing an explicit ROI
    * **xcoord** (float) - X coordinates of points that define the ROI.
    * **ycoord** (float) - Y Coordinates of points that define the ROI.

Note that **xcoord** and **ycoord** must appear at least three times to define an area of interest.  These paramters are taken as defining an ordered set of coordinats so, for example:

```
...?xcoord=100&xcoord=200&xcoord=200&ycoord=100&ycoord=100&ycoord=150....
```

Defines the region of interest as a triangle with coordinates:
```
(100,100)
(200,100)
(200,150)
```

### Response format detail

The **detail** of the response provides the integration details.  Note there are slight differencess betwen SpecTcl and rustogramer;  The attributes of the object are:

* **centroid**  - Centroid of the integration.  For SpecTcl; integrating a 1d, this is a scaler, or a 2 element array if a 2d.  For rustogramer, this is always an array with one element for a 1-d spectrum and two elements for a 2-d.
* **fwhm** - Full width at half maximum under gaussian shape assumptions.  SpecTcl may be a scalar float or 2 element float array; while rustogramer is a one or two element array of floats.  Same as for **centroid** above.
*  **counts** (unsigned integer) - total counts inside the AOI.

#### Sample Responses.

SpecTcl 1-d success.

```json
{
    "status" : "OK",
    "detail" {
        "centroid" : 102.512,
        "fwhm" : 5.32,
        "counts": : 124567

    }
}
```
Rustogramer 1-d success.

```json
{
    "status" : "OK",
    "detail" {
        "centroid" :[102.512],
        "fwhm" : [5.32],
        "counts": : 124567

    }
}
```


2-d success.

```json
{
    "status" : "OK",
    "detail" {
        "centroid" :[102.512, 50.7],
        "fwhm" : [5.32, 7.66],
        "counts": : 124567

    }
}

Failure (SpecTcl)

Below, the word ```$command``` is the ```integrate``` command the REST handler generated:

```json
{
    "status": "'$command' failed",
    "detail":  "<reason for the failure>"
}
```
Failure (Rustogramer) - only either **low** or **high** were provided as query parameters:

```json
{
    "status": "If using limits both low and high must be provided"
    "detail" :
    "detail" {
        "centroid" :[0.0],
        "fwhm" : [0.0],
        "counts": : 0

    }
}
```