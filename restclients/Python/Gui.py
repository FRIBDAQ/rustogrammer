''' Top level GUI (Main program)
    the following program parameters are supported

    --host - host running rustogramer or SpecTcl - defaults to 'localost'
    --port - port on which the REST server is listening (defaults to 8000)
    --service - Defaults to None - service the REST server advertises
    --service_user - User the service is advertised under defaults to the name of the current user.

'''

from argparse import ArgumentParser
import os
import sys
from PyQt5.QtWidgets import (
    QApplication, QMainWindow, QTabWidget, QWidget
)
from spectra import SpectrumWidget
import spectra
import argparse
import capabilities
import parametercontroller
import parameditor
import gatelist
import gates
from rustogramer_client import rustogramer as RestClient

PORTMAN_PORT=30000

parser = argparse.ArgumentParser(
    prog='Gui.py',
    description='User interface GUI for SpecTcl and rustogramer programs',
    epilog ='If --service and --port are provided, --service overrides.  If --service is not provided, --port is used.'
)
parser.add_argument('-H', '--host', 
    default='localhost', action='store', help='Host on which the histogram program is running'
)
parser.add_argument('-p', '--port', 
    default=8000, action='store', help='Port on which the histogramer REST server is listening for connections defaults to "8000"'
)
parser.add_argument('-s', '--service', default=None, action='store', help='Service the REST server advertises defaults to None')
parser.add_argument('-u', '--service-user', default=os.getlogin(), action='store', help=f'Username the REST server advertises under defaults to "{os.getlogin()}"')

args = parser.parse_args()

client_args = {'host' : args.host, 'port':args.port, 'pmanport': PORTMAN_PORT}
if args.service is not None:
    client_args['service'] = args.service
    client_args['user']    = args.service_user
    

client = RestClient(client_args)
spectra.set_client(client)
capabilities.set_client(client)
gatelist.common_condition_model.load(client)


#  Build the GUI:

app = QApplication(sys.argv)
main = QMainWindow()

tabs = QTabWidget()
spectrum_view = spectra.SpectrumWidget()
tabs.addTab(spectrum_view,'Spectra')
param_view= parameditor.ParameterEditor()
tabs.addTab(param_view, 'Parameters')
param_controller = parametercontroller.ParameterController(
    param_view, client, spectrum_view
)

if capabilities.get_program() == capabilities.Program.SpecTcl:
    tabs.addTab(QWidget(), 'Variables')
condition_view = gates.Gates()
condition_controller = gates.Controller(condition_view, client)
tabs.addTab(condition_view, 'Gates')

main.setCentralWidget(tabs)

main.show()
app.exec()
 
