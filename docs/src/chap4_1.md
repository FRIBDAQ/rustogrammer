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

The subsections below will describe each of the spectrum types and their editors:

* [1D spectrum editor](#1-d-spectrum)
* [2D Spectrum editor](#2-d-spectrum)
* [Summary Spectra](#summary-spectrum)
* [Gamma 1D spectra](#gamma-1-d-spectrum)
* [Gamma 2D spectrum editor](#gamma-2-d-spectrum)
* [Gamma Deluxe spectrum editor](#particle-gamma-spectrum)

### 1-D Spectrum

1-D spectra (SpecTcl type ```1```), histogram the values of a single parameter over a range with a specified binning.  The editor looks like this:

![1D Spectrum Editor](./images/spectrum-1d.png)

Select the parameter to histogram from the Parmater pull down.  It's a hierarchical menu using the ```.``` character as a path separator.  When you have selected a parameter, the full name of the parameter will appear below the pull down.  If the parameter has metadata definining the range and suggested binning, that information will initially populate the Axis pulldowns.  The Low, High and Bins pull-downs pre-load with a set of common values but you can also type a value you want in the pulldown.  If you do so, that value is added to the list of values.

The ```Array?``` checkbox allows you to create a group of spectra very quickliy.  Suppose, for example, you have paramters ```p.0, p.1...p.15``` and you want to create spectra named ```p.raw.0 ... p.raw.15```.  If yo uchose any of the ```p.```*n* parameters and set the name to ```p.raw```, checking the ```Array?``` checkbox will create those spectra.

### 2-d Spectrum

2-d spectram (SpecTcl type ```2```) are two dimensional heat map spectra that histogram the correlated values of an x and a y parameter.  The editor for these looks like this:

![2D Spectrum Editor](./images/spectrum-2d.png)

The mechanics of this editor are very similar to those of the [1D editor](#1-d-spectrum).  Select X and Y axis parameters from the pulldowns associated with each axis, ensure the axis specifications for the X and Y axes are what you want, provide a spectrum name and click ```Create/Replace``` to create the spectrum or overwrite (after confirmation) an exising spectrum with the same name.

### Summary Spectrum:

Summary spectra (SpecTcl type ```s```)  are a 2-d spectrum that allow you to look at the health of several identical spectra simultaneously.   Each X axis bin is associated with a parameter and the 1-d histogram of that parameter's values appears on the Y axis of that bin.

The summary spectrum editor editor looks like this:

![Summary Spectrum Editor](./images/spectrum-summary.png)

Parameter selection involves filling in the ```Parameters``` box to the right of the parameter chooser dropdown.  If you have selected a parameter you can add it to the list of parameters in the box by clicking the right arrow button.  To remove parameters fromt the list, select them and click the X button.  You can add a related set of parameters to the list box by checking the ```array``` checkbox before clicking the right arrow button.

For example, if you have parameters named ```p.0, p.1 ... p.15``` and you want them all in the list box, select one of them, check the ```array``` checkbox and click the right arrow button.  That will add all parameters that match ```p.*```

You can re-order parameters.  Move parameters up in the list by selecting them (you can select more than one) and clicking the up arrow button. Similarly the down arrow button moves parameters down in the list.  The ```Clear``` button clears the parameter list.

The Axis inputs define the Y axis.  If ```From  Parameters``` is checked, than the right arrow button also loads the axis description from the parameter metadata for the parameter(s) it added.

Once the desired list of parameters has been selected, the Y axis defined and a spectrum name input; Clicking the ```Create/Replace``` button will create the spectrum (or replace an identically named spectrum after confirming)

### Gamma 1-D spectrum

Gamma 1-d spectra (SpecTcl type g1) are multiply incremented 1-d spectra.  The histogram is incremented for all of the values of its parameters.  You can think of it as a sum of the 1D spectra of all of its parameters.  The Gamma-1D spectrum looks exactly like the summary spectrum:

![G1D Spectrum Editor](./images/spectrum-summary.png)

When creating a Gamma 1d spectrum, however, the list of parameters determines the set of parameters that can increment the spectrum for any event.  Note that while in summary spectra the order of the parameters in the list box determines which X axis bin that parameter occupies, for Gamma-1D Spectra, the order is meaningless.

### Gamma 2-D spectrum

Gamma 2d spectra (SpecTcl type ```g2```) are 2-d spectra that increment for all pairs of spectrum parameters present in any events.  The editor for this spectrum type is similar to the summary spectrum editor but has a pair of axes:

![G2 editor](images/spectrum-g2.png)

Normally the X and Y axes have the same definition but neither rustogramer nor SpecTcl require this

### Particle Gamma Spectrum.

Particle gamma spectra (SpecTcl type gd) allow you to define a set of X and Y parameters.  The resulting 2d spectrum is incremented for each ordered pair of parameters in the event.  In general, this results in more than one increment per event.

The editor, therefore looks like:

![Gamma Deluxe editor](./images/spectrum-gd.png)

The mechanics of using this editor are the same as using the editor for a 
[summary spectrum](#summary-spectrum), however:  There are two editable parameter lists. Having selected a parameter you can put it in either the X or Y parameters list depending on which of the arrow buttons you click.