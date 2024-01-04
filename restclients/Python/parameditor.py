''' This file contains the widgets needed to edit parameters.  
This includes:
   *  Limit - a QLineEdit widget  that is either empty or a valid floating number.
   *  Bins  - A QLineEdit widget constrained to have positive non-zero integers.
   * ParameterTable - Which is a dynamically expandable
     table of parameter loaders and widgets describing  parameters.


'''
from PyQt5.QtWidgets import (
    QLineEdit, QPushButton, QCheckBox, QWidget, QLabel, QTableWidget,
    QHBoxLayout, QVBoxLayout,
    QAbstractItemView,
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
        print('t: ', t,)
        if t.isspace() or (t == ''):
            print('is space')
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
        if t.isspace() or (t == ''):
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
        self.setRowCount(0)
        self.showGrid()
        self.setHorizontalHeaderLabels(['Name', 'Low', 'High', 'Bins', 'Units'])
        self.setSelectionBehavior(QAbstractItemView.SelectRows)
        self.setSelectionMode(QAbstractItemView.ExtendedSelection)

    # Public methods:

    def add_row(self, name, low=None, high=None, bins=None, units = None):
        ''' Add a new row to the table.  Note that any parameters that have
          None values fill their cell with blanks.
          *  name - value to put in the name field of the row.
          *  low  - value to put in the low field of the row.
          *  high - value to put in the high field of the row
          *  units - Value to put in the units field of the row. 
        '''
        rows = self.rowCount() + 1     # After adding:
        self.setRowCount(rows)

        self.set_row(rows-1, name, low, high, bins, units)
    
    def set_row(self, row_num, name, low = None, high = None, bins = None, units = None):
        ''' Fill a row with data for a parameter. Any previous information is
            overwitten.  name, low, high, bins, units have the same meaning as for
            the add_row method. 
            *  row_num - is the number of the row to fill in.
        '''
        self._check_row(row_num)
        _name = QLabel()
        _name.setText(name)
        self.setCellWidget(row_num, 0, _name)

        _low = Limit()
        _low.setValue(low)
        self.setCellWidget(row_num, 1, _low)

        _high = Limit()
        _high.setValue(high)
        self.setCellWidget(row_num, 2, _high)

        _bins = Bins()
        _bins.setValue(bins)
        self.setCellWidget(row_num, 3, _bins)

        _units = QLineEdit()
        if units is not None:
            _units.setText(units)
        self.setCellWidget(row_num, 4, _units)

    def get_row(self, row_num):
        ''' Returns the contents of a row as a dict containing the
           keys:
           *  'name' - name of the parameter
           *  'low'  - Contents of low (None if empty).
           *  'high' - Contents of high (None if empty).
           *  'bins' - Contents of bins (None if empty).
           *  'units' - Content sof units (None if empty).
        '''
        self._check_row(row_num)
        
        name = self.cellWidget(row_num, 0).text()
        low  = self.cellWidget(row_num, 1).value()
        high = self.cellWidget(row_num, 2).value()
        bins = self.cellWidget(row_num, 3).value()
        units = self.cellWidget(row_num, 4).text()
        if units.isspace() or (units == ''):
            units = None
        
        return {'name': name, 'low': low, 'high': high, 'bins': bins, 'units':units}

    def selected_rows(self):
        ''' Returns a list of selected rows.  For ourpurposes, this is much
           simpler to use than a selection range since we can just iterate
           over the rows.
           (returns a list of ints or empty if nothing is selected).

        '''
        result = list()
        selection_extents = self.selectedRanges()
        for srange in selection_extents:
            top = srange.topRow()
            bottom = srange.bottomRow()
            for r in range(bottom, top+1):
                result.append(r)
        return result


    # Private utilities:

    def _check_row(self, row_num):
        if row_num >= self.rowCount():
            raise IndexError(f'{row_num} is not a legal row number')


#-------------------- Test code -------------------------

def list_selection():
    print(w.selected_rows())
if __name__ == '__main__':
    app = QApplication([])
    main = QMainWindow()

    cw = QWidget()
    layout = QVBoxLayout()
    w = ParameterTable()
    w.add_row('junk', 0, 1024, 1024, 'cm')
    w.add_row('stuff')
    w.add_row('last')
    print(w.get_row(0))
    print(w.get_row(1))
    layout.addWidget(w)

    b = QPushButton('Selection')
    b.clicked.connect(list_selection)
    layout.addWidget(b)
    cw.setLayout(layout)

    main.setCentralWidget(cw)

    main.show()
    app.exec()