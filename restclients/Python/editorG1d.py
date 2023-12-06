'''  This module will provide a 1d gamma spectrum editor when implemented.
'''

from PyQt5.QtWidgets import QLabel

class Gamma1DEditor(QLabel):
    def __init__(self, *args):
        super().__init__(*args)
        self.setText('Not Implemented yet')
