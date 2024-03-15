# The Variables Tab


This tab is only available with SpecTcl.  The Variables GUI looks like this:

![Variables tab](./images/variables_tab.png)

The controls on this tab are similar to those on the [Parameters tab](./chap4_2.md).   The only differences are:

*  The only data associated with a variable are its value and units of measure.
*  Since spectra don't depend directly on the values of variables, there's no button to redefine spectra.

As with the parameters tab, there are several controls at the top of the window and a table of variables and their data below.    You use the controls in the top line of the window to populate the table:

*  At the left is a variable chooser.  You can drop it down to select a variable name.
*  Clicking ```Append``` adds the variable and its current values/units to the end of the table.  If the array checkbox is checked, the variable selected is taken to be an element of an array of variables and all elements are of the array are added.
*  Clicking ```Replace``` Replaces the one selected line in the table with the variable selected by the variable chooser.
*  Clicking ```Remove``` removes all seleted rows in the table.
*  The ```Load``` button loads the selected table rows with the current variable values and units.
*  The ```Set``` button stores into SpecTcl the variables in the selected rows with their values and units.

The values and units in the table can be edited, however the ```Set``` button must be used to transfer selected, edited variable values and units int SpecTcl.  Furthermore any number of rows can be selected to be operated on by the buttons in the to section of the UI.  Note that the selection need not be a contiguous set of rows.