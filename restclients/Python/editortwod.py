'''  This module provides a 2-d editor widget.   This editor has;
    - a spectrum name line edit
    - a pair of axis definintions
    - a pair of parameter selectors

    The axis selectors and parameter selectors share a common model with all
    editors and, therefore are identical in contents.

    Note that the concept of an array of 2ds does not make sense and is not
    supported by the GUI.

    
'''

from PyQt5.QtWidgets import (
    QWidget, QLabel, QLineEdit, QGridLayout, QVBoxLayout, QHBoxLayout,
    QPushButton
)
from PyQt5.QtCore import pyqtSignal, Qt

from axisdef import AxisInput
from ParameterChooser import Chooser


#  Internal subwidget that has parameter name and
#  axis definition associated with an axis:
class _AxisWidget(QWidget):
    
    parameterChanged = pyqtSignal(str)
    axisModified    = pyqtSignal(dict)
    def __init__(self, *args):
        super().__init__(*args)

        layout = QGridLayout()
        layout.addWidget(QLabel('Parameter: ', self), 0,0, Qt.AlignLeft)
        self._parameter = Chooser(self)
        layout.addWidget(self._parameter, 0, 1, Qt.AlignRight)
        self._selected_parameter = QLabel('', self)
        layout.addWidget(self._selected_parameter, 1, 0, Qt.AlignLeft)
        self._axis_spec = AxisInput(self)
        layout.addWidget(self._axis_spec, 2, 0, 1, 2, Qt.AlignLeft)

        self.setLayout(layout)

        # Estalish signal handlers to map widget events to our signals:

        self._parameter.selected.connect(self._parameterChanged)
        self._axis_spec.lowChanged.connect(self._axisChanged)
        self._axis_spec.highChanged.connect(self._axisChanged)
        self._axis_spec.binsChanged.connect(self._axisChanged)

    #  Internal slots.

    def _parameterChanged(self, new_path):
        name = '.'.join(new_path)
        self.setName(name)
        self.parameterChanged.emit(name)

    def _axisChanged(self, value):
        axis_def = {
            'low'  : self._axis_spec.low(),
            'hi' : self._axis_spec.high(),
            'bins' : self._axis_spec.bins()
        }
        self.axisModified.emit(axis_def)

    # Getters/setters (attributes)
    def name(self):                        
        return self._selected_parameter.text()
    def path(self):
        return self._parameter.current_item()
    def setName(self, value):
        self._selected_parameter.setText(value)
    def low(self):
        return self._axis_spec.low()
    def setLow(self, value):
        self._axis_spec.setLow(value)
    def high(self):
        return self._axis_spec.high()
    def setHigh(self, value):
        self._axis_spec.setHigh(value)
    def bins(self):
        return self._axis_spec.bins()
    def setBins(self, value):
        self._axis_spec.setBins(value)

class TwoDEditor(QWidget):
    '''
        Signals:
            nameChanged - Name of the spectrum changed.

            xparameterSelected - an X parameter was selected.
            xaxisModified  - attributes of the X axis were modified.

            yparameterSelected - A Y parameter was selected.
            yaxisModified  - attributes of the Y axis were selected.

        Attributes:
            name - spectrum name.
            xparameter - x axis parameter.
            yparameter - y axis parameter.
            xlow,xhigh,xbins - X axis specification.
            ylow,yhigh,ybins - Y axis specifications.
    '''
    nameChanged = pyqtSignal(str)

    xparameterSelected = pyqtSignal(str)
    xaxisModified  = pyqtSignal(dict)

    yparameterSelected = pyqtSignal(str)
    yaxisModified  = pyqtSignal(dict)

    commit = pyqtSignal()
    def __init__(self, *args):
    

        super().__init__(*args)

        layout = QGridLayout()

        # Spectrum name stuff:

        name_layout = QHBoxLayout()
        name_layout.addWidget(QLabel('Name: ', self))
        self._name = QLineEdit(self)
        name_layout.addWidget(self._name)

        layout.addLayout(name_layout, 0,0)

        # Axis stuff:

        layout.addWidget(QLabel('X', self), 1,0, Qt.AlignCenter)
        layout.addWidget(QLabel('Y', self), 1,1, Qt.AlignCenter)

        self._xaxis = _AxisWidget(self)
        layout.addWidget(self._xaxis, 2, 0, Qt.AlignLeft)

        self._yaxis = _AxisWidget(self)
        layout.addWidget(self._yaxis, 2, 1, Qt.AlignLeft)

        self._create = QPushButton('Create/Replace', self)
        layout.addWidget(self._create, 3, 0, 1,2, Qt.AlignCenter)

        self.setLayout(layout)

        # Connect individual signals... claim is that signals can be connected
        # to signals - this provides a neat relay scheme:

        self._name.textChanged.connect(self.nameChanged)

        self._xaxis.parameterChanged.connect(self.xparameterSelected)
        self._xaxis.axisModified.connect(self.xaxisModified)

        self._yaxis.parameterChanged.connect(self.yparameterSelected)
        self._yaxis.axisModified.connect(self.yaxisModified)
        
        self._create.pressed.connect(self.commit)
    
    #  Implement attribute getters.

    def name(self):
        return self._name.text()
    def setName(self, value):
        self._name.setText(value)
    def xparameter(self):
        return self._xaxis.name()
    def setXparameter(self, x):
        self._xaxis.setName(x)
    def yparameter(self):
        return self._yaxis.name()
    def setYparameter(self, y):
        self._yaxis.setName(y)
    def xlow(self):
        return self._xaxis.low()
    def setXLow(self, value):
        self._xaxis.setLow(value)
    def xhigh(self):
        return self._xaxis.high()
    def setXHigh(self, value):
        self._xaxis.setHigh(value)
    def xbins(self):
        return self._xaxis.bins()
    def setXBins(self, value):
        self._xaxis.setBins(value)
    def ylow(self):
        return self._yaxis.low()
    def setYLow(self, value):
        self._yaxis.setLow(value)
    def yhigh(self):
        return self._yaxis.high()
    def setYHigh(self, value):
        self._yaxis.setHigh(value)
    def ybins(self):
        return self._yaxis.bins()
    def setYBins(self, value):
        self._yaxis.setBins(value)
    
        







