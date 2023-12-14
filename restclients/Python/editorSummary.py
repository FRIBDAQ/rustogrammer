'''  This module provides a summary spectrum editor.  It needs to be paired
     with a controller that can handle its signals and knows how to create
     a summary spectrum.
     A summary spectrum needs the user to be able to select a  list of
     spectra for the y axis and a yaxis definition in addition to the
     spectrum name.  

     To make things simpler we support pushing an 'array'  of parameters
     into the list selection box.  The axis definitions can be filled in from
     metadata associated with the parameters, if there is any or
     manually.

     Here's a sketch of the editor:

     +------------------------------------------+
     | Name: [              ]                   |
     | {param selector}  [] array  +-----------+|
     |                    >        | parameter ||
     |                    <        |           ||
     | [] axis from parameter(s)   | list box  ||
     | {axis selector}  Y-axis     |           ||
     |                             +-----------+|
     |                               [^] [V]    |
     |                              [Clear]     |
     |          [ Create/replace ]              |
     +------------------------------------------+

     The idea is that users choose parameters and click the right arrow button
     to add those to the list of parameters in the parameter list box.
     If the array checkbutton is checked this should add a list of 
     parameters;  parameter names are period separated pathed items and
     an array substitues the last element of the path with * adding matching
     parameters.  Note that arrays of parameters are added alphabetically sorted.

     A certain amount of parameter list editing is supported:
     *  The clear button removes all parameters from the listbox.
     *  The < button removes all of the parameters from the list box.
     *  The ^ button moves the selected parameters up one slot in the list box
        if they are not already at the top.
     *  Similarly the V button moves the selected parameters down one slot in

        the list box if they are not already at the bottom.
     If the [] axis from paramerter(s) checkbutton is set, then as each parameter
     is added to the selected parameter list, if it has metadata, that metadata
     is used to populate the axis definition.

     Attributes:
        *  name       - Current spectrum name.
        *  selected_parameter - the parameter selected in the parameter selector
        *  axis_parameters - Ordered list of parameters in the list box.
        *  low       - Y axis low limit.
        *  high      - Y axis high limit.
        *  bins      - Y axis bins.
        *  array     - state of the array check button.
        *  axis_from_parameters - State of the axis from parameters checkbutton.

    Signals:
        commit - The create/replace button was clicked. Requires
        add    - The right arrow was clicked, the signal handler will need to
         access the 'selected_parameter', 'array' and 'axis_from_parameters' 
         attributes to properly function.
        remove = An item was removed from the selected parameters.
       
        The remove signal is provided if, in the future, we decide we want to
        prevent adding duplicate parameters by removing them from the 
        selection list.

        Note that editing, other than insertion, is handled autonomously via
        internal signals.

'''

from PyQt5.QtWidgets import (
    QLabel, QListWidget, QPushButton, QCheckBox, QLineEdit,
    QApplication, QMainWindow, QGridLayout, QVBoxLayout, QHBoxLayout,
    QStyle, QWidget 
)
from PyQt5.Qt import *
from PyQt5.QtCore import pyqtSignal
from ParameterChooser import Chooser as pChooser
from axisdef import AxisInput
from editablelist import EditableList

class SummaryEditor(QWidget):
    commit = pyqtSignal()
    add    = pyqtSignal()
    remove = pyqtSignal(str)
    parameter_changed = pyqtSignal(list)
    def __init__(self, *args, **kwargs):
        super().__init__(*args)
        
        main_layout = QGridLayout()

        # Top row has title and QLineEditor for name:

        main_layout.addWidget(QLabel('Name:'), 0,0, Qt.AlignRight)
        self._name = QLineEdit(self)
        main_layout.addWidget(self._name, 0,1, 1, 2)

        # Left side of next row is parameter chooser and 
        # array button.
        pclayout = QHBoxLayout()
        chooser_name = QVBoxLayout()
        self._parameter_chooser = pChooser(self)
        self._chosen_parameter = QLabel(self)
        chooser_name.addWidget(QLabel("Select Parameter(s)"))
        chooser_name.addWidget(self._parameter_chooser)
        chooser_name.addWidget(self._chosen_parameter)
        self._param_array       = QCheckBox('Array', self)
        pclayout.addLayout(chooser_name)
        pclayout.addWidget(self._param_array)
        main_layout.addLayout(pclayout, 3, 0)

        self._list = EditableList('Parameters')
        main_layout.addWidget(self._list, 1,1, 6, 1)

        # The axis specification with a from parameters checkbutton.

        self._axis = AxisInput(self)
        main_layout.addWidget(self._axis, 9, 0)
        if 'from_par_row' in kwargs:
            from_row = kwargs['from_par_row']
        else :
            from_row = 9
        self._from_params = QCheckBox('From Parameters', self)
        main_layout.addWidget(self._from_params, from_row, 1, Qt.AlignBottom)

        # Finally the Create/Replace button in 10, all centered

        self._commit = QPushButton('Create/Replace', self)
        main_layout.addWidget(self._commit, 10, 0, 1, 3, Qt.AlignHCenter)

        self.setLayout(main_layout)

        # Signal relays:

        self._list.add.connect(self.add)
        self._list.remove.connect(self.remove)
        self._commit.clicked.connect(self.commit)
        self._parameter_chooser.selected.connect(self.parameter_changed)

        # Internal signals 

        #self._delete.clicked.connect(self.deleteSelection)
        #self._clear.clicked.connect(self.clear)  # relay to listbox.
        #self._up.clicked.connect(self.up)
        #self._down.clicked.connect(self.down)

        
        self.main_layout = main_layout
    
    #  Implement the attributes:

    def name(self):
        return self._name.text()
    def setName(self, name):
        self._name.setText(name)

    def selected_parameter(self):
        return self._chosen_parameter.text()
    def setSelected_parameter(self, pname):
        self._chosen_parameter.setText(pname)
    
    def axis_parameters(self):
        return self._list.list()

    def setAxis_parameters(self, itemList):
        self._list.setList(itemList)

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

    def array(self):
        if self._param_array.checkState() == Qt.Checked:
            return True
        else:
            return False
    def setArray(self, onoff):
        if onoff:
            self._param_array.setCheckState(Qt.Checked)
        else:
            self._param_array.setCheckState(Qt.Unchecked)
    
    def axis_from_parameters(self):
        if self._from_params.checkState() == Qt.Checked:
            return True
        else:
            return False
    def setAxis_from_prameters(self, onoff):
        if onoff:
            self._from_params.setCheckState(Qt.Checked)
        else:
            self._from_params.setCheckState(Qt.Unchecked)
    
    # slots:

    

def test():
    app = QApplication([])
    c = QMainWindow()
    w = SummaryEditor(c)

    w.setAxis_parameters(['a', 'b', 'c', 'd'])

    c.setCentralWidget(w)
    c.show()
    app.exec()

if __name__ == "__main__":
    test()        
