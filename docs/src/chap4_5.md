# The File Menu

The file menu provides access to various file related operations.  The available commands differ depending on whether  the Gui is attached to SpecTcl or Rustogramer.

Here's the exhaustive list of operations the File menu can perform.  IF a menu item is only available in SpecTcl or Rustogramer that is noted:

*  [Save...](#file-save)
*  [Save Treevariables...](#file-save-treevariables) (SpecTcl only)
*  [Save Spectrum contents...](#file-save-spectrum-contents)
*  [Load...](#file-load)
*  [Read spectrum contents...](#file-read-spectrum-contents)
*  [Source Tcl Script...](#file-source-tcl-script) (SpecTcl only)
*  [Exit](#file-exit)
*  [Stop Histogramer](#file-stop-histogramer) (Rustogramer only)

## File->Save...

This menu item saves the cofiguration in an SQLite3 database file.  The schema for these files closely follows the SpecTcl database storage schema and is described in [Schema of configuration files](./chap7_6.md).  You will be prompted for a filename and the following will be saved:

*  Parameter definitions and their metadata.
*  Spectrum definitions.
*  Condition definitions.
*  Which conditions are applied to which spectra.
*  For SpecTcl, the treevariable definitions (names values and units.)

## File->Save Treevariables...

SpecTcl only can save tree variable definitions to an Sqlite3 data base file.  The schema for that file is the same as for the file saved by the [Save configuration](#file-save) operation, however only the tables germane to tree parameter definitions are populated.  

## File->Save spectrum contents...

Provides the capability to save the contents of one or more spectra to file.  Before being prompted for the output file you'll be prompted for the spectrum and file format as follows:

![Spectrum save prompt](./images/save_spectra.png)

In the top part, select the spectra to save.  The right arrow button adds the spectra selected on the lef to the listbox on the right.  The list box on the right is editable; you can remove and reorder its spectra or just remove them all from the box.

The radio buttons on the bottom select the file format.  The following file formats are supported:

*  ASCII this is SpecTcl ASCII format.
*  Binary (SpecTcl only) this is legacy Smaug format.
*  [JSON is Java Object Script Notation](./chap7_7.md) (click the link for format details).

Note that spectra are actually saved *by the server* not by the GUI.  This means that this operation will only work properly if the GUI is run on a system that shares the same file system as the server.

For example, a client running on a desktop system communicating with a SpecTcl or Rustogramer running on an FRIB linux system in general, will not be able to successfully save files as the file paths requested for the server won't exist in the server.

A more subtle case is if, on a desktop, you are running the GUI natively but rustogramer or SpecTcl in Windows Subsystem for Linux or a virtual machine, the save will, in general fail.

## File->Load...

Loads a configuration file from an [SQlite3 databas file](./chap7_6.md) into the server.   It is the GUI that interprets the database file contents.   After being prompted for the name a file, parameter definitions will be loaded from that file. You'll be prompted for what to do with duplicate spectrum definitions:

![Load dialog](./images/load_dialog.png)

You have the following choices:

*  ```Delete all existing``` all existing spectra will be deleted before restoring the spectrum definitions from file.
*  ```Overwrite existing defs``` any spectra defined in the server with the same name as one in the file get ovewritten by  the definition in the file.
*  ```Don't re-define duplicates```  any spectra defined in the server with the same name as one in the file don't get restored from file, retaining the old definition.

Conditions will unconditionally overwrite existing conditions with the same name.  All spectra will be bound to the display memory.

## File->Read spectrum contents...

Reads the contents of a spectrum file.  First you will be presented with the following dialog:

![Read spectrum dialog](./images/rdspec_dialog.png)

This dialog allows you to describe how spectra read from file will be loaded by the server and in what format the file is.

*  Save Options:
    *  ```Save as snapshots``` - spectra will be created as snapshots which means they will never increment as  new data are processed.
    *  ```Replace Spectra``` with same names - If unchecked new unique names will be used to avoid collisions with existing spectra.  If checked, any exising spectrum with the same name as one in file will be deleted.
    *  ```Bind to display memory``` - IF checked, the spectra are bound into display memory. If not, they are held local to the histogram server and can't be visualized until they are bound later.
* File format; as described in [File->Save spectrum contents...](#file-save-spectrum-contents), the format of the file that is being read.
## File->Source Tcl Script...

This option is only available  to SpecTcl servers.  As with spectrum contents files, since the file is opened in the server, the GUI must have a shared file system with the server.  Select a file and SpecTcl will run it as a Tcl script.

## File->Exit

After prompting if you are sure, exits the GUI.  Note that in general the server continues to run and you can connect to it again later with a new GUI instance.  For Rustogramer, see, however [File->Stop Histogramer](#file-stop-histogramer) below.

## File->Stop Histogramer

This is only available if the server is Rustogramer. After prompting if you are sure, requests the histogramer to exit.  Once the histogram responds that it is exiting, the GUI exits as well.
