''' Implements a spectrum editor.
   The spectrum editor is a tabbed widget that contains, on its tabs
   the editors for spectrum types supported by the server program.
   Next to that is a selector for the channel type of spectra that will be
   created (this is independent of the spectrum type).
   
   Here's an example of the layout:

   +-----------------------------------------+
   | | + 1d + + 2d +         |  Channel type |
   | |    ...                |  [combobox]   |
   |
   +-----------------------------------------+

   Note that we can use the fact that classes are first class objects
   when making this UI.
   '''
from capabilities import (
    SpectrumTypes, ChannelTypes, 
    set_client as set_capabilities_client, get_supported_spectrumTypes,
    get_client as get_capabilities_client,
    get_supported_channelTypes
)

from PyQt5.QtWidgets import (
    QTabWidget, QWidget, QHBoxLayout, QVBoxLayout, QApplication, QLabel,
    QMainWindow, QMessageBox
)
from PyQt5.QtCore import *
from rustogramer_client import rustogramer as Client, RustogramerException

import editor1d, editortwod, editor2dSum, editorBitmask, editorG1d
import  editorG2d, editorGD, editorProjection, editorStripchart
import editorSummary, EnumeratedTypeSelector

#------------------------- Spectrum controllers ----------------------
# Slots assume that capabilities.get_client won't return None.

# Utilities.

def default(value, default=0):
    if value is None:
        value = default
    return value
def confirm(question, parent=None):
    dlg = QMessageBox(QMessageBox.Warning, 'Confirm?', 
                    question,
                    QMessageBox.Yes | QMessageBox.No, parent
                )
    dlg = dlg.exec()
    return dlg == QMessageBox.Yes

def error(msg):
    dlg = QMessageBox(QMessageBox.Error, 'Error:', msg, QMessageBox.Ok)

# NoneController - for unimplemented creations:
class NoneController:
    def __init__(self, editor, view):
        pass
