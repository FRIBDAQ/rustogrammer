''' This module provides a megawidget (oneDEditor) that edits/creates
a 1d spectrum.  In addition to the spectrum type,
a 1d spectrum has:
   *   a name.
   *   a parameter
   *   a single axis definition.

 Therefore the followinng properties are provided:
  *  name - the spetrum the spectrujm name.
  *  parameter(readonly) - the currently selected parameter.
  *  low  - Axis low limit.
  *  high - axis high limit.
  *  bins - Axis bins.
  *  array - User wants an  'array' of spectra made.

  Note that if a parameter is specified and has the
  low/high/bins recommendations, these should be loaded
  into the axis properties by the client.

  The following signals are provided:

    *  nameChanged - the name of the spectrum has changed.
    *  parameterSelected - the parameter of the spectrum
        has been selected.
    *  axisModified - the axis has been modified.
    *  commit - The user pushed the create/modify button.

    The following slot is provided:

    *  update_parameters - the parameters loaded into the e paramter
    selector should be updated from the server program.
    This should be passed a rustogramer client object.

    nameChanged Signal - this signal provides the current text
    of the spectrum name.  If the spectrum is already an
    existing spectrum, the program shouild fill in the
    remaining properties (parameter name and axis definition) of
    that spectrum).

    parameterSelected Signal - this signal provides the current textual
    name of the parameter.  The client should load the axis
    definition of the spectrum with any recommended axis
    spefication associated with that parameter or leave the
    axis definition unchanged, if there is none.  Note that
    if the spectrum is created the axis definition _may_ he
    associated with the parameter for future use.

    axisModified Signal - a dict is passed to the slot that
    contains the following keys:  'low', 'high' and 'bins'
    with obvious contents.  Since the axis entries are 
    comboboxes, none of these will be None.

    '''



from axisdef import AxisInput
from ParameterChooser import Chooser as ParameterChooser
from PyQt5.QtWidgets import (
    QLineEdit, QWidget, QGridLayout, QVBoxLayout, QLabel,
    QApplication, QMainWindow, QPushButton, QCheckBox
)
from PyQt5.QtCore import pyqtSignal, Qt
from rustogramer_client import rustogramer as cl

class oneDEditor(QWidget):
    nameChanged = pyqtSignal(str)
    parameterSelected = pyqtSignal(str)
    axisModified = pyqtSignal(dict)
    commit = pyqtSignal()

    def __init__(self, *args):
        super().__init__(*args)

        # Define the widgets in the UI:

        namel = QLabel('Name:', self)
        self.sname  = QLineEdit(self)
        self.sname.setText('aaa')
        
        self.array = QCheckBox('Array?', self)
        
        paraml = QLabel('Parameter', self)
        self.pchooser = ParameterChooser(self)
        self.chosen_param = QLabel('')
        
        param_layout = QVBoxLayout()
        param_layout.addWidget(paraml)
        param_layout.addWidget(self.pchooser)
        param_layout.addWidget(self.chosen_param)


        axisl = QLabel('X Axis:', self)
        self.axis = AxisInput(self)
        axis_layout = QVBoxLayout()
        axis_layout.addWidget(axisl)
        axis_layout.addWidget(self.axis)

        c = QPushButton('Create/Replace')

        #  Lay them out in a (hopefully) visually
        #  appealing manner.

        label_align = Qt.AlignLeft | Qt.AlignBottom
        widget_align = Qt.AlignLeft | Qt.AlignTop

        layout = QGridLayout()
        layout.addWidget(namel,          0, 0, label_align)
        layout.addWidget(self.sname,      1, 0, widget_align)
        layout.addWidget(self.array,     1, 1, widget_align)

        layout.addLayout(param_layout,   2, 0, widget_align)
        layout.addLayout(axis_layout,    2, 1, widget_align)
        
        layout.addWidget(c,              3, 1, widget_align)
        
        self.setLayout(layout)

        # Connect internal signals to slots:

        self.sname.textChanged.connect(self.nameTextChanged)
        self.pchooser.selected.connect(self.parameterChanged)
        self.axis.lowChanged.connect(self.axisChanged)
        self.axis.highChanged.connect(self.axisChanged)
        self.axis.binsChanged.connect(self.axisChanged)
        c.pressed.connect(self.make_spectrum)

    # Attribute getter/setter methods.

    def name(self):
        return self.sname.text()
    def setName(self, text):
        self.sname.setText(text)
    
    def parameter(self):
        return self.chosen_param.text()
    
    def low(self):
        return self.axis.low()
    def setLow(self, value):
        self.axis.setLow()

    def high(self):
        return self.axis.high()
    def setHigh(self, value):
        self.axis.setHigh(value)
    
    def bins(self):
        return self.axis.bins()
    def setBins(self, value):
        self.axis.setBins(value)

    def array(self):
        return self.array.checkState() == Qt.Checked
    def setArray(self, value):
        if value :
            state = Qt.checked
        else:
            state = Qt.Unchecked
        self.array.setCheckState(state)


    

    # Define slot methods:

    def nameTextChanged(self, new_name):
        self.nameChanged.emit(new_name)
    

    def parameterChanged(self, new_path):
        # We turn the new_path, a list of path
        # elements into a full parameter name:

        path = '.'.join(new_path)
        self.chosen_param.setText(path)
        self.parameterSelected.emit(path)

    def axisChanged(self, value):
        # Marshall the dict:

        axis_def = {
            'low'  : self.axis.low(),
            'high' : self.axis.high(),
            'bins' : self.axis.bins()
        }
        self.axisModified.emit(axis_def)
    
    def make_spectrum(self):
        self.commit.emit()

    def update_parameters(self, client):
        self.pchooser.load_parameters(client)



# Test the UI:
client = None
editor = None

# A real one would test for and delete an existing 
# spectrum with that name.
def create():
    global editor
    global client

    sname = editor.name()
    if sname is not None and len(sname) > 0:
        param = editor.parameter()
        low   = editor.low()
        high  = editor.high()
        bins  = editor.bins()
        client.spectrum_create1d(sname, param, low, high, bins)
        client.sbind_spectra([sname])
def test(host, port):
    global client
    global editor
    app = QApplication(['editor1d-test'])
    c = QMainWindow()

    editor = oneDEditor(c)
    c.setCentralWidget(editor)
    client = cl({'host': host, 'port': port})
    editor.update_parameters(client)
    editor.commit.connect(create)

    c.show()
    app.exec()

      






