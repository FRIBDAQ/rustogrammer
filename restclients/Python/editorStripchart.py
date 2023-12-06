'''  This module will provide a 2d spectrum editor when implemented.
'''

from PyQt5.QtWidgets import QLabel

class StripChartEditor(QLabel):
    def __init__(self, *args):
        super().__init__(*args)
        self.setText('Not Implemented yet')
