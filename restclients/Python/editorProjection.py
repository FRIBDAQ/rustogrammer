'''  This module will provide a projection spectrum editor when implemented.
'''

from PyQt5.QtWidgets import QLabel

class ProjectionEditor(QLabel):
    def __init__(self, *args):
        super().__init__(*args)
        self.setText('Not Implemented yet')
