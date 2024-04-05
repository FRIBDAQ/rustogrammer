# Python REST reference

Rustogramer also provides a Python ReST client.  This is an object oriented wrapper for the ReST requests supported by Rustogramer and SpecTcl.  In fact, the 
[GUI](./chapter_4.md) uses this ReST client library.


This section provides:

* Recipies for [Importing the ReST client](#importing-the-client) on Linux and windows.
* [Reference for the RustogramerException](#rustogramerexception-reference) exception class.
* [Reference for the rustogramer](#rustogramer-client-reference) client class.


## Importing the client

The issue to consider for being able to import the Python ReST is how to set up the import path given the installation base of the Rustogrammer package.  This is because the winddows and linux installer install these in different subdirectories.  Here is sample code that should work in both Windows and Linux to locate and import both the [RustogramerException](#rustogramerexception-reference) exception class and the [client class](#rustogramer-client-reference):


The code below assumes the environment variable RG_ROOT contains the top level installation directory for rustogramer.

```python
import os
import sys

linux_subdir   = "/share/restclients/Python"

rg_root = os.getenv("RG_ROOT")                  # 1
if rg_root is None:
    print("You must define the environment variable 'RG_ROOT'")
    exit()

if os.name == "nt":                             # 2
    import_path = os.path.join(rg_root, 'restclients', 'python')
elif os.name == "posix":
    import_path = os.path.join(rg_root, 'share', 'restclients', 'python')
else:
    print("Unsupported platform: ", os.name)

sys.path.append(import_path)                  # 3

from rustogramer_client import RustogramerException, rustogramer  # 4


```
The numbers in the explanatory text below refer to the numbered comments in the code fragment above.

1. This code fetches the definition of the environment variable ```RG_ROOT``` which is the top-level installation directory for Rustogramer.
2. Depending on the operating system platform, ```nt``` for windows and ```posix``` for unix/linux systems, the correct full import path is computed as the variable ```import_path```
3. The correct import path is added to the import library search list.
4. The rustogramer_client library elements are imported into the script.

## RustogramerException Reference

If an error is detected performing a transaction with the server, the rustogramer client will 
raise a ```RustogramerException```  this method is dervived from ```Exception```.  It includes an implemenation of the ```str``` method which allows it to be printable.  For example:

```python
< Code from the previous section to import the libraries: >

client = rustogramer({"host":"localhost", "port":8000})
try:
    version = client.get_version()
    ...
except RustogramerException as e:
    print("Interaction with the server failed:" e)
    exit(-1)
```

## Rustogramer Client Reference

The ```rustogramer_client.rustogramer``` class is the client for rustogramer's ReST interface.  Instantiating it provides a client object.  Invoking the methods of that object results in transactions.  Failing transactions raise a [RustogramerException](#rustogramerexception-reference) which, if not caught results in program termination.

* ```debug```The rustogramer class provides this class variable to turn on debugging.  This is initialized to ```False``` if set to be True, the class will output the URIs of the requests it makes. For example

```python
< stuff needed to import rustogramer >
rustogramer.debug = True    # I want debugging output.
```

Below we describe the clent methods.

### __init__ (constructor)
#### Description 
Constructs a new instance of the client object.  Note that the connection to the server is not tested.  Only performing actions on the server result in connections to the server as ReST is a single transaction protocol at that level.

#### Parameters
*  ```connection``` (dict)- This is a dict that decribes how the connection to the server will be done.  The keys determine how the connection is done and where:
    *  **host** (string) - Required - The host on which the server is running. This can be the DNS name of the system or a dotted IP address.
    * **port** (unsigned integer) - If using explicit port numbers the value of this key shoulid be the port number.
    * **service** (string) - if using NSCLDAQ service lookups, this is the name of the service.  In that case, **port** should not be present and **pmanport** must be provided.
    * **pmanport** (unsigned integer) - the port on which the NSCLDAQ port manager is listening. If service lookup is being used, this must be present. Normally, this will have the value ```30000```
    * **user** (string) - If using NSLCDAQ service lookups and a user other than the user you are running under registered **service** this should be the username of the user that did.

#### Returns

An instance of a ```rustogramer``` class.  Methods on this object can be called to perform operations with the server.  In general, those operations will return a dict that has keys **status** and **detail**  note that if **status** was not ```OK``` a ```RustogramerException``` will be raised. The useful information will be in the value of the **detail** key.

### apply_gate
#### Description
Applies a gate to one or more spectra.  The gate and spectrum must, of course already be defined.
#### Parameters
* *gate_name*  (string)- Name of the gate to apply.
* *spectrum_name* (string or iterable of strings) - If a single string, this is the name of the one spectrum to which *gate_name* will be applied.  If an iterable of strings, this will be e.g. a list of the names of the spectra to which the gate will be applied.
#### Returns
 The **detail** key of the the returned dict will have nothing.

### apply_list
#### Description
   Returns a list of gate applications.
#### Parameters
* *pattern* (Optional string defaults to ```*```) - A pattern that spectrum names must match to be inclded in the list.

#### Returns
The **detail** key of the returned dict is an iterable that contains dicts with the following keys:

* **spectrum** (string)- name of a spectrum.
* **gate**  (string)- Name of the gate applied to that spectrum.

### ungate_spectrum
#### Description

Remove any gate from one or more spectra.

#### Parameters
* names (string or iterable of strings) - If a single string, the spectrum with that name will be ungated.  If an iterable, all of the named spectra in the iterable will be ungated.

#### Returns

**detail** has nothing useful.


### get_chan
#### Description

Get the value of a spectrum channel.

#### Parameters
* *name* (string) - name of the specturm.
* *x*    (number) - X channel.
* *y*    (number, optional) - Y channel, only required if the spectrum has two axes.

#### Returns

**detail** contains a number  which is the number of counts in the specified bin of the spectrum.

### set_chan
#### Description
Sets the contents of a spectrum bin to the desired value.


#### Parameters
* *name* (string) - name of the specturm.
* *x*    (number) - X channel.
* *value* (number) - counts to set in the desired channel
* *y*    (number, optional) - Y channel, only required if the spectrum has two axes.


#### Returns

**detail** contains nothing useful.









