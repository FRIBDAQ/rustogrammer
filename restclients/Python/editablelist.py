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

#------------------------- test code ------------------------------
if __name__ == '__main__':
    app = QApplication([])
    c   = QMainWindow()
    w   = EditableList('test')
    c.setCentralWidget(w)

    c.show()
    app.exec()



