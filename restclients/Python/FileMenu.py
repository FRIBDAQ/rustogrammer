from PyQt5.QtWidgets import QAction
from PyQt5.QtCore import QObject
import capabilities
from spectrumeditor import confirm

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
        self._menu.addAction(self._save)
        
        self._save_treevars = QAction('Save Treevariables...', self)
        self._menu.addAction(self._save_treevars)
        
        self._save_spectra = QAction('Save spectrm contents...', self)
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
        
    
    def _saveSpectra(self):
        #  Prompt for spectra to save and the format
        #  and prompt for a file to save them into...
        pass
        
    def _exitGui(self):
        #  Make sure the user is certain and if so, exit:
        if confirm('Are you sure you want to exit the GUI (note the histogramer will continue to run)'):
            exit()
            
    def _exitHistogramerAndSelf(self):
        if confirm('Are you sure you want to exit the GUI and the histogramer?'):
            self._client.kill_histogramer()
            exit()
            
            
        
        
        
        