''' Implements a spectrum editor.
   The spectrum editor is a tabbed widget that contains, on its tabs
   the editors for spectrum types supported by the server program.
   Next to that is a selector for the channel type of spectra that will be
   created (this is independent of the spectrum type).
   
   Here's an example of the layout:

   +-----------------------------------------+
   | | + 1d + + 2d +         |  Channel type |
   | |    ...                |  [combobox]   |
   |
   +-----------------------------------------+

   Note that we can use the fact that classes are first class objects
   when making this UI.
   '''
from capabilities import (
    SpectrumTypes, ChannelTypes, 
    set_client as set_capabilities_client, get_supported_spectrumTypes,
    get_client as get_capabilities_client,
    get_supported_channelTypes
)

from PyQt5.QtWidgets import (
    QTabWidget, QWidget, QHBoxLayout, QVBoxLayout, QApplication, QLabel,
    QMainWindow, QMessageBox, QPushButton, QComboBox
)
from PyQt5.QtCore import *
from rustogramer_client import rustogramer as Client, RustogramerException

import editor1d, editortwod, editorBitmask
import  editorG2d, editorGD, editorProjection, editorStripchart
import editorSummary, EnumeratedTypeSelector, editorGSummary
from direction import Direction

#------------------------- Spectrum controllers ----------------------
# Slots assume that capabilities.get_client won't return None.

# Utilities.

def default(value, default=0):
    if value is None:
        value = default
    return value
def confirm(question, parent=None):
    dlg = QMessageBox(QMessageBox.Warning, 'Confirm?', 
                    question,
                    QMessageBox.Yes | QMessageBox.No, parent
                )
    dlg = dlg.exec()
    return dlg == QMessageBox.Yes

def error(msg):
    dlg = QMessageBox(QMessageBox.Critical, 'Error:', msg, QMessageBox.Ok)
    dlg.exec()

def ok_to_create(client, editor, name):
    info = client.spectrum_list(name)
    if len(info['detail']) > 0:
        if confirm(f'Spectrum {name} exists, replace?'):
            try :
                client.spectrum_delete(name)
            except RustogramerException as e:
                error(f'Unable to delete {name} before replacing it: {e}')
                return False
            editor.spectrum_removed(name)                
            return True
        else:
            return False
    else:
        return True

#  Base class for controllers:  Supplies a visibility slot that
#  can be overidden.
class AbstractController:
    def __init__(self):
        pass
    def visible(self):
        ''' 
           This is called when the editor associated with the controller
           becomes visible.  Editors are normally in a tabbed widget which
           means only one is visible at a time.  Whe one becomes visible,
           this allows action to be taken.  For example, the projection
           controller can update its editor's list of projectable spectra
           and contours.
             Controllers that need this just override this method.
        '''
        pass
# NoneController - for unimplemented creations:
class NoneController(AbstractController):
    def __init__(self, editor, view):
        pass
