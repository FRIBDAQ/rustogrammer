'''
This moduele provides an editor widget for gamma summary spectra.
This spectra are like summary spectra but each x channel contains a multiincremented
1-d spectrum (g1).  Therefore we need to be able to:
  *  Create x channels via a tab that's kept on the right side of the tabs labeled '+'
  *  populate each x channel with a set of parameters.

all in addition to the usual: provide a name for the spectrum and and the y axis
specification.  The tabwidget has the editable list controls I wanted to 
re-use that module subistiting the list box but that runs into layout
problems.  Here's a sample layout:

+---------------------------------------------------------+
| Name: [                                 ]               |
| Parameer                     +-------------------------+|
| [ parameter choser] [] array |  tabbed paramete lists  ||
|  (selected param)            |       ...               || 
|                                                         |
|                             +--------------------------+|
|  [axis specfication]                                    |
|                 [ Create/replace ]                      |
+---------------------------------------------------------+

The above does not show the editable list box controls for brevity, however
they appear in the standard places for editable list boxes relative to the
tabbed widget.

Signals:
   *   commit  - the 'Create/Replace' button was clicked.
   *   addparameter - The parameter add button was checked.
Attributes:
   * name    - Name of the spectrum.
   * parameter - selected parameter.
   * array     - the array checkbox was set.
   * xchannels  - number of x channels defined. (readonly).
   * low, high,  bins - y axis specifications.
   * axis_from_param - the axis from parameter checkbox was checked.
   * channel  - Currently selected x channel number.
   * parameter - Selected parameter.

PublicMethods:
    * addChannel  - adds a new x  channel returns its index.
    * loadChannel - Loads the list box for a channel with names.
    * removeChannel - Removes the specified list
    * getChannel  - Gets the names in a channel.
    * clear       - removes all channel tabs (the '+' tab remains).
                  and adds an empty channel 0 list making it current.
'''
from PyQt5.QtWidgets import (
    QWidget, QLabel, QTabWidget, QPushButton, QCheckBox, QLineEdit,
    QListWidget, QStyle,
    QGridLayout, QHBoxLayout, QVBoxLayout,
    QApplication, QMainWindow
)
from PyQt5.QtCore import pyqtSignal
from PyQt5.Qt import *
from ParameterChooser import LabeledParameterChooser


