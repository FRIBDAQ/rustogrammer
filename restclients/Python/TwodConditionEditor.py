'''
   Provides an editor for two dimensional conditions like gates and bands.
   The idea is that you have an editable list in which x/y points can be  inserted
   2d gates need:
   *   A condition name.
   *   An X parameter
   *   A Y parameter.
   *   A list of points (settable minimum number of required points defaults to 3)
   
   
'''

from PyQt5.QtWidgets import (
    QLabel, QLineEdit, QPushButton, QWidget,
    QHBoxLayout, QVBoxLayout
)
from PyQt5.QtGui import QDoubleValidator
from PyQt5.QtCore import pyqtSignal


from editablelist import EditableList
from ParameterChooser import LabeledParameterChooser
from spectrumeditor import error
from parse import parse

class TwodConditionEditor(QWidget):
    '''
        Signals:
           commit - Create/replace the condition.
        Slots:
            validate - Called from the Create/Replace button's clicked signal. If
                -   There's a condition name.
                -   There's an x parameter.
                -   There's a Y parameter
                -   There are at lesat the minimum required points
                commit is emitted.
                If any of these conditions is not met, an error is popped up.
            addpoint - Add an x/y poiunt to the list.
        Attributes:
            name - name of the condition.
            xparam - name of the x parameter.
            yparam - name of the y parameter.
            x      - X value (for point)
            y      - Y value (for point)
            points - list of points, each point containing {'x':xxxx, 'y':yyyy}  
            minpoints - Minimum allowed # of points     
    '''
    commit = pyqtSignal()
    def __init__(self, *args):
        super().__init__(*args)
        
        layout = QVBoxLayout()
        
        # Top is the condition name prompter:
        
        line1 = QHBoxLayout()
        line1.addWidget(QLabel('Name: ', self))
        self._name = QLineEdit(self)
        line1.addWidget(self._name)
        
        layout.addLayout(line1)
        
        #   Line 2 are the X/Y parameters:
        
        line2 = QHBoxLayout()
        line2.addWidget(QLabel('X param', self))
        self._xparam = LabeledParameterChooser(self)
        line2.addWidget(self._xparam)
        line2.addWidget(QLabel('Y param', self))
        self._yparam = LabeledParameterChooser(self)
        line2.addWidget(self._yparam)
        
        layout.addLayout(line2)
        
        # Line 3 is a bit more complicated.
        #  |                                       |
        #  | X: [ entry ]     +------------------+ |
        #  | Y: [ entry ]     | Point list       | |
        #  |                  +------------------+ |
        #  |                                       |
        line3 = QHBoxLayout()
        x     = QHBoxLayout()
        y     = QHBoxLayout()
        coord = QVBoxLayout()
        
        x.addWidget(QLabel('X ', self))
        self._x = QLineEdit(self)
        self._x.setValidator(QDoubleValidator())
        x.addWidget(self._x)
        coord.addLayout(x)
        
        y.addWidget(QLabel("Y ", self))
        self._y = QLineEdit(self)
        self._y.setValidator(QDoubleValidator())
        y.addWidget(self._y)
        coord.addLayout(y)
        coord.addStretch(1)
        line3.addLayout(coord)
        
        self._points =  EditableList('Points', self)
        line3.addWidget(self._points)
        
        layout.addLayout(line3)
        
        #  The create/replace is bottom.
        
        layout.addStretch(1)
        self._commit = QPushButton('Create/Replace', self)
        layout.addWidget(self._commit)
        
        self.setLayout(layout)
        
        # Attribute initialization

        self._minpoints = 3        # Contour
        
        # Internally catch the Add signal from the editable list
        # IF all goes well it will call the addpoint slot.
        
        self._points.add.connect(self._addpoint)
        
        # Similarly the commit button goes to our validate slot
        
        self._commit.clicked.connect(self.validate)
    
    # Implement attributes:
    
    def name(self):
        return self._name.text()
    def setName(self, txt):
        self._name.setText(txt)
        
    def xparam(self):
        return self._xparam.parameter()
    def setXparam(self, name):
        self._xparam.setParameter(name)
        
    def yparam(self):
        return self._yparam.parameter()
    def setYparam(self, name):
        self._yparam.setParameter(name)
        
    def x(self):
        try:
            return float(self._x.text())
        except:
            return None      # Float conversion failed.
    def setX(self, value):
        self._x.setText(f'{value}')
    
    def y(self):
        try:
            return float(self._y.text())
        except:
            return None
    def setY(self, value):
        set._y.setText(f'{value}')
        
    def points(self):
        text_list = self._points.list()
        result = list()
        
        for item in text_list:
            parsed = parse("({}, {})", item)
            result.append({'x': parsed[0], 'y': parsed[1]})
        return result
    def setPoints(self, pts):
        items = list()
        for pt in pts:
            x = pt['x']
            y = pt['y']
            items.append(f'({x}, {y})')
        self._points.setList(items)
        
    def minpoints(self):
        return self._minpoints
    def setMinpoints(self, value):
        self._minpoints = value
        
    #   slots...note that if derived classes want additional
    #  Validations they should perform the first before invoking
    #  super().validate()
    #
    def validate(self):
        if self._empty(self.name()):
            error("A condition name is required")
            return
        if self._empty(self.xparam()) or self._empty(self.yparam()):
            error('Both X and Y parameters are must be selected')
            return
        m = self.minpoints()
        if len(self.points()) < m:
            error(f'At least {m} points are required for this type of condition.')
            return
        self.commit.emit()
    
    # internal slots:
    
    def _addpoint(self):
        x = self.x()
        y = self.y()
        if x is None or y is None:
            return         # Need both x and y.
        
        self._points.appendItem(f'({x}, {y})')
    
    #  Utilities
    def _empty(self, txt):
        return txt is None or txt == '' or txt.isspace()
        
        
#----------------- Test code ---------------------


def newgate():
    print("New gate accepted:")
    print("  name", wid.name())
    print("  xpar", wid.xparam())
    print("  ypar", wid.yparam())
    print("  pts ", wid.points())

if __name__ == '__main__':
    from PyQt5.QtWidgets import QApplication, QMainWindow
    from rustogramer_client import rustogramer as rc
    from ParameterChooser import update_model
    client = rc({'host': 'localhost', 'port': 8000})
    update_model(client)
    
    app = QApplication([])
    win = QMainWindow()
    
    wid = TwodConditionEditor()
    wid.commit.connect(newgate)
    
    win.setCentralWidget(wid)
    win.show()
    app.exec()
        
        
        