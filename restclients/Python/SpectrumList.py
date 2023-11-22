''' This module provides a widget that lists spectra
    It's mostly a table widget with a scrollbar In addition there are
    *  An update button which can emit a signal that
    requests and update of the list given the mask/pattern
    *  A line entry which contains a filtering glob pattern.
    *  A clear which clears the mask and also requests an
    update.
   
   Each table entry is defined by:
   *  The specctrum name
   *  The spectrum type string.
   *  Xparameters 
   *  X axis limits and binning.
   *  Optional Y parameters
   *  Optional Y parameter limits and binning.
   *  Optiounal applied gate.  This is automatically 
   suppressed if the gate is a True gate.  
      

      What I'm not 100% sure about is that I want the spectra with more than one X/Y
      parameter (e.g. gd) to:
    
    *  Indicate that they have more than one parameter.
    *  Allow the user to see all parameters.
'''

from PyQt5.QtWidgets import (
    QTableView, QWidget, QVBoxLayout, QHBoxLayout,
    QPushButton, QLineEdit,
)
from PyQt5.QtCore import pyqtSignal

'''  This is the view for spectra - the table that contains the spectra listed.
'''
class SpectrumView(QTableView):
    def __init__(self, parent=None):
        super().__init__(parent)

'''  This is the list with all the other bells and whistles.
     You construct this actually.
     The top consists of a SpectrumView (which can be fetched).
     The bottom is a horizontal arrangementy of controls:
     Update Button, filter line edit and clear button.
'''
class SpectrumList(QWidget) :
    # Custom signals so we don't have to expose the buttons:
    
    filter_signal = pyqtSignal(str)
    clear_signal  = pyqtSignal()


    def __init__(self, parent=None):
        super().__init__(parent)
        vlayout = QVBoxLayout()
        self.setLayout(vlayout)
        self.list = SpectrumView(self)
        vlayout.addWidget(self.list)

        self.controlbar = QWidget(self)
        hlayout = QHBoxLayout()
        self.controlbar.setLayout(hlayout)
        self.filter = QPushButton("Filter", self.controlbar)
        hlayout.addWidget(self.filter)
        self.mask = QLineEdit(self.controlbar)
        hlayout.addWidget(self.mask)
        self.clear = QPushButton('Clear', self.controlbar)
        hlayout.addWidget(self.clear)

        vlayout.addWidget(self.controlbar)

        # Set up  signal relays:

        self.filter.clicked.connect(self.filter_relay)
        self.clear.clicked.connect(self.clear_relay)
    ''' Provide access to the table  returns the QTableView widget
        that will display the spectrum list.
    '''

    def getList(self) :
        return self.list

    #  Button handlers to relay to signals
    #  Note that clear will also clear the filter line edit.

    def filter_relay(self) :
        self.filter_signal.emit(self.mask.text())

    def clear_relay(self):
        self.mask.setText('')
        self.clear_signal.emit()

# Test the widget:
from PyQt5.QtWidgets import QApplication
def test() :
    def onFilter(txt):
        print("Filter clicked: ", txt)
    def onClear() :
        print("cleared")
    app = QApplication([])
    window = SpectrumList()
    window.show()
    window.filter_signal.connect(onFilter)
    window.clear_signal.connect(onClear)
    app.exec()


   
    