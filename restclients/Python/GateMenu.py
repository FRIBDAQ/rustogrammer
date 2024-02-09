'''
Proivides the Gate class which defines and implements the Gate menu items.
'''

from PyQt5.QtWidgets import(
    QAction
)

class Gate():
    def __init__(self, menu, client, main, spectra):
        '''
          menu - the menu we populate
          client- a client to the server we're interacting with
          main- THe main window.
          spectra- The handler for the Spectra menu..so we can use the apply_gate method
                  from that object
        '''
        
        self._menu = menu
        self._client = client
        self._win = main
        self._spectra = spectra
        
        self._create = QAction('Create...')
        self._menu.addAction(self._create)
        
        self._apply = QAction('Apply...')
        self._apply.triggered.connect(self._spectra.apply_gate)
        self._menu.addAction(self._apply)
        
        self._menu.addSeparator()
        
        self._delete = QAction('Delete...')
        self._menu.addAction(self._delete)