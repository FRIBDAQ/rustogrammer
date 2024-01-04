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
    QAbstractItemView, QHeaderView,
    QApplication, QMainWindow
)
from PyQt5.QtGui import QDoubleValidator, QIntValidator
from PyQt5.QtCore import pyqtSignal, Qt


from  ParameterChooser import (
    Chooser as PChooser, 
    LabeledParameterChooser as PLChooser
)


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
        self.setColumnWidth(0, 75)
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

''' The parameter widget it's centered around a Paramteer Table but has
    other important controls:

    +------------------------------------------------------------+
    |  (parameter chooser) [New] [Replace]  [] array             |
    |  [  Load ] [  Set ]  [ Change Spectra]                     |
    | +--------------------------------------------------------+ |
    |    parameter table ...                                     |
    | +--------------------------------------------------------+ |
    +------------------------------------------------------------+

'''
class ParameterEditor(QWidget):
    '''
        This is the view for the parameter editor.  It provides
        
        Attributes:

        * parameter - Value of the parameter chooser
        * array     - bool state of the array checkbox.
        * table      - (readonly) the encapsulated ParameterTable widget.

        Signals:
        *  newRow - the New button was clicked. The controller
           should fetch the parameter name, and create a new row
           containing its attributes.
        *  replaceRow - The Replace button was clicked.  The
           controller should:
            -  Determine the current row however they deem appropriate.
            -  Replace the contents of that row with the attributes
               of the selected parameter.
        *  loadClicked - The seleced row contents should be updated 
           with the current attributes of the parameter.
        *  setClicked  - The selected row attributes should be 
           loaded into the associated parameters.  Note  that if
           array is true, where row parameters are templates of a
           parameter array, all elements should be modified.
        *  changeClicked - Spectra that use parameters in the 
           selected rows should have the appropriate axis updated to
           match the attributes of the specified parameters.  Array should
           function as for setClicked - that is each parameter name
           that is a template for an array of parameters should 
           modify spectra that use any element of the array 
        
        What is an array of parameters?  This is a bit expanded
        from the TreeParameter's definition.  In our case if
        a parameter name is of the form a. ... .tail,  tail is replaced
        by * and any parameter whose name matches that glob pattern
        is considered part of the array.  For example consider
        parmaeters
        a.1, a.2 a.c  I _think_ that in treegui, only a.1 and 1.2 
        are considered array alements but in this GUI, a.1,a.2, and a.c
        are all array elements (think of allowing fully textual
        subscripts).
           
    '''
    newRow        =   pyqtSignal()
    replaceRow    = pyqtSignal()
    loadclicked   =   pyqtSignal()
    setclicked    =   pyqtSignal()
    changeclicked =   pyqtSignal()

    def __init__(self, *args):
        super().__init__(*args)

        layout = QVBoxLayout()    # Overall layout.

        top_row = QHBoxLayout()   # Top row of widgets:

        self._parameter = PLChooser(self)
        top_row.addWidget(self._parameter)

        self._new = QPushButton('New', self)
        top_row.addWidget(self._new)

        self._replace = QPushButton('Replace', self)
        top_row.addWidget(self._replace)

        self._array = QCheckBox('Array', self)
        top_row.addWidget(self._array)

        layout.addLayout(top_row)

        action_row = QHBoxLayout()
        self._load = QPushButton('Load', self)
        action_row.addWidget(self._load)
        self._set  = QPushButton('Set', self)
        action_row.addWidget(self._set)
        self._changeSpectra = QPushButton('Change Spectra', self)
        action_row.addWidget(self._changeSpectra)

        layout.addLayout(action_row)

        self._table = ParameterTable(self)
        layout.addWidget(self._table)

        self.setLayout(layout)

        # Connect the button click relays to our defined signals:

        self._new.clicked.connect(self.newRow)
        self._replace.clicked.connect(self.replaceRow)
        self._load.clicked.connect(self.loadclicked)
        self._set.clicked.connect(self.setclicked)
        self._changeSpectra.clicked.connect(self.changeclicked)

    #  Implement the attributes:

    def parameter(self):
        ''' Return the currently selected parameter if None is returned
           a full parameter path has not been selected.
        '''
        name = self._parameter.parameter()
        if name.isspace() or (name == ''):
            return None
        else:
            return name
    def setParameter(self, new_name):
        self._parameter.setParameter(new_name)
    
    def array(self):
        if self._array.checkState() == Qt.Checked:
            return True
        else:
            return False
    def setArray(self, value):
        if value:
            self._array.setCheckState(Qt.Checked)
        else:
            self._array.setCheckState(Qt.Unchecked)
    def table(self):
        return self._table


#-------------------- Test code -------------------------

def new_row():
    print("Want a new row")

def replace_row():
    print("replace row: ", w.table().currentRow())

def log(what):
    rows = w.table().selected_rows()
    array = w.array()

    print(what , rows, " array state: ", array)

def load():
    log('load data')

def set_params():
    log("set parameters")

def change():
    log('change spectra')
if __name__ == '__main__':
    app = QApplication([])
    main = QMainWindow()

    w = ParameterEditor()
    w.newRow.connect(new_row)
    w.replaceRow.connect(replace_row)
    w.loadclicked.connect(load)
    w.setclicked.connect(set_params)
    w.changeclicked.connect(change)

    table = w.table()
    for name in ['moe', 'curly', 'shemp', 'laurel', 'hardy', 'stan', 'ollie']:
        table.add_row(name)

    main.setCentralWidget(w)

    main.show()
    app.exec()