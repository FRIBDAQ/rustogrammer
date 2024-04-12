# Shared memory Mirror service

SpecTcl maintains a list of all of the mirror clients that have registered with it.  See the [reference documentation about the mirror server](./chap7_mirror.md) for more information about the mirror server protocol in general and mirror registration specifically.

The registration of a mirror client provides the host on which the client runs and the shared memory it created.  SpecTcl's mirror client application and API use this to share mirrored shared memories between clients.   The mirror client either establishes a new mirror, if one does not yet exist for the host, or maps to the existing mirror if there already is one.

Rustogramer provides this service as well.  Note that in windows, it's assumed there's typically only one client so the client code unconditionally creates a new mirror internal to the process that requests it.

## /spectcl/mirror

Returns a list of the mirrors being remotely maintained.

### Query parameters

No query paramters are supported.

### Response format detail

The **detail** is an array of structs where each struct describes a mirror registration and contains the following attributes:

*  **host** (string) - DNS name or IP address in dotted notation of the host maintaining a mirror.
*  **shmkey** (string) - Shared memory identifier.  See the [shmem requests for information about this](./chap7_2_shmem.md)

#### Sample Responses.
  
```json
{
    "status" : "OK",
    "detail" : [
        {
            "host" : "some.host.at.adomain",
            "shmkey" : "Xa3b"
        }
    ]
}
```