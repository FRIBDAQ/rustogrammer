# /spectcl/fold requests

The ```/spectcl/fold``` request domain creates and manipulates *folds*.  Folds are used in &gamma;-ray spectroscopy to untangle decay chains.   SpecTcl and Rustogramer both support multiply-incremented spectrum types that are tailored for these sorts of experiments.

The Rustogramer mutiply-incremented 1-d/SpecTcl g1 spectrum can be used with an array of &gamma; detectors; incremented for each &gamma; ray detected by the array.  In decay cascades, if fully captured, each decay will increment the spectrum.   A decay chain will result in a set of correlated increments.

A fold is like a conditiont that, when applied to a  g1 spectrum, will increment all hits that are *not* in the condition.  Suppose, therefore, that you know the peak that corresponds to one of the decays in the sequential decay.  If you set a condition (slice) around that peak and apply that as a fold, and gate the spectrum on that condition as well, the spectrum will only show peaks that are in coincidence with the peak used to fold the spectrum.  These are the other decays in the sequential decay chain.

The following operations are defined on folds:

*  [```/spectcl/fold/apply```](#spectclfoldapply) - Apply a condition as a fold to a spectrum
*  [```/spectcl/fold/list```](#spectclfoldlist) - List the folds.
*  [```/spectcl/fold/remove```](#spectclfoldremove) - Remove a fold.

## /spectcl/fold/apply

Apply a fold to a spectrum.  Note that folds can only be applied to an appropriate spectrum type. 

### Query parameters

* **gate** (string) - Mandatory name of the condition/gate to use as a fold.
* **spectrum** (string) - Mandatory name of the spectrum to fold with this gate.

### Response format detail

The response is a generic response.

#### Sample Responses.

Success:

```json
{
    "status": "OK"
}
```

Failure (Spectcl):

```json
{
    "status": "'fold -apply' command failed",
    "detail": "<erorr message from fold -apply>"
}
```
Failure (Rustogramer):
```json
{
    "status": "Could not fold spectrum",
    "detail": "<why the fold failed>"
}
```

## /spectcl/fold/list

List the folds that are applied to spectra.

### Query parameters

* **pattern** (string) - optional glob pattern to filter the listing to only spectra with names that match the pattern.  If omitted, the pattern defaults to ```*``` which matches all spectra.



### Response format detail

The **detail** is a vector of objects.  Each object has the following attributes:

* **spectrum** (string) - the name of a spectrum.
* **gate** (string) - the fold applied to the spectrum.

Note that the listing will only contain spectra that match the **pattern** and have a fold applied.

#### Sample Responses.

Success with a spectrum named **gamma** folded on a condition named **peak** and no other folded spectra that match whatever the pattern was:


```json
{
    "status" : "OK",
    "detail" : [
        {
            "spectrum" : "gamma",
            "gate"     : "peak"
        }
    ]
}
```

## /spectcl/fold/remove

Removes a fold applied to a spectrum.

### Query parameters

* **spectrum** (string) Mandatory name of the spetrum that will have folds removed.

### Response format detail

The response is a generic response

#### Sample Responses.

Sucess:

```json
{
    "status": "OK"
}
```

Failure (SpecTcl)

```json
{
    "status" : "'fold -remove' command failed: ",
    "detail" : "<fold -removce error message>"
}
```
Failure (Rustogramer)

```json
{
    "status" : "Failed to remove fold",
    "detail" : <reason the fold could not be removed>"
}
```
