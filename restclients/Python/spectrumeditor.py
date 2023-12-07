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
    QMainWindow
)
from PyQt5.QtCore import *
from rustogramer_client import rustogramer as Client

import editor1d, editortwod, editor2dSum, editorBitmask, editorG1d
import  editorG2d, editorGD, editorProjection, editorStripchart
import editorSummary, EnumeratedTypeSelector

#------------------------- Spectrum controllers ----------------------
# Slots assume that capabilities.get_client won't return None.

# NullController - for unimplemented creations:

def default(value, default=0):
    if value is None:
        value = default
    return value

class NoneController:
    def __init__(self, editor, model):
        pass
###
#   Controller that handles the Oned editor view signals:
class OneDController:
    def __init__(self, editor, model):
        self._editor = editor
        self._model = model
        model.commit.connect(self.create)
        model.parameterSelected.connect(self.load_param)
    
    def create(self):
        print("create spectrum")
    def load_param(self, parameter_name):
        client = get_capabilities_client()
        current_name = self._model.name()
        if current_name is None or len(current_name) == 0:
            self._model.setName(parameter_name)
        # Regardless if the parameter has metadata load that into the axis definition:

        param_info = client.parameter_list(parameter_name)['detail'][0]
        self._model.setLow(default(param_info['low'], 0))
        self._model.setHigh(default(param_info['high'], 100.0),)  # like tree params.
        self._model.setBins(default(param_info['bins'], 512))


    
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
    '2D': (SpectrumTypes.Twod, editortwod.TwoDEditor, NoneController),
    'Summary': (SpectrumTypes.Summary, editorSummary.SummaryEditor, NoneController),
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
    'double': ChannelTypes.Double,
    '32 Bits': ChannelTypes.Long,
    '16 bits': ChannelTypes.Short,
    'Byte' : ChannelTypes.Byte
}
#   This class assumes that the capabilities client has already been set:
class Editor(QWidget):
    new_spectrum = pyqtSignal(str)
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
    
    # Slot that can be called when a controller makes a new spectrum:

    def spectrum_added(self, name):
        self.new_specttrum.emit(name)


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

