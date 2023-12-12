'''  This module provides a gamma 2d editor.  Really this is just a 
    summary/gamma1d editor with an additional axis input.
    We use the base class editor's axis input for the y axis and
    our additional one as the Xaxis as we have additional space above
    the axis for more stuff.

    Signals are the same for editorSummary.  Attributes, in addition to
    those of editorSummary are:

    xlow    \
    xhigh    >  X axis definition
    xbins   /
    ylow   \
    yhigh   >  Y axis definition
    ybins  /
'''

from PyQt5.QtWidgets import QLabel, QApplication, QMainWindow, QGridLayout
from editorSummary import SummaryEditor
from axisdef import AxisInput
class Gamma2DEditor(SummaryEditor):
    def __init__(self, *args):

        super().__init__(*args,from_par_row=8)

        
        # Additional axis input:
        
        self.main_layout.addWidget(QLabel('Y axis'), 8, 0)
        self._yaxis = AxisInput(self)
        self.main_layout.addWidget(self._yaxis, 7, 0)
        self.main_layout.addWidget(QLabel('X axis'), 6, 0)

if __name__ == "__main__":
    app = QApplication([])
    c   = QMainWindow()
    w   = Gamma2DEditor()
    c.setCentralWidget(w)

    c.show()
    app.exec()
