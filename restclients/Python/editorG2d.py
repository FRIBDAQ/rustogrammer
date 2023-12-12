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
        self._xaxis = AxisInput(self)
        self.main_layout.addWidget(self._xaxis, 7, 0)
        self.main_layout.addWidget(QLabel('X axis'), 6, 0)
    # Xaxis is internal
    def xlow(self):
        return self._xaxis.low()
    def setXlow(self, value):
        self._xaxis.setLow(value)
    def xhigh(self):
        return self._xaxis.high()
    def setXhigh(self, value):
        self._xaxis.setHigh(value)
    def xbins(self):
        return self._xaxis.bins()
    def setXbins(self, value):
        self._xaxis.setBins(value)

    # Y axis is the low,high,bins of  our super class.

    def ylow(self):
        return super().low()
    def setYlow(self, value):
        super().setLow(value)
    def yhigh(self):
        return super().high()
    def setYhigh(self, value):
        super().setHigh(value)
    def ybins(self):
        return super().bins()
    def setYbins(self, value):
        super().setBins(value)


if __name__ == "__main__":
    app = QApplication([])
    c   = QMainWindow()
    w   = Gamma2DEditor()
    c.setCentralWidget(w)

    c.show()
    app.exec()

    

