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
    QAction, QFileDialog, QDialog, QDialogButtonBox, QRadioButton, QPushButton,
    QLabel, QWidget, QLineEdit,
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
            self._online.triggered.connect(self._attach_online)
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
        self._detach.triggered.connect(self._detach_source)
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
    
    def _detach_source(self):
        #  This is only slightly complicated;  For histogramers that 
        #  dont' have a rest detach request we use an attach to /dev/null -- which
        #  implicitly assumes they run in an unix environment (e.g. SpecTcl):
        # Must stop but we don't know if it's running:
        
        try:
            self._client.stop_analysis()
        except:
            pass
        try:
            if capabilities.can_rest_detach():
                self._client.detach_source()
            else:
                self._client.attach_source('file', '/dev/null')
        except Exception as e:
            error(f"Unable to detach because: {e}")
    
    def _attach_online(self):
        #  We need:
        #   The ringselector 
        #   Ring URL
        #   Format of data (ring format).
        #  We get that from OnlinePrompter:
        
        prompter = OnlinePrompter(self._menu)
        if prompter.exec():
        
            #  Fish waht we need from the prompter:
        
            url = prompter.ring()
            helper = prompter.ringselector()
            format = prompter.format()
            
            # Build the source string:
            
            source = f'{helper} --source={url} --sample=PHYSICS_EVENT --non-blocking'
            try:
                self._client.attach_source('pipe', source)
                self._client.ringformat_set(format)
                self._client.start_analysis()
            except Exception as e:
                error(f'Unable to attach online source {url} using {helper}: {e}')
        
    def _isRawFile(self, filename, filter):
        # Figure out if the filename is a raw event file:
        
        parts = os.path.splitext(filename)
        print(parts)
        return parts[1] == '.evt'
    
    

#  A widget for selecting the RingFormat:

class RingFormat(QWidget):  
    _format_versions = [10, 11, 12]
    def __init__(self, *args):
        super().__init__(*args)
        formats = QHBoxLayout()
        self._formats = list()
        for version in self._format_versions:
            fmt = QRadioButton(f'NSCLDAQ-{version}', self)
            self._formats.append(fmt)
            formats.addWidget(fmt)
            
        #  Default to the most recent format:
        
        self._formats[-1].setChecked(True)    
        self.setLayout(formats)
    #  Attributes:
    
    def setFormat(self, level):
        for (i, version) in enumerate(self._format_versions):
            if level == version:
                self._formats[i].setChecked(True)
                return
        raise IndexError(f'Unrecognized format level {level}')
    def format(self):
        for (i, version) in enumerate(self._format_versions):
            if self._formats[i].isChecked():
                return version
            
        raise AssertionError("No radio buttons are checked!!")
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
        
            
        self._format = RingFormat(self)
        layout.addWidget(self._format)
        
        #  Now the button box:
        
        self._buttonBox = QDialogButtonBox(QDialogButtonBox.Ok | QDialogButtonBox.Cancel, self)
        self._buttonBox.accepted.connect(self.accept)
        self._buttonBox.rejected.connect(self.reject)
        
        layout.addWidget(self._buttonBox)
        
        self.setLayout(layout)
    
    def exec(self):
        if super().exec():
            return self._format.format()
        else:
            return 0                       # Cancel.

class OnlinePrompter(QDialog):
    # Prompt for what's needed to attach online:
    
    def __init__(self, *args):
        super().__init__(*args)
        
        # Figure out the helper string... use $DAQBIN if not defined
        # we'll give up:
        
        daqbin = os.getenv('DAQBIN')
        if daqbin is None:
            helper = ''
        else:
            helper = os.path.join(daqbin, 'ringselector')
        
        layout = QVBoxLayout()
        
        # At the top is the ringURL:
        
        ringurl = QHBoxLayout()
        self._url = QLineEdit(self)
        ringurl.addWidget(self._url)
        ringurl.addWidget(QLabel('Ring Buffer URL', self))
        
        layout.addLayout(ringurl)
        
        # Next is the helper with a Browse button.
        
        helper_layout = QHBoxLayout()
        self._helper = QLineEdit(helper, self)
        helper_layout.addWidget(self._helper)
        self._browse = QPushButton('Browse...', self)
        helper_layout.addWidget(self._browse)
        
        layout.addLayout(helper_layout)
        
        #  Next down is the ring format chooser:
        
        layout.addWidget(QLabel('ring format:', self))
        self._format = RingFormat(self)
        layout.addWidget(self._format)
        
        # Finally the dialog button box:
        
        self._buttonBox = QDialogButtonBox(QDialogButtonBox.Ok | QDialogButtonBox.Cancel, self)
        self._buttonBox.accepted.connect(self.accept)
        self._buttonBox.rejected.connect(self.reject)
        
        layout.addWidget(self._buttonBox)
        
        self.setLayout(layout)
        
        # Handle internal signals:
        
        self._browse.clicked.connect(self.browse_helper)
    
    #  Attribute implementations:
    
    def ring(self):
        return self._url.text()
    def setRing(self, url):
        self._url.setText(url)
    
    def ringselector(self):
        return self._helper.text()
    def setRingselector(self, helper):
        self._helper.setText(helper)
    
    def format(self):
        return self._format.format()
    def setFormat(self, level):
        self._format.setFormat(level)
        
    # slots:
    
    def browse_helper(self):
        #  Slot to browser for a helper file:
        #  We just use QFileDialog.getOpenFileName:
        
        filename = QFileDialog.getOpenFileName(
            self, "Select Helper program", os.getcwd(), "All Files (*)", "All Files (*)"
        )
        if filename[0] != '' :
            self._helper.setText(filename[0])