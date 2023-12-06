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

#  This dict is a table, indexed by tab name, of the class objects
#  that edit that spectrum type and the enumerator type in capabilities.
#  e.g. '1D': (SpectrumTypes.Oned, editor1d.onedEditor) - means
#  The tab labeled 1D will be added if the SpectrumTypes.Oned is supported by
#  the server and will contain an editor1d.onedEditor.
#
#  In the future, the classes may be self contained MVC bundles so we don't
#  have to concern ourselves with connecting slots etc.
_spectrum_widgets = {
    '1D': (SpectrumTypes.Oned, editor1d.oneDEditor),
    '2D': (SpectrumTypes.Twod, editortwod.TwoDEditor),
    'Summary': (SpectrumTypes.Summary, editorSummary.SummaryEditor),
    'Gamma 1D' : (SpectrumTypes.Gamma1D, editorG1d.Gamma1DEditor),
    'Gamma 2D' : (SpectrumTypes.Gamma2D, editorG2d.Gamma2DEditor),
    'P-Gamma'  : (SpectrumTypes.GammaDeluxe, editorGD.GammaDeluxeEditor),
    '2D Sum'   : (SpectrumTypes.TwodSum, editor2dSum.TwoDSumEditor),
    'Projection' : (SpectrumTypes.Projection, editorProjection.ProjectionEditor),
    'StripChart' : (SpectrumTypes.StripChart, editorStripchart.StripChartEditor),
    'Bitmask' : (SpectrumTypes.Bitmask, editorBitmask.BitmaskEditor)

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
        # Stock it with the supported spectrum editors:

        supported_specs = get_supported_spectrumTypes()
        for label in _spectrum_widgets.keys():
            info = _spectrum_widgets[label]
            if info[0] in supported_specs:
                self.editors[label] = info[1](self)  # So we can get this in the editors.
                self.tabs.addTab(self.editors[label], label)
        

        self.channelType = EnumeratedTypeSelector.TypeSelector()
        supported_ctypes = get_supported_channelTypes()

        print(supported_ctypes)
        for label in _channel_types.keys():
            t = _channel_types[label]
            print(t)
            if t in supported_ctypes:
                print("supported: ", label, t)
                self.channelType.addItem(label, t)

        layout.addWidget(self.tabs)
        typs = QVBoxLayout()
        typs.addWidget(QLabel('Channel Type:'))
        typs.addWidget(self.channelType)
        layout.addLayout(typs)
        self.setLayout(layout)
        #self.tabs.adjustSize()
        self.adjustSize()



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

