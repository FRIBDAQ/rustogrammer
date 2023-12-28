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
    QApplication, QMainWindow, QSizePolicy, QMessageBox
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

        
        load_parameters(_client)

        # assumption is that set_client has been called

        set_cap_client(_client)

        # two frames in a vbox layout in the widget, the top frame
        # contains the editor, the bottom the spectrum list.abs
        
        layout = QVBoxLayout()
        top    = QFrame(self)
        top.setFrameShape(QFrame.Box)
        self._editor = Editor(top)
        self._editor.setSizePolicy(QSizePolicy.Expanding, QSizePolicy.Fixed)
        self._editor.load_gates(_client)
        layout.addWidget(self._editor)

        bottom = QFrame(self)
        bottom.setFrameShape(QFrame.Box)
        self._listing = SpectrumList(bottom)
        layout.addWidget(self._listing)

        self._spectrumListModel = SpectrumModel()
        self._listing.getList().setModel(self._spectrumListModel)
        self._listing.getList().horizontalHeader().setModel(self._spectrumListModel)
        self._spectrumListModel.load_spectra(_client)


        self.setLayout(layout)

        # Connect to be able to update the view:

        self._editor.new_spectrum.connect(self._add_to_listing)
        self._editor.spectrum_deleted.connect(self._remove_from_listing)
        self._editor.clear_selected.connect(self._clear_selected)
        self._editor.clear_all.connect(self._clear_all)
        self._editor.delete_selected.connect(self._delete_selected)
        self._editor.gate_selected.connect(self._gate_selected)
        self._editor.ungate_selected.connect(self._ungate_selected)

        self._listing.filter_signal.connect(self._filter_list)
        self._listing.clear_signal.connect(self._clear_filter)

    def _add_to_listing(self, new_name):
        # Get the definition:

        sdef = _client.spectrum_list(new_name)
        sdef = sdef ['detail']
        if len(sdef) == 1:
            self._spectrumListModel.addSpectrum(sdef[0])
    def _remove_from_listing(self, old_name):
        self._spectrumListModel.removeSpectrum(old_name)

    def _filter_list(self, mask):
        global _client
        self._spectrumListModel.load_spectra(_client, mask)
        self._editor.load_gates(_client)

    def _clear_filter(self):
        global _client
        self._listing.setMask("*")
        self._filter_list("*")

    # internal slots:

    def _clear_selected(self):
        # Clear seleted spectra:

        spectra = self._listing.getSelectedSpectra()
        for name in spectra:
            _client.spectrum_clear(name)
    def _clear_all(self):
        # Clear all spectra

        _client.spectrum_clear('*')
    def _delete_selected(self):
        # Upon confirmation, delete selected spectra:
        spectra = self._listing.getSelectedSpectra()
        if len(spectra) == 0:
            return
        spectrum_list = ', '.join(spectra)
        dlg = QMessageBox(
            QMessageBox.Warning, 'Confirm',
            f'Are you sure you want to delete the spectra: {spectrum_list} ?',
            QMessageBox.Yes | QMessageBox.No, self
        )
        if dlg.exec() == QMessageBox.Yes:
            for s in spectra:
                _client.spectrum_delete(s)
                self._remove_from_listing(s)
    def _gate_selected(self):
        global _client
        spectra = self._listing.getSelectedSpectra()
        gate    = self._editor.selected_gate()
        if len(spectra) == 0 or gate.isspace():
            return   
        for spectrum in spectra:
            _client.apply_gate(gate, spectrum)
        # Update the list:
        self._spectrumListModel.load_spectra(_client, self._listing.mask())
    def _ungate_selected(self):
        global _client
        spectra = self._listing.getSelectedSpectra()
        if len(spectra) == 0:
            return               # So we don't need to regen list.
        for spectrum in spectra:
            _client.ungate_spectrum(spectrum)
        self._spectrumListModel.load_spectra(_client, self._listing.mask())


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

# For test with debugger:

if __name__ == '__main__':
    test('localhost', 8000)