###
#   Controller that handles the Oned editor view signals:
class OneDController(AbstractController):
    def __init__(self, editor, view):
        self._editor = editor
        self._view = view
        view.commit.connect(self.create)
        view.parameterSelected.connect(self.load_param)
    
    def create(self):
        client = get_capabilities_client()
        sname = self._view.name()
        param = self._view.parameter()
        data_type = self._editor.channeltype_string()

        # Preconditions to making a spectrum; there must be a spectrum and parameter
        # name.
        if sname is not None and len(sname) > 0 and param is not None and len(param) > 0:
            if not self._view.array():
                if not ok_to_create(client, self._editor, sname):
                    return
                low   = self._view.low()
                high  = self._view.high()
                bins  = self._view.bins()
                try:
                    client.spectrum_create1d(sname, param, low, high, bins, data_type)
                except RustogramerException as e:
                    error(f'{e} while creating spectrum')
                    return
                try: 
                    client.sbind_spectra([sname])
                except RustogramerException as e:
                    error(
                        f'{e} while binding spectrum to shared memory {sname} is defined but not displayable'
                    )
                self._view.setName('')
                self._editor.spectrum_added(sname)
            else:
                self._make_spectrum_array(client, sname, param)

    def load_param(self, parameter_name):
        client = get_capabilities_client()
        current_name = self._view.name()
        if current_name is None or len(current_name) == 0:
            self._view.setName(parameter_name)
        # Regardless if the parameter has metadata load that into the axis definition:

        param_info = client.parameter_list(parameter_name)['detail'][0]
        self._view.setLow(default(param_info['low'], 0))
        self._view.setHigh(default(param_info['hi'], 512.0),)  # like tree params.
        self._view.setBins(default(param_info['bins'], 512))

     # Internal methods:

    def _gen_name(self, sname, pname):
        spath = sname.split('.')
        if len(spath) > 1:
            spath = spath[0:-1]
        ppath = pname.split('.')
        spath.append(ppath[-1])
        return '.'.join(spath)
    
    #  If any of the spectra are defined, prompt to proceed or not with their
    #  replacement:
    #   - Assume there's at least one name
    #   - Assume all names can be generated by replacing the last path element with *
    #
    def _proceed(self, client, names) :
        
        template_name = names[0]  #assume there's at least one
        pattern = template_name.split('.')[0:-1]
        pattern.append('*')
        pattern = '.'.join(pattern)

        defs = client.spectrum_list(pattern)['detail']
        existing_names = [x['name'] for x in defs]

        duplicate_names = [x for x in names if x in existing_names]
        if len(duplicate_names) > 0 :
            c = confirm(f'These spectra already exist {duplicate_names} continuing will replace them, do you want to continue?', self._view)
            if c:
                for s in duplicate_names:
                    client.spectrum_delete(s)    # Delete the dups so we can replace.
                    self._editor.spectrum_removed(s)
            return c
        else:
            return True                       # no confirmations needed.
    def _make_spectrum_array(self, client, sname, param):

        #  Get the list of parameters with params base:

        param_base = '.'.join(param.split('.')[0:-1])
        param_pattern = param_base + '.*'
        parameters    = self._param_names(client, param_pattern)
        data_type = self._editor.channeltype_string()

        # Generate the spectrum names:

        spectrum_names = [self._gen_name(sname, x) for x in parameters]
        if self._proceed(client, spectrum_names):
            low = self._view.low()
            high = self._view.high()
            bins = self._view.bins()

            for sname, pname in  zip(spectrum_names, parameters):
                try:
                    client.spectrum_create1d(sname, pname, low, high, bins, data_type)
                except RustogramerException as e:
                    error(f"Failed to create {sname}; {e} won't try to make any more")
                    return
                self._editor.spectrum_added(sname)
                
            try:
                client.sbind_spectra(spectrum_names)
            except RustogramerException as e:
                error(f"Failed to bind all spectram: {e} some may not be displayable")                
            self._view.setName('')

    def _param_names(self, client, pattern):
        
        defs = client.parameter_list(pattern)['detail']
        result =  [x['name'] for x in defs]
        result.sort()
        return result
##
#  Controller for the 2-d editor.
#  This is much simpler than the 1d editor since we don't have to handle
#  arrays.

class TwodController(AbstractController):
    def __init__(self, editor, view):
        self._client = get_capabilities_client()
        self._editor = editor
        self._view = view

        view.commit.connect(self.create)
        view.xparameterSelected.connect(self.load_xaxis)
        view.yparameterSelected.connect(self.load_yaxis)
    
    # SLots:

    def create(self):
        # Fetch the spectrum definition from the editor view:
        name = self._view.name()
        
        xparam = self._view.xparameter()
        xlow = self._view.xlow()
        xhigh = self._view.xhigh()
        xbins = self._view.xbins()

        yparam = self._view.yparameter()
        ylow = self._view.ylow()
        yhigh = self._view.yhigh()
        ybins = self._view.ybins()

        chantype = self._editor.channeltype_string()

        # Require a nonempty name and parameters:

        if len(name) > 0 and len(xparam) > 0 and len(yparam) > 0:
            #  Get confirmation if the spectrum exists.

            if not ok_to_create(self._client, self._editor, name):
                return
            try:
                self._client.spectrum_create2d(
                    name, xparam, yparam, 
                    xlow, xhigh, xbins, ylow, yhigh, ybins,
                    chantype
                )
            except RustogramerException as e:
                error(f'Failed to create {name} : {e}')
                return
            self._editor.spectrum_added(name)
            self._view.setName('')

            try:
                self._client.sbind_spectra([name])
            except RustogramerException as e:
                error(f'Failed to bind {name} - but spectrum was created: {e}')
    
    def load_xaxis(self, pname):
        param_info = self._client.parameter_list(pname)['detail'][0]
        self._view.setXLow(default(param_info['low'], 0))
        self._view.setXHigh(default(param_info['hi'], 512.0),)  # like tree params.
        self._view.setXBins(default(param_info['bins'], 512))
    def load_yaxis(self, pname):
        param_info = self._client.parameter_list(pname)['detail'][0]
        self._view.setYLow(default(param_info['low'], 0))
        self._view.setYHigh(default(param_info['hi'], 512.0),)  # like tree params.

        self._view.setYBins(default(param_info['bins'], 512))

