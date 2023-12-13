'''  Provides an editable list component. for Qt5.  This can be used for
parameter lists in e.g.  Summary or GD editors.
It factors out what used to be a hard coded internal part of the summary
spectrum editor.
   The widget consts of a listbox.  To the left of the list box are two buttons,
the top labeled with a > is clicked by the user to add an entry from somewhere
to the list box.  The bottom, labeled with an X is clicked by the user to 
delete the selected items from the listbox.  Below the list box is a row of
three buttons: ^ moves the selection up one spot in the list. Similarly
V moves the selection down one spot.  A button labeled Clear clears the
entire list box.  These buttons are autonomous, however Clear will signal
the client that items have been removed.

Signals:
   add - the add button was clicked.
   remove - The X button or clear button was clicked.  This is signaled for
         each item removed from the list box and the text of items 
         removed is provided.

Attributes:
    list - The entire listbox contents as a list.
    label - Label text above the listbox.

Notable public functions:
    appendItem - appends a new item to the list box.
    insertItem - inserts an item at a specific position in the list box.
'''

from PyQt5.QtWidgets import (
    QApplication, QMainWindow,
    QStyle,
    QWidget, QListWidget, QLabel, QPushButton,
    QHBoxLayout, QVBoxLayout, QGridLayout
)
from PyQt5.QtCore import pyqtSignal
from PyQt5.Qt import *

class EditableList(QWidget):
    add = pyqtSignal()
    remove = pyqtSignal(str)

    def __init__(self, label, *args):
        super().__init__(*args)

        # The label is above the list box at
        #  row 0, col 1:

        layout = QGridLayout()
        self._label = QLabel(label, self)
        layout.addWidget(self._label, 0, 1)

        # The list box is in 1,1 and spans 6 rows:

        self._list = QListWidget(self)
        self._list.setSelectionMode(QAbstractItemView.ContiguousSelection)
        layout.addWidget(self._list, 1,1, 6,1)

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

        self._delete.clicked.connect(self._delete_selection)
        self._clear.clicked.connect(self._delete_all)
        self._up.clicked.connect(self._move_selection_up)
        self._down.clicked.connect(self._move_selection_down)
    
    #Attribute implementations:

    def list(self):
        return [self._list.item(x).text() for x in range(self._list.count())]
    def setList(self, items):
        for i in items:
            self._list.addItem(i)
    def label(self):
        return self._label.text()
    def setLabel(self, newLabel):
        self._label.setText(newLabel)

    # Internal signal handlers:

    def _delete_selection(self):
        # Deletes all selected items and does a remove signal for each item
        # that is deleted.

        selection = self._list.selectedItems()
        self._delete_items(selection)

    def _delete_all(self):
        # Deletes all items int he list, signalling remove for each of them.
        items = [self._list.item(x) for x in range(self._list.count())]
        self._delete_items(items)

    def _move_selection_up(self):
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

    def _move_selection_down(self):

        rows = self._get_selected_rows()
        rows.sort(reverse=True)
        
        if (len(rows) == 0) or (rows[0] == self._list.count()-1):
            return                   # already at bottom or no selection.

        for r in rows:
            item = self._list.takeItem(r)
            self._list.insertItem(r+1, item)
    # Private utilities:

    def _delete_items(self, items):
        for item in items:
            deleted = self._list.takeItem(self._list.row(item))
            self.remove.emit(deleted.text())

    def _get_selected_rows(self):
        return [self._list.row(x) for x in self._list.selectedItems()]


#------------------------- test code ------------------------------

def test_remove(txt):
    print(txt, 'was removed')

if __name__ == '__main__':
    app = QApplication([])
    c   = QMainWindow()
    w   = EditableList('test')

    print("Currently labeled: ", w.label())
    w.setLabel('altered')

    w.setList(['a','b','c','d','e','f'])
    print(w.list())

    w.remove.connect(test_remove)

    c.setCentralWidget(w)

    c.show()
    app.exec()



