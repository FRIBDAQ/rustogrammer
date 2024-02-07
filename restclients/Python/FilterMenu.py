
from PyQt5.QtWidgets import (
    QAction, QWizard, QWizardPage, QLabel, QLineEdit, QPushButton, QCheckBox, QFileDialog,
    QVBoxLayout, QHBoxLayout
)
from PyQt5.QtCore import QObject
from PyQt5.Qt import *

import os

from gatelist import ConditionChooser, common_condition_model
from ParameterChooser import ParameterTree, update_model
from editablelist import EditableList
from spectrumeditor import error

class FilterMenu(QObject):
    
    def __init__(self, menu, client, gui, data_source, *args):
        super().__init__(*args)
        
        self._menu = menu
        self._client = client
        self._gui = gui
        self._data_source = data_source
        
        # ALl items are SpecTcl because that's the only thing that supports filters so:
        
        self._wizard = QAction('Filter Wizard...')
        self._wizard.triggered.connect(self._filter_wizard)
        self._menu.addAction(self._wizard)
        
        self._enables = QAction('Enable/Disable Filters...')
        self._menu.addAction(self._enables)
        
        self._menu.addSeparator()
        
        self._read = QAction('Read Filter File...')
        self._read.triggered.connect(self._data_source.attach_filter)
        self._menu.addAction(self._read)
        
    def _filter_wizard(self):
        wiz = FilterWizard(self._client, self._menu)
        if wiz.exec():
            name = wiz.name()
            gate = wiz.gate()
            parameters = wiz.parameters()
            filename = wiz.file()
            enable = wiz.enable()
            
            try:
                self._client.filter_new(name, gate, parameters)
                self._client.filter_setfile(name, filename)
                if enable:
                    self._client.filter_enable(name)
            except Exception as e:
                error(f'Unable to create filter {name}: {e}')
                return

# Code int this section creates the filter wizard:

class FilterWizard(QWizard):
    def __init__(self, client, *args):
        super().__init__(*args)
        self._client = client
        
        # Wizard pages:
        
        self._name = NamePage(self)
        self.addPage(self._name)
        
        self._gate = GatePage(self._client, self)
        self.addPage(self._gate)
        
        self._parameters = ParametersPage(self._client, self)
        self.addPage(self._parameters)
        
        self._file = FilePage(self)
        self.addPage(self._file)
    def name(self):
        return self.field('name')
    def gate(self):
        return self._gate.gate()    
    def parameters(self):
        return self._parameters.parameters()
    def file(self):
        return self.field('file')
    def enable(self):
        return self.field('enable')
    
class NamePage(QWizardPage):
    # Introduces the filter wizard and lets the filter name be set:
    
    
    def __init__(self, *args):
        super().__init__(*args)
        self.setTitle('Filter Name')
        
        layout = QVBoxLayout()
        
        intro = QLabel("\
Welcome to the filter wizard.  This wizared guides you through the process of creating and, optionally, \
enabling a filter.  Filters allow you to write a reduced data set to a filter file \
Defining a filter requires: \n\
    1. A filter name \n\
    2. A gate which determines which events are written to the filter \n\
    3. The set of parameters to write for each event \n\
    4. The file to which the filtered data are written. \n\
    5. The enable flag which, if set, enables the filter to write data \n\
let's start with the filter name: \
",self)
        intro.setWordWrap(True)
        layout.addWidget(intro)
        
        prompt = QHBoxLayout()
        prompt.addWidget(QLabel('Filter name: '))
        self._name = QLineEdit(self)
        prompt.addWidget(self._name)
        layout.addLayout(prompt)
        self.registerField('name', self._name)
        
        self.setLayout(layout)
        
class GatePage(QWizardPage):
    def __init__(self, client, *args):
        super().__init__(*args)
        self._client = client
        self.setTitle('Choose Gate for page')
        
        # We'll use a combobox for the gate:
        
        layout = QVBoxLayout()
        prompt  = QLabel('\
Filters only write events that satisfy their gates.  If you do want to write all events to file \
simply choose a True gate from the list below. \
', self)
        prompt.setWordWrap(True)
        layout.addWidget(prompt)
        
        prompt = QHBoxLayout()
        prompt.addWidget(QLabel("Gate: ", self))
        self._gate = ConditionChooser(self)
        prompt.addWidget(self._gate)
        self.registerField('gate', self._gate)
        
        
     
        layout.addLayout(prompt)
        
        # Give the user a button to update the gate list:
        
        self._update = QPushButton('Update Gate list', self)
        self._update.clicked.connect(self._update_list)
        layout.addWidget(self._update)
        
            
        self.setLayout(layout)
    def _update_list(self):
        common_condition_model.load(self._client)
    
    def gate(self):
        return self._gate.currentText()
    
        

class ParametersPage(QWizardPage):
    def __init__(self, client, *args):
        super().__init__(*args)
        self._client = client
        self.setTitle('Choose parameters to write')
        layout = QVBoxLayout()
        
        chooser = QHBoxLayout()
        self._tree = ParameterTree(self)
        self._list = EditableList('Selected Parameters', self)
        chooser.addWidget(self._tree)
        chooser.addWidget(self._list)
        layout.addLayout(chooser)
        
        self._update = QPushButton("Update Parameters", self)
        
        layout.addWidget(self._update)
        
        self.setLayout(layout)
        
        # Internal signal handling:
        
        self._update.clicked.connect(self._update_parameters)
        self._list.add.connect(self._add)
        
    def _update_parameters(self):
        update_model(self._client)
        
    def _add(self):
        for parameter  in self._tree.selection():
            self._list.appendItem(parameter)
    def parameters(self):
        return self._list.list()
class FilePage(QWizardPage):
    def __init__(self, *args):
        super().__init__(*args)
        self.setTitle('Filter file')
        
        layout = QVBoxLayout()
        
        file = QHBoxLayout()
        self._file = QLineEdit(self)
        self.registerField('file', self._file)
        file.addWidget(self._file)
        
        self._browse = QPushButton('Browse...', self)
        file.addWidget(self._browse)
        layout.addLayout(file)
        
        self._enable = QCheckBox('Enable Filter', self)
        self.registerField('enable', self._enable)
        layout.addWidget(self._enable)
        
        self.setLayout(layout)
        
        # Signal handling:
        
        self._browse.clicked.connect(self._browsefile)
    
    def _browsefile(self):
        file = QFileDialog.getSaveFileName(
            self, 'Filter file', os.getcwd(),
            "Filter Files (*.flt);;All Files (*)", "Filter Files (*.flt)"
        )
        name = file [0]
        if name != '':
            self._file.setText(name)