##
#  Controller for summary spectra.
#
class SummaryController(AbstractController):
    def __init__(self, editor, view):
        self._client = get_capabilities_client()
        self._editor = editor
        self._view = view

        # Connect the signals to our handlers.

        self._view.parameter_changed.connect(self.select_param)
        self._view.add.connect(self.add_params)
        self._view.commit.connect(self.create_spectrum)
    
    #  Called to create a spectrum from the current definition.
    #  note that the name must be non-empty else we do nothing
    #  After successful completion, we prepare the UI for the next
    #  definition:
    #    - Clear the parameter box.
    #    - Clear the spectrum name.
    def create_spectrum(self):

        # Pull the definitions:
        name = self._view.name()
        params = self._view.axis_parameters()
        low = self._view.low()
        high = self._view.high()
        bins = self._view.bins()
        chantype = self._editor.channeltype_string()

        if len(name) > 0:
            # Are we replacing:

            if not ok_to_create(self._client, self._editor, name):
                return
                
            # If we get here we're ready to create the new spectrum:

            try:
                self.create_actual_spectrum(name, params, low, high, bins, chantype)
            except RustogramerException as e:
                error(f'Unable to create {name}: {e}')
                return
            self._view.setName('')
            self._view.setAxis_parameters([])
            try:
                self._client.sbind_spectra([name])
            except RustogramerException as e:
                error(f'Unable to bind {name} to display memory but it has been created.')

    # Support subclassing with different spectrum type:
    def create_actual_spectrum(self, name, params, low, high, bins, chantype):
        self._client.spectrum_createsummary(name, params, low, high, bins, chantype)
        self._editor.spectrum_added(name)
    
    def client(self):
        return self._client
    def view(self):
        return self._view

    # If a parameter is selected:
    #    put it's full name into the parameter text:

    def select_param(self, path):
        name = '.'.join(path)
        self._view.setSelected_parameter(name)

    #  Called when the arrow key to put a parameter into the param list
    #  is clicked.  
    #   - If array is checked we need to get the name of the parameters
    #     given the one in the param chooser
    #   - If the axis should be loaded from parameter metadata we load
    #
    def add_params(self):
        name = self._view.selected_parameter()
        if name == '':
            return
        
        # Note _parameter_list takes care of loading the axis definition
        # if desired and available.

        names = self._parameter_list(name)
        names.sort()
        full_list = self._view.axis_parameters() + names
        self._view.setAxis_parameters(full_list)

    # Private utilities.abs

    # _parameter_list - create a list of parameters to add to the list
    # box and, if requested, update the axis definitions from parameter metadat
    #
    def _parameter_list(self, base):
        pattern = base

        if self._view.array():
            pattern = base.split('.')[0:-1]
            pattern.append('*')
            pattern = '.'.join(pattern)
        
        #  Get the parameter definiions and:
        #  extract the names into a list and, if axis_from_parameters is
        #  in fill in the axis  values when we have a parameter with metadata.

        descriptions = self._client.parameter_list(pattern)['detail']
        names = [x['name'] for x in descriptions]  # list of matching names

        if self._view.axis_from_parameters():
            for p in descriptions:
                self.setaxis_from_parameter(p)
                

        return names
    def setaxis_from_parameter(self, p):
        if p['low'] is not None:
            self._view.setLow(p['low'])
        if p['hi'] is not None:
            self._view.setHigh(p['hi'])
        if p['bins'] is not None:
            self._view.setBins(p['bins'])


