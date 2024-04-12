# The Filters Menu (SpecTcl Only).

As we have described in [our documentation of the Data Source->Filter file](./chap4_6.md#data-source-filter-file-spectcl-only) menu  SpecTcl an write filtered event data files.  See that section for more information about what filter files are and pointers to SpecTcl documentation on filter files.   The ```Filters``` menu provides:

*  The ability to create filters via a [filter wizard](#filter-filter-wizard).
*  The ability to [control which filters are enabled](#filter-enabledisable-filters); enabled filters write data.
*  Another menu entry to [attach a filter file](#filter-read-filter-files) for analyais.


Note again, that these menu entries are only available with SpecTcl and on rustogramer, the Filter menu itself will ont be present.

## Filter->Filter Wizard

Creating a filter is a multi-step process.  The filter wizard leads you through the process of:

* [Naming the filter](#naming-the-filter)
* [Selecting, if desired, a gate](#selecting-the-filter-gate) to determine which events are written to the  filter file.
* [Choosing the parameters](#selecting-the-parameters-to-output) to write for each event that makes the gate true.
* [Choosing the file](#choosing-the-output-file) to which filter data will be written and whether the filter should be created
enabled.

The next subsections will look at each of these steps, showing you the Wizard GUI for each step:


### Naming the filter.

THe firs step of the filter wizard:

![Filter name](./images/fwiizard_name.png)

Describes the process of making a filter and presents you with an editable text input in which you can type the filter's name.  Filter names must be unique.

When you are satisfied with your filter name, click the ```Next >``` button.

### Selecting the filter gate

A filter is gated.   Only events that make the filter true are written to the output file.  This allows you to do things like write only events with a specific particle ID for example.  The second stage of the filter wizard prompts you for a filter gate:

![Filter gate](./images/fwizard_gate.png)

Select a gate from the pull down:
*   If you've updated the gates your gate may not be visible. Clicking the ```Update Gate list``` button, will then update the list of gates that populate the gate chooser pull-down.
*   If you want all events to be written, create and select a ```True``` gate as the filter gate.

When you hvae the desired gate selected, click the ```Next >``` Button.

### Selecting the parameters to output.

Only a subset of parameters need to be written to filter files.  The next stop provides a parameter chooser/editable list to allow you to choose the parameters you want output:

![Filter parameters](./images/fwizard_params.png)

Improvements to this stage of the GUI are planned see Issue [#169](https://github.com/FRIBDAQ/rustogrammer/issues/169) for more information.   When the parameters you want written are all in the editable list box, click the ```Next >``` button to advance to the last stage of the wizard.




### Choosing the output file.

The final step of the filter creation process is to specify where the filtered data will be written and, optionally to enable the filter.

![File](./images/fwiizard_file.png)

Only enabled filters will actually write data.


## Filter->Enable/Disable Filters

This menu entry allows you to enable and disable filters.  It brings up this dialog:

![Enable filters](./images/filter_enables.png)

The table has a line per filter.  The left column are filter names and the right columns checkboxes.  Check the boxes for the filters you want enbled then click the ```Ok``` button to apply your selections.

## Filter->Read Filter Files

This menu command allows you to attach a filter file.  THe mechanics of this are described in 
the [Data Source Menu](chap4_6.md#data-source-filter-file-spectcl-only).