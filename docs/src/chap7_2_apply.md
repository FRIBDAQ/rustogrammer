# /spectcl/apply requests

Conditions are only useful when applied to a spectrum.  When a condition/gate is applied to a spectrum it is said to *gate that spectrum*.  This means that for events, which normally could increment the histogram, that increment will only occur if the gate is satisfied  (the condition is true for that event).

The ```/spectcl/apply``` domain of URIs allow you to apply unapply and list applications:

*   [```/spectcl/apply/apply```](#spectclapplyapply) - Applies a gate/condition to one or more spectra.
*   [```/spectcl/apply/list```](#spectclapplylist) - Produces a list of gates applied to spectra.
*   [```/spectcl/ungate```](#spectclungate) - Removes any gate a spectrum has.

 
 ## /spectcl/apply/apply

 Applies a gate to one or more spectra.

### Query parameters

* **gate** - Name of the gate to apply.
* **spectrum** A spectrum to apply the gate to.  In rustogramer, this can appear more than once; e.g. ```../spectcl/apply?gate=agate&spectrum=larry&spectrum=moe&spectrum=curly```
applies the gate ```agate``` to the spectra ```larry```, ```curly``` and ```moe```

### Response format detail

Generic rsponse


## /spectcl/apply/list

List the gates applied to spectra.

### Query parameters

* **pattern** - glob pattern that, filters the listing to only contain spectra that match the pattern.  If not supplied defaults to ```*``` and all spectra are listed.

### Response format detail

The detail is a vector of structs with the fields:

* **spectrum** - name of a spectrum.
* **gate** - Name of the gate applied to the spectrum.

In SpecTcl, spectra are always gated.  When reated they are gated by the ```-TRUE-``` gate which is always true.  In Rustogramer, spectra can be ungated in which case the **gate** field is ```null```

#### Sample Responses.


```json
{
    "status" : "OK",
    "detail" : [{
        "spectrum" : "raw.00",
        "gate"     : "-TRUE-"
    }]
}
```

This represents an ungated spectrum.

## /spectcl/ungate

Removes the gate from one or more spectra.

Note that in Rustogramer spectra can exist without gates.  In SpecTcl, all spectra are gated and this operation gates the specrat with the ```-Ungated-``` gate which  is a True gate.


### Query parameters

* **name** name of a spectrum to ungate.   This parameter an appear more than once and allows you to ungate more than one spectrum.


### Response format detail

Generic response

#### Sample Responses.

Here's an error return from SpecTcl attempting to ungate a spectrum ```event.raw.00``` that dos not exist:

```json
{
  "status": "'ungate' command failed",
  "detail": "{event.raw.00 {Failed search of dictionary by Key string\nKey was:  event.raw.00 Id was: -undefined-\n}}"
}
```