###
#   Controller that handles the Oned editor view signals:
class OneDController:
    def __init__(self, editor, view):
        self._editor = editor
        self._view = view
        view.commit.connect(self.create)
        view.parameterSelected.connect(self.load_param)
    
    def create(self):
        client = get_capabilities_client()
        sname = self._view.name()
        param = self._view.parameter()
        data_type = self._editor.channeltype_string()

        # Preconditions to making a spectrum; there must be a spectrum and parameter
        # name.
        if sname is not None and len(sname) > 0 and param is not None and len(param) > 0:
            if not self._view.array():
                if len(client.spectrum_list(sname)['detail']) > 0:
                    if not confirm(f'{sname} already exists replace it?', self._view):
                        return

                    # Delete the existing spectrum

                    client.spectrum_delete(sname)
                    self._editor.spectrum_removed(sname)
                # Create what is now guaranteed to be a new spectrum.
                low   = self._view.low()
                high  = self._view.high()
                bins  = self._view.bins()
                try:
                    client.spectrum_create1d(sname, param, low, high, bins, data_type)
                except RustogamerException as error:
                    error(f'{error} while creating spectrum')
                    return
                try: 
                    client.sbind_spectra([sname])
                except RustogramerException as error:
                    error(
                        f'{error} while binding spectrum to shared memory {sname} is defined but not displayable'
                    )
                self._view.setName('')
                self._editor.spectrum_added(sname)
            else:
                self._make_spectrum_array(client, sname, param)

    def load_param(self, parameter_name):
        client = get_capabilities_client()
        current_name = self._view.name()
        if current_name is None or len(current_name) == 0:
            self._view.setName(parameter_name)
        # Regardless if the parameter has metadata load that into the axis definition:

        param_info = client.parameter_list(parameter_name)['detail'][0]
        self._view.setLow(default(param_info['low'], 0))
        self._view.setHigh(default(param_info['high'], 512.0),)  # like tree params.
        self._view.setBins(default(param_info['bins'], 512))

     # Internal methods:

    def _gen_name(self, sname, pname):
        spath = sname.split('.')
        if len(spath) > 1:
            spath = spath[0:-1]
        ppath = pname.split('.')
        spath.append(ppath[-1])
        return '.'.join(spath)
    
    #  If any of the spectra are defined, prompt to proceed or not with their
    #  replacement:
    #   - Assume there's at least one name
    #   - Assume all names can be generated by replacing the last path element with *
    #
    def _proceed(self, client, names) :
        
        template_name = names[0]  #assume there's at least one
        pattern = template_name.split('.')[0:-1]
        pattern.append('*')
        pattern = '.'.join(pattern)

        defs = client.spectrum_list(pattern)['detail']
        existing_names = [x['name'] for x in defs]

        duplicate_names = [x for x in names if x in existing_names]
        if len(duplicate_names) > 0 :
            c = confirm(f'These spectra already exist {duplicate_names} continuing will replace them, do you want to continue?', self._view)
            if c:
                for s in duplicate_names:
                    client.spectrum_delete(s)    # Delete the dups so we can replace.
                    self._editor.spectrum_removed(s)
            return c
        else:
            return True                       # no confirmations needed.
    def _make_spectrum_array(self, client, sname, param):

        #  Get the list of parameters with params base:

        param_base = '.'.join(param.split('.')[0:-1])
        param_pattern = param_base + '.*'
        parameters    = self._param_names(client, param_pattern)
        data_type = self._editor.channeltype_string()

        # Generate the spectrum names:

        spectrum_names = [self._gen_name(sname, x) for x in parameters]
        if self._proceed(client, spectrum_names):
            low = self._view.low()
            high = self._view.high()
            bins = self._view.bins()

            for sname, pname in  zip(spectrum_names, parameters):
                try:
                    client.spectrum_create1d(sname, pname, low, high, bins, data_type)
                except RustogramerException as e:
                    error(f"Failed to create {sname}; {e} won't try to make any more")
                    return
                self._editor.spectrum_added(sname)
                
            try:
                client.sbind_spectra(spectrum_names)
            except RustogramerException as e:
                error(f"Failed to bind all spectram: {e} some may not be displayable")                
            self._view.setName('')

    def _param_names(self, client, pattern):
        
        defs = client.parameter_list(pattern)['detail']
        result =  [x['name'] for x in defs]
        result.sort()
        return result
##
#  Controller for the 2-d editor.
#  This is much simpler than the 1d editor since we don't have to handle
#  arrays.

class TwodController:
    def __init__(self, editor, view):
        self._client = get_capabilities_client()
        self._editor = editor
        self._view = view

        view.commit.connect(self.create)
        view.xparameterSelected.connect(self.load_xaxis)
        view.yparameterSelected.connect(self.load_yaxis)
    
    # SLots:

    def create(self):
        # Fetch the spectrum definition from the editor view:
        name = self._view.name()
        
        xparam = self._view.xparameter()
        xlow = self._view.xlow()
        xhigh = self._view.xhigh()
        xbins = self._view.xbins()

        yparam = self._view.yparameter()
        ylow = self._view.ylow()
        yhigh = self._view.yhigh()
        ybins = self._view.ybins()

        chantype = self._editor.channeltype_string()

        # Require a nonempty name and parameters:

        if len(name) > 0 and len(xparam) > 0 and len(yparam) > 0:
            #  Get confirmation if the spectrum exists.

            sdef = self._client.spectrum_list(name)
            sdef = sdef['detail']
            if len(sdef) != 0:
                if confirm(f'{name} already exists replace it?', self._view):
                    self._client.spectrum_delete(name)
                    self._editor.spectrum_removed(name)
                else:
                    return     # don't replace.
            try:
                self._client.spectrum_create2d(
                    name, xparam, yparam, 
                    xlow, xhigh, xbins, ylow, yhigh, ybins,
                    chantype
                )
            except RustogramerException as e:
                error(f'Failed to create {name} : {e}')
                return
            self._editor.spectrum_added(name)

            try:
                self._client.sbind_spectra([name])
            except RustogramerException as e:
                error(f'Failed to bind {name} - but spectrum was created: {e}')
    
    def load_xaxis(self, pname):
        param_info = self._client.parameter_list(pname)['detail'][0]
        self._view.setXLow(default(param_info['low'], 0))
        self._view.setXHigh(default(param_info['high'], 512.0),)  # like tree params.
        self._view.setXBins(default(param_info['bins'], 512))
    def load_yaxis(self, pname):
        param_info = self._client.parameter_list(pname)['detail'][0]
        self._view.setYLow(default(param_info['low'], 0))
        self._view.setYHigh(default(param_info['high'], 512.0),)  # like tree params.

        self._view.setYBins(default(param_info['bins'], 512))

