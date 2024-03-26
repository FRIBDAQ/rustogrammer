# /spectcl/sbind requests

SpecTcl and rustogrramer maintain a shared memory into which spectra can be put.  Such spectra can be accessed by local display programs providing a high speed channel to send histogram data to the displayer.

Spectra placed in shared memory are said to be *bound* to shared memory.  In SpecTcl, there is no cost to binding spectra, the spectrum bins are moved into shared memory and histograming directly occurs in shared memory.  In Rustogramer, the underlying histograming engine does not allow this so channels are periodically copied o that shared memory.

Note that ```sbind``` has its origins in the original SpecTcl where the more natural ```bind``` collides with the Tk ```bind``` command for binding events in display elements to scripts.

The ```/spectcl/sbind``` URI domain has the follwing URIs:

* [```/spectcl/sbind/all```](#spectclsbindall) - Bind all spectra to display memory.
* [```/spectcl/sbind/sbind```](#spectclsbind sbind) - Bind a single spectrum to the display.
* [```/spectcl/sbind/list```](#spectclsbindlist) - List th current bindings.
* [```/spectcl/sbind/set_update```](#spectclsbindsetupdate) Rustogramer only, specifies the number of seconds between updates to the shared memory.
* [```/spectcl/sbind/get_update```](#spectclsbindgetupdate) Rustogramer only, returns the shared memory refresh rate.

