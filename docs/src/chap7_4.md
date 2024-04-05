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