class ParametersWidget(QWidget):
    ''' This is the tabbed widget of list boxes
        surrounded by the editor controls that are
         in the editablelist.EditablList widget
        Signals:
          add - the Add button was clicked.
        Note:
          The delete, clear and movement buttons are autonomous.
        Slots:
           delete, clear, up, down connected to those internal signals.
        Attributes:
            currentIndex - the currently visible index.
            count        - number of list boxes (readonly).
        Public Methods:
            getValues   - Get the list of items in listbox n.
            setValues   - Set the list of items in listbox n.
            clearValues - clear all valuesin listbox n.
            removeList  - Remove listbox n
            clearAll       - Remove all but channel 0 and + and clear 0.
    '''
    add = pyqtSignal()
    def __init__(self, label, *args):
        super().__init__(*args)
        layout = QGridLayout()
        self._label = QLabel(label, self)
        layout.addWidget(self._label, 0, 1)

        # Tabwidget an initial tabs:

        self._channels = QTabWidget(self)
        self._channels.addTab(QListWidget(), '0')
        self._channels.addTab(QLabel(''),'+')
        self._channels.setCurrentIndex(0)

        # currently selected list

        self._list = self._channels.widget(0)
        self._list.setSelectionMode(QAbstractItemView.ContiguousSelection)
        layout.addWidget(self._channels, 1,1, 6,1)

        # In 4,0 is a vboxlayout that contains the
        # > and X buttons:

        adddel_layout = QVBoxLayout()
        self._add = QPushButton(self)
        self._add.setIcon(self.style().standardIcon(getattr(QStyle, 'SP_MediaPlay')))
        self._add.setMaximumWidth(25)
        self._delete = QPushButton(self)
        self._delete.setIcon(self.style().standardIcon(getattr(QStyle, 'SP_DialogCancelButton')))
        self._delete.setMaximumWidth(25)
        adddel_layout.addWidget(self._add)
        adddel_layout.addWidget(self._delete)
        layout.addLayout(adddel_layout, 4, 0)

        #  Below the list in 7, 1 is an HBoxLayout containing the 
        #  ^ V clear buttons:

        edit_layout = QHBoxLayout()
        self._up = QPushButton(self)
        self._up.setIcon(self.style().standardIcon(getattr(QStyle, 'SP_TitleBarShadeButton')))
        self._up.setMaximumWidth(25)
        self._down = QPushButton(self)
        self._down.setIcon(self.style().standardIcon(getattr(QStyle, 'SP_TitleBarUnshadeButton')))
        self._down.setMaximumWidth(25)
        self._clear = QPushButton('Clear', self)

        edit_layout.addWidget(self._up)
        edit_layout.addWidget(self._down)
        edit_layout.addWidget(self._clear)
        layout.addLayout(edit_layout, 7,1)


        self.setLayout(layout)

        #Signal relays:

        self._add.clicked.connect(self.add)

        # Internally handled signals (note some may signal as well).

        self._delete.clicked.connect(self.delete)
        self._clear.clicked.connect(self.clear)
        self._up.clicked.connect(self.up)
        self._down.clicked.connect(self.down)
        self._channels.currentChanged.connect(self._tabChanged)

    # Attribute implementations:
    def currentIndex(self):
        self._channels.currentIndex()
    def setCurrentIndex(self,i):
        self._channels.setCurrentIndex(i)
    def count(self):
        return self._channels.count()

    # Public methods:
    def getValues(self, n):
        self._check_index(n)
        w = self._channels.widget(n)
        return [w.item(x).text() for x in range(w.count())]
        
    def clearValues(self, n):
        self._check_index(n)
        w = self._channels.widget(n)
        while w.count > 0:
            w.takeItem(0)
    def setValues(self, n, l):
        self.clearValues(self, n)     # also raises error on bad n.
        w = self._chanels.widget(n)   #empty now:
        w.addItems(l)
    def removeList(self, n):
        self._check_index(n)
        # Can't remove the last one:

        if self._list.count() == 2:
            raise ValueError(f"Can't remove the last tab.")
        self._channels.removeTab(n)

        # Now we need to relable tab n and higher:

        for i in range(n, self._channels.count()-1):
            self._channels.setTabText(i, str(i))
        # If necessary
        #   - Dont' allow + to be current.
        #   - Update self._list as that might be the tab we deleted:

        sel = self._channels.currentIndex()
        if sel == self._channels.count() -1:
            self._channels.setCurrentIndex(self._channels.currentIndex()-1)
        self._list = self._chanels.widget(self._channels.currentIndex())
        
    def clearAll(self):
        # Get rid of the existing channel tabs:

        for i in range(self._channels.count()-1):
            self._channels.removeTab(n)
        # add a new '0' and make it currentL

        self._list = QListWidget()
        self._list.setSelectionMode(QAbstractItemView.ContiguousSelection)
        self._channels.insertTab(0, self._list, '0')


    # Slots:

    def delete(self):
        # Deletes all selected items and does a remove signal for each item
        # that is deleted.

        selection = self._list.selectedItems()
        self._delete_items(selection)

    def clear(self):
        # Deletes all items int he list, signalling remove for each of them.
        items = [self._list.item(x) for x in range(self._list.count())]
        self._delete_items(items)

    def up(self):
        # Moves all the items in the selection up one notch.  Note
        # we have a contiguous selection so if the one with the lowest
        # row # is row 0 nothing to do:

        rows = self._get_selected_rows()
        rows.sort()

        if (len(rows) == 0) or (rows[0] == 0):
            return                   # already at the top or no selection
        
        for r in rows:
            item = self._list.takeItem(r)
            self._list.insertItem(r-1, item)

    def down(self):

        rows = self._get_selected_rows()
        rows.sort(reverse=True)
        
        if (len(rows) == 0) or (rows[0] == self._list.count()-1):
            return                   # already at bottom or no selection.

        for r in rows:
            item = self._list.takeItem(r)
            self._list.insertItem(r+1, item)

    # Internal signal handler to deal with tab changes:
    def _tabChanged(self, index):
        # If the new tab is an existing list just change the list, otherwise
        # add  new list and make it current.

        num_tabs = self._channels.count()
        if index < num_tabs -1:
            self._list = self._channels.widget(index)
        else:
            self._list = QListWidget()
            self._list.setSelectionMode(QAbstractItemView.ContiguousSelection)
            self._channels.insertTab(num_tabs-1, self._list, str(num_tabs-1))
            self._channels.setCurrentIndex(num_tabs-1)
        
    # Private utilities:

    def _delete_items(self, items):
        for item in items:
            deleted = self._list.takeItem(self._list.row(item))
            self.remove.emit(deleted.text())

    def _get_selected_rows(self):
        return [self._list.row(x) for x in self._list.selectedItems()]

    def _check_index(self, n):
        if n >= self._channels.count() - 1:
            # Invalid index:
            raise IndexError(f'{n} is not a valid tab number')

class GammaSummaryEditor(QWidget):
    
    def __init__(self, *args):
        super().__init__(*args)
        layout = QVBoxLayout()

        name_layout = QHBoxLayout()
        name_layout.addWidget(QLabel("Name:", self))
        self._name = QLineEdit(self)
        name_layout.addWidget(self._name)
        layout.addLayout(name_layout)

        param_layout = QHBoxLayout()
        self._parameter = LabeledParameterChooser(self) 
        param_layout.addWidget(self._parameter)
        self._array = QCheckBox('Array', self)
        param_layout.addWidget(self._array, Qt.AlignTop)
        self._channels = ParametersWidget('Channels', self)
        param_layout.addWidget(self._channels)
        layout.addLayout(param_layout)



        
        self.setLayout(layout)


#  Tests:

if __name__ == "__main__":
    app = QApplication([])
    c   = QMainWindow()

    w   = GammaSummaryEditor()

    c.setCentralWidget(w)

    c.show()
    app.exec()