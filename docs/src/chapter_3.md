# Chapter 3 - Running Rustogramer

Rustogramer tries to be compatible with NSCLSpecTcl in several ways:

*  It supports the most commonly used spectrum types defined by SpecTcl
*  It supports the most commonly used condition (gate) types defined by SpecTcl
*  It implements a REST server that is highly compatible with SpecTcl's allowing clients to work both with SpecTcl and Rustogramer.
*  It implements a share display memory that has a structure that is compatible with SpecTcl's, though rather than placing that shared memory in a SYSV shared memory segment, the ```mmap``` subsystem which maps shared memory to ordinary files is used.
*  It implements a mirror server that allows remote clients to create a mirror of the display memory in remote systems.
*  It can either user hard coded port values for its servers or it can use the NSCLDAQ port manager software to allocated a named service.


For simplicity, we assume that rustogramer was installed in ```/usr/opt/rustogramer/version```
In practice, ```version``` in the path above would be a rustogramer version string of the form a.b.c where a,b,c are integers.

## Running rustogramer with hard coded server port values.

Before you take this approach note that you'll need to assign port numbers that are unique system wide.  That means you must avoid port numbers used by other instances of rustogramer you run in your system as well *those run by other users*.  This can be very difficult and is why we recommend that
you use the NSCLDAQ port manager to assign and advertise services.  See
[the next section](#using-the-port-manager-to-assign-server-port-values) below for a description of this process and the prerequisites it requires.

The rustogramer command is in the ```bin``` directory of the rustogramer installation tree and is named ```rustogrammer``` (note  the doubled m). The command accepts serveral options that are described fully in [Command Line Options](./chap7_1.md).  To run with manuall assigned ports you'll need:

* ```--shm-mbytes``` (which can be abbreviated ```-s```).  The value that follows this option are the number of megabytes of shared spectrum memory rustogramer will create.  Note that you must have sufficient disk quota in your home directory to support the creation of the shared memory file.  If not specified, this defaults to ```32```
* ```--rest-port``` (which can be abbreviated ```-r```).  Specifies the port on which the REST server will listen for connections.  This defaults to ```8000```.
* ```--mirror-port``` Specifies the port on which the shared memory mirror server listens for connections.  This defaults to ```8001```.

Typically, then to run rustogramer with hard coded port values you choose the amount of spectrum memory you will need, assign prot values for the REST and mirror servers and supply appropriate values to the options above.  Given the defaults; if ```rustogrammer``` is in your path:

```bash
rustogramer
```

is equivalent to 

```bash
rustogrammer --shm-mbytes 32 --rest-port 8000 --mirror-port 8001
```

## Using the port manager to assign server port values

The NSCLDAQ port manager can be run to reduce the work you have to do to ensure that rustogramer ports are uniquely chosen.  The port manager assigns free ports from a pool it maintains and associates them with a name (service name) and the user whose program made the request.  As long as the program holds the connection over which it asked for a port open, the port remains allocated to the requester and clients can interact with the port manager to translate service name/user pairs into port numbers.

Where possible, this is the recommended way to use rustogramer.   Rustogramer knows that the port manager itself has allocated port ```30000``` and can interact with it by making connections to that port. 

You still must take care that for every instance of rustogramer *you* run within a single system, you choose a unique service name.  You don't have to worry about choosing a unique name across all system users, but, if you run rustogramer more than once in a single system, you need unique service names for each rustogramer.

You can specify service names using the ```--rest-service``` and ```--mirror-service``` command line options rather than the ```---rest-port``` and ```--mirror-port``` options described in 
[Using hard coded values](#running-rustogramer-with-hard-coded-server-port-values) above.
There are no default values for these options.  Supplying these options will override any port values you may have otherwise specified.  Here's a sample command line:

```bash
rustogrammer --shm-mbytes 128 --rest-service RG_REST --mirror-service RG_MIRROR
```

It runs rustogramer creating a 128 Megabytes spectrum memor region, and allocates/advertises with the port manager:

*   RG_REST - for the REST server.
*   RG_MIRROR - for the mirror server.

At any run of rustogramer with these values the actual ports allocated may vary.  Note that if another user runs rustogramer with the same service names, there is no collision as the NSCLDAQ port manager qualifies the service name with a username so, if my username is ``rusty`` and another user named ``graham`` runs rustogramer in the same way, four distinct ports wil be allocated and advertised as:

```
| Service   |  user  | 
|--------------------|
| RG_REST   | rusty  |
| RG_MIRROR | rusty  |
| RG_REST   | graham |
| RG_MIRROR | graham |
```


If ```graham``` looks up the port ```RG_REST``` he'll get the port his rustogramer rserved and ```rusty``` will get the one she allocated.