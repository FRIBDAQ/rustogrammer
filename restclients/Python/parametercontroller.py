''' This file implements the controller for the parameter editor.
   it is connected to the various signals in the editor to 
   provide so-called business logic for the editor, that connects it to 
   actions requested of the server.
'''
import ParameterChooser
import parameditor
import rustogramer_client

from PyQt5.QtWidgets import (QApplication, QMainWindow)
class ParameterController:
    ''' Slots:
        * new_row  - add the current parameter as a new row.
        * replace_row - Replace the 'current' row with the parameter.
        * load     - Load selected rows with updated server info.
        * set_params - set selected rows as new parameter metadata.
        * change_spectra - Change relevant spectra so that new axes
                      definitions from parameter metadata will be used.
    '''
    def __init__(self, view, client):

        #  Save the view and client to support signal handling.
        #

        self._view = view
        self._client = client

        #  Connect to the view's signals:

        view.newRow.connect(self.new_row)
        view.replaceRow.connect(self.replace_row)
        view.loadclicked.connect(self.load)
        view.setclicked.connect(self.set_params)
        view.changeclicked.connect(self.change_spectra)
    
    # slots:

    def new_row(self):
        info = self._get_parameter_metadata()
        
        if info is not None:
            self._view.table().add_row(
                info['name'], info['low'], info['hi'], info['bins'],
                info['units']
            )
            
    def replace_row(self):
        row = self._view.table().currentRow()
        if row == -1:
            return               # No current row.
        info = self._get_parameter_metadata()
        self._view.table().set_row(
            row,
            info['name'], info['low'], info['hi'], info['bins'],
            info['units']
        )
    def load(self):
        rows = self._view.table().selected_rows()
        for row in rows:
            name = self._view.table().get_row(row)['name']
            info = self._get_metadata(name)
            if info is not None:
                self._view.table().set_row(
                    row,
                    info['name'], info['low'], info['hi'], info['bins'],
                    info['units']
            )

    def set_params(self):
        pass
    def change_spectra(self):
        pass

    # utilities:

    def _get_parameter_metadata(self):
        # We don't worry about multiple matches but we do worry
        # about no matches - returning None.
        name = self._view.parameter()
        
        if (name == '') or name.isspace():
            return None
        return self._get_metadata(name)
        
    def _get_metadata(self, name):
        reply = self._client.parameter_list(name)
        if len(reply['detail']) == 0:
            return None
        return reply['detail'][0]



#-------------------- Testing -----------------------------

if __name__ == '__main__':
    client = rustogramer_client.rustogramer({'host': 'localhost', 'port':8000})

    app = QApplication([])
    win = QMainWindow()

    ParameterChooser.update_model(client)

    view = parameditor.ParameterEditor()
    win.setCentralWidget(view)
    controller = ParameterController(view, client)
    win.show()
    app.exec()

