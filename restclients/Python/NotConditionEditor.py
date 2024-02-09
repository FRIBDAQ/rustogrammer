'''
This module provides an EditorView object for editing/creating a Not condition.
Not conditions take a single dependent condition and evaluate the logical inverse of
that condition
'''

from PyQt5.QtWidgets import (
    QLabel, QLineEdit, QPushButton, QWidget,
    QVBoxLayout, QHBoxLayout
)
from PyQt5.QtCore import pyqtSignal

from gatelist import ConditionChooser

class EditorView(QWidget):
    '''
    The editor view (requires a controller to work).  The layout
    has the usual name entry at the top.  In the middle is gate chooser
    used to select the condition to invert.
    At the bottom, the usual Create/Replace pushbuton.
    
    Signals:
        commit - The Create/Replace button was clicked.
    Attributes:
        name  - Name of the condition.
        condition - dependent condition name.
        
    '''
    commit = pyqtSignal()
    
    def __init__(self, *args):
        super().__init__(*args)
        
        layout = QVBoxLayout()
        
        # Top has name label and entry:
        
        top = QHBoxLayout()
        top.addWidget(QLabel('Name:', self))
        self._name = QLineEdit(self)
        top.addWidget(self._name)
        
        layout.addLayout(top)
        
        #  Middle is the dependent gate:
        
        mid = QHBoxLayout()
        mid.addWidget(QLabel('Gate: ', self))
        self._condition = ConditionChooser(self)
        mid.addWidget(self._condition)
        
        layout.addLayout(mid)
        
        #  Bottom is just the commit button:
        
        layout.addStretch(1)
        self._commit = QPushButton('Create/Replace', self)
        layout.addWidget(self._commit)
        
        self.setLayout(layout)
        
        # Export the commit signal:
        
        self._commit.clicked.connect(self.commit)
        
    # Implement the attributes:
    
    def name(self):
        return self._name.text()
    def setName(self, txt):
        self._name.setText(txt)
    
    def condition(self):
        return self._condition.currentText()
    def setCondition(self, name):
        self._condition.setCurrentText(name)

#------------------------ Test code --------------------------------------

def create():
    print("Create not gate:")
    print("  name   : ", widget.name())
    print('  inverts: ', widget.condition())

if __name__ == '__main__':
    from gatelist import common_condition_model
    from PyQt5.QtWidgets import QApplication, QMainWindow
    from rustogramer_client import rustogramer as rc
    
    common_condition_model.load(rc({'host': 'localhost', 'port':8000}))
    
    app = QApplication([])
    win = QMainWindow()
    
    widget = EditorView()
    widget.commit.connect(create)
    win.setCentralWidget(widget)
    
    win.show()
    app.exec()