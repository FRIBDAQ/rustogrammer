''' Provides a Parameter Chooser widget with a shared model.
  By shared model I mean that all instances of a parameter tree
  will contain the same tree of parameters.  This means we only have to
  turn the parameter names into a tree once and that's done it
  for all parameter choosers.

  This is done by constructing a ComboTree and replacing its model
  with our shared model.  loading the tree into any parameter chooser
  will therefore load it into all.

'''


from PyQt5.QtGui import QStandardItemModel
from PyQt5.QtWidgets import (
    QApplication, QWidget, QHBoxLayout, QMainWindow, QWidget
)
from ComboTree import ComboTree
from rustogramer_client import rustogramer
import TreeMaker as tm

_parameter_model = QStandardItemModel()

class Chooser(ComboTree):
    def __init__(self, *args):
        super().__init__(*args)
        self.setModel(_parameter_model)
    def load_parameters(self, client):
        self.clear()              # Don't accumulate
        parameters = client.parameter_list()
        names = []
        for parameter in  parameters['detail']:
            names.append(parameter['name'])
        names.sort()
        tree = tm.make_tree(names)
        self.load_tree(tree)
    

#  Test - Make widget 1, connect to SpecTcl to load the model,
#  make widget 2... the two widgets should both list all parameters:

p=None

def sel(l):
    print("Selected: ", l)
    print("Parameter: ", '.'.join(l))

def new_index(i):
    global p
    print("Index ", i)
    print("That's ", p.current_item())
def test(host, port):
    global p
    client = rustogramer({'host': host, 'port': port})
    app = QApplication([])
    mw = QMainWindow()

    c = QWidget()
    l = QHBoxLayout()
    p1 = Chooser(c)
    p1.load_parameters(client)
    p1.selected.connect(sel)
    p = p1
    p1.currentIndexChanged.connect(new_index)
    l.addWidget(p1)
    p2 = Chooser(c)
    l.addWidget(p2)
    c.setLayout(l)


    mw.setCentralWidget(c)
    mw.show()
    app.exec()