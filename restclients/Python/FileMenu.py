from PyQt5.QtWidgets import (
    QAction, QDialog, QDialogButtonBox, QVBoxLayout, QHBoxLayout, QRadioButton, QFileDialog
)
from PyQt5.QtCore import QObject
import capabilities
from spectrumeditor import confirm, error
import SpectrumList
import os
from  rustogramer_client import RustogramerException
import DefinitionIO

class FileMenu(QObject):
    ''' 
       Implements the file menu... init will instantiate it and
       connect us to the action signals which we will process.
    
        Note that the lifetime of this object must be program lifetime.
    '''
    def __init__(self, menu, client, *args):
        '''
        Note that deriving us from QObject allows us to own children.
          *   menu - The File QMenu object.
          *   client - the REST client.
        '''
        super().__init__(*args)
        program = capabilities.get_program()
        self._program = program   
        self._menu = menu
        self._client = client
        
        # We need to retain ownership of our actions:
        
        self._save = QAction('Save...', self)
        self._save.triggered.connect(self._save_definitions)
        self._menu.addAction(self._save)
        
        self._save_treevars = QAction('Save Treevariables...', self)
        self._menu.addAction(self._save_treevars)
        
        self._save_spectra = QAction('Save spectrum contents...', self)
        self._save_spectra.triggered.connect(self._saveSpectra)
        self._menu.addAction(self._save_spectra)
        
        self._menu.addSeparator()
        
        self._load = QAction('Load...', self)
        self._menu.addAction(self._load)
        
        self._read_spectrum = QAction('Read Spectrum contents...', self)
        self._menu.addAction(self._read_spectrum)
        
        # SpecTcl supports sourcing a Tcl script:
        
        if program == capabilities.Program.SpecTcl:
            
            self._source  = QAction('Source Tcl Script', self)
            self._menu.addAction(self._source)
            
        # We'll add exit:
        
        self._menu.addSeparator()
        
        # we can stop the histogramer if it's rustogramer:
        
        
        self._exit = QAction('Exit', self)
        self._exit.triggered.connect(self._exitGui)
        self._exit = self._menu.addAction(self._exit)
        
        if program == capabilities.Program.Rustogramer:
            self._kill = QAction('Stop Histogramer')
            self._menu.addAction(self._kill)
            self._kill.triggered.connect(self._exitHistogramerAndSelf)
        
    def _save_definitions(self):
        #  Prompt for the file and defer the actual save to the 
        #  DefintionIO module.
        
        file = QFileDialog.getSaveFileName(self._menu, 'Definition File', os.getcwd(), 'sqlite')
        if file == ('',''):
            return
        filename = self._genfilename(file)
        print(filename)
        
        # If the file exists, delete it:
        
        try:
            os.remove(filename)
        except:
            pass
        saver = DefinitionIO.DefinitionWriter(filename)
        
    
    def _saveSpectra(self):
        #  Prompt for spectra to save and the format
        #  and prompt for a file to save them into...
        namePrompter = SpectrumSaveDialog(self._menu)
        if namePrompter.exec():
            names = namePrompter.selectedSpectra()
            format = namePrompter.format()
            
            wdir = os.getcwd()
            if format == 'json':
                default_ext = 'json'
            elif format == 'ascii':
                default_ext = 'spec'
            elif format == 'binary':
                default_ext = 'bin'
            else:
                default_ext = 'spec'
            if len(names) > 0:
                name = QFileDialog.getSaveFileName(self._menu, 'Spectrum File', wdir, default_ext)
                if not name == ('', ''):
                    filename  = self._genfilename(name)
                    
                    try:
                        self._client.spectrum_write(filename, format, names)
                    except RustogramerException as e:
                        error(f'Failed to save spectra to {filename} : {e}')
        
    def _exitGui(self):
        #  Make sure the user is certain and if so, exit:
        if confirm('Are you sure you want to exit the GUI (note the histogramer will continue to run)'):
            exit()
            
    def _exitHistogramerAndSelf(self):
        if confirm('Are you sure you want to exit the GUI and the histogramer?'):
            self._client.kill_histogramer()
            exit()
    # utilities
    
    def _genfilename(self, dialog_name):
        # Generate he filename from what comes back from QFileDialog:
        name = dialog_name[0]
        
        # IF this has an extension we're golden - that's the case if splitext[1] has a period.
        # otherwise we need to glue on the default extension.
        
        parts = os.path.splitext(name)
        print(parts)
        if '.' in parts[1]:
            return name
        else:
            return name + '.' + dialog_name[1]
            
            
        
        
        
class SpectrumSaveDialog(QDialog):
    '''
    This class provides:
    *   A standard way to select spectra.
    *   A selection of formats with which to save the spectra (appropriate to the server)
    *   Buttons for Ok and Cancel.
    *  Selectors to get the selected spectra and the format name.
    '''
    def __init__(self, *args):
        super().__init__(*args)
        layout = QVBoxLayout()
        
        self._selection = SpectrumList.SpectrumSelector(capabilities.get_client(), self)
        layout.addWidget(self._selection)
        
        #  Radio boxes for formats:
        
        formats = capabilities.get_supported_spectrum_format_strings()
        radio_layout = QHBoxLayout()
        
        self._formats = []
        for (i, format) in enumerate(formats):
            button = QRadioButton(format, self)
            self._formats.append(button)
            radio_layout.addWidget(button)

    
        if len(self._formats) > 0:
            self._formats[0].setChecked(True)   # Default is first format.
        
        layout.addLayout(radio_layout)
        
        # The buttons at the bottom of the dialog:
        
        self._buttonBox = QDialogButtonBox(QDialogButtonBox.Ok | QDialogButtonBox.Cancel, self)
        self._buttonBox.accepted.connect(self.accept)
        self._buttonBox.rejected.connect(self.reject)
        
        layout.addWidget(self._buttonBox)
        
        self.setLayout(layout)
        

    
    def selectedSpectra(self):
        return self._selection.selected()
    def format(self):
        result = None    # In case we have an unknown.
        
        for format in self._formats:
            if format.isChecked():
                result = format.text()
                return result
        
        return result
        
        
        
        