'''  This module provides the spectrum widget.  The Spectrum widget looks like this:

+--------------------------------+
|   Spectrum editor              |
+--------------------------------+
|  spectrum list                 |
+--------------------------------+

*   Where spectrum editor is an instance of spectrumeditor.Editor
*   Where spectrum list is an instance of SpectrumList.SpectrumList


'''

from PyQt5.QtWidgets import (
    QWidget, QVBoxLayout, QFrame,
    QApplication, QMainWindow
)
from SpectrumList import (SpectrumList, SpectrumModel)
from spectrumeditor import Editor
from capabilities import set_client as set_cap_client
from ParameterChooser import update_model as load_parameters
from  rustogramer_client import rustogramer as RClient
_client = None

def set_client(c):
    ''' Set the client used to interact with the server
    '''
    global _client
    _client = c


class SpectrumWidget(QWidget):
    def __init__(self, *args):
        global _client
        super().__init__(*args)

        # assumption is that set_client has been called

        set_cap_client(_client)

        # two frames in a vbox layout in the widget, the top frame
        # contains the editor, the bottom the spectrum list.abs
        
        layout = QVBoxLayout()
        top    = QFrame(self)
        top.setFrameShape(QFrame.Box)
        self._editor = Editor(top)
        layout.addWidget(self._editor)

        bottom = QFrame(self)
        bottom.setFrameShape(QFrame.Box)
        self._listing = SpectrumList(bottom)
        layout.addWidget(self._listing)

        self._spectrumListModel = SpectrumModel()
        self._listing.getList().setModel(self._spectrumListModel)
        self._spectrumListModel.update(_client)

        load_parameters(_client)

        self.setLayout(layout)

        # Connect to be able to update the view:

        self._editor.new_spectrum.connect(self._add_to_listing)

    def _add_to_listing(self, new_name):
        # Get the definition:

        sdef = _client.spectrum_list(new_name)
        sdef = sdef ['detail']
        if len(sdef) == 1:
            self._spectrumListModel.addSpectrum(sdef[0])
        

class NullSpectrumController:
    def __init__(self, model):
        pass

def test(host, port):
    ''' Exercise this module host.
     *  host = host running a server.
     *  port = port on which that server is listening for connections.
     '''
    set_client(RClient({'host': host, 'port': port}))
    app = QApplication([])

    c   = QMainWindow()
    view = SpectrumWidget(c)
    c.setCentralWidget(view)

    c.show()
    app.exec()