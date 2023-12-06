'''  This module will provide a bitmaws spectrum editor when implemented.
'''

from PyQt5.QtWidgets import QLabel

class BitmaskEditor(QLabel):
    def __init__(self, *args):
        super().__init__(*args)
        self.setText('Not Implemented yet')
