''' This module provides a spectrum type selector.
The user is assumed to have stocked it with a list of
spectrum type names and corresponding capabilities.SpectrumTypes
The signal 'selected' is emitted when a spectrum type is selected
from the list and the slot is provided with the corresponding
spectrum type Enum as well as the type name (which can be used to
title other UI elements e.g. a labeled framo for editing spectra of
type type).
'''

from PyQt5.QtWidgets import (
    QTableView, QMainWindow, QComboBox, QApplication
)
from PyQt5.QtCore import *

from capabilities import SpectrumTypes

class SpectrumTypeSelector(QComboBox):
    selected = pyqtSignal(str, SpectrumTypes)
    def __init__(self, parent = None):
        super().__init__(parent)
        self.setEditable(False)
        self.currentIndexChanged.connect(self.select_type)
    '''
       Override for add item that forces you to provide user data
       which normally is a SpectrumTypes value
       e.g. box.addItem('1-d', SpectrumTypes.Oned)
    '''
    def addItem(self, text, value):
        super().addItem(text, value)

    def select_type(self, index):
        sptype = self.currentData()
        text = self.currentText()
        self.selected.emit(text, sptype)

### Test code:

def sel_handler(type_str, type_val):
    print("Selected: ", type_str, type_val)

def test():
    app = QApplication(['test'])
    win = QMainWindow()
    list = SpectrumTypeSelector()
    list.addItem('1d', SpectrumTypes.Oned)
    list.addItem('2d', SpectrumTypes.Twod)
    list.addItem('Projection', SpectrumTypes.Projection)
    win.setCentralWidget(list)
    list.selected.connect(sel_handler)
    win.show()
    app.exec()