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
    QPushButton, QLineEdit, QAbstractItemView, QListView
)
from PyQt5.QtCore import pyqtSignal, Qt
from PyQt5.QtGui import QStandardItemModel, QStandardItem

from rustogramer_client import rustogramer
import editablelist


'''  This is the view for spectra - the table that contains the spectra listed.
'''
class SpectrumView(QTableView):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setSelectionBehavior(QAbstractItemView.SelectRows)
        self.setSelectionMode(QAbstractItemView.ExtendedSelection)
        self.setEditTriggers(QAbstractItemView.NoEditTriggers)
        self._selected_spectra = []
        self._selected_rows = []

    def mouseReleaseEvent(self, e):
        super().mouseReleaseEvent(e)
        self._selected_spectra = [x.data() for x in self.selectedIndexes() if x.column() == 0]
        self._selected_rows = [x.row() for x in self.selectedIndexes() if x.column() == 0]
        
    def getSelectedSpectra(self):
        return self._selected_spectra
    def getSelectedDefinitions(self):
        # Return a list of lists where each sublist is the contents of the 
        # selected row in the table.
        result = []
        cols = self.model().columnCount()
        for row in self._selected_rows:
            arow = []
            for c in range(cols):
                arow.append(self.model().item(row, c).data(Qt.DisplayRole))
            result.append(arow)
        return result

class SpectrumNameList(QListView):
    '''
        List of spectrum names
        
    '''
    def __init__(self, client, *args):
        super().__init__(*args)
        self._model = SpectrumModel(self)   
        self._model.load_spectra(client)
        self.setModel(self._model)
        self.setModelColumn(0)
        self.setSelectionMode(QAbstractItemView.ExtendedSelection)

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
        self._mask = QLineEdit(self.controlbar)
        self._mask.setText('*')
        hlayout.addWidget(self._mask)
        self.clear = QPushButton('Clear', self.controlbar)
        hlayout.addWidget(self.clear)

        vlayout.addWidget(self.controlbar)

        # Set up  signal relays:

        self._mask.returnPressed.connect(self.filter_relay)
        self.filter.clicked.connect(self.filter_relay)
        self.clear.clicked.connect(self.clear_relay)

        
    
    
    ''' Provide access to the table  returns the QTableView widget
        that will display the spectrum list.
    '''

    def getList(self) :
        return self.list
    def mask(self):
        return self._mask.text()
    def setMask(self, s):
        self._mask.setText(s)
    def getSelectedSpectra(self):
        return self.list.getSelectedSpectra()
    def getSelectedDefinitions(self):
        return self.list.getSelectedDefinitions()
    


    #  Button handlers to relay to signals
    #  Note that clear will also clear the filter line edit.

    def filter_relay(self) :
        self.filter_signal.emit(self._mask.text())

    def clear_relay(self):
        self._mask.setText('*')
        self.clear_signal.emit()
        self.filter_relay()



   
#--------------------------------------------------------------

#  Now provide a view for the spectra.  

