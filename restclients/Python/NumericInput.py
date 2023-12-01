''' This module contains Comboboxes and validators for 
    Different types of numeric inputs.  The comboboxes are editable with the
    idea that frequently used values will therefore be added to the
    combobox and be selectable rather than needing to be typed in.

'''

from PyQt5.QtWidgets import (QWidget, QComboBox, QMainWindow, 
    QApplication, QHBoxLayout
)
from PyQt5.QtGui import  (QIntValidator, QDoubleValidator)


''' RealInputBox - this is editable and supports a validator for Floats.
    the limit setting methods are re-exported/delegated for simplicity.
    It can be a bad thing for you to change the validator if you don't know
    what you're doing.
'''
class RealInputBox(QComboBox):
    def __init__(self, *args):
        super().__init__(*args)
        self.setEditable(True)
        self.v = QDoubleValidator(self)
        self.setValidator(self.v)
        self.setInsertPolicy(QComboBox.InsertAtTop)
        self.setSizeAdjustPolicy(QComboBox.AdjustToContents)
        self.setMaxVisibleItems(20)
    def lowLimit(self): 
        return self.validator().bottom()
    def setLowLimit(self, value) :
        self.validator().setBottom(value)
    def upperLimit(self):
        return self.validator().top()
    def setUpperLimit(self, value) :
        self.validator().setTop(value)

class IntegerComboBox(RealInputBox):
    def __init__(self, *args):
        super().__init__(*args)
        self.v = QIntValidator(self)
        self.setValidator(self.v)

class UnsignedComboBox(IntegerComboBox):
    def __init__(self, *args):
        super().__init__(*args)
        self.setLowLimit(0)


def test():
    app = QApplication([])
    w   = QMainWindow()
    l   = QHBoxLayout()
    wd = QWidget()
    f   = RealInputBox(w)
    f.setLowLimit(-100.0)
    f.setUpperLimit(100.0)
    i   = IntegerComboBox(w)
    i.setLowLimit(-100)
    i.setUpperLimit(100)
    u   = UnsignedComboBox(w)
    u.setUpperLimit(1024)

    l.addWidget(f)
    l.addWidget(i)
    l.addWidget(u)
    wd.setLayout(l)
    w.setCentralWidget(wd)

    w.show()
    app.exec()
