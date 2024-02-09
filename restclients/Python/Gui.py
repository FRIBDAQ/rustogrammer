''' Top level GUI (Main program)
    the following program parameters are supported

    --host - host running rustogramer or SpecTcl - defaults to 'localost'
    --port - port on which the REST server is listening (defaults to 8000)
    --service - Defaults to None - service the REST server advertises
    --service_user - User the service is advertised under defaults to the name of the current user.

'''

from argparse import ArgumentParser
import OsServices
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
from treevariable import TreeVariableView, common_treevariable_model
from treevariableController import TreeVariableController
from rustogramer_client import rustogramer as RestClient

import FileMenu
import DataSourceMenu
import FilterMenu
import SpectraMenu
import GateMenu
import HelpMenu


def setup_menubar(win, client):
    '''
    Sets up the menubar and the menus in it:
    File, "Data Source", Filters[If SpecTcl], Gate, Help
    
    Note as well, that the contents of some menus will depend on the 
    program capabilties as well.
    
    Parameters:
    * win - Must be the application main window.
    * client -  is the client object through which we make REST requests of the server.
    '''
    # Our menu objects need to be global:
    
    global file_menu_object
    global data_source_menu_object
    global filter_menu_object
    global spectra_menu_object
    global gate_menu_object
    global help_menu_object
    
    menubar = win.menuBar()
    file_menu = menubar.addMenu('&File')
    file_menu_object = FileMenu.FileMenu(file_menu, client, win)
    
    data_source_menu = menubar.addMenu('Data &Source')
    data_source_menu_object = DataSourceMenu.DataSourceMenu(data_source_menu, client, win)
    
    if capabilities.get_program() == capabilities.Program.SpecTcl:
        filter_menu = menubar.addMenu('Filters')
        filter_menu_object = FilterMenu.FilterMenu(filter_menu, client, win, data_source_menu_object)
    spectrum_menu = menubar.addMenu("&Spectra")
    spectra_menu_object = SpectraMenu.SpectraMenu(spectrum_menu, client, win, file_menu_object)
    
    gate_menu = menubar.addMenu("&Gate")
    gate_menu_object = GateMenu.Gate(gate_menu, client, win, spectra_menu_object)
    
    
    help_menu = menubar.addMenu("&Help")
    help_menu_object = HelpMenu.Help(help_menu)
    

PORTMAN_PORT=30000

parsed_args = argparse.ArgumentParser(
    prog='Gui.py',
    description='User interface GUI for SpecTcl and rustogramer programs',
    epilog ='If --service and --port are provided, --service overrides.  If --service is not provided, --port is used.'
)
parsed_args.add_argument('-H', '--host', 
    default='localhost', action='store', help='Host on which the histogram program is running'
)
parsed_args.add_argument('-p', '--port', 
    default=8000, action='store', help='Port on which the histogramer REST server is listening for connections defaults to "8000"'
)
parsed_args.add_argument('-s', '--service', default=None, action='store', help='Service the REST server advertises defaults to None')
parsed_args.add_argument('-u', '--service-user', default=OsServices.getlogin(), action='store', help=f'Username the REST server advertises under defaults to "{OsServices.getlogin()}"')

args = parsed_args.parse_args()

client_args = {'host' : args.host, 'port':args.port, 'pmanport': PORTMAN_PORT}
print(client_args)
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
    common_treevariable_model.load(client)
    var_view = TreeVariableView()
    var_controller = TreeVariableController(var_view, client)
    tabs.addTab(var_view, 'Variables')
condition_view = gates.Gates()
condition_controller = gates.Controller(condition_view, client)
tabs.addTab(condition_view, 'Gates')

main.setCentralWidget(tabs)

# 

setup_menubar(main, client)

main.show()
app.exec()
 
