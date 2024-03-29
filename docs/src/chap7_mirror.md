# Shared memory Mirror service

Shared memory provides low overhead, high speed access to the display memory of SpecTcl and Rustogramer, however it does only allow access to those data within a single machine, or virtual machine or persistent container. 
The mirror service, provided by both SpecTcl and Rustogramer allow software that is not necessarily running in the same system access to this shared memory in an efficient manner.  

In Linux systems, mirror clients operate by registering a local shared memory region with the server, and setting up a network pipeline to refresh the contents of that local shared memory from the server's shared memory.  Subsequent clients are able to detect, via the [/spectcl/mirror](./chap7_2-mirror.md) ReST rquest if a shared memory  mirror has already been set up and, if so, simply connect the requesting process to that mirror.

This section will document:

*  [The network application level protocol](#network-messages) the client and server exchange with each other.
*  [The SpecTcl client C++ software available](#client-software) for your application.

The actual structure of the shared memory is outside the scope of this document, however the SpecTcl header ```xamineDataTypes.h``` provides that information.  

## Network messages

The mirror client (the software settting up the mirror) and mirror server operate by exchanging binary messages.  This section descsribes the structure of these messages using C++ struct definitions.  These messages structures are also availble in the SpecTcl header file: ```MirrorMessages.h```

For non C++ authors; the mirror messages can assumed to be packed.  Note that the [CutiePie](https://github.com/FRIBDAQ/CutiePie) displayer includes a Python encapsulation of the mirror software as well as a C++ implementation for Linux and Windows.

### The message header

All messages have a message header as their first 64 bits.  The structure of this header is:

```c++
#include <stdint.h>
...
namespace Mirror {
...
    struct MessageHeader {
        uint32_t s_messageSize;
        uint32_t s_messageType;
    };
    ...
}
```


* **s_messageSize** -- is the total message size (including the header) in bytes.
* **s_messageType** -- is a message type which describes what, if any, payload might follow.  The currently defined types are defined symbolically (also in the Mirror namespace) in the ```MirrorMessages.h``` header as:
    * **MSG_TYPE_SHMINFO** (1) - The client is sending the server information about the shared memory section it is going to create on its local host.
    * **MSG_TYPE_REQUEST_UPDATE** (2) - The client is requesting an update of the contents of its local shared memory from the server's shared memory.
    * **MSG_TYPE_FULL_UPDATE** (3) - In response to a **MSG_TYPE_REQUEST_UPDATE** message, the server is sending a full update of the used part of the shared memory.   The shared memory consists of two subsections. A header describes the spectra that are held in the memory and a *channel soup* contains the actual channel values of the spectra described in the header.  The **MSG_TYPE_FULL_UPDATE** message contains both the header and the used part of the channel soup.
    * **MSG_TYPE_PARTIAL_UPDATE** (4) - If the mirror server determines that there have been no changes to the shared memory header since the client's last **MSG_TYPE_REQUEST_UPDATE** request, it will send only the channel soup part of the shared memory in this type of message.   Since header data seems relatively stable compared with channel data this can result in a bandwidth improvements for updates given that header data are rather substantial.


Any payload required by the messages immediately follows the header (withi no padding) and will described in subsequent sections.

### **MSG_TYPE_SHMINFO**

This message type should be sent to the mirror server once it realizes, by using the ```/spectcl/mirror``` ReST URI that it is the first mirror client in its host to register information about the shared memory section that it will create locally.  Other clients will retrieve this information via that ReST request and can simply map to that exising mirror on behalf of their clients.

The payload for this message is a textual memory key whose lenght is determined by the header's **s_messageSize**.  In the ```MirrorMessages.h``` header this is declared as:

```c++
#include <stdint.h>
...
namespace Mirror {
...
 
    typedef char MemoryKey[4];
...
}
```

Which is appropriate for SYSV shared memory keys, however, within the SpecTcl and Rustogramer mirror servers it can be any length to accomodate shared memory information for other types of shared memory systems.  See the
documentation of [/spectcl/mirror](./chap7_2_mirror.md), and [/spectcl/shmem](./chap7_2_shmem.md) for information about:

1.  How to obtain the shared memory keys that are in use.
2.  The meanings of the memory key values for various types of shared memory subsystem.


**NOTE:** Since windows sytems are generally considered personal desktops, the mirror clients don't bother to create a local shared memory but simply maintain the mirror within the private memory space of the client process, and the key is generated from the process id of the client.

### **MSG_TYPE_REQUEST_UPDATE**

This messagse has no payload.

### **MSG_TYPE_FULL_UPDATE** and **MSG_TYPE_PARTIAL_UPDATE**

The payloads of these messages are just the memory contents.  For a **MSG_TYPE_FULL_UPDATE** the payload can be  read directly into the local mirror memory.  For **MSG_TYPE_PARTIAL_UPDATE** the payload can be directly read into the spctrum soup part of the shared memory  (the **dsp_spectra** field of the **Xamine_Shared** type).

## How client software should work:

Client software will need to use both the ReST an Mirror services as the ReST API provides informational data the client will need.

* The first thing a client should do is make a ReST request of [/spectcl/mirror](./chap7_2_mirror.md) and see if the list of existing mirrors includes one for the local host.  Note that hosts may appear in the mirror list in many ways so you may need to do DNS requests to resolve the the host names in the returned data to IP addresses and compare those with the IP addresse(s) of the local system.   If a match is found the client should use the shmkey to map to the shared memory and return the pointer to the application.
* If it is necessary to create a shared memory region, the client will need to use the
    * Form a persistent connection to the mirror server.
    * use  [/spectcl/shmem/size](./chap7_2_shmem.md) to learn the size of the server's display memory.
    * Create a shared memory region of that size.
    * Send a **MSG_TYPE_SHMINFO** message to the mirror server.
    * Periodically send **MSG_TYPE_REQUEST_UPDATE** messages to the mirror server and use its response to to update the contents of the mirror.  Note tha the first update request should be sent and processed prior to returning the pointer to the mirror to the application code so that the shared memory has the correct contents prior to use.  You are guaranteed that first update response will be a **MSG_TYPE_FULL_UPDATE**
    * Maintain a connection to the mirror server as long as the mirror is required.  This is important because once the connection is closed, the mirror server will forget about the remote mirror as far as its replise to [/spectcl/mirror](./chap7_2_mirror.md).

## Client software

SpecTcl and Cutiepie provide mirror client software. 

*  SpecTcl provides a [mirrorclient](#the-mirrorclient-program) application (Linux only)
*  Both SpecTcl and CutiePie provide a [mirror client library](#the-mirror-client-library) for programs to use (Linux and Windows)

### The mirrorclient program.

The simplest way, in Linux to set up a mirror is to use the
```$SpecTclHome/bin/mirrorclient``` program.  If necessary, it will setup and maintain a mirror.  Your programs, can then use the [/spectcl/mirror](./chap7_2_mirror.md) ReST service to locate the shared memory of the mirror and map it. The mirrorclient program takes care of updating the mirror periodically.  It has the following command options:

* **--host** - Mandatory - the value of this option should be the host in wich the histogram server (SpecTcl or Rustogramer) is running
* **--restport** - Mandatory - the value of this option should be the ReST server port of the histogram server.  If the service is advertised via the DAQPort manager, this can also be the name of that service.
* **--mirrorport** - Mandatory - the value  of this option sould be the port on which the histogram's mirror server.  Once more if this is advertised inthe DAQPortManager, this can be the name of the service.
* **--user** - Optional - the name of the user that is running the histogram server.  This defaults to your login name on the client system.

Note that once the mirror client program has been run and is maintaining a mirror shared memory, the  [mirror client library](#the-mirror-client-library)  can be used to get a pointer to the mirror in your program.


### The mirror client library.

Both SpecTcl and CutiePie provide a mirror client library (actually in SpecTcl 5.14 and later, then library is incoroprated fromt he CutiePie source tree).  This is available on Linux and Windows (Cutipie can be installed on windows).

*  In SpecTcl installations, the library is in $SpecTclHome/lib/libMirrorClient.so and the headers are in $SpecTclHome/include/SpecTclMirrorClient.h
*  In CutiePie installations if the installation top level directory is $CUTIEPIE: 
   *  In Linux the library is in $CUTIEPIE/lib/libMirrorClient.so and the header is in #CUTIEPIE/include/SpecTclMirrorClient.hi
   *  In windows, the library is in $CUTIEPIE/Script/MirrorClient.dll  THe header is not installed but can be gotten from [The git repository for Cutiepie](https://github.com/FRIBDAQ/CutiePie/blob/main/main/mirrorclient/SpecTclMirrorClient.h).


The mirrorclient library header defines the entry points into the library (some parts omitted for brevity).

```c++
#ifdef __cplusplus
extern "C" {
#endif
/**
 *  getSpecTclMemory
 *     Returns a pointer to a SpecTcl display memory mirror.
 *
 *  @param host - the host in which SpecTcl is running.
 *  @param rest - The Rest service.  This can be a port number or an NSCLDAQ
 *                advertised service name.
 *  @param mirror - The mirror service.  This can be a port number or an NSCLDAQ
 *                advertised service name.
 *  @param user - If neither rest nor mirror are NSCLDAQ services, this optional argument
 *               is ignored.  If either is a service:
 *               *  A nullptr will qualifiy service discovery by the name of the
 *                  user running this program.
 *               *  Anything else is a username that's assumed to be running SpecTcl.
 *                  This supports, in a collaborative environment such as an
 *                  experiment, user a looking at spectra accumulated by the
 *                  SpecTcl run by user b.
 * @return void* - Pointer to the shared memory region that holds the mirror.
 * @retval nullptr - The mirror, for some reason, could not be created.
 */
EXPORT void*
getSpecTclMemory(
    const char* host, const char* rest, const char* mirror, const char*user = 0
);


/**
 * errorCode
 *    Can only be called after a failed call to getSpecTclMemory - returns
 *    the error code that describes the failure.  These are given symbolically
 *    towards the bottom of this file.
 *  
 *  @return int  - Error status from the failed getSpecTclMemory call.
 */
EXPORT int
Mirror_errorCode();

/**
 * errorString
 *     Returns a human readable description of the error from the code gotten
 *     via errorCode().
 *
 * @param code - the error code gotteen from errorCode().
 * @return const char*  - Pointer to the static error message string.
 */
EXPORT const char*
Mirror_errorString(unsigned code);

/*------------------------------------------------------------------------*/
/*  Error code symbolic values:                                           */

static const unsigned MIRROR_SUCCESS = 0;   // Successful completion.
static const unsigned MIRROR_NORESTSVC=1;   // REST service not advertised.
static const unsigned MIRROR_NOMIRRORSVC=2; // Mirror service not advertised.
static const unsigned MIRROR_CANTGETUSERNAME=3; // getlogin failed.
static const unsigned MIRROR_CANTGETSIZE=4;
static const unsigned MIRROR_CANTGETMIRRORS=5;
static const unsigned MIRROR_SETUPFAILED=6;
static const unsigned MIRROR_CANTGETHOSTNAME = 7;

```

Note that the ```extern "C"``` means that C or C++ code can call this.
There are just three entry points.  The comments in the header describe the paramters and return values. to expect.
A note, you should only call ```getSpecTclMemory``` once for each mirror.  Thus you might want to encapsulate getting the memory pointer in a singleton object:

```c++
class Memory {
    private:
        static void* m_pMemory;
        static Memory* m_pInstance;
        Memory(const char* host, const char* rest, const char* mirror, const char*user) {
            m_pMemory = getSpecTclMemory(host, rest, mirror, user);
        }
    public:
        static void* getMemory(const char* host, const char* rest, const char* mirror, const char*user = 0) {
            if (!m_pInstance) {
                m_pInstance = new Memory(host, rest, mirror, user);
            }
            return m_pInstance->m_pMemory;
        }
}

void* Memory::m_pMemory(nullptr);
Memory* Memory:m_pInstance(nullptr);
```


Then calling ```Memory::getMemory(....)``` is safe to call more than once, if needed