##  Gamma 1d is just like a summary spectrum but makes a different specturm type:

class G1DController(SummaryController):
    def __init__(self, editor, view):
        super().__init__(editor, view)
    def create_actual_spectrum(self, name, params, low, high, bins , chantype):
        client = self.client()
        client.spectrum_createg1(name, params, low, high, bins , chantype)
        self._editor.spectrum_added(name)

## Gamma 2d is just Summary controller with overrides for both
#  create_actual_spectrum and setaxis_from_parameter
#

class G2DController(SummaryController):
    def __init__(self, editor, view):
        super().__init__(editor, view)

    def create_actual_spectrum(self, name, params, low, high, bins, chantype):
        # low, high, bins are for the yaxis:

        view = self.view()
        xlow = view.xlow()
        xhigh = view.xhigh()
        xbins = view.xbins()

        self.client().spectrum_createg2(
            name, params, xlow, xhigh, xbins, low, high, bins, chantype
        )
        self._editor.spectrum_added(name)

    def setaxis_from_parameter(self, p):
        view = self.view()
        if p['low'] is not None:
            view.setXlow(p['low'])
            view.setYlow(p['low'])
        if p['hi'] is not None:
            view.setXhigh(p['hi'])
            view.setYhigh(p['hi'])
        if p['bins'] is not None:
            view.setXbins(p['bins'])
            view.setYbins(p['bins'])

#
#   Controller to build particle gamma spectra (GD).
#
class PGammaController(AbstractController):
    def __init__(self, editor, view):
        self._editor = editor
        self._view   = view
        self._client = get_capabilities_client()
        self._view.addXParameters.connect(self.addx)
        self._view.addYParameters.connect(self.addy)
        self._view.parameterChanged.connect(self.set_param_name)
        self._view.commit.connect(self.commit)
    def addx(self):
        params = self._get_parameters()
        if self._view.axis_from_parameters():
            self._set_axis_defs(params)
        names = [x['name'] for x in params]
        names.sort()
        for name in names:
            self._view.addXparameter(name)
    def addy(self):
        params = self._get_parameters()
        if self._view.axis_from_parameters():
            self._set_axis_defs(params)
        names = [x['name'] for x in params]
        names.sort()
        for name in names:
            self._view.addYparameter(name)
    def commit(self):
        name = self._view.name()
        if name == '':
            return

        #  If there's already a spectrum of this name ensure we can replace:

        if self._create_ok(name):
            xparams = self._view.xparameters()
            yparams = self._view.yparameters()
            xlow    = self._view.xlow()
            xhigh   = self._view.xhigh()
            xbins   = self._view.xbins()
            ylow    = self._view.ylow()
            yhigh   = self._view.yhigh()
            ybins   = self._view.ybins()
            dtype   = self._editor.channeltype_string()

            # Try to create the spectrum:

            try:
                self.create_actual_spectrum(
                    name, xparams, yparams, xlow, xhigh, xbins, ylow, yhigh, ybins, dtype
                )
            except RustogramerException as e:
                error(f'Failed to create {name}: {e}')
                return

            self._view.setName('')
            self._view.setXparameters([])   # Clear the editor for next time.
            self._view.setYparameters([])

            # Try to bind it to display memory:

            try:
                self._client.sbind_list([name])
            except RustogramerException as e:
                error(f'Failed to bind {name} to shared memory: {e}, Spectrum was made, however')

    def create_actual_spectrum(self, name, xparams, yparams, xlow, xhigh, xbins, ylow, yhigh, ybins, dtype):
        self._client.spectrum_creategd(
                    name, xparams, yparams, xlow, xhigh, xbins, ylow, yhigh, ybins, dtype
                )
        self._editor.spectrum_added(name)
    def set_param_name(self, path):
        name = '.'.join(path)
        self._view.setSelectedParameter(name)
    # Utility methods

    def _create_ok(self, name):
        # Returns true if it's ok to make the spectrum.
        # If the spectrum exists, we require the user to confirm the
        # replacement and delete the spectrum.

        return  ok_to_create(self._client, self._editor, name)
        

    def _get_parameters(self):
        #Get the defs of the parameters to add:

        name = self._view.selectedParameter()
        if self._view.array():
            params = self._make_parameter_list(name)
        else:
            params = self._client.parameter_list(name)['detail']
        return params

    def _make_parameter_list(self, sample):
        # Given a sample parameter return the list of paramater descriptions
        # that are in the array sample is in:

        # Create the listing search path:

        path = sample.split('.')
        path = path[0:-1]
        path = '.'.join(path)
        pattern = path + '.*'
        
        #  get the matching parameter descriptions:

        descriptions = self._client.parameter_list(pattern)['detail']
        return descriptions

    def _set_axis_defs(self, parameters):
        #  Given a set of parameter descriptions, set the axis definitions
        #  from them.

        for param in parameters:
            low = param['low']
            high= param['hi']
            bins= param ['bins']
            if low is not None:
                self._view.setXlow(low)
                self._view.setYlow(low)
            if high is not None:
                self._view.setXhigh(high)
                self._view.setYhigh(high)
            if bins is not None:
                self._view.setXbins(bins)
                self._view.setYbins(bins)
                


