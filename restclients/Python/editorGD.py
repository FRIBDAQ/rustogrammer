'''  This module provides an editor for particle gamma spectra,
which SpecTcl calls, thanks to Dirk, Gamma Deluxe spectra.  These have
an arbitrary number of X parameters and an arbitrary number of Y parameters.
The GUI looks like a single parameter chooser with two lists, one each for
the X and Y parameters.  There are, therefore, two add arrows and two delete 
arrows, again, one each for each axis.  There is one array checkbutton.
The user chooses a parameter in the single selector and clicks the appropriate
arrow to add that parameter or parameter array to the desired axis list.
Each axis list supports the same sorts of editing that is supported by
the Summary,Gamma1D, Gamma2D editor.   Selected blocks of parameters can
be moved up or down, selected parameters can be removed from each  list
and lists can be cleared.

Here's a sample configuration:

+----------------------------------------+
| Name [    line edit  ]                 |
|                          X parameters  |
|                         +------------+ |
|                    >       ...         |
|                    x    +------------+ |
|  parameter chooser        ^ V [clear]  |
|  [ ] array                             |
|                           Y parameters | 
|                         +------------+ |
|                     >        ...       |
|                     X   +------------+ |
|                          ^ V [clear]   |
|  X axis          Y axis                |
| [axis input]    [ axis input]          |
|           [Create/Replace]             |
+----------------------------------------+

'''

from PyQt5.QtWidgets import (
    QLabel, QLineEdit, QListWidget, QCheckBox, QPushButton, QWidget,
    QApplication, QMainWindow,
    QVBoxLayout, QHBoxLayout, QGridLayout
)
from PyQt5.QtCore import pyqtSignal, Qt

from axisdef import AxisInput
from ParameterChooser import Chooser


##  Internal widget that is a labeled axis input:

class _Axis(QWidget):
    ''' A labeled axis definition.
        no signals, but attributes are:
        label - text used to label the widget.
        low   - axis low limit
        high  - axis high limit.
        bins  - axis bin count.
    '''
    def __init__(self, label, *args):
        super().__init__(*args)
        layout = QVBoxLayout()
        self._label = QLabel(label, self)
        layout.addWidget(self._label)
        self._axis = AxisInput(self)
        layout.addWidget(self._axis)
        self.setLayout(layout)

    # Attribute getter/setters.

    def label(self):
        return self._label.text()
    def setLabel(self, newLabel):
        self._label.setText(newLabel)
    def low(self):
        return self._axis.low()
    def setLow(self, value):
        self._axis.setLow(value)
    def high(self):
        return self._axis.high()
    def setHigh(self, value):
        self._axis.setHigh(value)
    def bins(self):
        return self._axis.bins()
    def setBins(self, value):
        self._axis.setBins(value)
    


class GammaDeluxeEditor(QLabel):
    def __init__(self, *args):
        super().__init__(*args)
        self.setText('Not Implemented yet')

# test code

def axis_test():
    app = QApplication([])
    c   = QMainWindow()
    w   = _Axis('test')
    c.setCentralWidget(w)


    c.show()
    app.exec()
