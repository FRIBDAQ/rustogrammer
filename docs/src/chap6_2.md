# Python REST interface


They Python Rest interface provides an API and a sample GUI based on the capabilities of the SpecTcl Tree GUi for Rustogramer and SpecTcl.  We have already described the use of the GUI in [Using the Rustogramer GUI](http://localhost:3000/chapter_4.html).  Refer back there for very detailed documentation.  

This section is intended to get users started that want to write their own 
[Python](https://docs.python.org/3/) applications for Rustogramer or SpecTcl.

To use the REST API you must:
*  Add the package directory to the Python import path.
*  Import tha ```rustogramer_client```  package in your Python script.
*  Instantiate a ```rustogramer_client.rustogramer``` object with the connection parameters set to allow it to connect to the server.
*  Make requests through the object you created in the previous step.

Note that failed requests will raise a ```rustogramer_client.RustogramerException``` which is derived from the standard [Python Exception base class](https://docs.python.org/3/tutorial/errors.html).
Normally, the only exceptions you will see are those that heppen when the server has exited (remember each REST request forms and destroys a new connection to the server).

## Extending the import path and importing the module

In python there are two ways to extend the built-in module search path:
1.  Define the environment variable ```PYTHONPATH``` to be colon separated set of paths in which modules can be found.
2. Appending paths to the ```sys.path``` variable.

Note that in both Linux and Windows envirionments the Unix path separator (```/```) is acceptable and should be preferred.

My preference is to extend ```sys.path``` as it's less error prone.

Let's assume that there's an environment variable called RUSTOGRAMER_TOP that is pointing to the directory in which rustogramer was installed.  In Linux, 

```bash
$RUSTOGRAMER_TOP/share/restclients/Python
```

points to the Python Rest client directory while on Windows:

```bash
%RUSTOGRAMER_TOP%/restclients/Python
```

Points to the package.

Supposing that ```RUSTOGRAMER_TOP``` is defined here's a bit of scriptery that will extend the search path irregardless of the environment and import the ```rustogramer_client``` package

```python
import sys     # 1
import os

if sys.platform == 'linux':                                            # 2
    suffix = os.path.join('share', 'restclients', 'Python')
elif sys.platform == 'win32':
    suffix = os.path.join('restclients', 'Python')
else:
    print("Error - unsupported os:", sys.platform)                     # 3
    exit()

rust_top = os.getenv('RUSTOGRAMER_TOP')                                # 4
if rust_top is None:
    print('Error - you must define the "RUSTOGRAMER_TOP" environment variable')
    exit()

module_path = os.path.join(rust_top, suffix)                         # 5
sys.path.append(module_path)

import rustogramer_client
```


## Sample Script.

In the exmaple below, we create a rustogramer client object and get the version of the histogramer that is running:

```python
import sys
import os
def extend_path():                      
    if sys.platform == 'linux':                                            
        suffix = os.path.join('share', 'restclients', 'Python')
    elif sys.platform == 'win32':
        suffix = os.path.join('restclients', 'Python')
    else:
        print("Error - unsupported os:", sys.platform)                     
        exit()

    rust_top = os.getenv('RUSTOGRAMER_TOP')                               
    if rust_top is None:
        print('Error - you must define the "RUSTOGRAMER_TOP" environment variable')
        exit()

    module_path = os.path.join(rust_top, suffix)                                                        
    sys.path.append(module_path)


if __name__ == '__main__':
    extend_path()                       # 1
    from rustogramer_client import rustogramer, RustogramerException  # 2

    host = 'localhost'     # 3
    port=  8000

    try:                                                          # 4
        client = rustogramer({'host'=host, 'port'= port})         # 5
        versionInfo = client.get_version()                        # 6

        major = versionInfo['major']
        minor = versionInfo['minor']
        edit  = versionInfo['editLevel]
        if 'program_name' in versionInfo.keys():                 # 7
            name = versionInfo['program_name']
        else:
            name = '*unknown*'
        
        print('Runing program: ', name, 'Version', f'{}.{}-{}', major, minor, edit) # 8
    except RustogramerException as e:
        print("An operation failed", e)                          # 9
        exit()
```

1. We've extracted the program fragment that shows how to extend the import path [from the previous section](#extending-the-import-path-and-importing-the-module) into the function ```extedn_path``` here we call that in the main program.
2. This imports the names we need from the ```rustogramer_client``` module.
3. These are the connection parameters we will use.
4. Our code all runs in a try block so  that we can catch any exceptions raised by the client object, output them and exit.  If you want a bit more error granularity, you can encapsulate each client request in a ```try/except``` block, however that can make the code look a bit unwieldy.
5. This creates a client object.  The client supports both hard-coded ports an service lookup.  That's why the parameter to its constructor is a dict.   If you want to do service lookup, instead of providing the 'port' key, provide the 'service', 'pmanport' and optionally the 'user' keys to fully specify the service and how to contact the port manager ('pmanport' should normally be ```30000```) 
6. We ask the client to request and return the version of the server.   The returned value for all requests is a dict with keys that are specific to each request.  In this case we will have the following keys:
    *  'major' - the program major version.
    *  'minor' - the program minor version.
    *  'editLevel' - the version's edit level.
    *  'program_name'  Rustogramer will always provide this but older versions of SpecTcl will not.  If provided it is the name of the server program.
7. This section of code unpacks the version bits and pieces and the program name, providing the default value of ```*unknown*``` if the server did not provide it.
8.  Print out the program and version information.
9.  If a ```RustogramerException``` was raised this code is executed to print out the reason for the exception and exit the program (presumably in a real program there might be more code to follow.)