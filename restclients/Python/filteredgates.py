''' 
This module provides a filterd gate list.
To provide this we provide our own gate model which is independentl loaded,
a table, to view that model and controls to set the filter, update the list 
based on the new filter and clear the filter back to the default '*'
'''

from PyQt5.QtWidgets import (
    QTableView, QAbstractItemView, QWidget, QPushButton, QLineEdit,
    QVBoxLayout, QHBoxLayout, 
    QApplication, QMainWindow
)
from PyQt5.QtCore import pyqtSignal
import gatelist
import parse

#  We need this separate model so that filters applied here don't affect the
#  comboboxes etc.
filtered_gate_model = gatelist.ConditionModel()

class GateView(QTableView):
    ''' This is a gate (condition) list table where the selection model is set to allow
    users to select a single condition.   Specifically, a single item can be
    selected (selection mode) and that this is a single row (selection behavior).
    This allows a condition to be selected.
    '''
    def __init__(self, *args):
        global filtered_gate_model
        super().__init__(*args)
        self.setModel(filtered_gate_model)
        self.setSelectionMode(QAbstractItemView.SingleSelection)
        self.setSelectionBehavior(QAbstractItemView.SelectRows)

class FilteredConditions(QWidget):
    ''' This is a filtered gate list with the needed elements to set the
    filter:
    Signals:
        update - Update button was clicked, user should update the filtered_gate_model
           in accordance with the filter pattern.
        selectionChanged - An item in the list was selected.
        clear  - Clear the pattern back to whatever the default should be.

    Attributes:
        filter - the filter pattern
        selection - (readonly) the selected row.
    '''
    update = pyqtSignal()
    selectionChanged = pyqtSignal()
    clear = pyqtSignal()
    def __init__(self, *args):
        super().__init__(*args)
        # Layout is two frames.  The top one is the list itself:
        layout = QVBoxLayout()

        self._list = GateView(self)
        layout.addWidget(self._list)

        # Bottom frame has the controls:

        bottom = QHBoxLayout()
        self._update = QPushButton('Update', self)
        bottom.addWidget(self._update)
        self._pattern = QLineEdit('*', self)
        bottom.addWidget(self._pattern)
        self._clear = QPushButton('Clear', self)
        bottom.addWidget(self._clear)

        layout.addLayout(bottom)
        self.setLayout(layout)

        # set up signal relays:

        self._update.clicked.connect(self.update)
        self._clear.clicked.connect(self.clear)
    
    #  Provide the selection changed signal:

    def selectionChanged(self, new, old):
        # (override)

        super().selectionChangted(new, old)
        self.selectionChanged.emit()   

    # Attributes:

    def filter(self):
        return self._pattern.text()
    def setFilter(self, pattern):
        self._pattern.setText(pattern)

    def selection(self):
        # Returns none if there is no selection else the contents of the
        # selection in a manner similar to that of a query for that gate:

        selection = self._list.selectedIndexes()
        if len(selection) == 0:
            return None
        row_num = selection[0].row()
        name = self.model().item(row_num, 0).text()
        type_str = self.model().item(row_num, 1).text()
        gates    = self._make_string_list(self.model().item(row_num, 2).text())
        params = self._make_string_list(self.model().item(row_num, 3).text())
        points  = self._make_point_list(self.model().item(row_num, 4).text())
        hilo    = self._make_limits(self.model().item(row_num, 5).text())
        return {
            'name': name, 'type': type_str, 'gates' : gates,
            'parameters': params, 'points': points, 'low': low, 'high': high
        }
    #  private utilities:
    def _make_string_list(self, s):
        if s == '' or gates.isspace():
            return None
        return s.split(', ')
    def _make_point_list(self, s):
        if s == '' or gates.isspace():
            return None
        text_list = s.split(', ')
        result = list()
        for pt_s in text_list:
            pt = parse('({}, {})')
            result.append({'x': pt[0], 'y': pt[1]})
        return result
    def _make_limits(self, s):
        if s == '' or gates.isspace():
            return None
        return parse('{}, {}')


if __name__ == "__main__":
    from rustogramer_client import rustogramer as rc
    c = rc({'host': 'localhost', 'port': 8000})
    

    app = QApplication([])
    win = QMainWindow()
    filtered_gate_model.load(c)
    w = GateView()
    

    win.setCentralWidget(w)
    win.show()
    app.exec()
