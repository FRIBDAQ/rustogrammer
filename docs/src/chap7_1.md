# Command Line Options


## Rustogramer command line options

Rustogramer supports a few command line arguments.  These arguments are inthe form of options that have a long form and, in some cases a short form.  Many options also have default values:


If you run rustogramer with the  --help option
(e.g. on linux /usr/opt/rustogramer/bin/rustogramer --help) you will get a summary of the command line options.

* --shm-mbytes (short form -s)  the value of this option is the number of megabytes of shared spectrum memory that will be created by rustoramer when it starts up.   The default value is 32 (32 Megabytes).  The maximum value is system dependent.  Rustogramer creates the shared memory using mmap backed up ordinary files.   This was, as near as I could tell, the only portable shared memory interface between windows and Linux.  If rustogramer exits cleanly (you tell it to exit using the GUI e.g.) these files get cleaned up by rustogramer.  If not, they are in the home directory on Linux and ```\Users\username``` on Windows where *username* is your windows username.  They will have names like ```.tmp```*6chars*   where *6chars* means 6 random characters.   Note as well that in addition to the requested spectrum memory size, rustogramer will allocate a significant amount of additional storage for spectrum descsriptions.  If your home directory is in a file system with quotas enforced, you'll need to have sufficient free quota to create these files.
* --rest-port (short form -r) the value of this option is the port on which rustogramer's REST server will listen for connections.  The default value of this option is ```8000```.  Where possible, you are encouraged to use the --rest-service option instead.
* --rest-service - provides  a service name which Rustogramer will advertise with the NSCLDAQ port manager.  If the NSCLDAQ port manager is not running; rustogramer will fail.  There is no short form and no default for this option.
* ---mirror-port - The value of this option is the port on wich rustogramer's mirror server will listen.  This has no short form and defaults to ```8001``` though again, where possible, you are encouraged to use --mirror-service (see below).
* --mirror-service - The value of this option is the service name that rustogramer will use to advertise the mirror servers.   This has no default.

Examples, assuming rustogramer is in the path:

```bash
rustogramer  --shm-mbytes 100 --rest-service RUSTO_REST --mirror-service RUSTO_MIRROR

rustogramer --rest-port 10000 --mirror-port 10001 -s 128
```


## GUI Command line options.

The GUI allows you to specify command line options that control how it connects to Rustogramer or SpecTcl.  These have short forms and long forms and, in some cases, default values.  The ```--help``` option will display a summary of the options e.g.
```bash
/usr/opt/rustogramer/bin/gui --help
```

*  --host (short option -H) the value of this option is the host in which rustogramer or SpecTcl are running.
*  --port (Short option -p), the value of this option is the numeric port on which rustogramer or SpecTcl is listening for REST requests.   Default is 8000
*  --service (Short option -s) the service on which the REST server of rustogramer or SpecTcl has advertised with the NSCLDAQ port manager if that's how it got its server port.
*  --user (short option -u)  Username under which Rustogramer/SpecTcl is running.  This is only needed if you are using the --service option to translate the port.  This defaults to your logged in user name.

Examples (assuming the gui is in the path):

```bash
gui --host localhost --service RUSTO_REST
gui --host localhost --port 8000
```

