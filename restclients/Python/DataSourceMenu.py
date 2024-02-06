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
    QAction, QFileDialog, QDialog, QDialogButtonBox, QRadioButton, QLabel,
    QVBoxLayout, QHBoxLayout
)
from PyQt5.QtCore import QObject

import os

import capabilities
from spectrumeditor import error


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
        self._file.triggered.connect(self._read_event_file)
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
                                   
        
        
    def _read_event_file(self):
        #  Prompt for an event file and 
        # Figure out the possible event file formats:
        
        formats = list()
        if capabilities.can_read_raw_events():
            formats.append('Raw (*.evt)')
        if capabilities.can_read_parfiles():
            formats.append('Parameterized (*.par)')
        
        if len(formats) == 0:
            error('The histogramer has no supported event file formats')
            return
        
        filters = ';;'.join(formats)
        
        file = QFileDialog.getOpenFileName(
            self._menu, 'Choose event file', os.getcwd(), filters, formats[0]
        )
        filename = file[0]
        if filename == '':                  # Cancelled out of file choice.
            return
        try:
            self._client.attach_source('file', filename)
            if self._isRawFile(filename, file[1]):
                # Need to set ringformat:
                format_prompt = FormatPrompter(self._menu)
                ringlevel = format_prompt.exec()
                if ringlevel > 0:
                    self._client.ringformat_set(ringlevel)
                else:
                    return                              # canceled.
            else:
                self._client.ringformat_set(12)         # Parameter files are ring 12.
            
            # Start analysis:
            
            self._client.start_analysis()
        except Exception as e:
            error(f'Unable to analyze data from file {filename} : {e}')
    
    
    def _isRawFile(self, filename, filter):
        # Figure out if the filename is a raw event file:
        
        parts = os.path.splitext(filename)
        print(parts)
        return parts[1] == '.evt'
    
    
    
class FormatPrompter(QDialog):
    #  Provides a prompter dialog for the ring format.
    #  exec returns:
    #   0     - User cancelled.
    #   otherwise the major version of the NSCLDAQ format to use (e.g 11).
    _format_versions = [10, 11, 12]
    def __init__(self, *args):
        super().__init__(*args)
        layout = QVBoxLayout()
        layout.addWidget(QLabel('Select Ring Item format:', self))
        #  Top is a row of formats NSCLDAQ 10-12.
        
            
        formats = QHBoxLayout()
        self._formats = list()
        for version in self._format_versions:
            fmt = QRadioButton(f'NSCLDAQ-{version}', self)
            self._formats.append(fmt)
            formats.addWidget(fmt)
            
        #  Default to the most recent format:
        
        self._formats[-1].setChecked(True)
        layout.addLayout(formats)
        
        #  Now the button box:
        
        self._buttonBox = QDialogButtonBox(QDialogButtonBox.Ok | QDialogButtonBox.Cancel, self)
        self._buttonBox.accepted.connect(self.accept)
        self._buttonBox.rejected.connect(self.reject)
        
        layout.addWidget(self._buttonBox)
        
        self.setLayout(layout)
    
    def exec(self):
        if super().exec():
            for (i, version) in enumerate(self._format_versions):
                if self._formats[i].isChecked():
                    return version
                
            # No match is not possible but be graceful and return cancel code:
            return 0
        else:
            return 0                       # Cancel.