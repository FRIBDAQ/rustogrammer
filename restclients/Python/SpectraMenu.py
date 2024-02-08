'''
  This module provides the Spectra menu. It has 
   
   Save Contents of Spectra...
   Read Spectrum File...
   Clear All
   Delete...
   Apply Gate...
   
   Where we work really hard to ensure that we don't re-invent code that's e.g. in the
   spectrum editor....which already does much if not all of this.
   
   
'''
from PyQt5.QtWidgets import (
  QAction
)
class SpectraMenu():
  def __init__(self, menu, client, win, file_menu):
    '''
    menu  - the menu we populate and handle.
    client - The client object to the server.
    win  - The main window.
    '''
    self._menu = menu
    self._cient = client
    self._win = win
    self._file_menu = file_menu
    
    self._save =  QAction('Save Contents of Spectra...')
    self._save.triggered.connect(self._file_menu.saveSpectra)
    self._menu.addAction(self._save)
    
    self._read = QAction('Read Spectrum file...')
    self._menu.addAction(self._read)
    
    self._menu.addSeparator()
    
    self._clearall = QAction("Clear all ")
    self._menu.addAction(self._clearall)
    
    self._menu.addSeparator()
    
    self._create = QAction("Create...")
    self._menu.addAction(self._create)
    
    self._delete = QAction('Delete...')
    self._menu.addAction(self._delete)
    
    self._menu.addSeparator()
    
    self._apply = QAction('Apply Gate...')