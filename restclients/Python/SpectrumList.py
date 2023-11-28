''' This module provides a widget that lists spectra
    It's mostly a table widget with a scrollbar In addition there are
    *  An update button which can emit a signal that
    requests and update of the list given the mask/pattern
    *  A line entry which contains a filtering glob pattern.
    *  A clear which clears the mask and also requests an
    update.
   
   Each table entry is defined by:
   *  The spectrum name
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
    QPushButton, QLineEdit
)
from PyQt5.QtCore import *


from rustogramer_client import rustogramer

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
        self.mask.setText('*')
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
        self.mask.setText('*')
        self.clear_signal.emit()
        self.filter_relay()

# Test the widget:
from PyQt5.QtWidgets import QApplication
def test_view() :
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


   
#--------------------------------------------------------------

#  Now provide a view for the spectra.  

''' The Spectrum View is subclassed from Abstract List model
    class.  We don't use QStringList because we need to be able
    to provide structure to the parameters cells which, in general,
    contain a list of parameters not a single parameter.
'''
class SpectrumModel(QAbstractTableModel):
    data = None
    rows = 0
    cols = 11
    colheadings = ['Name', 'Type', 
        'XParameter(s)', 'Low', 'High', 'Bins',
        'YParameter(s)', 'Low', 'High', 'Bins', 'Gate'
    ]
    def __init__(self, parent = None) :
        super().__init__(parent)
    def rowCount(self, idx) :
        return self.rows
    def columnCount(self, idx):
        return self.cols
    
    ''' Provide the header data for the view
    '''
    def headerData(self, section, orientation, role) :
        if role != Qt.DisplayRole:
            return None
        
        if (orientation == Qt.Horizontal) and (section < len(self.colheadings)) :
            return self.colheadings[section]
        else :
            return None

    def data(self, index, role) :
            
        if role != Qt.DisplayRole:
            return None
        r = index.row()
        c = index.column()

        if self.data is None:    # Needs to update to get data.
            return None      

        if r < len(self.data):
            row = self.data[r]
            if c < len(row):
                return row[c]
        return None

    ''' This method updates the data and rows variables.
        the client parameter must be a rustogramer client
        object and is used to get data from the
        histogramer.
    '''
    def update(self, client, pattern = '*'):
        json = client.spectrum_list(pattern)
        spectra = json['detail']
        self.data = []
        self.rows = len(spectra)

        for spectrum in spectra :
            info = [
                spectrum['name'],
                spectrum['type'],
                ', '.join(spectrum['xparameters'])

            ]
            if spectrum['xaxis'] is not None:
                info.append(spectrum['xaxis']['low'])
                info.append(spectrum['xaxis']['high'])
                info.append(spectrum['xaxis']['bins'])
            else :
                info.append(None)
                info.append(None)
                info.append(None)
            info.append(', '.join(spectrum['yparameters']))
            if spectrum['yaxis'] is not None:
                info.append(spectrum['yaxis']['low'])
                info.append(spectrum['yaxis']['high'])
                info.append(spectrum['yaxis']['bins'])
            else :
                info.append(None)
                info.append(None)
                info.append(None)
            info.append(spectrum['gate'])
            self.data.append(info)
        self.dataChanged.emit(self.createIndex(0,0), self.createIndex( self.rows, 10))


#  Test the model/view.


def testmv(host, port):
    def update(pattern):
        model.update(client, pattern)
    client = rustogramer({'host': host, 'port': port})

    # Make parameter(s) and spectra try/catch in case we've already
    # run:

    try:
        client.rawparameter_create('test', {})
        client.rawparameter_create('x', {})
        client.rawparameter_create('y', {})
        client.spectrum_create1d('test', 'test', 0.0, 1024.0, 1024)
        client.spectrum_create2d('2d', 'x', 'y', 0.0, 1024.0, 256, 0.0, 4096.0, 246)
        client.spectrum_createg1('g1', ['x', 'y', 'test'], 0.0, 1024, 1024)
        client.sbind_all()
    except:
        pass

    # These should not fail:

    client.condition_make_true('Acond')
    client.apply_gate('Acond', '2d')

    app = QApplication(['test'])
    win = SpectrumList()
    win.show()
    model = SpectrumModel()
    model.update(client)      # Initial data.
    
    list = win.getList()
    list.setModel(model)
    list.showGrid()

    
    #  If Filter is clicked, update the model:

    win.filter_signal.connect(update)

    app.exec()