##
#  Controller for summary spectra.
#
class SummaryController:
    def __init__(self, editor, view):
        self._client = get_capabilities_client()
        self._editor = editor
        self._view = view

        # Connect the signals to our handlers.

        self._view.parameter_changed.connect(self.select_param)
        self._view.add.connect(self.add_params)
        self._view.commit.connect(self.create_spectrum)
    
    #  Called to create a spectrum from the current definition.
    #  note that the name must be non-empty else we do nothing
    #  After successful completion, we prepare the UI for the next
    #  definition:
    #    - Clear the parameter box.
    #    - Clear the spectrum name.
    def create_spectrum(self):

        # Pull the definitions:
        name = self._view.name()
        params = self._view.axis_parameters()
        low = self._view.low()
        high = self._view.high()
        bins = self._view.bins()
        chantype = self._editor.channeltype_string()

        if len(name) > 0:
            # Are we replacing:

            if len(self._client.spectrum_list(name)['detail']) > 0:
                if not confirm(f'Spectrum {name} already exists replace?'):
                    return
                else:
                    try:
                        self._client.spectrum_delete(name)
                    except:
                        error(f'Unable to delete {name} maybe it has a wildcard character in it?')
                        return
                
            # If we get here we're ready to create the new spectrum:

            try:
                self._client.spectrum_createsummary(name, params, low, high, bins, chantype)
            except RustogramerException as e:
                error(f'Unable to create {name}: {e}')
                return
            self._view.setName('')
            self._view.setAxis_parameters([])
            try:
                self._client.sbind_spectra([name])
            except RustogramerException as e:
                error(f'Unable to bind {name} to display memory but it has been created.')

    # If a parameter is selected:
    #    put it's full name into the parameter text:

    def select_param(self, path):
        name = '.'.join(path)
        self._view.setSelected_parameter(name)

    #  Called when the arrow key to put a parameter into the param list
    #  is clicked.  
    #   - If array is checked we need to get the name of the parameters
    #     given the one in the param chooser
    #   - If the axis should be loaded from parameter metadata we load
    #
    def add_params(self):
        name = self._view.selected_parameter()
        if name == '':
            return
        
        # Note _parameter_list takes care of loading the axis definition
        # if desired and available.

        names = self._parameter_list(name)
        names.sort()
        full_list = self._view.axis_parameters() + names
        self._view.setAxis_parameters(full_list)

    # Private utilities.abs

    # _parameter_list - create a list of parameters to add to the list
    # box and, if requested, update the axis definitions from parameter metadat
    #
    def _parameter_list(self, base):
        pattern = base

        if self._view.array():
            pattern = base.split('.')[0:-1]
            pattern.append('*')
            pattern = '.'.join(pattern)
        
        #  Get the parameter definiions and:
        #  extract the names into a list and, if axis_from_parameters is
        #  in fill in the axis  values when we have a parameter with metadata.

        descriptions = self._client.parameter_list(pattern)['detail']
        names = [x['name'] for x in descriptions]  # list of matching names

        if self._view.axis_from_parameters():
            for p in descriptions:
                if p['low'] is not None:
                    self._view.setLow(p['low'])
                if p['high'] is not None:
                    self._view.setHigh(p['high'])
                if p['bins'] is not None:
                    self._view.setBins(p['bins'])


        return names
        


