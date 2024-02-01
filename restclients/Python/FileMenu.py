from PyQt5.QtWidgets import (
    QAction, QDialog, QDialogButtonBox, QVBoxLayout, QHBoxLayout, QRadioButton, QFileDialog,
    QLabel
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
        
        # Only for SpecTcl:
        
        if program == capabilities.Program.SpecTcl:
            self._save_treevars = QAction('Save Treevariables...', self)
            self._menu.addAction(self._save_treevars)
            self._save_treevars.triggered.connect(self._save_vars)
            
        self._save_spectra = QAction('Save spectrum contents...', self)
        self._save_spectra.triggered.connect(self._saveSpectra)
        self._menu.addAction(self._save_spectra)
        
        self._menu.addSeparator()
        
        self._load = QAction('Load...', self)
        self._load.triggered.connect(self._load_definitions)
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
        
        file = self. _getSqliteFilename()
        if file == ('',''):
            return
        filename = self._genfilename(file)
        
        # If the file exists, delete it:
        
        try:
            os.remove(filename)
        except:
            pass
        saver = DefinitionIO.DefinitionWriter(filename)
        
        try:
        
        # Save the parameters:
        
            parameter_defs = self._client.parameter_list()['detail']
            saver.save_parameter_definitions(parameter_defs)
            
            # Spectrum definitions:
            
            spectrum_defs = self._client.spectrum_list()['detail']
            saver.save_spectrum_definitions(spectrum_defs)
            
            #  Conditions:
            
            condition_defs = self._client.condition_list()['detail']
            saver.save_condition_definitions(condition_defs)
            
            gate_defs = self._client.apply_list()['detail']
            saver.save_gates(gate_defs)
            
            # SpecTcl has variables:
            
            if self._program == capabilities.Program.SpecTcl:
                var_defs = self._client.treevariable_list()['detail']
                saver.save_variables(var_defs)
        except Exception as  e:
            error(f'Failed to write {filename}: {e}')
            
        
    def _save_vars(self):
        #  Save only the tree variables to a database file
        file = self._getSqliteFilename()
        if file == ('',''):
            return
        filename = self._genfilename(file)
        
        # If the file exists, delete it:
        
        try:
            os.remove(filename)
        except:
            pass
        try:
            writer = DefinitionIO.DefinitionWriter(filename)
            vars = self._client.treevariable_list()['detail']
            writer.save_variables(vars)
        except Exception as e:
            error(f'Failed to write tree variables to {filename} : {e}')
        
    def _saveSpectra(self):
        #  Prompt for spectra to save and the format
        #  and prompt for a file to save them into...
        namePrompter = SpectrumSaveDialog(self._menu)
        if namePrompter.exec():
            names = namePrompter.selectedSpectra()
            format = namePrompter.format()
            
            wdir = os.getcwd()
            if format == 'json':
                default_ext = 'Json (*.json);; Text (*.txt)'
            elif format == 'ascii':
                default_ext = 'Spectrum (*.spec);; Text (*.txt)'
            elif format == 'binary':
                default_ext = 'Binary (*.bin);; Any (*.*)'
            else:
                default_ext = 'spec'
            if len(names) > 0:
                name = QFileDialog.getSaveFileName(
                    self._menu, 'Spectrum File', wdir, default_ext)
                if not name == ('', ''):
                    filename  = self._genfilename(name)
                    
                    try:
                        self._client.spectrum_write(filename, format, names)
                    except RustogramerException as e:
                        error(f'Failed to save spectra to {filename} : {e}')
        
    def _load_definitions(self):
        #  Load definitions from a database file:
        
        file = self._getExistingSqliteFilename()
        if file[0] == '':
            return
        filename = self._genfilename(file)
        reader = DefinitionIO.DefinitionReader(filename)
        
        #  Read the parameters from the database and restore them (not so simple actually):
        
        parameters = reader.read_parameter_defs()
        self._update_parameters(parameters)
        
        spectra = reader.read_spectrum_defs()
        # There are several things they may want to do with existing spectra:
        # figure them out:
        if len(spectra) > 0:
            existing_dialog = DupSpectrumDialog(self._menu)
            choice = existing_dialog.exec()
            existing = set()               # Will be existing indexed by name:
            existing_spectra = self._client.spectrum_list()['detail']
            if choice == 1:
                # Existing is an empty set after we delete everything:
                for spectrum in existing_spectra:
                    self._client.spectrum_delete(spectrum['name'])
            elif choice == 2 or choice == 3:
                # Need the existing spectrum names set:
                for spectrum in existing_spectra:
                    existing.add(spectrum['name'])
            else:
                pass
            self._restore_spectra(choice, spectra, existing)
        
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
        if '.' in parts[1]:
            return name
        else:
            return name + '.' + dialog_name[1]
    def _getSqliteFilename(self):
          return  QFileDialog.getSaveFileName(
            self._menu, 'Definition File', os.getcwd(), 
            'Sqlite3 (*.sqlite)'
        )      
    def _getExistingSqliteFilename(self):
           return  QFileDialog.getOpenFileName(
            self._menu, 'Definition File', os.getcwd(), 
            'Sqlite3 (*.sqlite)'
        )        
    def _update_parameters(self, definitions):
        #  Takes a set of parameter definitions and does what's needed.
        # What's needed:
        #   If the parameter does not exist, first just create it.
        #   Then edit the parameter's properties to match what's in its new definition.
        
        existing_defs = self._client.parameter_list()['detail']
        existing_map = dict()
        for p in existing_defs:
            existing_map[p['name']] = p

        # Now we can run through the new definitions:
        
        names = existing_map.keys()
        for p in definitions:
            name = p['name']
            if not name in names:
                self._client.parameter_create(name, {})
            # Only pull the non null ones out:
            
            mods = dict()
            if p['low'] is not None:
                mods['low'] = p['low']
            if p['high'] is not None:
                mods['high'] = p['high']
            if p['units'] is not None:
                mods['units'] = ['units']
            
            if mods:
                # Dicts are true if non-empty:
                self._client.parameter_modify(name, mods)
    def _restore_spectra(self, dupchoice, spectra, existing):
        #  dupchoice - selection from the DupSpectrumDialog   
        #  spectra   - Description of spectra to restore.
        #  existing  - Set of existing spectrum names.
        
        for spectrum in spectra:
            if spectrum['name'] in existing:
                if dupchoice == 2:          # Replace
                    self._client.spectrum_delete(spectrum['name'])
                else:
                    # Else is ok because existing is empty if dupchoice is 1 so
                    # Choice must be 3 (keep existing).
                    continue            # Do nothing with that definition.
            # At this point we can create the spectrum:
            
            self._create_spectrum(spectrum)
    def _create_spectrum(self, definition):
        
        # Create a spectrum given its definition;  how depends on type:
        
        name = definition ['name']
        stype = definition['type']
        dtype = definition['datatype']
        
        
        # If interchanging spectra between SpecTcl <--> Rustogramer we may need to 
        # massage the data type:
        
        if capabilities.DataTypeStringsToChannelTypes[dtype] not in \
            capabilities.get_supported_channelTypes():
            dtype = capabilities.ChannelTypesToDataTypeStrings[
                capabilities.get_default_channelType()]
        
        
        if stype == '1':
            self._client.spectrum_create1d(
                name, definition['xparameters'][0], 
                definition['xaxis']['low'], definition['xaxis']['high'], definition['xaxis']['bins'],
                dtype
            )
        elif stype == '2':
            self._client.spectrum_create2d(
                name, definition['xparameters'][0], definition['yparameters'][0],
                definition['xaxis']['low'], definition['xaxis']['high'], definition['xaxis']['bins'],
                definition['yaxis']['low'], definition['yaxis']['high'], definition['yaxis']['bins'],
                dtype
            )
        elif stype == 'g1':
            self._client.spectrum_createg1(
                name, definition['parameters'],
                definition['xaxis']['low'], definition['xaxis']['high'], definition['xaxis']['bins'],
                dtype
            )
        elif stype == 'g2':
            self._client.spectrum_createg2(
                name, definition['parameters'],
                definition['xaxis']['low'], definition['xaxis']['high'], definition['xaxis']['bins'],
                definition['yaxis']['low'], definition['yaxis']['high'], definition['yaxis']['bins'],
                dtype
            )
        elif stype == 'gd':
            self._client.spectrum_creategd(
                name, definition['xparameters'], definition['yparameters'],
                definition['xaxis']['low'], definition['xaxis']['high'], definition['xaxis']['bins'],
                definition['yaxis']['low'], definition['yaxis']['high'], definition['yaxis']['bins'],
                dtype
            )
        elif stype == 's':
            # Axis definition...if there's a y axis we use it otherwise the X
            # That may be a SpecTcl Rustogramer difference:
            
            if definition['yaxis'] is not None:
                axis = definition['yaxis']
            else:
                axis = definition['xaxis']
            self._client.spectrum_createsummary(
                name, definition['parameters'],
                axis['low'], axis['high'], axis['bins'],
                dtype
            )
        elif stype == 'm2':
            # If there's no y parameters, then every other parameter in x/y is 
            # x,y
            if definition['yparameters'] is None:
                xparams =list()
                yparams = list()
                p  = iter(definition['parameters'])
                for x in p:
                    xparams.append(x)
                    yparams.append(next(p))
            else:
                xparams = definition['xparameters']
                yparams = definition['yparameters']
            self._client.spectruM_create2dsum(
                name, xparams, yparams,
                definition['xaxis']['low'], definition['xaxis']['high'], definition['xaxis']['bins'],
                definition['yaxis']['low'], definition['yaxis']['high'], definition['yaxis']['bins'],
                dtype
            )
        elif stype == 'S':
            self._client.spectrum_createstripchart(
                name, definition['xparameters'][0], definition['yparameters'][1],
                definition['xaxis']['low'], definition['xaxis']['high'], definition['xaxis']['bins'],
                definition['yaxis']['low'], definition['yaxis']['high'], definition['yaxis']['bins'],
                dtype
            )
        elif stype == 'b':
            self._client.spectrum_createbitmask(
                name, definition['xparameters'][0],
                definition['xaxis']['low'], definition['xaxis']['high'], definition['xaxis']['bins'],
                dtype
            )
        elif stype == 'gs':
            print(' gamma summary def looks like: ', definition)
        else:
            error(f'Specturm if type {stype} is not supported at this time.')
            
        
        
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
        
        

# This dialog is used to get how to handle spectra read in.  There are three choices:
#  -   Delete all first.
#  -   Replace any duplicates.
#  -   Keep the duplicates as is.
#
#  Instantiate this dialog then its exec method will
#  return one of:
#    - 1  - Delete all
#    - 2  - Replace duplicates.
#    - 3  - Keep duplicates.
#    - 0  - The dialog was cancelled you should not recover the spectra.
#        
class DupSpectrumDialog(QDialog):
    def __init__(self, *args):
        super().__init__(*args)
        layout = QVBoxLayout()
        
        # At the top is the explanatory text.
        
        self._explanation = QLabel(self)
        self._explanation.setWordWrap(True)
        self._explanation.setText(" \
You are about to recover spectrum definitions from a file.  It is possible there are already \
spectra defined with the same names as this in the file.  Decide what you want to do with \
existing spectra:")
        layout.addWidget(self._explanation)
        
        # Second row is an hbox layout that has the radio buttons.  The
        # initial value is delete all spectra first.
        
        radios = QHBoxLayout()
        self._deleteall = QRadioButton('Delete all existing', self)
        self._deleteall.setChecked(True)
        radios.addWidget(self._deleteall)
        
        self._deletedups = QRadioButton("Ovewrite existing defs", self)
        radios.addWidget(self._deletedups)
        
        self._keepdups = QRadioButton("Don't re-define duplicates", self)
        radios.addWidget(self._keepdups)
        
        layout.addLayout(radios)
        
        # Now the dialog buttons:
        
        self._buttonBox = QDialogButtonBox(QDialogButtonBox.Ok | QDialogButtonBox.Cancel, self)
        self._buttonBox.accepted.connect(self.accept)
        self._buttonBox.rejected.connect(self.reject)
        
        layout.addWidget(self._buttonBox)
        
        self.setLayout(layout)
    
    def exec(self):
        # Override the exec - after the base class finishes interrogate the radios
        # to decide what to return:
        
        if super().exec():
            if self._deleteall.isChecked():
                return 1
            if self._deletedups.isChecked():
                return 2
            if self._keepdups.isChecked():
                return 3
            # Should not land here but...
            
            return 0      # Treat it like  a cancel.
        else:
            return 0      #  Canacel