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



class TwoDEditor(QWidget):
    def __init__(self, *args):
        nameChanged = pyqtSignal(str)

        xparameterSelected = pyqtSignal(str)
        xaxisModified  = pyqtSignal(dict)
        
        yparameterSelected = pyqtSignal(str)
        yaxisModified  = pyqtSignal(dict)

        commit = pyqtSignal()

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

        layout.addWidget(QPushButton('Create/Replace', self), 3, 0, 1,2, Qt.AlignCenter)

        self.setLayout(layout)







