'''  This module will provide a bitmaws spectrum editor when implemented.
'''

from PyQt5.QtWidgets import (
    QWidget, QComboBox, QLabel,  QLineEdit, QPushButton,
    QVBoxLayout, QHBoxLayout,
    QApplication, QMainWindow
)
from PyQt5.QtCore import Qt, pyqtSignal
from ParameterChooser import LabeledParameterChooser as Parameter

class BitmaskEditor(QWidget):
    def __init__(self, *args):
        super().__init__(*args)
        
        layout = QVBoxLayout()
        name_l = QHBoxLayout()
        name_l.addWidget(QLabel("Name", self))
        self._name = QLineEdit(self);
        name_l.addWidget(self._name)
        layout.addLayout(name_l)

        param_l = QHBoxLayout()
        param_l.addWidget(QLabel('Parameter:', self))
        self._param = Parameter(self)
        param_l.addWidget(self._param)
        layout.addLayout(param_l)

        bits_l = QHBoxLayout()
        bits_l.addWidget(QLabel('Bits:', self))
        self._bits = QComboBox(self)
        self._bits.setEditable(False)
        for i in range(1,32):
            self._bits.addItem(f'{i}')
        bits_l.addWidget(self._bits)
        layout.addLayout(bits_l)

        self._commit = QPushButton('Create/Replace', self)
        layout.addWidget(self._commit)

    

        self.setLayout(layout)


if __name__ == "__main__":
    app = QApplication([])
    c   = QMainWindow()

    w  = BitmaskEditor()
    c.setCentralWidget(w)



    c.show()
    app.exec()

