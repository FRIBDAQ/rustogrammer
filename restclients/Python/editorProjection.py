'''  
    This module will provides a projection definition editor.
    Projection spectra take an existing 2d spectrum and create a
    projection of the spectrum onto a specific axis.  Optionally,
    the projection can be:
    *  A snapthot, in which case it will not be incremented in future events.
    *  Within a named contour in which case the resulting spectrum will only be
       composed of counts within the contour on the parent spectrum and, if not a
       snapshot only increment when the contour gate is true.
    Therefore the editor will look something like:

    +-----------------------------------------------+
    |  Name [ Line edit                           ] |
    |    Project:            [ ] snapshot           |
    | +-------------------+  [ ] in contour         |
    | |  2d spectrum list |  +--------------------+ |
    | +-------------------+  |  contour list      | |
    |                        +-------------------|  |
    |                [  Create/replace ]            |
    +-----------------------------------------------+
'''

from PyQt5.QtWidgets import QLabel

class ProjectionEditor(QLabel):
    def __init__(self, *args):
        super().__init__(*args)
        self.setText('Not Implemented yet')
