''' 
This model contains a class that acts as the controller for the treevariable view.
'''
from treevariable import (common_treevariable_model)


class TreeVariableController:
    ''' 
        This is a controller for the TreeVariableView.  It connects to all of
        the signals the view emits and interacts with both the view and the
        model (the server program) to perform the actions requested by the
        view.
        
        This class has no public methods.  Once the event loop is running,
        it is fully autonomous.
    '''
    def __init__(self, view, client):
        '''
           view - the TreeVariableView object we're the controller form.
           client - The client object (rustogramer_client.rustogramer) that we use
              to communicate with the model (server).'
            
        '''
        self._view = view
        self._client = client
        
        # Let's also cache the view's components to save some time:
        
        self._selector = view.selector()
        self._table    = view.table()
        
        view.append.connect(self._append)
        view.replace.connect(self._replace)
        view.remove.connect(self._remove)
        view.load.connect(self._load)
        view.set.connect(self._set)
    
    def _append(self):
        pass
    def _replace(self):
        pass
    def _remove(self):
        pass
    def _load(self):
        pass
    def _set(self):
        pass

#----------------------------------------------------------------------------
# Test code:

if __name__ == '__main__':
    from rustogramer_client import rustogramer as rcl
    from PyQt5.QtWidgets import (QApplication, QMainWindow)
    from treevariable import TreeVariableView
    
    client = rcl({'host': 'localhost', 'port':8000})
    common_treevariable_model.load(client)
    
    app = QApplication([])
    win = QMainWindow()
    
    wid = TreeVariableView()
    ctl = TreeVariableController(wid, client)
    
    win.setCentralWidget(wid)
    win.show()
    app.exec()
    
    