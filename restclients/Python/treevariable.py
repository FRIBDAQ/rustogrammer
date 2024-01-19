'''
    This file contains code to make a standard item model to hold tree variables.
    Each tree variable has the following simple 'textual' fields:
    
    name - the name of the variable.
    value - The value of the variable.
    units - the variable units.
    
    We also provide a load method to fill the model given a client.
'''

from PyQt5.QtGui import QStandardItemModel, QStandardItem
from PyQt5.QtCore import Qt
from PyQt5.QtWidgets import (QComboBox)


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

#------------------------------------- Test code ------------------------

if __name__ == '__main__':
    def definition(name):
        info = wid.model().get_definition(name)
        print(info)
    
    from rustogramer_client import rustogramer as rcl
    from PyQt5.QtWidgets import (QApplication, QMainWindow)
    
    client = rcl({'host':'localhost', 'port': 8000})
    common_treevariable_model.load(client)
    
    app = QApplication([])
    win = QMainWindow();
    
    wid = VariableChooser()
    wid.textActivated.connect(definition)
    
    win.setCentralWidget(wid)
    win.show()
    app.exec()
        
    
    

