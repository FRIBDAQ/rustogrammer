# The Spectra Tab

The spectra tab has three components:
*  A set of tabs that selec spectrum editors.  The actual set of tabs depends on the program the GUI is connected to as SpecTcl implements a few spectrum types that are not implemented in Rustogramer.
*  A vertical bar of buttons that provide access to operations on spectra.
*  A filterable spectrum list. Spectra in this list can be selected so that they become the target of some opeartions. 

## The Button bar.

The button bar contains the controls show below:

![Spectrum button bar](./images/spectra_buttonbar.png)

The button bar works in conjunction with selected spectra in the spectrum listing 
(see [The Spectrum Listing](#the-spectrum-listing)).

As we will see in the section that describes the spectrum listing, you can select any number of spectra in the list.

*  The ```Clear``` button clears the contents of all selected spectra.
*  The ```Clear All``` button clears the contents of all spectra.
*  The ```Delete``` button, after prompting for confirmation, deletes the selected spectra.
*  The Pull down menu below the ```Delete``` button allows you to select a condition.
*  The ```Gate``` button, applies the selected conditions to all selected spectra.
*  The ```Ungate``` button removes any condition applied to the selected spectra.
*  The ```Load editor``` button requires that only one spectrum be selected.  It loads the definition of that spectrum into the appropriate spectrum editor and selects the tab of that editor.  This allows you to either modify the definition of that spectrum or, by changing the name, to copy the spectrum.
*  The pulldown menu labeled ```Channel Type:``` allows you to select the data type for the channels of spectra that are created.  The values in the pull down will reflect the capabilities of the program the GUI is connected to:
    *  Rustogramer only supports channels that are 64 bit floats.  Note that these get stored in shared spectrum memory as uint_32's.
    *  SpecTcl supports the following channel types;
        *   ```long``` (32 bit unsigned integer).
        *   ```short``` (16 bit unsigned integer).
        *   ```bytes``` (8 bit unsigned integer)

## The Spectrum listing

The Spctrum listing part of the Spectrum Tab looks like this:

![Spectrum listing](./images/spectrum_list.png)

Note that you can stretch the listing horizontally and, if you stretch the entire GUI vertically, all additional space is added to the listing.

Let's start with the controls at the bottom of the listing.  The ```Filter``` button updates the listing to only show the spectra with names that match the pattern in the editable text field to its right.  This pattern can contain any filesystem wild card characters.  The ```Clear``` button clears the pattern back to the default ```*```, which matches all spectra and updates the list.

The columns on the table reflect the contents of the column for each spectrumin the list.  From left to right:

* ```Name``` The name of the spectrum.
* ```Type``` The SpecTcl spectrum type string for the spectcrum.
* ```XParameter(s)``` lists _all_ parameters on the X axis of the spectrum.
* ```Low```, ```High```, ```Bins``` to the right of ```XParameter(s)``` describe the X axis of the spectrum.
* ```YParameter(s)``` lists _all_ parameters on the Y axis of the spectrumn.
* ```Low```, ```High```, ```Bins``` to the right of ```YParameter(s)``` describe the Y axis of the spectrum.
* ```Gate``` If not blank, contains the name of the condition applied to the Spectrum.

You can also stretch the column widths to match your needs.

You can select any number of spectra simultaneously and selection regions need not be contiguous.  The selected spectra are operated on by the buttons in the 
[Button bar](#the-button-bar)

## The Spectrum editors.

The tabs allow you to select spectrum editors that allow you to create/replace spectra.  Each of these editors has a 
*  ```Name``` edit which is mandatory and into which you should put the spectrum name.
*  ```Create/Replace``` button which you should click to create the spectrum.

If, when you click the ```Create/Replace``` button, spectra with the same name exist, a dialog will ask you to confirm the replacement of those spectra (which will be listed in the dialog).

The set of editors (tabs) will depend on the spectrum types that are supported by the histogramer we are connected to.
