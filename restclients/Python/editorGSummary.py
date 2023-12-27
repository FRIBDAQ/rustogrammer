'''
This moduele provides an editor widget for gamma summary spectra.
This spectra are like summary spectra but each x channel contains a multiincremented
1-d spectrum (g1).  Therefore we need to be able to:
  *  Create x channels via a tab that's kept on the right side of the tabs labeled '+'
  *  populate each x channel with a set of parameters.

all in addition to the usual: provide a name for the spectrum and and the y axis
specification.  We use the editablelist module but swap  the listbox as the
tabs are selected.  Here's a sample layout:

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
Attributes:
   * name    - Name of the spectrum.
   * xchannels  - number of x channels defined. (readonly).
   * low, high,  bins - y axis specifications.
   * channel  - Currently selected x channel number.

PublicMethods:
    * addChannel  - adds a new x  channel returns its index.
    * loadChannel - Loads the list box for a channel with names.
    * removeChannel - Removes the specified list
    * getChannel  - Gets the names in a channel.
    * clear       - removes all channel tabs (the '+' tab remains).
                  and adds an empty channel 0 list making it current.
'''
from PyQt5.QtWidgets import (
    QWidget, QLabel, QTabWidget, QPushButton, QCheckBox,
    QGridLayout, QHBoxLayout, QVBoxLayout,
    QApplication, QMainWindow
)
from PyQt5.QtCore import pyqtSignal


class GammaSummaryEditor(QLabel):
    def __init__(self, *args):
        super().__init__(*args)
        self.setText("Unimplemented at this time")


#  Tests:

if __name__ == "__main__":
    app = QApplication([])
    c   = QMainWindow()

    w   = GammaSummaryEditor()

    c.setCentralWidget(w)

    c.show()
    app.exec()