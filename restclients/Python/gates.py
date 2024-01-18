'''
    The Gates widget in this package is the widge for the GUI's gate tab.
    At the top it contains the gate editor tabbed widget while at the bottom, the
    filteredgates.FilteredConditions.  Normally, no signals escape this module from either
    widget.  We are aware of the client and thus can update both the parameter
    and the condition models as needed.
'''

from PyQt5.QtWidgets import(
    QWidget, QVBoxLayout
)
from PyQt5.QtCore import pyqtSignal

import conditionEditor
import filteredgates

class Gates(QWidget):
    '''
    Widget that is what belongs in the Gates tab of the GUI.
    Signals:
        update_gates  - The conditions model must be updated.
        update_params - The parameter omdel must be updated.
        select        - The selected conditions changed.
        clear         - The pattern for the gates list was cleared.
        load          - Load a single selected condition into the appropriate editor and
                        select it.
        delete_selected - Delete all selected conditions
        delete_displayed- Delete all conditions in the list.
        condition_removed - A named condition was deleted from the server
        condition_added - A named condition was added to the server
        NOTE: typically, condition_removed and condition_added come in pairs
              with condition_removed and then immediately following a condition_added both
              with the same name.
    Slots:
        validate_load - Called when the load button is clicked to ensure that
            load can be emitted.  If  overriding this to add validations, call super().validate_load()
            last to ensure the signal does not get emitted prematurely.
        select    - Called when the gate selection has changed.  The current selection is 
                    marshalled and our select is emitted with that selection as a a parameter
    Attributes:
        gatelist - (readonly) returns the filtered gate list widget.
        editor   - (readonly) returns the editor widget.
        actions  - (readonly) returns the GateActionView at the bottom of the table.
    '''
    update_gates = pyqtSignal()
    update_params= pyqtSignal()
    select      = pyqtSignal(list)
    clear       = pyqtSignal()
    load        = pyqtSignal(dict)
    delete_selected = pyqtSignal(list)
    delete_displayed = pyqtSignal(list)
    condition_removed = pyqtSignal(str)
    condition_added   = pyqtSignal(str)
    
    def __init__(self, *args):
        super().__init__(*args)
        
        layout = QVBoxLayout()
        
        # Top line is a condition editor:
        
        self._editor = conditionEditor.ConditionEditor(self)
        layout.addWidget(self._editor)
        
        # Second row down is the GateActionView:
        
        self._actions = filteredgates.GateActionView(self)
        layout.addWidget(self._actions)
        
        # Finally the  list itself:
        
        self._list = filteredgates.FilteredConditions(self)
        layout.addWidget(self._list)
        
        self.setLayout(layout)
    
    
#-----------------------------------------------------------------
# Test code:

if __name__ == '__main__':
    from rustogramer_client import rustogramer as rc
    from gatelist import common_condition_model    # Condition model.
    from ParameterChooser import update_model      # Update parameter model.   
    from capabilities import set_client 
    
    from PyQt5.QtWidgets import (
        QApplication, QMainWindow
    )
    
    client = rc({'host': 'localhost', 'port':8000})
    set_client(client)
    common_condition_model.load(client)
    update_model(client)
    
    app = QApplication([])
    win = QMainWindow()
    
    wid = Gates()
    
    win.setCentralWidget(wid)
    win.show()
    app.exec()
    