'''
    This file contains code to make a standard item model to hold tree variables.
    Each tree variable has the following simple 'textual' fields:
    
    name - the name of the variable.
    value - The value of the variable.
    units - the variable units.
    
    We also provide a load method to fill the model given a client.
'''

from PyQt5.QtGui import QStandardItemModel, QStandardItem
from PyQt5.QtCore import Qt, pyqtSignal
from PyQt5.QtWidgets import (
    QComboBox, QWidget, QPushButton, QCheckBox,
    QHBoxLayout, QVBoxLayout
)


class TreeVariableModel(QStandardItemModel):
    _colheadeings = ['Name', 'Value', 'Units']
    def __init__(self, *args):
        super().__init__(*args)
    def headerData(self, col, orient, role):
        if role == Qt.DisplayRole:
            if orient == Qt.Horizontal:
                return self._colheadings[col]
            else:
                return None
    # Public methods
                
    def load(self, client):
        raw_data = client.treevariable_list()
        self.clear()             # Get rid of prior data.
        for var in raw_data['detail']:
            self._add_line(var)
    
    def get_definition(self, name):
        ''' 
        Finds the one item that matches 'name' and returns it's definition.
        If there is no match, returns None  if there is a match returns a map of
        'name'   - name of the item.
        'value'  - Value of the item.
        'units'  - Units of the item.
        
        '''
        matches = self.findItems(name, Qt.MatchExactly, 0)   # only match name field.
        if len(matches) == 1:
            index = self.indexFromItem(matches[0])
            row   = index.row()
            
            valueItem = self.item(row, 1)
            unitsItem = self.item(row, 2)
            value = float(valueItem.text())
            units = unitsItem.text()
            
            return {
                'name': name, 'value': value, 'units': units
            }
            
        else:
            return None     # We don't know how to handle multiple matches.
    # Private methods:
    
    def _add_line(self, var):
        name = var['name']
        value= var['value']
        strvalue = f'{value}'
        units= var['units']
        
        self.appendRow([
            QStandardItem(name), QStandardItem(strvalue), QStandardItem(units)
        ])
        
common_treevariable_model = TreeVariableModel()
# Now some views:

class VariableChooser(QComboBox):
    '''
       Combobox that allows users to choose a variable.
    '''
    def __init__(self, *args):
        super().__init__(*args)
        self.setEditable(False)
        self.setModel(common_treevariable_model)
        
class VariableSelector(QWidget):
    '''
    Provides the controls needed to choose tree variables and add them to some
    editable thing.  This consists of a variable chooser and the buttons:
    Append, Replace, Remove, Load and Set. A checkbutton called 'array' is also present.
    
    Signals:
        append  - the append button was clicked.
        replace - the replace button was clicked.
        remove  - the remove button was clicked.
        load    - the load button was clicked.
        set     - the set button was clicked.
        
    Slots:
        None
    Attributes:
        name - (readonly) Selected name.
        definition - (readonly) - definition of selected item.
        array - Value of the array checkbutton.
        
    '''
    append = pyqtSignal()
    replace = pyqtSignal()
    remove = pyqtSignal()
    load   = pyqtSignal()
    set    = pyqtSignal()
    
    def __init__(self, *args):
        super().__init__(*args)
        
        layout = QVBoxLayout()
        
        # Top row is the chooser, the append replace and array controls:
        
        top = QHBoxLayout()
        
        self._chooser = VariableChooser(self)
        top.addWidget(self._chooser)
        self._append = QPushButton('Append', self)
        top.addWidget(self._append)
        self._replace = QPushButton('Replace', self)
        top.addWidget(self._replace)
        self._remove = QPushButton('Remove', self)
        top.addWidget(self._remove)
        self._array = QCheckBox('Array', self)
        top.addWidget(self._array)
        
        layout.addLayout(top)
        
        # The bottom row is just the Load and Set buttons...with a strectch on the
        # end:
        
        bottom = QHBoxLayout()
        
        self._load = QPushButton('Load', self)
        bottom.addWidget(self._load)
        self._set  = QPushButton('Set', self)
        bottom.addWidget(self._set)
        bottom.addStretch(1)
        
        layout.addLayout(bottom)
        
        self.setLayout(layout)
    
        # Relay signals:
        
        self._append.clicked.connect(self.append)
        self._replace.clicked.connect(self.replace)
        self._remove.clicked.connect(self.remove)
        self._load.clicked.connect(self.load)
        self._set.clicked.connect(self.set)
    # Implement attributes:
    
    def name(self):
        return self._chooser.currentText()
    def definition(self):
        name = self._chooser.currentText()
        return self._chooser.model().get_definition(name)
    def array(self):
        return self._array.checkState() == Qt.Checked
    def setArray(self, value):
        if value:
            newstate = Qt.Checked
        else:
            newstate = Qt.Unchecked
        self._array.setCheckState(newstate)
    
    

#------------------------------------- Test code ------------------------

def selected():
    print(wid.definition())
def append():
    print("append")
    selected()
def replace():
    print('replace')
    selected()
def remove():
    print('remove')
    selected()
def load():
    print('load', wid.array())
def setvalue(): 
    print('set', wid.array())

if __name__ == '__main__':
    
    
    from rustogramer_client import rustogramer as rcl
    from PyQt5.QtWidgets import (QApplication, QMainWindow)
    
    client = rcl({'host':'localhost', 'port': 8000})
    common_treevariable_model.load(client)
    
    app = QApplication([])
    win = QMainWindow();
    
    wid = VariableSelector()
    wid.append.connect(append)
    wid.replace.connect(replace)
    wid.remove.connect(remove)
    wid.load.connect(load)
    wid.set.connect(setvalue)
    
    win.setCentralWidget(wid)
    win.show()
    app.exec()
        
    
    

