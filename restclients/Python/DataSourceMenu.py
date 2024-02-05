'''
This module contains code to process the entries in the data source menu.
The following menu items are present:

Oneline - Take data from NSCLDAQ (SpecTcl only).
File    - Take data from an event/parameter file.
Pipe    - Take data from a program via a pipe (SpecTcl only).
-----
List of Runs - Take data from a list of run. (cluster file) - SpecTcl later than version 5.13-(???)
Filter File - Take data from a filter file. (SpecTcl only).
-----
Detach - Detach from the data source.
Abort Cluster File (SpecTcl only later than 5.13-xxx)

'''
from PyQt5.QtWidgets import (
    QAction
)
from PyQt5.QtCore import QObject
import capabilities

class DataSourceMenu(QObject):
    def __init__(self, menu, client, gui, *args):
        super().__init__(*args)
        
        # Fill in the items in the menu:
        
        program = capabilities.get_program()
        
        # Simplify program comparisons:
        
        spectcl = capabilities.Program.SpecTcl
        rustogramer = capabilities.Program.Rustogramer
        # Save our internal data:
        
        self._program = program 
        self._menu = menu
        self._client = client
        self._ui     = gui
        
        #  Add the menu items appropriate to the server:
        
        if program == spectcl:
            self._online = QAction('Online...', self)
            self._menu.addAction(self._online)
        
        self._file = QAction('File...', self)
        self._menu.addAction(self._file)
        
        if program == spectcl:
            self._pipe = QAction('Pipe...', self)
            self._menu.addAction(self._pipe)
        
        # This list of stuff is all SpecTcl at this time.
        
        if program == spectcl:
            self._menu.addSeparator()

            if capabilities.has_rest_runlist():
                self._cluster = QAction('Cluster file...', self)
                self._menu.addAction(self._cluster)
                
            self._filter = QAction('Filter file...', self)
            self._menu.addAction(self._filter)
        
        self._menu.addSeparator()
        
        self._detach = QAction('Detach')
        self._menu.addAction(self._detach)
        
        if program == spectcl:
            if capabilities.has_rest_runlist():   # Part of runlist functionality.
                self._abortlist = QAction('Abort Cluster File')
                self._menu.addAction(self._abortlist)
                                   
        
        
        
        