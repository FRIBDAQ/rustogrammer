# /spectcl/exit requests

This request is only supported at this time by Rustogramer.   Since Rustogramer never has a command processor, only a ReST request can be used to get it to exit cleanly.

Rustogramer sends a Generic response:

```json
{
    "status" : "OK",
    "detail" : ""
}
```

And then exits normally.  If Rustogramer exits abnormally, it most likely will leave behind the file that is used for  its shared display memory.