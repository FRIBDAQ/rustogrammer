from PyQt5.QtWidgets import (
    QTabWidget, QWidget, 
    QApplication, QMainWindow
)
from PyQt5.QtCore import pyqtSignal

import TrueFalseConditionEditor
import CompoundConditionEditor
from capabilities import (
    ConditionTypes, get_supported_condition_types, set_client, get_client
)
from rustogramer_client import rustogramer as rc, RustogramerException 
from spectrumeditor import error
from gatelist import common_condition_model

''' This module provides a tabbed widget that is the gate editing part of
    the Gate tab.  As with the spectrum editor, each supported gate type.
    The key  driver is a set of widgets and controllers associated with them
    that are used to edit each gate type.  There is alwso some capabilities query
    magic used to supress tabs for editor views that are not suported.
    
    What drives all of this is a map keyed by condition type with values
    that are n-tuples containing:
    *  The tab label for that gate type
    *  The class for that gate type's editor view.
    *  The class for that gate type's editor controller.
    
    
'''

#   gate controller types:

class GateController:          # Base class
    def __init__(self, view, client, editor):
        self._view = view
        self._client = client
        self._editor = editor
    

class ConstantGateController(GateController):
    # Base class for  T and F gates.
    def __init__(self, view, client, editor):
        super().__init__(view, client, editor)
        view.commit.connect(self.make_gate)
    
    def make_gate(self):
        name = self._view.name()
        if name == '' or name.isspace():
            return
        type = self._view.gate_type()
        try:
            if type:
                self._client.condition_make_true(name)
            else:
                self._client.condition_make_false(name)
        except RustogramerException as e:
            error(f'Failed to create condition: {name}: {e}')
            return
        
        # success if we got here so:
        
        self._editor.signal_removal(name)    # in case this is a replace.
        self._editor.signal_added(name)
        self._view.setName('')               # Clear the gate name on success.

    
class TrueGateController(ConstantGateController):
    def __init__(self, view, client, editor):
        super().__init__(view, client, editor)
        view.setGate_type(True)
        
        
    
class FalseGateController(ConstantGateController):
    def __init__(self, view, client, editor):
        super().__init__(view, client, editor)
        view.setGate_type(False)


class CompoundGateController(GateController):
    # Base class for And/Or gates, concrete derivations
    # must implement make_gate(name, dependencies).
    # We'll handle the errors and signalling.
    #
    def __init__(self, view, client, editor):
        super().__init__(view, client, editor)
        view.commit.connect(self._commit)
    
    def _commit(self):
        name = self._view.name()
        if name ==  '' or name.isspace():
            return                 # Just do nothing if no name.
        try:
            self.make_gate(name, self._view.dependencies())
        except RustogramerException as e:
            error(f'Failed to create condition {name} : {e}')
            return

        # Success so signal and blank the name:
        
        self._editor.signal_removal(name)
        self._editor.signal_added(name)
        self._view.setName('')
    
class AndGateController(CompoundGateController):
    def __init__(self, view, client, editor):
        super().__init__(view, client, editor)
    def make_gate(self, name, dependencies):
        self._client.condition_make_and(name, dependencies)

class OrGateController(CompoundGateController):
    def __init__(self, view, client, editor):
        super().__init__(view, client, editor)
    def make_gate(self, name, dependencies):
        self._client.condition_make_or(name, dependencies)
            
            

_condition_table = {
    ConditionTypes.And: ("And", CompoundConditionEditor.EditorView, AndGateController),
    ConditionTypes.Band: ("Band", QWidget, GateController),
    ConditionTypes.Contour: ("Contour", QWidget, GateController),
    ConditionTypes.FalseCondition: ('False', 
        TrueFalseConditionEditor.TrueFalseView,  FalseGateController
    ),
    ConditionTypes.TrueCondition: ('True', 
        TrueFalseConditionEditor.TrueFalseView, TrueGateController
    ),
    ConditionTypes.GammaContour: ("G Contour", QWidget, GateController),
    ConditionTypes.Not: ("Not", QWidget, GateController),
    ConditionTypes.Or: ("Or", CompoundConditionEditor.EditorView, OrGateController),
    ConditionTypes.Slice: ('Slice', QWidget, GateController),
    
}

class ConditionEditor(QTabWidget):
    
    '''This provides the tabbed widget and stocks it with tabs that are germane to the
       server program.   This module aslo includes 'controller' classes, instances of which
       are created and used to handle events/signals from the editor views that are in each tab
       to actually do the stuff needed to the model (server program) to create/modify gates.
    '''
    condition_removed = pyqtSignal(str)
    condition_added   = pyqtSignal(str)
    def __init__(self, *args):
        super().__init__(*args)
        client = get_client()
        supported_conditions = get_supported_condition_types()
        self._controllers = dict()
        for ctype in _condition_table.keys():
            if ctype in supported_conditions:
                (label, viewclass, controllerclass) = _condition_table[ctype]
                widget = viewclass(self)
                controller = controllerclass(widget, client, self)
                self._controllers[label] = controller
                self.addTab(widget, label)
    #  Utilities the controller might need.
    
    def signal_removal(self, name):
        self.condition_removed.emit(name)
    def signal_added(self, name):
        self.condition_added.emit(name)
        


if __name__ == '__main__':
        set_client(rc({'host':'localhost', 'port':8000}))
        common_condition_model.load(get_client())
        app = QApplication([])
        win = QMainWindow()
        wid = ConditionEditor()
        
        win.setCentralWidget(wid)
        win.show()
        app.exec()
        
        