# Making a 2d sum is like making a gamma deluxe .. we'll let the
# server enforce that the number of x/y params must be the same:

class TwoDSumController(PGammaController):
    def __init__(self, editor, view):
        super().__init__(editor, view)
    def create_actual_spectrum(self, name, xparams, yparams, xlow, xhigh, xbins, ylow, yhigh, ybins, dtype):
        self._client.spectrum_create2dsum(
            name, xparams, yparams, xlow, xhigh, xbins, ylow, yhigh,ybins, dtype
        )
        self._editor.spectrum_added(name)
#  Controller for spectrum projections:

class ProjectionController(AbstractController):
    def __init__(self, editor, view):
        self._editor = editor
        self._view   = view
        self._client = get_capabilities_client()

        # we have to load up the view with the current spectrum list
        # _and_ since we don't get a signal for the intial loading
        # of that list we need to load contours for the contours visibl
        # on the first of those spectra:

        self._loadspectra()
        self._loadContours(self._view.spectrum())

        # Connect to singal handlers

        self._view.spectrumChosen.connect(self._loadContours)
        self._view.commit.connect(self._create)
    
    # slot overrides:

    def visible(self):
        self._loadspectra()
        self._loadContours(self._view.spectrum())
    #  Create the spectrum:

    def _create(self):
        # If there's no proposed name give up:

        name = self._view.name()
        if name.isspace():
            return
        source = self._view.spectrum()
        snap   = self._view.snapshot()
        incontour = self._view.contour()
        if incontour:
            contour_name = self._view.contour_name()
        else:
            contour_name = None
        direction = self._view.direction()
        if direction == Direction.X.value:
            direction_str ='x'
        else:
            direction_str = 'y'

        #  IF name is an existing spectrum we need permission
        # to overwrite it:

        if not ok_to_create(self._client, self._editor, name):
            returns
        
        # Now we can get on with making the spectrum and
        # binding it into display memory.

        try:
            self._client.project(
                source, name, direction_str, snap, contour_name
            )
        except RustogramerException as e:
            error(f'Could not create {name} projection of {source}: {e}')
            return
        # Got made so now try to sbind it:

        self._editor.spectrum_added(name)
        try:
            self._client.sbind_list([name])
        except RustogramerException as e:
            error(f'Could not bind {name} to display memory but it it has been created: {e}')


    #  Utilties:

    def _loadspectra(self):
        all_spectra = self._client.spectrum_list()['detail']
        twod_spectra = [x['name'] for x in all_spectra if self._isprojectable(x)]
        twod_spectra.sort()
        self._view.setSpectra(twod_spectra)
        pass

    def _loadContours(self, spectrum_name):
        spectrum_def = self._client.spectrum_list(spectrum_name)['detail']
        if len(spectrum_def) > 0:
            spectrum_def = spectrum_def[0]
            all_conditions = self._client.condition_list()['detail']
            displayable_contours = [x['name'] for x in all_conditions  \
                if self._is_displayable_contour(spectrum_def, x)]
            self._view.setContours(displayable_contours)

    def _isprojectable(self, spectrum):
        # True if the definition in spectrum is a 2d (projectable).
        # since g2d is 2d as are 2dsum and p-gamma a spectrum is 2d
        # if it is not a summary and has both axis definitions.
        return ((spectrum['type'] != 's') and (spectrum['type'] != 'gd') and
            (spectrum['xaxis'] is not None) and 
            (spectrum['yaxis'] is not None))
    
    def _is_displayable_contour(self, spectrum_def, condition):
        #   Return true if the codition is
        #   1. A contour or multi contour ('c' or 'gc')
        #   2. Its x and y parameters are all present on the spectrum_def

        xpars = spectrum_def['xparameters']
        ypars = spectrum_def['yparameters']

        gate_params = condition['parameters']

        # Interpretation  of gate_params depends on the condition type.

        if condition['type'] == 'c':
            return gate_params[0] in xpars and gate_params[1] in ypars
        elif condition['type'] == 'gc':
            for p in gate_params:
                if p not in xpars and p not in ypars:
                    return False
            return True
        else:
            return False


