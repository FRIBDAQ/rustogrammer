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
    QStyle,
    QVBoxLayout, QHBoxLayout, QGridLayout
)
from PyQt5.QtCore import pyqtSignal
from PyQt5.Qt import *

from axisdef import AxisInput
from ParameterChooser import Chooser as ParameterChooser
from editablelist import EditableList


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
    


class GammaDeluxeEditor(QWidget):
    def __init__(self, *args):
        super().__init__(*args)
        
        top_layout = QGridLayout()

        # Top row cols 0, 1 are the spectrum name input:

        top_layout.addWidget(QLabel('Name', self), 0,0, Qt.AlignRight)
        self._name =QLineEdit()
        top_layout.addWidget(self._name, 0, 1, 1,2)

        #  Xparameter list:

        self._xparameters = EditableList('X Parameters', self)
        top_layout.addWidget(self._xparameters, 1,1, 6, 1)


        #  Row 8, col 0 has a labeled parameter chooser,
        #  and label for the parameter.

        param_layout = QVBoxLayout()
        param_layout.addWidget(QLabel('Parameter(s):', self))
        self._parameter_chooser = ParameterChooser(self)
        param_layout.addWidget(self._parameter_chooser)
        self._selected_parameter = QLabel('', self)
        param_layout.addWidget(self._selected_parameter)
        top_layout.addLayout(param_layout, 8, 0)

        #  Row 8 col1 has the array checkbox:

        self._array = QCheckBox('Array', self)
        top_layout.addWidget(self._array, 8,1)

       
        
        self._yparameters = EditableList('Y Parameters', self)
        top_layout.addWidget(self._yparameters, 9, 1, 6, 1)

        
        # THe two axes in row 17 cols 0, 1:

        self._xaxis = _Axis('X axis', self)
        self._yaxis = _Axis('Y axis', self)
        top_layout.addWidget(self._xaxis, 16, 0)
        top_layout.addWidget(self._yaxis, 16, 1)

        #  Finally the create/replace button

        self._commit = QPushButton('Create/Replace', self)
        top_layout.addWidget(self._commit, 17,0, 1,3, Qt.AlignHCenter)


        self.setLayout(top_layout)

# test code

def axis_test():
    app = QApplication([])
    c   = QMainWindow()
    w   = _Axis('test')
    c.setCentralWidget(w)

    c.show()
    app.exec()

def test_editor():
    app = QApplication([])
    c   = QMainWindow()
    w   = GammaDeluxeEditor(c)
    c.setCentralWidget(w)

    c.show()
    app.exec()

if __name__ == '__main__':
    test_editor()
