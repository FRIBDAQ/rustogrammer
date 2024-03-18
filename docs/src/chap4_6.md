# The Data Source Menu

The data source menu allows you to attach data sources of various sorts to the histogram server.  Some source types are not (yet?) available to Rustogramer.  

* [Online](#data-source-online)  (SpecTcl only) Select an NSCLDAQ helper (e.g. rinselector) and take data from an online system.
* [File](#data-source-file)  Take data from a file.  Note that as of SpecTcl version 5.13-002,  SpecTcl can analyze data from a parameter file prepared for Rustogramer as well as from raw event files.
* [Pipe](#data-source-pipe) (SpecTcl only) Read data from an arbitrary helper program.
* [Cluster File](#data-source-cluster-file) (SpecTcl 5.14 and later only) Use a file to drive analysis from several event files.
* [Filter file](#data-source-filter-file) (SpecTcl only) take data from a filter file.
* [Detach](#data-source-detach) Stop analyzing data from the current data source.
* [Abort Cluster File](#data-source-abort-cluster-file) (SpecTcl only) abort an in progress cluster file.

Notes on cluster file processing.  The SpecTcl REST handlers to support cluster files (added in SpecTcl 5.14) depend on software that is part of the Tree GUI to function.  cluster file processing may fail in the server if the tree GUi is not being used.

## Data Source->Online (SpecTcl Only)

The ```Data Source->Online``` menu command allows you to get data from an online data source.  Really this is just the same as a Pipe data source (see [Data Source->Pipe](#data-source-pipe) below), however additional options  are automatically added to the *helper program* by the GUI.  These options make the attachment suitable for online data taking by ensuring the server does not limit the data flow if its analysis cannot keep up with the data rates.

WHen you select this option, you will see the following dialog:

![Online prompter](./images/online_prompt.png)

In the input edit line labeled ```Ring Buffer URL``` enter the URL of the ring buffer from which the server's helper should get data.   This is of the form ```tcp://```*hostname*```/```*ringname*  where 

*  *hostname* is the DNS name or IP address of the system from which you wish to get data.
*  *ringname* is the name of the ringbuffer in that system.

In the input edit line with the ```Browse...``` button to the right of it, find the name of the helper progam.   For NSCLDAQ, this is the ```ringselector``` program in the ```bin``` director of the version of NSCLDAQ you are using.   When starting the pipe data source, the GUI will automatically add appropriate options and values.  For example, suppose you selected ```/usr/opt/daq/12.1-000/bin/ringselector``` as the helper program.  The full command line that the GUI will specify for the helper for the ring ```tcp:///spdaq49/e1400x``` is:

```bash
/usr/opt/daq/12.1-000/bin/ringselector ---source=tcp://spdaq49/e1400x \\
    --sample=PHYSICS_EVENT \
    --non-blocking```
```

Finally, select the radio button that corresponds to he format of the data you are analyzing.  This is the format of the NSCLDAQ program that generates data in the ringbuffer.  Normally, this will be the version of NSCLDAQ that the readout programs were built under.



## Data Source->File

Initiates analysis of data from either a raw event file or a parameter file.  Note that rustogramer can only analyze data from parameter files.  SpecTcl, with an appropriate analysis pipeline can analyze data from either.

You will first be prompted for an event or parameter file.  Once that has been selected, you'll be asked to provide the version of NSCLDAQ the data are in via this prompter:

![DAQVersion promter](./images/daqversion_prompt.png)

Once that is selected the server will start analyzing data from that file.

## Data Source->Pipe (SpecTcl Only)

Pipe data sources allow the server to take data from the stdout of any program.  This is actually how 
```Data Source->Online``` works.   One example of how you can use this;  After performing your experiment, you could use e.g. ```gzip``` to compress all of your event files.  If you then analyze event files by using ```gzcat``` as a pipe data source, you could analyze data without actually every having the uncompressed event files on disk.  Since, in general, I/O is quite a bit slower than Disk I/O doing this *might* very well be faster than analyzing the raw event data, once you've paid the computational price of compressing the data in the first place.

When you select ```Data Source->Pipe``` you will be greeted with the following dialog:

![Pipe data source dialog](./images/pipsrc_dialog.png)

The ```Program``` line edit and its accompanying ```Browse...``` button allow you to enter the program to use as the pipe data source.

The Entry and editable list box below it allow you to provide an ordered set of parameters to the pipe program.  Finally the radio buttons at the bottom of the dialog allow you to select thd format of the NSCLDAQ data you are analyzing.   Clicking the ```Ok``` button commences analysis from that pipe data source.

## Data Source->Cluster File (SpecTcl Only)

## Data Source->Filter file... (SpecTcl Only)

## Data Source->Detach

## Data Source->Abort Cluster File (SpecTcl Only)