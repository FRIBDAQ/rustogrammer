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

## Data Source->Online

## Data Source->File

## Data Source->Pipe

## Data Source->Cluster File

## Data Source->Filter file...

## Data Source->Detach

## Data Source->Abort Cluster File