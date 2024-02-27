
''' A common need in spectrum editors is to select a list of parameters.
    This module provides re-usable widgets to support that.  The idea as that
    we have a parameter selector and an array checkbutton.  This is then
    paired with either one or two editable lists which can be stocked by
    interacting with signals from these widgets by the client.
    The two widgets we provide are:
    
    SingleList - Which selects a single list of parameters and
    DoubleList - Which selects a pair of lists (e.g. x and y parameters for Twod sum or Gamma deluxe).
    
'''


from PyQt5.QtWidgets import (
    QLabel, QVBoxLayout, QHBoxLayout, QWidget, QCheckBox
)
from PyQt5.Qt import *
from PyQt5.QtCore import pyqtSignal
from ParameterChooser import LabeledParameterChooser as pChooser
from editablelist import EditableList

class SingleList(QWidget):
    '''
       Provides a selection into a single list oif parameters.  
       Attributes:
          selectedParameters - The list of parameters currently selected
          parameter          - The parameter in the parameter selection.
          chooserLabel       - label above the parameter chooser (defaults to 'parameter')
          listLabel          = Label on list box, defaults to 'Parameters'
          array              - state of array checkbutton.
        Signals:
           add - Add a parameter to the list.
           remove - A parameter was removed from the list.  This signal
                 is passed the name of the removed parameter.
                 NOTE: Normally it's not necessary to catch this signal as
                 the editable list box handles removal autonomously
        Methods:
            appendItem - Adds a parameter to the list
            insertItem - Inserts an item at a specific row.
            clear      - Clears th parameter list
            
            Note:  These are just delegates to the editable list:
            
    '''
    add = pyqtSignal()
    remove = pyqtSignal(str)
    
    def __init__(self, *args):
        super().__init__(*args)
        layout = QHBoxLayout()
        
        #  At the left is a labeled parameter chooser with a label
        # above it:
        
        left = QVBoxLayout()
        self._chooser_label = QLabel('Parameter', self)
        left.addWidget(self._chooser_label)
        self._parameter_chooser = pChooser(self)
        left.addWidget(self._parameter_chooser)
        left.addStretch(1)
        
        layout.addLayout(left)
        
        # In the middle an array checkbox/
        
        self._array = QCheckBox('array', self)
        layout.addWidget(self._array)
        
        #  At the right is just an editable list box:
        
        self._list = EditableList('Parameters', self)
        layout.addWidget(self._list)
        
        layout.addStretch(1)
        
        # Set our layout
        
        self.setLayout(layout)
        
        # Relay the Editable list signals:
        
        self._list.add.connect(self.add)
        self._list.remove.connect(self.remove)
        
    #  Implement the attributes:
    
    def selectedParameters(self):
        return self._list.list()
    def setSelectedParameters(self, l):
        self._list.setList(l)
        
    def parameter(self):
        return self._parameter_chooser.parameter()
    def setParameter(self, pname):
        self._parameter.setParameter(pname)
        
    def chooserLabel(self):
        return self._chooser_label.text()
    def setChooserLabel(self, t):
        self._chooser_label.setText(t)
        
    def listLabel(self):
        return self._list.label()
    def setListLabel(self, l):
        self._list.setLabel(l)
        
    def array(self):
        return self._array.checkState() == Qt.Checked
    def setArray(self, selected):
        if selected:
            state = Qt.Checked
        else:
            state =Qt.Unvhecked
        self._array.setCheckState(state)
        
    # Public methods:
    
    def appendItem(self, s):
        self._list.addItem(s)
    def insertItem(self, row, s):
        self._list.insertItem(row, s)
    def clear(self):
        self._list.clear()
        