''' The Spectrum View is subclassed from Abstract List model
    class.  We don't use QStringList because we need to be able
    to provide structure to the parameters cells which, in general,
    contain a list of parameters not a single parameter.
'''
class SpectrumModel(QStandardItemModel):
    
    _colheadings = ['Name', 'Type', 
        'XParameter(s)', 'Low', 'High', 'Bins',
        'YParameter(s)', 'Low', 'High', 'Bins', 'Gate'
    ]
    def __init__(self, parent = None) :
        super().__init__(parent)

    def headerData(self, col, orient, role):
        if role == Qt.DisplayRole:
            if orient == Qt.Horizontal:
                return self._colheadings[col]
            else:
                return None

    ''' This method updates the data and rows variables.
        the client parameter must be a rustogramer client
        object and is used to get data from the
        histogramer.
    '''
    def load_spectra(self, client, pattern = '*'):
        self.clear()
        json = client.spectrum_list(pattern)
        spectra = json['detail']
        self.rows = len(spectra)

        for spectrum in spectra :
            self._addItem(spectrum)
        
        self.sort(0)

    def addSpectrum(self, definition):
        self.rows = self.rows+1
        self._addItem(definition)
        self.sort(0)

    def removeSpectrum(self, name):
        items = self.findItems(name)
        for item in items:      # Deals correctly with no/multiple matches:
            idx = self.indexFromItem(item)
            self.removeRow(idx.row())

    def _addItem(self, spectrum):
        info = [
            self._item(spectrum['name']),
            self._item(spectrum['type']),
            self._item(','.join(spectrum['xparameters']))

        ]
        if spectrum['xaxis'] is not None:
            info.append(self._item(str(spectrum['xaxis']['low'])))
            info.append(self._item(str(spectrum['xaxis']['high'])))
            info.append(self._item(str(spectrum['xaxis']['bins'])))
        else :
            info.append(self._item(''))
            info.append(self._item(''))
            info.append(self._item(''))
        info.append(self._item(','.join(spectrum['yparameters'])))
        if spectrum['yaxis'] is not None:
            info.append(self._item(str(spectrum['yaxis']['low'])))
            info.append(self._item(str(spectrum['yaxis']['high'])))
            info.append(self._item(str(spectrum['yaxis']['bins'])))
        else :
            info.append(self._item(''))
            info.append(self._item(''))
            info.append(self._item(''))
        if spectrum['gate'] is None:
            info.append(self._item(''))
        else:
            info.append(self._item(spectrum['gate']))
        
        self.appendRow(info)

    def _item(self, s):
        result = QStandardItem(s)  
        result.setEditable(False)
        return result
    

# A widget for selecting spectra from a SpectrumNameList:

class SpectrumSelector(QWidget):
    ''' 
    This widget consists of a SpectrumNameList on the left and an editablelist
    on the right.   The editablelist signals are autonmosly handled so that
    spectr are loaded into the editable box etc. etc.
    
    '''
    def __init__(self, client, *args):
        super().__init__(*args)
        layout = QHBoxLayout()
        self._list = SpectrumNameList(client, self)
        layout.addWidget(self._list)
        
        self._selected = editablelist.EditableList('Selected Spectra', self)
        layout.addWidget(self._selected)
        self.setLayout(layout)
        
        self._selected.add.connect(self._add_selected)
    
    def selected(self):
        return self._selected.list()
        
    def _add_selected(self):
        selected_indices = self._list.selectedIndexes()
        for index in selected_indices:
            item = self._list.model().item(index.row(), 0)
            self._selected.appendItem(item.text())
        self._list.clearSelection()      # Unselect the transfered items.
        
        
#--------------------------   test code --------------------

# Test the SpectrumList view.
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


#  Test the model/view.

theClient = None
model = None
def update(pattern):
    global theClient
    global model
    model.load_spectra(theClient, pattern)
def testmv(host, port):
    global theClient
    global model
    client = rustogramer({'host': host, 'port': port})
    theClient = client
    # Make parameter(s) and spectra try/catch in case we've already
    # run:

    try:
        client.rawparameter_create('test', {})
    except:
        pass
    try:
        client.rawparameter_create('x', {})
    except:
        pass
    try:
        client.rawparameter_create('y', {})
    except:
        pass
    try:
        client.spectrum_create1d('test', 'test', 0.0, 1024.0, 1024)
    except:
        pass
    try:
        client.spectrum_create2d('2d', 'x', 'y', 0.0, 1024.0, 256, 0.0, 4096.0, 256)
    except:
        pass
    try:
        client.spectrum_createg1('g1', ['x', 'y', 'test'], 0.0, 1024, 1024)
    except:
        pass
    try:
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
    model.load_spectra(client)      # Initial data.
    
    list = win.getList()
    list.setModel(model)
    list.showGrid()

    
    #  If Filter is clicked, update the model:

    win.filter_signal.connect(update)

    app.exec()

