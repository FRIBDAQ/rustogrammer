## The FRIB analysis pipeline

This document describes how to use the FRIB analysis pipeline to create data sets for Rustogramer.  Refer, as needed to full documentation of the [FRIB Analysis Pipeline](https://docs.nscl.msu.edu/daq/newsite/apipeline/index.html).

This section will walk through a simple example that uses the FRIB event processing pipeline (from now on called the pipeline for brevity) to decode some data from a fixed format event.

The key point is that the pipeline processes raw events from some file (e.g. event data acquired at the FRIB) to a simple self-describing data set that Rustogramr can understand without any further processing.  In [the next section](./chap2_2.md), we'll describe the format of the data expected by Rustogramer so that, if you want, you can use whatever other method you want to prepare your data.

Our example is going to show how to take event data which are stored in fixed length, fixed format events and use the pipeline to produce a rustogramer data-set.

The payload of the ringbuffer for each event will consist of something that looks like:

```
+----------------------+
| Value of parameter 1 | (32 bits)
+----------------------+
| Value of parameter 2 | (32 bits)
+----------------------+
| Value of parameter 3 | (32 bits)
+----------------------+
| Value of parameter 4 | (32 bits)
+----------------------+
```
Note that while the raw data contains 32 bit integer parameters; The file we generate will consist of double precision floating point values.  This accomodates computed parameters that are not integral.

The steps we need to follow are to:
*  Prepare a paramter definition file for the output stage of the pipeline
*  Write a worker and main for an events -> parameters pipeline.
*  Build a Makefile to build the program.
*  Run our application.   

For a more general and detailed look at this process, see the documentation on [Raw parameters to parmaeters](https://docs.nscl.msu.edu/daq/newsite/apipeline/md_pages_rawtoparameters.html#rawtoparams) in the FRIB analysis pipeline documentation.

### Preparing the parameter definition file

The purpose of the parameter definition file is to attach
names to data we unpack from the raw event data.  Rustogramer will allow you to refer to those names when constructing Spectra and conditions (*conditions* are what NSCLSpecTcl calls *gates*).

In our case, the data are array like so we're going to make an logical array of four parameters named; ```parameters.0 - parameters.3```

Let's make a file ```defintions.tcl``` and write the following in it:

```tcl
treeparameterarray parameters 0 4096 4096 arbitrary 4 0
```

The file is interpreted by a Tcl interpreter with some extended commands.  See [the description of this file](https://docs.nscl.msu.edu/daq/newsite/apipeline/md_pages_tcldeffile.html#tcldeffile) for a detailed description of these extended commands.

For the purpose of this example, we just need to know that the
```treeparamterarray``` command creates an array of parameters named as we described above.  The additional command line paramters provide, in order:

* A base name for the generated parameters.
* The suggested low limit for histogram axes on these parameters.
* The suggested high limit for histogram axes on this parmaeter.
* Suggested number of bins for histogram axes on these parameters
* Metadata descdribing the units of measure for these parameters.
* The number of parameters in the array.
* The base index of the array.  Had we specified ```1```,instead of 0, the parameters produced would be named
```parameters.1``` - ```parameters.4```

### Writing the Code

Recall that we need to write both a worker class and a main for the application.  

#### The worker class

The worker class will be given events to decode.  When you run the program you can determine how many workers will be run in parallel (between 1 and many).  Each worker runs in a separate processs.  There is some resemblance between the worker class and the NSCLSpecTcl event analysis pipeline.  This is no coincidence.

If you alread have an event processing pipeline from NSCLSpecTcl, you might want to look at [SpecTcl compatibility software](https://docs.nscl.msu.edu/daq/newsite/apipeline/md_pages_spectclworker.html#spectclworker) in the FRIB pipeline documentation.

We're going to ignore that.  Workers that decode raw event data are derived from the ```frib::analysis::CMPIRawToParametersWorker```class. Our worker has to construct in a way that it can bind some of its data to the parameters defined in the [previous section](#preparing-the-parameter-definition-file).  We then must override ```unpackData``` which actually unpacks a raw event into the parameters.  

We're going to assume that the input data are NSCLDAQ-11 ring items.  We're also going to assume that each ring item has a body header.  The pipeline ensures that ```unpackData``` only receives ```PHYSICS_EVENT``` ring items, the ring items that contain raw event data.

##### MyWorker.h
Our header looks like this:

```c++
#include <MPIRawToParametersWorker.h>
#include <AnalysisRingItems.h>               // 1
#include <CTreeParamteerArray.h>
#include <cstdint>
using namespace frib::analysis;               // 2
ckass AbstractApplication;
class MyWorker : public CMPIRawToParametersWorker {
private:
    CTreeParameterArray*  m_pParams;         // 3
public:
    MyWorker(AbstractApplication& app) :
        CMPIRawToParameterWorker(app),
        m_pParams(new CTreeParameterArray("parameters", 4, 0)) {} // 4
    ~MyWorker() {
        delete m_pParams;
    }
    void unpackData(const void* pData);
};
```

The numbered comments refer to the points below:

1.   The ```AnalysisRingItems.h``` header defines the shape of ring items as well as the types of the new ring items the pipeline creates in the output file.
2.   All of the pipeline names are encapsulated in the ```frib::analsyis``` namespace.  For simplicity we bring those names into the file namespace.  In a more complex application you might want to defer this action to the implementation of this class.
3.  The ```CTreeParameterArray``` class represents a binding between an array like object (implements indexing) of real number like objects and names.  We'll bind this to the names of the paramters we made in our
[parameter definition file](#preparing-the-parameter-definition-file)
4. The constructor simply initializes the ```m_pParams``` member data to point to a tree parameter array object that has the same base name we used in the ```treeparameterarray``` command in our parameter definition file.   This is all that's needed to bind this to those parameters.  Note that bindings to parameters are many-to-one; that is you can bind more than one variable to the same name.  This can be convenient when you compose your worker class with components objects.

The destructor simply deletes the parameter binding variable.
We also declare the ```unpackData``` method which we'll implement in a separate file.

#### MyWorker.cpp

This file contains the actual code for unpackData.  What it has to do is:
1.  Find the body header of the ring item.
2.  Skip the body header (or lack of one) if it exists.
3.  Unpack sequential ```uint32_t``` values from the ring item payload into m_pParams.

Here's one possible implementation:

```c++
#include <MyWorker.h>
void MyWorker::unpackData(const void* pData) {
    union {
        const RingItemHeader* u_pHeader;
        const std::uint8_t*   u_pBytes;
        const std::uint32_t*   u_pLongs;
    } p;
    p.u_pBytes = static_cast::<const std::uint8_t*>(pData); // 1
    p.u_pHeader++;                                          // 2
    std::uint32_t bodyHeaderSize = *(p.u_pLongs);
    if (bodyHeaderSize == 0) {
        bodyHeaderSize = sizeof(std::uint32_t);            // 3
    }
    p.u_pBytes += bodyHeaderSIze;                          // 4

    CTreeParameterArray& rParams(*m_pParams);
    for (int i = 0; i < 4; i++ ) {
        rParams[i] = *(p.u_pLongs);                       // 5
        p.u_pLongs++;
    }


}
```

The numbers in the list below refer to the numbered comments in the code sample above:
1. This initializes the union p to point to the raw data that was passed in.  We use a union because there are times when it's conveniion to refer to the data using all of the pointer types in the union and this is simpler than doing casts every time we need to use a different type of pointer.
2. Here's an example of what I meant in  1. above.  This allows us to skip over the ring item header simply by incrementing the ```u_pHeader``` element of the union ```p```.
3. Since all elements of the union occupy the same storage, the ```u_pLongs``` element points to the size of the body header.   In NSCLDAQ-11, this size is ```0``` if there is no body header.  In NSCLDAQ-12 or greater it is the size of a ```std::uint32_t```.  By the end of this block of code, ```bodyHeaderSize``` contains either the sizes of the body header or ```sizeof(std::uint32_t)```.
4.  Now, since we want to skip forwards ```bodyHeaderSize``` bytes, it's convenient to increment ```p.u_pBYtes``` to skip over either a body header or, if one is not present, the longword that indicates that.
5.  Now that the union is pointing at the body of the event, we just need to sequentially unpack the four ```std::uint32_t``` values into consecutive elements of the ```CTreeParameterArray``` that ```m_pParams``` points to.  Initializing the reference ```rParams``` makes this notationally simple.

A production version of this may be a bit more complex.  Specifically, it's probably a good idea to ensure that the total size of the ring item passed in is not smaller than the amount of data we reference.
#### The main program

The pipeline program we are writing consists of several processes:
1.  A dealer process is responsible for reading the raw event file and passing events to workers (there is one of these).
2.  As many workers as  you desire are responsible for executing the unpacking code and generating parameters from events.  These run in parallel.  Since MPI runs them in separate processes, you don't have to worry about writing workers to be thread-safe.
3. A farmer process collects data from the workers and re-orders the events back to the order in which they were read from the event file.  There is only one of these.
4.  The Outputter outputs data sorted by the farmer to the output file.


With the exception of the worker, there are pre-written classes for each of these.  The main program, must, therefore:
1.  Create an application class that is derived from ```frib::analysis::AbstractApplication```
2.  Instantiate that application,
3.  Create an object to read the parameter definition script (a ```frib::analysis::CTCLParameterREader```).
4.  Start up all of the processes that make up the application passing them the reader.

Note that you use miprun to run the application and specify the total number of processes it should run using the ```-n``` option.  You are required to specify a minimum of 4 for n, that will create one of each type of process.  Larger values for ```-n``` will create more worker processes to run in parallel.

mpirun passes all parameters it does not understand back to the application in the usual  ```argc, argv``` variables.  For our application these are:
*  ```argv[0]``` - the program path as passed to mpirun.
*  ```argv[1]``` - the name of the raw event file to process.
*  ```argv[2]``` - the name of the file to generate.  These files will be referred to as *parameter files*
*  ```argv[3]``` - The path to the Tcl parameter definition file.


Here is a very simple main program that does all of those things:

```c++
#include <AbstractApplication.h>
#include <MPIRawReader.h>
#include <MPIParameterOutputter.h>
#include <MPIParameterFarmer.h>
#include "MyWorker.h"  
#include <stdlib.h>
#include <iostream>

using namespace frib::analysis;

class MyApp : public AbstractApplication {                                   // 1 
public:
    MyApp(int argc, char** argv);                                            // 2
    virtual void dealer(int argc, char** argv, AbstractApplication* pApp);   // 3
    virtual void farmer(int argc, char** argv, AbstractApplication* pApp);   // 4
    virtual void outputter(int argc, char** argv, AbstractApplication* pApp); // 5
    virtual void worker(int argc, char** argv, AbstractApplication* pApp); // 6
};
void
MyApp::dealer(int argc, char** argv, AbstractApplication* pApp) {
    MPIRawReader dealer(argc, argv, pApp);                         // 7
    dealer();                                              
}
void
MyApp::farmer(int argc, char** argv, AbstractApplication* pApp) {
    CMPIParameterFarmer farmer(argc, argv, *pApp);                // 8
    farmer();                                            
}
void
MyApp::outputter(int argc, char** argv, AbstractApplication* pApp) {
    CMPIParameterOutput outputter(argc, argv, pApp);           // 9
    outputter();                                       
}
void
MyApp::worker(int argc, char** argv, AbstractApplication* pApp) {
    MyWorker worker(*pApp);                                  // 10
    worker(argc, argv);                              
}


int main(int argc, char** argv) {                            // 11
    if (argc <   4) {                                        // 12
        std::cerr <<
            "Usage: mpirun <mpirun-parameters> program-name infile outfile paramdef-file\n";
        exit(EXIT_FAILURE);
    }
    MyApp app(argc, argv);                                 // 13
    CTCLParameterReader reader(argv[3]);                   // 14
    app(reader);                                           // 15
    exit(EXIT_SUCCESS);                                    // 16
}

```
Refer to the numbered comments above when reading the list of descriptions below
1.  This section of code defines the ```MyApp``` class. This class ties together the entire pipeline and must be derived from ```frib::analysis::AbstractApplication``` (note we've pulled in the ```frib::analysis``` namespace just above this line).
2. The  Constructor should contain initialization that is done prior to the startup of the parallel application.  Normally the only thing the constructor does (in the base class) is store the arguments.
3.  When the application is started up in MPI, an instance of ```MyApp``` will be created in each of the processes that make up the application.  The base class will figure out if its instance is the correct one in which to run the dealer process and, if so, invoke this method
4. Similarly this method will be invoked in the farmer process
5. ...and this one in the outputter process.
6. Finally the ```worker``` method is invoked in each of the work er processes. 
7. The implementation of the dealer process simply instantiates an ```MPIRawReader``` and runs it by invoking its ```operator()```.  Raw readers are appropriate for any type of ring item file.  Note that parameter files are also composed of ring items so you can use this reader if you are writing a parameter file to parameter file pipeline.  The only difference, in that case, is that the worker class must be derived from the base class ```CMPIParametersToParametersWorker``` which will select parameter items and fill any parameters (```CTreeParameter``` or ```CTreeParameterArray``` elements) from those events.
8. The farmer just instantiates a ```CMPIParameterFarmer``` and runs it.
9. The outputter instantiates a ```CMPIParameterrOutput``` object which knows how to write parameter files and runs it.  If you know how, you can create an outputter that writes data in some other format.
10.  The worker, method is called in each worker process. We instantiate the custom worker that we wrote in the [The worker class](#the-worker-class) section above.
11. The next bit of business is writing the ```main``` function.  It is important to note that main is run in each of the proceses that make up the MPI applications.
12. We need four parameters, as described above. If we don't getting, we describe how we really should be run.  Note that this will be output for each of the processes.  If you only want one of them to  output this, you can also conditionalize on the rank of the process being 0 (see [MPI_Comm_rank](https://www.mpich.org/static/docs/v3.3/www3/MPI_Comm_rank.html))
13. Next an application object is instantiated.  When the ```operator()``` of this is called analysis will start.
14.  Some ranks will need to know about the parameter definitions we put in our parameter definition file (since each process has a separate process address space our ```CTreeParameter``` and ```CTreeParameterArrays``` created in our worker class are not known to e.g. the outputter which needs them).
15. Runs the application and, as each process complete....
16. ```exit``` is called to exit the application.

### Writing the Makefile.

Writing the Makefile will require that you 
*  Know where the FRIB analysis pipeline is installed.
*  Where the version of MPI it used is installed.

At the FRIB, frib-analysis pipeline versions are installed in containers at 
```/usr/opt/frib-analysis``` with versious version numbers below that top level directory.

Mpi at the FRIB is installed in various versions under various versions in /usr/opt/mpi.  For example OpenMpi version 4.0.1, if installed is in  /usr/opt/mpi/openmpi-4.0.1

Naturally, if you depend on any other software component's you'll need to know how to compile and link against them.

```makefile
FRIB_ANALYSIS=/usr/opt/frib-analysis/1.0-000
MPI=/usr/opt/mpi/openmpi-4.0.1

CXXFLAGS=-I$(FRIB_ANALYSIS)/include -I$(MPI)/include
FRIB_ANALYSIS_LDFLAGS=-L$(FRIB_ANALYSIS)/lib -Wl,-rpath=$(FRIB_ANALYSIS)/lib \
    -lSpecTclFramework -lfribore -ltclPlus -lException

MPI_LDFLAGS=-L$(MPI)/lib -Wl,-rpath=$(MPI)/lib -lmpi

CXXLDFLAGS=$(FRIB_ANALYSIS_LDFLAGS) $(MPI_LDFLAGS)

CXX=$(MPI)/bin/mpicxx


myapp: MyWorker.o main.o
    $(CXX) -o$@ $^  $(CXXLDFLAGS) 

MyWorker.o: MyWorker.cpp MyWorker.h
    $(CXX) $(CXXFLAGS) -c $<

main.o:  main.cpp MyWorker.h
    $(CXX) $(CXXFLAGS) -c $<

```

The key points are that we defined ```FRIB_ANALYSIS``` to be the top level directory of the FRIB analysis pipeline version we're using and similalrly, defined ```MPI``` to be the toplevel directory for the version of MPI we're using.  Note that different versions of MPI (e.g. mpich) may require different linking flags.  See your MPI documentation for more information.

Once those definitions are made the remainder of the Makefile is pretty trivial.