#   Controller to handle stript chart spectra.

class StripChartController(AbstractController):
    def __init__(self, editor, view):
        self._editor = editor
        self._view = view
        self._client = get_capabilities_client()

        self._view.commit.connect(self._commit)

    def _commit(self):
        # Get the information we need.. confirm we're a go:

        name = self._view.name()
        if name.isspace():
            return
        if ok_to_create(self._client, self._editor, name):
            tparam = self._view.xparam()
            vparam = self._view.yparam()
            low = self._view.low()
            high = self._view.high()
            bins = self._view.bins()

            try:
                self._client.spectrum_createstripchart(
                    name, tparam, vparam, low, high, bins, 
                    self._editor.channeltype_string()
                )
                self._editor.spectrum_added(name)
            except RustogramerException as e:
                error(f"Unable to create: {name} : {e}")
                return
            try:
                self._client.sbind_list([name])
            except RustogramerException as e:
                error(f'Unable to bind {name} to display memory, though it was created: {e}')

# Controller for bitmask spectra:
class BitMaskController(AbstractController):
    def __init__(self, editor, view):
        self._editor = editor
        self._view = view
        self._client = get_capabilities_client()

        self._view.commit.connect(self._commit)
    def _commit(self):
        name = self._view.name()
        if name.isspace():
            return
        if ok_to_create(self._client, self._editor, name):
            try:
                self._client.spectrum_createbitmask(
                    name, 
                    self._view.parameter(), self._view.bits(), 
                    self._editor.channeltype_string()
                )
                self._editor.spectrum_added(name)
            except RustogramerException as e:
                error(f'Unable to create {name}: {e}')
                return
            try :
                self._client.sbind_list([name])
            except RustogramerException as e:
                error('Unable to bind {name} to spectrum memory but it has been created: {e}')
                
#  This dict is a table, indexed by tab name, of the class objects
#  that edit that spectrum type and the enumerator type in capabilities.
#  e.g. '1D': (SpectrumTypes.Oned, editor1d.onedEditor, onedcontroller) - means
#  The tab labeled 1D will be added if the SpectrumTypes.Oned is supported by
#  the server and will contain an editor1d.onedEditor and that onedcontroller
#  will be instantiated to handle signals from the editor.
#
#  In the future, the classes may be self contained MVC bundles so we don't
#  have to concern ourselves with connecting slots etc.
_spectrum_widgets = {
    '1D': (SpectrumTypes.Oned, editor1d.oneDEditor, OneDController),
    '2D': (SpectrumTypes.Twod, editortwod.TwoDEditor, TwodController),
    'Summary': (SpectrumTypes.Summary, editorSummary.SummaryEditor, SummaryController),
    'Gamma 1D' : (SpectrumTypes.Gamma1D, editorSummary.SummaryEditor,G1DController),
    'Gamma 2D' : (SpectrumTypes.Gamma2D, editorG2d.Gamma2DEditor, G2DController),
    'P-Gamma'  : (SpectrumTypes.GammaDeluxe, editorGD.GammaDeluxeEditor, PGammaController),
    '2D Sum'   : (SpectrumTypes.TwodSum, editorGD.GammaDeluxeEditor, TwoDSumController),
    'Projection' : (SpectrumTypes.Projection, editorProjection.ProjectionEditor, ProjectionController),
    'StripChart' : (SpectrumTypes.StripChart, editorStripchart.StripChartEditor, StripChartController),
    'Bitmask' : (SpectrumTypes.Bitmask, editorBitmask.BitmaskEditor, BitMaskController),
    'Gamma summary' : (SpectrumTypes.GammaSummary, editorGSummary.GammaSummaryEditor, NoneController)

}

