''' This file contains the widgets needed to edit parameters.  
This includes:
   *  Limit - a QLineEdit widget  that is either empty or a valid floating number.
   *  Bins  - A QLineEdit widget constrained to have positive non-zero integers.
   * ParameterTable - Which is a dynamically expandable
     table of parameter loaders and widgets describing  parameters.


'''
from PyQt5.QtWidgets import (
    QLineEdit, QPushButton, QCheckBox, QWidget, QLabel, QTableWidget,
    QHBoxLayout, 
    QApplication, QMainWindow
)
from PyQt5.QtGui import QDoubleValidator, QIntValidator
from PyQt5.QtCore import pyqtSignal

from  ParameterChooser import (
    Chooser as PChooser, 
    LabeledParameterChooser as PLChooser
)
import capabilities

class Limit(QLineEdit):
    def __init__(self, *args):
        super().__init__(*args)
        self.setValidator(QDoubleValidator(self))
    
    def value(self):
        t = self.text()
        if t.isspace():
            return None
        return float(t)
    def setValue(self, value):
        if value is not None:
            self.setText(f'{value}')
        else: 
            self.setText('')

class Bins(QLineEdit):
    def __init__(self, *args):
        super().__init__(*args)
        validator = QIntValidator(self)
        validator.setBottom(1)
        self.setValidator(validator)
    
    def value(self):
        t = self.text()
        if t.isspace():
            return None
        return int(t)
    def setValue(self, value):
        if value is None:
            self.setText('')
        else:
            self.setText(f'{value}')

class ParameterTable(QTableWidget):
    '''
        This is a table of parameters.  Each line in the table
        consists of a label (name of the parameter) two limit widgets
        (high, low) and a Bins widget as well as an unconstrained line edit
        widget containing the units.  Unlike the SpecTcl tree GUI, the
        GUI wrapping this will have load/set/change_spectra buttons that
        operate on all lines in the table that are selected.  Similarly for the 'array'
        checkbutton.  

        Those are all external to the ParameterTable widget which is only a table.

        Attributes (per row)
            name  - Name of the parameter on the row.
            low   - low value for the row,
            high  - High v alue for the row.
            bins  - bins for the row.
            units - Units for the row.

        Methods:
           addRow -  Add a new row to the table.
           setRow -  Set the contents of a row.
           getRow -  Retrieves all there is about a row as a map containing:
                    ['name'], ['low'], ['high'], ['bins'], ['units'] with obvious
                    meanings.
           selectedRows- return the selected rows.
    '''
    def __init__(self, *args):
        super().__init__(*args)
        self.setColumnCount(5)
        self.showGrid()
        self.setHorizontalHeaderLabels(['Name', 'Low', 'High', 'Bins', 'Units'])


if __name__ == '__main__':
    app = QApplication([])
    main = QMainWindow()

    w = ParameterTable()
    main.setCentralWidget(w)

    main.show()
    app.exec()