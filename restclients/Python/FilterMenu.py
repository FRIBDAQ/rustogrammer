
from PyQt5.QtWidgets import (
    QAction
)
from PyQt5.QtCore import QObject

class FilterMenu(QObject):
    
    def __init__(self, menu, client, gui, data_source, *args):
        super().__init__(*args)
        
        self._menu = menu
        self._client = client
        self._gui = gui
        self._data_source = data_source
        
        # ALl items are SpecTcl because that's the only thing that supports filters so:
        
        self._wizard = QAction('Filter Wizard...')
        self._menu.addAction(self._wizard)
        
        self._enables = QAction('Enable/Disable Filters...')
        self._menu.addAction(self._enables)
        
        self._menu.addSeparator()
        
        self._read = QAction('Read Filter File...')
        self._read.triggered.connect(self._data_source.attach_filter)
        self._menu.addAction(self._read)