#  This dict is a table, indexed by tab name, of the class objects
#  that edit that spectrum type and the enumerator type in capabilities.
#  e.g. '1D': (SpectrumTypes.Oned, editor1d.onedEditor, onedcontroller) - means
#  The tab labeled 1D will be added if the SpectrumTypes.Oned is supported by
#  the server and will contain an editor1d.onedEditor and that onedcontroller
#  will be instantiated to handle signals from the editor.
#
#  In the future, the classes may be self contained MVC bundles so we don't
#  have to concern ourselves with connecting slots etc.
_spectrum_widgets = {
    '1D': (SpectrumTypes.Oned, editor1d.oneDEditor, OneDController),
    '2D': (SpectrumTypes.Twod, editortwod.TwoDEditor, TwodController),
    'Summary': (SpectrumTypes.Summary, editorSummary.SummaryEditor, SummaryController),
    'Gamma 1D' : (SpectrumTypes.Gamma1D, editorG1d.Gamma1DEditor, NoneController),
    'Gamma 2D' : (SpectrumTypes.Gamma2D, editorG2d.Gamma2DEditor, NoneController),
    'P-Gamma'  : (SpectrumTypes.GammaDeluxe, editorGD.GammaDeluxeEditor, NoneController),
    '2D Sum'   : (SpectrumTypes.TwodSum, editor2dSum.TwoDSumEditor, NoneController),
    'Projection' : (SpectrumTypes.Projection, editorProjection.ProjectionEditor, NoneController),
    'StripChart' : (SpectrumTypes.StripChart, editorStripchart.StripChartEditor, NoneController),
    'Bitmask' : (SpectrumTypes.Bitmask, editorBitmask.BitmaskEditor, NoneController)

}

#  This dict has channel type names as keys and channel type values as values:

_channel_types = {
    'f64': ChannelTypes.Double,
    'long': ChannelTypes.Long,
    'word': ChannelTypes.Short,
    'byte' : ChannelTypes.Byte
}
#   This class assumes that the capabilities client has already been set:
class Editor(QWidget):
    new_spectrum = pyqtSignal(str)
    spectrum_deleted = pyqtSignal(str)
    def __init__(self, *args):
        global _spectrum_widgets
        global _channel_types

        super().__init__(*args)

        # We use a hbox layout:

        layout = QHBoxLayout()

        #At the left is a tabbed widget:

        self.tabs = QTabWidget(self)
        self.tabs.setUsesScrollButtons(True)
        self.editors = dict()
        self.controllers = dict()
        # Stock it with the supported spectrum editors:

        supported_specs = get_supported_spectrumTypes()
        for label in _spectrum_widgets.keys():
            info = _spectrum_widgets[label]
            if info[0] in supported_specs:
                self.editors[label] = info[1](self)  # So we can get this in the editors.
                self.tabs.addTab(self.editors[label], label)
                self.controllers[label] = info[2](self, self.editors[label]) # hook in controller.
        

        self.channelType = EnumeratedTypeSelector.TypeSelector()
        supported_ctypes = get_supported_channelTypes()

        for label in _channel_types.keys():
            t = _channel_types[label]
            if t in supported_ctypes:
                self.channelType.addItem(label, t)

        layout.addWidget(self.tabs)
        typs = QVBoxLayout()
        typs.addWidget(QLabel('Channel Type:'))
        typs.addWidget(self.channelType)
        layout.addLayout(typs)
        self.setLayout(layout)
        self.adjustSize()
    
    # Get the currently selected channel type string
    
    def channeltype_string(self):
       return self.channelType.selectedText()

    # Slot that can be called when a controller makes a new spectrum:

    def spectrum_added(self, name):
        self.new_spectrum.emit(name)
    def spectrum_removed(self, name):
        self.spectrum_deleted.emit(name)
        
    

# --- tests

def test(host, port):
    c = Client({'host': host, 'port': port})
    set_capabilities_client(c)

    app = QApplication([])
    c = QMainWindow()

    w = Editor(c)
    c.setCentralWidget(w)
    c.adjustSize()

    c.show()
    app.exec()

