from PyQt5.QtWidgets import (
    QLineEdit, QRadioButton, QLabel, QWidget,
    QVBoxLayout, QHBoxLayout,
    QApplication, QMainWindow, QPushButton
)

class TrueFalseView(QWidget):
    ''' Provides a view for editing a true or false gate.
        The view is quite simple - a name entry and radio buttons to 
        selecte between true/false type.
        Attributes:
           name - Name of gate
           gate_type - True/False.
        No Signals,
        No Slots.
    '''
    def __init__(self, *args):
        super().__init__(*args)
        
        layout = QVBoxLayout()
        
        # Top row is the name label and name entry:
        
        top = QHBoxLayout()
        top.addWidget(QLabel('Name:', self))
        self._name = QLineEdit(self)
        top.addWidget(self._name)
        
        layout.addLayout(top)
        
        #  Seocond row is the True/False radio buttons.
        
        bottom = QHBoxLayout()
        self._true = QRadioButton('True', self)
        bottom.addWidget(self._true)
        self._false = QRadioButton('False', self)
        bottom.addWidget(self._false)
        
        layout.addLayout(bottom)
        
        self.setLayout(layout)
        self.setGate_type(True)
        
    #  Implement the attributes.
    
    def name(self):
        return self._name.text()
    def setName(self, txt):
        self._name.setText(txt)
        
    def gate_type(self):
        return self._true.isChecked()
    def setGate_type(self, which):
        if which:     # Assume exclusivity applies to programmatic changes.
            self._true.setChecked(True)
        else:
            self._false.setCHecked(True)
            

#---------------------- Testing -------------------------------

def _show():
    print('Name:', editor.name())
    print('Type', editor.gate_type())
    
    editor.setGate_type(True)
    editor.setName('')

if __name__ == "__main__":
    app = QApplication([])
    win = QMainWindow()
    
    wid = QWidget()
    layout = QVBoxLayout()
    editor = TrueFalseView()
    layout.addWidget(editor)
    show = QPushButton("Click-me")
    show.clicked.connect(_show)
    layout.addWidget(show)
    
    wid.setLayout(layout)
    win.setCentralWidget(wid)
    
    win.show()
    app.exec()