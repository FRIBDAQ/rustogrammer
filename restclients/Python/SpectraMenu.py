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
  QAction, QDialog, QDialogButtonBox, 
  QVBoxLayout
)
from spectrumeditor import Editor
from SpectrumList import SpectrumSelector
import capabilities
class SpectraMenu():
  def __init__(self, menu, client, win, file_menu):
    '''
    menu  - the menu we populate and handle.
    client - The client object to the server.
    win  - The main window.
    '''
    self._menu = menu
    self._client = client
    self._win = win
    self._file_menu = file_menu
    
    self._save =  QAction('Save Contents of Spectra...')
    self._save.triggered.connect(self._file_menu.saveSpectra)
    self._menu.addAction(self._save)
    
    self._read = QAction('Read Spectrum file...')
    self._read.triggered.connect(self._file_menu.read_spectrum_file)
    self._menu.addAction(self._read)
    
    self._menu.addSeparator()
    
    self._clearall = QAction("Clear all ")
    self._clearall.triggered.connect(self._client.spectrum_clear_all)  # Defaults to '*' pattern.
    self._menu.addAction(self._clearall)
    
    self._menu.addSeparator()
    
    self._create = QAction("Create...")
    self._create.triggered.connect(self._create_spectra)
    self._menu.addAction(self._create)
    
    self._delete = QAction('Delete...')
    self._delete.triggered.connect(self._delete_spectra)
    self._menu.addAction(self._delete)
    
    self._menu.addSeparator()
    
    self._apply = QAction('Apply Gate...')
    
  def _create_spectra(self):
    dlg = SpectrumCreator(self._menu)
    dlg.exec()
    
  def _delete_spectra(self):
    dlg = SelectSpectra(self._menu)
    if dlg.exec():
      spectra = dlg.selectedSpectra()
      for spectrum in spectra:
        self._client.spectrum_delete(spectrum)
class SpectrumCreator(QDialog):
  def __init__(self, *args):
    super().__init__(*args)
    
    layout = QVBoxLayout()
    
    self._editor = Editor(self)
    self._editor.hideSidebar()
    layout.addWidget(self._editor)
    
    self._buttonBox = QDialogButtonBox(QDialogButtonBox.Ok, self)
    self._buttonBox.accepted.connect(self.accept)
  
    
    layout.addWidget(self._buttonBox)
    
    self.setLayout(layout)

class SelectSpectra(QDialog):
  def __init__(self, *args):
    super().__init__(*args)
    
    layout          = QVBoxLayout()
    self._selection = SpectrumSelector(capabilities.get_client(), self)
    layout.addWidget(self._selection)
    
    self._buttonBox = QDialogButtonBox(QDialogButtonBox.Ok | QDialogButtonBox.Cancel, self)
    self._buttonBox.accepted.connect(self.accept)
    self._buttonBox.rejected.connect(self.reject)
    
    layout.addWidget(self._buttonBox)
    
    self.setLayout(layout)
    
  def selectedSpectra(self):
    return self._selection.selected()