# Chapter 4 - Using the Rustogramer GUI

Rustogrmer suplies a sample GUI that is based on the NSCLSpecTcl treegui with, what I think are, some improvements in how objects are creatd. The GUI is a sample of a REST client written in Python against the [Python Rest API](./chap6_2.md).

If the envirionment variable ```RUST_TOP``` is defined to point to the top installation directory of Rustogramer, you can run the gui as follows:

```bash
$RUST_TOP/bin/gui [--host rghost] [[--port rest_port] | [--service rest_service] [--service-user rg_user]]
```

The gui supports the following command options

*  ```--host``` specifies the host on which the rustogramer you want to control is running.  This defaults to ```localhost``` if not specified.
* One of two methods to specify the REST port that Rustogramer is using:
    * ```--port``` specifies the numeric port on which Rustogramer's REST server is listening.  This defaults to ```8000``` which is Rustogrammer's default REST port.
    *  If rustogramer is using the NSCLDAQ port manager to advertise a service name:
        *   ```--service```  specifies the name of service rustogramer is advertising.
        *   ```--service-user``` specifies the name of the user that rustogramer is running under.  This defaults to your login username and, in general, should not be used.

When connected to Rustogramer, the GUI will look like this:
![Initial GUI view](images/gui_spectra.png)

Prior to describing each of the user interface elements let's look at a few features of this image.

*  The menubar at the top of the window provides access to operations that are not as frequent as those available on the main window.  Note that the contents of the menubar depends on the actual application the GUI is connectec to. 
*  The tabs below the menu-bar provide access to the sections of functionality of the GUI.  Note that the set of tabs will, again, depend on the application the GUI is connected to.   For example, Rustogramer does not have TreeVariable like functionality as that capability is pushed back onto the code that prepares data-sets.  If connected to SpecTcl, however, a ```Variables``` tab will be present.
*  Below the Tabs are controls for the things that Tab manages.  In the figure above, the ```Spectra``` tab is selected and shows a tabbed notebook that allows you to create and edit the definitions of Spectra as well as a filtered list of spectra and their applications.  More about this tab in the documentation [the spectra tab](./chap4_1.md)
* Note that the only thing the ```Help``` menu provides is information about the program (the ```About``` menu command).

For information about the contents of each tab:
- [The ```Spectra``` Tab](./chap4_1.md)
- [The ```Parameters``` Tab](./chap4_2.md)
- [The ```Variables``` Tab](./chap4_3.md) (SpecTcl only).
- [The ```Gate``` Tab](./chap4_4.md)

For information about the Menus:
- [The ```File``` Menu](./chap4_5.md)
- [The ```Data Source``` Menu](./chap4_6.md)
- [The ```Filters``` Menu](./chap4_filters.md) (SpecTcl only).
- [The ```Spectra``` Menu](./chap4_7.md)
- [The ```Gate``` Menu](./chap4_8.md)