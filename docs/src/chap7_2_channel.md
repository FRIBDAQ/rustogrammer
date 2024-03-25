# /spectcl/channel requests

The requests in this domain support accessing single channels of a spectrum:

*  [/spectcl/channel/set](#spectclchannelset) allows you to set the value of a channel.
*  [/spectcl/channel/get](#spectclchannelget) provides the value of a channel

## /spectcl/channel/set

Sets the value of a single bin/channel in  a spectrum.

### Query parameters

* **spectrum** (string) - mandatory parameter that provides the name of the spectrum ot modify.
* **xchannel** (unsigned) - mandatory parameter that provides the bin on the X axis to set.
* **ychannel** (unsigned) - optional parameter that provides the bin on the Y axis to set for spectra with X and Y axes.  For spectra without a Y bin axis, this can be omitted.
* **value** (float) - mandatory paramter that provides the new value for the channel.

### Response format detail

The response is a generic respones.

#### Sample Responses.


Successful return:

```json
{
    "status":"OK",
    "detail":""
}
```

Failure (no such spectrum):

```json
{
    "status":"Unable to set channel: ",
    "detail":"No such spectrum: araw.04"
}
```

Failure (bad channel number):

```json
{
    "status":"Unable to set channel: ",
    "detail":"X index is out of range"
}
```

## /spectcl/channel/get

Returns the value of a channel of a spectrum.

### Query parameters

* **spectrum** (string) - mandatory parameter that provides the name of the spectrum ot modify.
* **xchannel** (unsigned) - mandatory parameter that provides the bin on the X axis to set.
* **ychannel** (unsigned) - optional parameter that provides the bin on the Y axis to set for spectra with X and Y axes.  For spectra without a Y bin axis, this can be omitted.

### Response format detail
The detail of this request, on success, is a floating point value (generally the float is a valid unsigned integer).

#### Sample Responses.


Succes:
```json
{
    "status":"OK",
    "detail":1234.0
}
```

Failure (bad channel):

```json
{
    "status":"Could not get channel: X index is out of range",
    "detail":0.0
}
```
While this shows the **detail** field to be zero, you should not rely on that.  If **status** is not ```OK``` you must ignore the **detail** field.

Failure (no such spectrum):

```json
{
    "status":"Could not get channel: No such spectrum 'araw.04'",
    "detail":0.0
}