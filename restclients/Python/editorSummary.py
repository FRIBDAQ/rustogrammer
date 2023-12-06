'''  This module will provide a summary spectrum editor when implemented.
'''

from PyQt5.QtWidgets import QLabel

class SummaryEditor(QLabel):
    def __init__(self, *args):
        super().__init__(*args)
        self.setText('Not Implemented yet')