#  This dict has channel type names as keys and channel type values as values:

_channel_types = {
    'f64': ChannelTypes.Double,
    'long': ChannelTypes.Long,
    'word': ChannelTypes.Short,
    'byte' : ChannelTypes.Byte
}
#   This class assumes that the capabilities client has already been set:
class Editor(QWidget):
    new_spectrum = pyqtSignal(str)
    spectrum_deleted = pyqtSignal(str)
    clear_selected = pyqtSignal()
    clear_all      = pyqtSignal()
    delete_selected = pyqtSignal()
    gate_selected   = pyqtSignal()
    ungate_selected = pyqtSignal()
    def __init__(self, *args):
        global _spectrum_widgets
        global _channel_types

        super().__init__(*args)

        # We use a hbox layout:

        layout = QHBoxLayout()

        #At the left is a tabbed widget:

        self.tabs = QTabWidget(self)
        self.tabs.setUsesScrollButtons(True)
        self.editors = dict()
        self.controllers = dict()
        # Stock it with the supported spectrum editors:

        supported_specs = get_supported_spectrumTypes()
        for label in _spectrum_widgets.keys():
            info = _spectrum_widgets[label]
            if info[0] in supported_specs:
                self.editors[label] = info[1](self)  # So we can get this in the editors.
                self.tabs.addTab(self.editors[label], label)
                self.controllers[label] = info[2](self, self.editors[label]) # hook in controller.
        

        self.channelType = EnumeratedTypeSelector.TypeSelector()
        supported_ctypes = get_supported_channelTypes()

        for label in _channel_types.keys():
            t = _channel_types[label]
            if t in supported_ctypes:
                self.channelType.addItem(label, t)

        layout.addWidget(self.tabs)
        right = QVBoxLayout()
        self._clear = QPushButton('Clear', self)
        right.addWidget(self._clear)
        self._clearall= QPushButton('Clear all', self)
        right.addWidget(self._clearall)
        self._load = QPushButton('Copy', self)
        right.addWidget(self._load)
        self._del = QPushButton("Delete", self)
        right.addWidget(self._del)
        self._gateselection = QComboBox( self)
        right.addWidget(self._gateselection)
        self._gate = QPushButton('Gate');
        right.addWidget(self._gate)
        self._ungate = QPushButton('Ungate')
        right.addWidget(self._ungate)
        right.addWidget(QLabel('Channel Type:'))
        right.addWidget(self.channelType)
        layout.addLayout(right)
        self.setLayout(layout)
        
        self._clear.clicked.connect(self.clear_selected)
        self._clearall.clicked.connect(self.clear_all)
        self._del.clicked.connect(self.delete_selected)
        self._gate.clicked.connect(self.gate_selected)
        self._ungate.clicked.connect(self.ungate_selected)

        self.tabs.currentChanged.connect(self._new_editor_visible)

    def _new_editor_visible(self, index):
        
        controller = self.controllers[self.tabs.tabText(index)]
        controller.visible()
       
    # Get the currently selected channel type string
    
    def channeltype_string(self):
       return self.channelType.selectedText()

    def load_gates(self, client):
        #  Load gates into self._gateselection
        while self._gateselection.count() > 0:
            self._gateselection.removeItem(0)

        condition_names = [x['name'] for x in client.condition_list()['detail']]
        condition_names.sort()    # Alpha so easy to find.
        self._gateselection.addItems(condition_names)

    def selected_gate(self):
        return self._gateselection.currentText()

    # Slot that can be called when a controller makes a new spectrum:

    
    def spectrum_added(self, name):
        self.new_spectrum.emit(name)

    # Slot to call when a spectrum was deleted.
    def spectrum_removed(self, name):
        self.spectrum_deleted.emit(name)



    
    


# --- tests

def test(host, port):
    c = Client({'host': host, 'port': port})
    set_capabilities_client(c)

    app = QApplication([])
    c = QMainWindow()

    w = Editor(c)
    c.setCentralWidget(w)
    c.adjustSize()

    c.show()
    app.exec()

