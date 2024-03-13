#  Preparing data for Rustogramer

## How Rustogramer differs from SpecTcl
The material in this chapter describes how to prepare a data set for Rustogramer.  This differs a bit from what you are used to if you came to Rustogramer from NSCLSpecTcl.

In [NSCLSpecTcl](https://docs.nscl.msu.edu/daq/newsite/spectcl-5.0/pgmguide/index.html), analyzing a data set required that you prepare a data analysis pipeline that you then used to create a customized version of NSCLSpecTcl. 

The event processing pipeline then ran each time you processed an event file to turn the raw event data into  a set of parameters that could then be histogramed.  If you had an error in your event processing pipeline, your modified SpecTcl could crash.   Furthermore, if you had to analyze an event file more than once, you would do this decode each time you analyzed that event file.

Rustogramer expects the processing that was done by the NSCLSpecTcl event processing pipeline to be done by an external program *that only runs once on each event file*.
While you can do this processing any way you want; we recommend you use the FRIB analysis pipeline as described [here](./chap2_1.md) to create the processed event data files expected by Rustogramer (as a side note, beginning with  version 5.13, NSCLSpecTcl can also take these files as input and bypass the event processing pipeline to go straight to histograming).

The FRIB analysis pipeline supports:
*  Paralelizing the decoding of events.
*  Taking input from a previous pass and extending the data (e.g. with computed parameters) to produce another data-set.
*  By combining these two mechanisms, you can compute that which can be parallelized in one process, which is run parallelized, and those which cannot (e.g. cross event computations) in another which is not parallelized.

