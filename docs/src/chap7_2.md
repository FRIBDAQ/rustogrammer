# REST requests and responses


REST Requests look like URLs sent to a webserver with query parameters (in fact they *are* URLs sent to a web server).   Rustogramer's REST requests/responses were patterned after SpecTcl's.  Therefore all requests use URLs of the form:

```url
http://hostname:rest-port/spectcl/...
```
where
* hostname - is of course the host in which the rustogramer REST server is running.
* port - is the port on which the rustogramer REST server is listening for connections
* spectcl is the fixed string ```spectcl``` and
* ... is the remainder of the URL.

The remainder of the URL are divided into functional categories, for exmaple 

```
http://hostname:rest-port/spectcl/spectrum/...
```

requests manipulate spectra.


The number of request families is large.   Refer to the sidebar table of contents to home in on a particular family.

## Pre-requisites to understand this reference section:

*  You should understand the form of Uniform Resource Identifiers URIs; specifically query parameters and how to format them.
*  You should be able to read simple Java Script Object Notation (JSON) objects.