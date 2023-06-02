""" This module provides a client interface to rustogramer

The way to use this module is to instantiate a rustogramer 
object and then invoke methods on th at object to communicate
with a running rustogramer program.  
"""

import requests
import PortManager
import os

class RustogramerException(Exception):
    """Exception type raised if the server replies with an error JSON
    
        Attributes:

        *   status - the status field of the response.
        *   detail - the detail field of the response.
    """

    def __init__(self, response):
        self.status = response["status"]
        self.detail = response["detail"]

class rustogramer:
    """
       The rustogramer class is the client side object for Rustogramer

        Methods of the rustogramer class communicate with the server
        via the REST interface the server exports. 
    """

    def _service_port(self, host, port, name):
        #  Translate the service 'name' using the port manager on
        #  'port'  to a service port, returning the port.

        pm = PortManager.PortManager(host, port)
        matches = pm.find(service=name, user=os.getlogin())
        if len(matches) != 1:
            raise NameError(name=name)
        return matches[0]["port"]

    def _transaction(self, request, queryparams = {}):
        # perform a transaction returning the JSON on success.
        # On failures an exception is raised.
        
        # Create the URI:

        uri = "http://" + self.host + ":" + str(self.port) + "/spectcl/" + request
        response = requests.get(uri, params=queryparams)
        response.raise_for_status()     # Report response errors.and
        result = response.json()
        if result["status"] != "OK":
            raise RustogramerException(result)
        return result

    def _marshall(self, iterable, key):
        return [x[key] for x in iterable]

    def _format_axis(self, low, high, bins):
        return "{low:f} {high:f} {bins:d}".format(low=low, high=high, bins=bins)
    
    def _format_xyaxes(self, xlow, xhigh, xbins, ylow, yhigh, ybins):
        x = self._format_axis(xlow, xhigh, xbins)
        y = self._format_axis(ylow, yhigh, ybins)
        return "{" + x + "} {" + y + "}"

    def _format_stringlist(self, strings):
        result=""
        for s in strings:
            result = result + s + ' '
        return result.rstrip(' ')

    def __init__(self, connection):
        """ 
        Create a new rustogramer client object.

        The connection parameter describes how to do the connection.
        It is a dict which has two mandatory members and one 
        optiona member:

        *   'port' (mandatory) - This is either the port on
        which the rustogramer listens for connections or the port on which
        the NSCLDAQ port manager listens for connections.  See below.
        *   'host' (mandatory) - Host running the rustogramer.
        *   'service'  (optional) - If provided, the port key provides
        the port manager listener port and this parameter is the service name
        the rustogramer is advrtising for the current user.  This is translated
        to a port once.

        The constructor makes no actual connection to the rustogramer
        REST interface.  This connection by each service request to that
        port.
        """
        self.port = connection["port"]
        self.host = connection["host"]
        if "service" in connection:
            self.port = self._service_port(connection['host'], self.port, connection["service"])

    #--------------- Gate application domains: /apply, /ungate

    def apply_gate(self, gate_name, spectrum_name):
        """ Apply the condition gate_name to spectrum_name """

        response = self._transaction('apply/apply', {"gate":gate_name, "spectrum": spectrum_name})
        return response

    def apply_list(self, pattern="*"):
        """ List gate applications fro spectra that match the optional pattern

        The optional pattern, is a glob pattern defaults to *.  Only applications
        of gates to spectra that match the pattern are shown.
        """
        response = self._transaction('apply/list', {"pattern" : pattern})
        return response
    
    def ungate_spectrum(self, names):
        """ Ungate the named spectrum"""

        response =self._transaction("ungate", {"name": names})
        return response

    #----------------- channel domain - get/fetch channels

    def get_chan(self, name, x, y = 0):
        """ Get the value of the channel at bins x/y 
        
        typical Y is only needed if the spectrum has two dimensions
        """
        return self._transaction(
            "channel/get", {"spectrum": name, "xchannel": x, "ychannel": y}
        )

    def set_chan(self, name, x, value, y=0):
        return self._transaction(
            "/channel/set", 
            {"spectrum": name, "xchannel": x, "ychannel": y, "value": value}

        )
    
    #-------------- Data processing: /attach and /analyze:

    def attach_source(self, type, source, size=8192):
        """ Attach a data source
           *   type - is the type of data source 'pipe' or 'file' Note that
           rustogramer only supports 'file' but SpecTcl supports both.
           *   source  - the type deependent sourc identification.
           *   size    - (relevant only to SpecTcl - read block sizes). 
        """
    
        return self._transaction(
            "attach/attach", {"type": type, "source": source, "size":size}
        )

    def attach_show(self) :
        """ Show what's attached"""
        return self._transaction("attach/list")

    def detach_source(self) :
        """ Detach the data source"""
        return self._transaction("attach/detach")
    
    def start_analysis(self):
        """Start processing data from the attached source"""
        return self._transaction("analyze/start")

    def stop_analysis(self):
        """ Stop processing from the attached source"""
        return self._transaction("analyze/stop")
    
    def set_batch_size(self,num_events):
        """ set the analysis event batch size to num_events"""
        return self._transaction("analyze/size", {"events": num_events})

    # ------------------------------  Event builder unpacker:

    def evbunpack_create(self, name, frequency, basename):
        """ Create an unpacker for event built data:
          *   name - name of the new unpacker.
          *   frequency - common clock frequency of the timestamps.
          *   basename - base name of diagnostic parameters produced.

          Note that rustogramer does not implement this but SpecTcl does.
        """
        return self._transaction(
            "evbunpack/create", 
            {"name": name, "frequency" : frequency, "basename": basename}
        )
    
    def evbunpack_add(self, name, source_id, pipeline_name):
        """ Set the pipeline that processes fragments from a source id:
          *   name of the event builder unpacker being manipulated.
          *   source_id - source id of the fragments that will be processed
          by this pipeline.
          *   pipeline_name - name of an event builder pipeline that will
          be used to process fragments from source_id.  This pipeline
          must have been registered with the pipeline manager (see the
          pman_* methods)

          Note that rustogramer does not implement this however SpecTcl does.
        """
        return self._transaction(
            "evbunpack/add",
            {"name": name, "source": source_id, "pipe": pipeline_name}
        )
    
    def evbunpack_list(self, pattern="*"):
        """ List the eventbuilder unpackers that have been defined.

            * pattern is an optional glob pattern.  Only event builder unpackers
            that match the pattern will be listed.  The pattern, if not supplied,
            defaults to "*" which matches evertying.

            Note rustogramer does not implement this, however SpecTcl does.
        """
        return self._transaction(
            "evbunpack/list", {"pattern": pattern}
        )
    #---------------------  exit:

    def request_exit(self):
        """ Asks the application to exit.
        Rustogramer implements this but not  SpecTcl.
        """
        return self._transaction("exit")
    
    #--------------------   Filters:

    def filter_new(self, name, gate, parameters):
        """ Create a new filter:
        *   name - the name of the new filter.
        *   gate - the gate that determines which events get into the
        filter output file (this can be a True gate).
        *   parameters - the parameters that are included in filtered events
        note that this can be a single string or a list of strings.

        Rustogramer does not implement filters, but SpecTcl does.
        """
        return self._transaction(
            "filter/new",
            {"name": name, "gate": gate, "parameter":parameters}
        )
    def filter_delete(self, name):
        """ Delete an existing filter:

        *  name - the name of the filter to delete.

        Rustogramer does not implement filters but SpecTcl does.
        """
        return self._transaction(
            "filter/delete", {"name": name}
        )
    def filter_enable(self, name):
        """ Enable an existing filter:

        *  name - the name of a filter that must have an associated file.

        rustogramer does not implement filters but SpecTcl does.
        """
        return self._transaction("/filter/enable", {"name":name})

    def filter_disable(self, name):
        """ Disables an enabled filter.

        *  name - the name of a filter that must be enabled.

        rustogramer does not implement filters but SpecTcl does.
        """
        return self._transaction("filter/disable", {"name": name})

    def filter_regate(self, name, gate):
        """ Apply a different gate to an existing filter.   The filter
        must not be enabled as that could dynamically change the meaning
        of its output.

        *   name - name of the filter.
        *   gate - Name of the new gate applied to the filter.

        Rustogramer does not implement filters but SpecTcl does.
        """
        return self._transaction("filter/regate", {"name": name, "gate": gate})
    
    def filter_setfile(self, name, path):
        """ Set the output file for a specific filter.

        *   name - filtername.
        *   path - path to the output file. path is interpreted in the
        context of the server not the client.

        Rustogramer does not implement filters but SpecTcl does.
        """
        return self._transaction("filter/file", {"name": name, "file": path})
    
    #--------------------------- fit API.

    def fit_create(self, name, spectrum, low, high, type) :
        """ Create a new fit object (SpecTcl only):
        * name - name to assign to the fit (must be unique).
        * spectrum - Spectrum on whose channels the fit will be performed.
        * low - low limit of the fitted region.
        * high - high limit of the fitted region.
        * type - type of fit to be performed.

        Note that SpecTcl only supports fits on 1-d spectra.
        """
        return self._transaction(
            "fit/create", 
            {"name": name, "spectrum" : spectrum, "low": low, "high": high, "type": type}
        )
    
    def fit_update(self, pattern = "*"):
        """ (SpecTcl only)
        Update the fits that match the optional pattern parameter.
        pattern is a glob pattern that, if not supplied, defaults to "*"
        which matches all fits.
        """

        return self._transaction("fit/update",{"pattern": pattern})
    
    def fit_delete(self, name):
        """ Deletes the named fit (SpecTcl only)
        """
        return self._transaction("fit/delete", {"name":name})
    
    def fit_list(self, pattern = "*"):
        """SpecTcl only
        lists the fits and their parameterization.
        Only the fits with names matching the optional pattern parameter
        are returned.  If pattern is omitted it defaults to "*"
        """
        return self._transaction("fit/list", {"pattern": pattern})

    def fit_proc(self, name):
        """SpecTcl only

        This  returns the proc used to
        evaluate the fit named.  This requires SpecTcl both because
        only SpecTcl implements fits and because the URI to performa
        an aribtrary Tcl ommand only applies to SpecTcl
        """
        return self._transaction("fit/proc", {"name":name})

    #----------------------- Fold API.

    def fold_apply(self, fold, spectrum):
        """ Apply a fold to a gamma spectrum (SpecTcl only).

        *   fold - name of the fold to apply.
        *   spectrum - name of a gamma spectrum to apply the fold to.

        """
        return self._transaction("fold/apply", {"gate": fold, "spectrum": spectrum})

    def fold_list(self,pattern="*"):
        """ SpecTcl only : Lists the folds whose names match the optional
        pattern parameter.  pattern is a glob parameter that, if omitted,
        defaults to "*"
        """

        return self._transaction("fold/list", {"pattern": pattern})
    
    def fold_remove(self, spectrum):
        """ SpecTcl only - removes any fold applied to the named spectrum.
        """
        self._transaction("fold/remove", {"spectrum": spectrum})
    
    #------------------------- Gate API

    def condition_list(self, pattern="*"):
        """ Returns a list of defined conitions.  Conditions returned must
        have names that match the optional pattern parameter which is a glob pattern.
        If the pattern is omitted, it defaults to "*" which matches all gates.
        """
        return self._transaction("gate/list", {"pattern": pattern})
    
    def condition_delete(self, name) :
        """ Delete the named condition. Note that the semantics of deleting
        a gate in SpecTcl differ from those of rustogramer.  In rustogramer,
        the gate can actually be deleted rather than turning into a false gate.
        The deleted gate is treated as a false condition in compound gates.
        Applied to a spectrum, however, deleting a gate essentially ungates
        the spectrum while in SpecTcl, deleted applied gates prevent a spectrum
        from being incremented.
        """
        return self._transaction("gate/delete", {"name": name})

    # The remainder of the gate API are helpers that invoke
    # the edit REST method but for specific gate types.

    def condition_make_true(self, name):
        """   Create a True gate - that is one that is true for all
        events.  The 'name' parameter is the name of the gate.
        Note that for this method and all other gate makers, if the
        named condition already exists, it is replaced by the new condition definition
        dynamically (that is all spectra gated by the condition are now gated
        by the modified condition.
        """
        return self._transaction("gate/edit", {"name":name, "type":"T"})
    
    def condition_make_false(self, name):
        return self._transaction("gate/edit", {"name": name, "type": "F"})

    def condition_make_not(self, name, negated):
        """ Create a not condition.  This condition is the logical opposite
        of its dependent condition.  That is if an event makes its dependent
        condition true, the not condition will be false.

        *   name - name of the new or modified condition.
        *   negated -name of the gate that will be negated to form the 
        'name' gate.
        """
        return self._transaction("gate/edit", {"name": name, "type":"-", "gate":negated})

    def condition_make_and(self, name, components):
        """ Creates a condition that is true if all of the  component
        conditions are also true:

        *  name - name of the condition.
        *  components - name of the component conditions.
        """
        return self._transaction("gate/edit", {"name":name, "type":"*", "gate": components})

    def condition_make_or(self, name, components):
        """ Same as condition_make_and but the condition is true if _any_ of the
        components is True
        """
        return self._transaction("gate/edit", {"name":name, "type":"+", "gate":components})

    def condition_make_slice(self, name, parameter, low, high):
        """ Create a slice condition.  Slices are a 1-d region of interest
        in a single parameter.  They are evaluated in raw parameter space.

        *   name -name of the condition.
        *   parameter - name of the parameter on which the slice is evaluated.
        *   low, high - the slice is true for events that lie between these limits.
        """
        return self._transaction(
            "gate/edit", 
            {"name":name, "type":"s", "parameter":parameter, "low":low, "high":high}
        )
    def condition_make_contour(self, name, xparameter, yparameter, coords):
        """ Create a contour condition.  Contour conditions are two dimnensional
        closed regions in the space defined by two parameters.  They are true for
        events that have both parameters and for which the point defined by
        the two parameters is 'inside' the contour.  Inside is defined by the
        'odd crossing rule'  That is if you extend a line in any direction from the
        point, it is inside the object if an odd number of object lines are
        crossed.  This supports a consistent definition for extremely pathalogical
        figures.   It is also commonly used to define 'insidedness' for flood fill
        algorithms in graphics so therefore is reasonably intuitive.

        *   name -name of the condition.
        *   xparameter - name of the parameter that is on the x axis of the figure.
        *   yparameter - name of the parameter that is on the y axsis of the figure.
        *   coords - an iterable object  whose members are dicts with the keys
            "x", and "y" which define the x and y coordinates of each  point
            in the condition's contour.
    
        NOTE:  A final segment is 'drawn' between the last and first point to
        close the contour.
        """
        xcoords = self._marshall(coords, "x")
        ycoords = self._marshall(coords, "y")
        return self._transaction(
            "gate/edit", 
            {"name":name, "type":"c",
            "xparameter":xparameter, "yparameter": yparameter, 
            "xcoord": xcoords, "ycoord": ycoords}
        )
    def condition_make_band(self, name, xparameter, yparameter, coords):
        """ Same as for condition_make_contour but the resulting condition
        is a band.   Bands are true of points that are below the open figure.
        Note that sawtooth bands are true for points below the highest 'tooth'.
        """
        xcoords = self._marshall(coords, "x")
        ycoords = self._marshall(coords, "y")
        return self._transaction(
            "gate/edit", 
            {"name":name, "type":"b",
            "xparameter":xparameter, "yparameter": yparameter, 
            "xcoord": xcoords, "ycoord": ycoords}
        )
    #----------------------- Statistics API.

    def get_statistics(self, pattern="*"):
        """ returns the under/overflow statistics of the spectra
        whose name matches the otpional 'pattern' parameter.  If
        omitted, 'pattern' defaults to '*'
        """
        return self._transaction("specstats", {"pattern":pattern})

    #--------------------- Integrate

    def integrate_1d(self, spectrum, low, high):
        """ Integrate a region of interesti n a 1d spectrum.

        *  spectrum - name of the spectrum.
        *  low, high - Define the limits of integration.

        """
        return self._transaction("integrate", {"spectrum":spectrum, "low":low, "high":high})
    
    def integrate_2d(self, spectrum, coords):
        """ Integrate a 2d spectrum within a contour.

        * spectrum name of the spectrum.
        * coords -iterable object containing maps with keys "x" and "y"
        defining the coordinates of the contour within which the integration
        is perfromed.
        """
        xcoords = self._marshall(coords, "x"),
        ycoords = self._marshall(coords, "y")
        return self._transaction(
            "integrate", 
            {"spectrum":spectrum, "xcoords":xcoords, "ycoords": ycoords}
        )
    #--------------- parameter/treeparamter API.

    def parameter_list(self, pattern="*"):
        """ List information about the parameters with names that
        match the glob pattern "pattern" if the pattern parameter is omitted
        it defaults to "*", which matches all names.
        """
        return self._transaction("parameter/list", {"filter":pattern})

    def parameter_version(self):
        """ Requests version information about the tree parameter version
        implemented by the application
        """
        return self._transaction("parameter/version", {})
    
    def parameter_create(self, name, properties):
        """ Creates a new parameter Since so many of the parameter properties
        are optional and can be null, the paramter properties are dict:

        *  name - name of the parameter being created. It is an error to 
        provide the name of an existing parameter.
        *  properties - a dict containing optional properties of the parameters.
        Allowed keys are:
            -  low - suggested low limit of histogram axes on this parameter.
            - high - suggested high limit of histogram eaxes on this parameter.
            - bins - suggested number of bins for an axis on this parameter.
            - units - units of measure for the parameter.
            - description - (ignored by spectcl) - a long form descriptin of the parameter.
        """
        props = properties
        props["name"] = name
        return self._transaction("/parameter/create", props)

    def parameter_modify(self, name, properties):
        """ Modify the metadata associated with a parameter:
        *  name - name of an existing parameter.
        *  properties - dict with the same keys as parameter_create for each
        present key, that property is mdified.
        NOTE:  There is no way to remove metadata.
        """
        props = properties
        props["name"] = name
        return self._transaction("parameter/edit", props)

    def parameter_promote(self, name, properties):
        """ promotes a raw parameter to a tree parameter.
        *  name - name of the parameter.
        *  properties - dict of parameter metadata properties.

        Note: in rustogramer all parameters have metadata.
        """
        props = properties
        props["name"] = name
        return self._transaction("parameter/promote", props)

    def parameter_check(self, name):
        """ Sets the check flag on the named parameter.
        """
        return self._transaction("parameter/check", {"name":name})

    def parameter_uncheck(self, name):
        """Clears the check flag on a the named parameter
        """
        return self._transaction("parameter/uncheck", {"name":name})

    #--------- rawparameter interface.

    def rawparameter_create(self, name, properties):
        """ Create a new raw parameter (this is only different from
        the parameter_create in SpecTcl).

        *  name the name of the new parameter.
        *  properties - dict with optional properties for the paramerter:
            - 'low', 'high', 'bins' - suggested binning and limits.
            - 'units'  units of measure for the parameter.
            - 'description' (rustogramer only) - textual description of the parameter.
        """
        props = properties
        props['name'] = name
        return self._transaction("rawparameter/new", props)

    def rawparameter_list_byname(self, pattern="*"):
        return self._transaction("rawparameter/list", {"pattern": pattern})
    def rawparameter_list_byid(self, id):
        return self._transaction("rawparameter/list", {"id":id})

    #----------------- Ring format:

    def ringformat_set(self, major):
        """ Set the major verison of the ring format. """

        return self._transaction("ringformat", {"major": major, "minor":0})
    
    def ringformat_get(self):
        """ Get the ring format information:
        """
        return self._transaction("ringformat/get", {})

    #-----------------  sbind interface:

    def sbind_all(self):
        """ Attempt to bind all spectra to shared memory:"""

        return self._transaction("sbind/all")
    
    def sbind_spectra(self, spectra):
        """ sbind an iterable collection of spectra:
        """
        return self._transaction("sbind/sbind", {"spectrum":spectra})

    def sbind_list(self, pattern="*"):
        """ list bindings"""

        return self._transaction("sbind/list", {"pattern":pattern})
        
    #---------- Shared memory information:

    def shmem_getkey(self):
        """ Get the shared memory key.  This can be in one of several
        forms (it's in the detail of the returned Dict):

        *  a four letter string - this is a SYSV shared memory key.
        *  "file:" followed by a path - this is an memory mapped file
        where the path is the path to the backing file.
        *  "posix:/" followed by a name - this is a POSIX shared memory 
        region.
        *  "sysv:" followed by the four letter SYSV shared memory key.
        """
        return self._transaction("shmem/key", {})

    def shmem_getsize(self):
        """ return the number of bytes in the shared memory region. 
        This value includes the header (not just the spectrum pool).
        """
        return self._transaction("shmem/size", {})
    def shmem_getvariables(self):
        """ Returns the values of several SpecTcl variables.  Note
        that some of these have no rustogramer equivalents and will
        have values of '-undefined-'

        *  DisplayMegabytes  - The number of 1024*1024 bytes in the shared memory
        spectrum pool.
        *  OnlineState - True if connected to an online data source.
        *  EventListSize - Number of events in each processing batch.
        *  ParameterCount - Number of parameters in the initial flattened
        event.
        *  SpecTclHome - the home directory of the installation tree.
        *  LastSequence - Sequence number of the most recently processed
        data
        *  RunNumber - run number of the run being processed.
        *  RunState - "Active" if processing is underway, "Inactive" if not.
        *  DisplayType - Type of integrated displayer started by the program
        for Rustogramer this is always "None"
        *  BuffersAnalyzed - Number of items that have been analyzed.  For
        SpecTcl (not Rustogramer), this taken with LastSequence allows a rough
        computation of the fraction of data analyzed online.  Note that
        Rustogramer always analyzes offline (well there are ways but....).
        *  RunTitle - Title string of the most recent run (being) analyzed.
        """
        return self._transaction("shmem/variables", {})
    
    #--------------------------Spectrum API.

    def spectrum_list(self, pattern="*"):
        """ Return a list of spectra that match 'patttern' and their
        properties.  Note that 'pattern' is an optional parameter that is
        supports glob wild-cards.  If not provided, it defaults to '*' which
        matches all names.
        """
        return self._transaction("spectrum/list", {"filter": pattern})
    
    def spectrum_delete(self, name):
        """ Delete the named spectrum"""
        return self._transaction("spectrum/delete", {"name":name})
    
    def spectrum_create1d(self, name, parameter, low, high, bins):
        """ Create a simple 1d spectrum:
        *   name - The name of the new spectrum (must be unique)
        *   parameter - the parameter that will be histogramed
        *   low, high, bins - definition of the histogram X axis.
        """
        axis = self._format_axis(low, high, bins)
        return self._transaction(
            "spectrum/create", 
            {"name":name, "type":"1", "parameters": parameter, "axes":axis}
        )

    def spectrum_create2d(self, name, xparam, yparam, xlow, xhigh, xbins, ylow, yhigh, ybins):
        """ Create a simple 2d spectrum:
        *  name - the name of the new spectrum.
        *  xparam,yparam - the x and y parameters to be histogramed.
        *  xlow, xhigh,xbins - the X axis defintion.
        *  ylow, yhigh, ybins -the y axis definition.
        """

        axes = self._format_xyaxes(xlow, xhigh, xbins, ylow, yhigh, ybins)
        return self._transaction(
            "spectrum/create",
            {"type":2, "name":name, "parameters":xparam + " " + yparam, "axes":axes}
        )

    def spectrum_createg1(self, name, parameters, low, high, bins):
        """ Create a gamma 1 spectrum (multiply incremented 1d).
        *  name - name of the spectrum.
        *  parameters - iterable collection of parameter names
        *  low, high, bins - definition of spectrum x axis.
        """
        axes = self._format_axis(low, high, bins)
        params = self._format_stringlist(parameters)
        return self._transaction(
            "spectrum/create",
            {"type":"g1", "name":name, "parameters":params, "axes":axes}
        )

    def spectrum_createg2(self, name, parameters, xlow, xhigh, xbins, ylow, yhigh, ybins):
        """ Create a gamma 2 spectrum (multiply incremented 2d).
        *  name - name of the spectrum
        *  parameters - parameters - incremented for each ordered pair present in the spectum.
        *  xlow, xhigh, xbiins - x axis definition.
        *  ylow, yhigh, ybins  - y axis definition.
        """
        axes = self._format_xyaxes(xlow, xhigh, xbins, ylow, yhigh, ybins)
        params = self._format_stringlist(parameters)
        return self._transaction(
            "spectrum/create",
            {"type":"g2", "name":name, "parameters":params, "axes":axes}
        )
    def spectrum_creategd(self, name, xparameters, yparameters, xlow, xhigh, xbins, ylow, yhigh, ybins):
        """ Create a 'gamma deluxe' spectrum This is normally used for particle-gamma
        coincidence spectra.  Increments are done for every x/y pair that's defined.
        Consider e.g. that xparameters are  gamma detectors and y parameters are particle ids.
        *  name - name of the spectrum.
        *  xparameters - Parameters on the x axis.
        *  yparameters - Parametrs on the y axis.
        *  xlow, xhigh,xbins - xaxis definition.
        *  ylow, yhigh, ybins - yaxis defintion.
        """
        axes = self._format_xyaxes(xlow, xhigh, xbins, ylow, yhigh, ybins)
        xpars = self._format_stringlist(xparameters)
        ypars = self._format_stringlist(yparameters)
        param_list = "{" + xpars + "}{" + ypars + "}"
        return self._transaction(
            "spectrum/create",
            {"type":"gd", "name":name, "parameters":param_list, "axes":axes}
        )
    def spectrum_createsummary(self, name, parameters, low, high,  bins):
        """ Make a summary spectrum.  This is a 2d spectrum where every vertical
        channel strip is actually the one dimensional spectrum of one of the
        parameters in the spectum.

        *   name - the spectrum name
        *   parameters - an iterable list of parameters to histogram.
        *   low, high, bins - the Y axis defintion of the spectrum.  
        Note the X axis is defined as 0 - len(parameters) with len(parameters) bins.
        """
        pars = self._format_stringlist(parameters)
        axis = self._format_axis(low, high, bins)
        return self._transaction(
            "/spectrum/create",
            {"type":"s", "name":name, "parameters":pars, "axes":axis}
        )
    def spectrum_create2dsum(self, name, xpars, ypars, xlow, xhigh, xbins, ylow,yhigh,ybins):
        """Create a 2d spectrum that is the sum of the 2d spectra defined
        by corresopnding xpars/ypars parameters. Note that the server enforces
        that len(xpars) must be the same as len(ypars)
        *   name - spectrum name.
        *   xpars - X axis parameters
        *   ypars - Y axis parameters
        *   xlow, xhigh, xbins - x axis defintion.
        *   ylow, yhigh, ybins - y axis definition
        
        Increments are done for corresponding x/y pars e.g. for 
        xpars[0], ypars[0]  if those parameters are present in the event.
        """
        xp = self._format_stringlist(xpars)
        yp = self._format_stringlist(ypars)
        pars = '{' + xp + '}{' + yp + '}'
        axes = self._format_xyaxes(xlow, xhigh, xbins, ylow, yhigh, ybins)
        return self._transaction(
            "spectrum/create",
            {"type":"m2", "name":name, "parameters":pars, "axes":axes}
        )
    def spectrum_getcontents(self, name, xl, xh, yl=0,yh=0):
        """ Get the contents of a spectrum within a region of interest.
        *   name - name of the spectrum.
        *   xl,xh - x range we're interested in.
        *   yl,yh - y range we're interested in.  These default to 0
        so you only need to provide the xl,xh for 1d spectra.
        """
        return self._transaction(
            "spectrum/contents",
            {"name":name, "xlow": xl, "xhigh": xh, "ylow":yl, "yhigh":yh}
        )
    def spectrum_clear(self, pattern="*"):
        """ Clear the contents of spectra that have names matching the
        'pattern' parameter.  Where 'pattern' is a glob match pattern.
        If omitted, 'pattern' defaults to "*" which matches all spectra.
        """
        return self._transaction("spectrum/clear", {"pattern": pattern})

    #--------------- Spectrum I/O

    def spectrum_read(self, filename, format, options={}):
        """ Read one or more spectra from file.

        *   filename - is the path to the file - in the context of
        rustogramers cwd (safest to use full paths then.
        *   format - Is the file format.  Three possible values are allowed:
            -  'ascii' (both Spectcl and Rustogramer) - file is in SpecTcl
            ASCII format
            -  'json' (Rustogramer only) file is in rustogramr JSON format.
            -  'binary' (SpecTcl only) file is in Smaug binary format (this is
            a pretty obsolete format; SMAUG was a pre NSCL offline analysis program
            used by the 'Lynch' written by an undergraduate in their employ whose
            name now escapes me so I can't credit him).
        *  options a dict (defaults to empty) that can override the options
        that determine how the spectrum is read.  It is optional and the
        following keys matter:
            - snapshot - boolean - if true, the default, the spectrum will not
            be connected to analysis.  If false, the spectrum will be connected to
            the analysis if possible.  This is done differently between SpecTcl and
            Rustogramer:
                .   SpecTcl has a special wrapper for spectra called a snapshot wrapper.
                snapshot spectra are wrapped in that and can never be incremented.  They are
                transparent to the histogramer code.  If a spectrum is not to be
                a snapshot, then if all of its parameters are defined, it will increment.
                .   Rustogramer, simply handles snapshot spectra by gating them on a 
                special False condition it creates if necessary called
                '_snapshot_condition'.   If parameters associated with a read spectrum
                don't exist, they will be created.  This means that if any future
                parameter data file is analyzed with that spetrum's parameter name
                and the gate on the spectrum allows it events from that file will
                increment that spectrum.
            - replace - boolean - if true (the defaut is false), and a spectrum with the
            same name as a spectrum in the file already exists, that spectrum is
            deleted and replaced by the spectrum from file (the type of the existing
            and new spectrum can be different).   If false, and a spectrum has the name
            of an existing spectrum, a new name is generated that is unique and assigned
            to the spectrum that is read in.
            -  bind - boolean - if true (the default) the spectra read in are also put in 
            shared spectrum memory where displayers can access them.
        
        Note the format is checked by the server not the API so that new formats
        can be transparently added.
        """
        parameters = {}
        for key in ["snapshot", "replace", "bind"]:
            option_value = options.get(key)
            if option_value is not None:
                parameters[key] = option_value
        parameters["filename"] = filename
        parameters["format"] = format

        return self._transaction("sread", parameters)
    
    def spectrum_write(self, filename, format, spectra):
        """ Write one or more spectra to file.

        *  filename - is the path to the file to write.  This is interpreted
        within rustogramer hence it may be safest to use full paths.
        *  format - Is a legal format type (see spectrum_read).
        *  spectra is a single spectrum name or an iterable collection of
        spectrum names for spectra that should be written to the file.
        """

        return self._transaction("swrite", {"file": filename, "format": format, "spectrum":spectra})
    
    #------------------- unbind api:

    def unbind_byname(self, spectrum):
        """ Unbind the spectrum 'spectrum' from the shared memory.
        Note only one unbinding can be performed per call with the exception
        unbind_all which removes all bindings.
        """
        return self._transaction("unbind/byname", {"name":spectrum})

    def unbind_byid(self, id):
        """ This is not supported by rustogramer as spectra don't have
        ids.  It is supported by SpecTcl so the API element is implemented.
        In SpecTcl, the spectrum numbered 'id' will be unbound from shared memory.
        """
        return self._transaction("unbind/byid",{"id":id})

    def unbind_all(self):
        """ Unbinds all spectra from display shared memory.
        """
        return self._transaction("unbind/all", {})

    #----------------- Mirror interface:

    def list_mirrors(self, pattern="*"):
        """  List the mirrors that are currently active. Note that
        this is _currently_ SpecTcl only though there are plans to 
        implement mirroring later.  Only the mirrors that match the
        'pattern' parameter are returned.  'pattern' defaults to "*"
        """
        return self._transaction("mirror", {"pattern":pattern})

    #-----------------event processing pipeline management (SpecTcl only)

    def pipeline_create(self, name):
        """ Create a new event processing pipeline and assigne the name
        'name' to it.  This is only implemented in SpecTcl and likely 
        will never be implemented in rustogramer as it has no meaning
        """
        return self._transaction("pman/create", {"name":name})
    
    def pipeline_current(self):
        """ Provide information about the currently selected event
        processing pipeline.  This is only implemented in SpecTcl.
        """
        return self._transaction("pman/current", {})
    
    def pipeline_list_details(self, pattern="*"):
        """ For each pipeline whose name matches the pattern (defaults
        to '*'),  returns the name of the pipeline and the array
        of processing elements in the pipeline.
        This is only implemented in SpecTcl.
        """
        return self._transaction("pman/lsall", {"pattern":pattern})
    def pipeline_list_processors(self, pattern="*"):
        """ List the names of all event processors that have been
        registered
        This is only implemented in SpecTcl.
        """
        self._transaction("pman/lsevp", {"pattern":pattern})

    def pipeline_use(self, name):
        """ Selects the pipeline 'name' as the current event processing
        pipeline. This is only implemented in SpecTcl.
        """
        return self._transaction("pman/use", {"name": name})
    
    def pipeline_add_processor(self, pipe, processor):
        """ Appends an event processor to an event processing pipeline:
        *   pipe -name of the pipeline.
        *   processor - name of the processor to append.

        This is only implemented in SpecTcl.
        """
        return self._transaction("pman/add", {"pipeline": pipe, "processor": processor})

    def pipeline_remove_processor(self, pipe, processor):
        """ Removes  a processor from an event processing pipeline:
        *  pipe - name of the pipeline.
        *  processor - name of the event processor.

        This is only implemented in SpecTcl.
        """
        return self._transaction("pman/rm", {"pipeline":pipe, "processor":processor})

    def pipeline_clear(self, pipe):
        """ Remove all event processing elements from 'pipe'
        This is only implemented in SpecTcl.
        """
        return self._transaction("pman/clear", {"pipeline":pipe})

    def pipeline_clone(self, existing, newpipe):
        """ Make a functional clone of an existing event processing pipeline
        *   existing - name of an existing pipeline.
        *   newpipe - name to assig to the clone of 'existing'

        This is only implemented in SpecTcl.
        """
        return self._transaction("pman/clone", {"source":existing, "new":newpipe})

    #------------- Pseduo paramemter API

    def pseudo_create(self, name, dependent, computation):
        """ Adds a new pseudo parameter.  
        *  name -  name of the new parameter.
        *  dependent - iterable container of names of paramters needed 
        compute the pseudo.
        *  computation - The computation to perform as Tcl.  See the
        SpecTcl pseudo command for more information about this.

        Note This is only implemented in SpecTcl.
        Likely this will never be implemented in rustogramer as
        the best way to get pseudo parameters is to comute them in the
        event processing that creates the parameter file.
        """
        return self._transaction(
            "pseudo/create", 
            {"pseudo": name, "parameter": dependent, "computation":computation}
        )
    
    def pseudo_list(self, pattern="*"):
        """ Returns information about pseudo parameters with names
        that match 'pattern', which defaults to "*" if not provided

        This is only implemented in SpecTcl.
        Likely this will never be implemented in rustogramer
        """
        return self._transaction("pseudo/list", {"pattern":pattern})

    def pseudo_delete(self, name):
        """ Delete the named pseudo parameter.

        This is only implemented in SpecTcl.
        Likely this will never be implemented in rustogramer
        """
        return self._transaction("pseudo/delete",{"name":name})

    #------------ projection

    def project(self, oldname, newname, direction, snapshot, contour=None, ):
        """ Create a spectrum that is a projection of an existing 2-d spectrum:
        *  oldname - name of the spectrum being projected:
        *  newname - name assigned to the new spectrum.
        *  direction - the direction in which to project ("x" or "y").
        *  contour - If not None, only counts within the contour will be projected.
        *  snapshot - If true, the result in the spectrum is a snapshot If not true,
        and contour is None, the new spectrum has no gate.  If true and contour
        is defined, the spectrum is initially gated by the contour so that the
        projection, unless the gate is changed is a faithful representation of
        the projection as new events arrive.  The spectrum must be separately
        bound to display memory if desired.

        This is only implemented in SpecTcl.
        """
        params = {
            "source":oldname, 
            "newname":newname, 
            "direction":direction, 
            "snapshot":snapshot
        }
        if contour is not None:
            params["contour"] = contour
        
        return self._transaction("project", params)
    
    #------------------------------- Root tree API.

    def roottree_create(self, tree, parameters, gate=None):
        """ Creates a root output tree.   This is not available on
        Rustogramer.  Root output trees are a lot like filters, however
        the otput are root trees and they are output in a file per run
        analyzed.

        *  tree - name of the tree.
        *  parameters - parameters that will be output to the tree.
        this can be a single parameter name, but more usually is an
        iterable collection of parameter names.
        *  gate - optional gate that, if provided will filter the events
        written to the tree.

        Note:  Root's output mechanisms are quite slow compared with 
        data acqusition rates.  Unless you have a very restrictive gate,
        it is not recomended to create root trees in online analysis.

        """
        params = {"tree": tree, "parameter":parameters}
        if gate is not None:
            params["gate"] = gate
        
        return self._transaction("roottree/create", params)

    def roottree_delete(self, tree):
        """ Deletes a previously created root tree.  This, of course,
        is not available in Rustogramer.

        * tree -name of the tree to delete.

        Note:  This takes effect immediately.  If data analysis is ongoing,
        the file is closed and the section of data anlalyzed to date remains
        in the output fle.

        """

        return self._transaction("roottree/delete", {"tree":tree})

    def roottree_list(self, pattern="*"):
        """ Lists the properties of root trees that match the optional
        pattern glob pattern.  If 'pattern' is not provided it defaults to
        '*' which matches all trees.

        """
        return self._transaction("roottree/list", {"pattern":pattern})
    
    #--------------------- Scripting interface.

    def execute_tcl(self, script):
        """ (unsupported in Rustogramer):
        Execute an arbitrary script in the server.  Note this is a bit
        dangerous.  
        *  script - the script to execute. This must be in the language
        of the server's script interpreter.  In SpecTcl this is Tcl
        although with the proper package requires and wrapping it can also
        be Python.
        """
        return self._transaction("script", {"command": script})

    #------------------- Dictionary trace support.
    
    def trace_establish(self, retention_secs):
        """ Establishes a trace.  Since HTTPD does not directly support
        the server sending clients messages, traces will be accumulated in 
        a queue where they will remain for at least 'retention_secs' seconds
        before being discarded.  It is up to the client to fetch any queued
        traces within that time (normal practice is to poll for traces at
        something like 1/2 the retention_sec period e.g.).
        The method returns a token which must be used to fetch the traces or 
        turn othem off.

        One major purpose of the retention_period parameter is to ensure that
        if a client does _not_ turn off its traces before it exits (e.g. abnormally),
        queued trace data won't grow without bound.

        This is not supported by rustogramer -- at this time.  It may be
        added at some later point in time.
        """

        return self._transaction("trace/establish", {"retention":retention_secs})

    def trace_done(self, token):
        """ Not supported in Rustogramer - stops accumulating the
        trace data associated with 'token'.  The 'token' parameter was
        gotten from the dict returned by trace_establish above.

        Once this is successfully executed, the 'token' is no longer 
        valid.
        """
        return self._transaction("trace/done", {"token":token})
    
    def trace_fetch(self, token):
        """ Not siupported in rustogramer - fetches the accumulated
        trace information for the 'token'.  The 'token' parameter is a 
        token value that was returned in the dict returned by trace_establish.

        Data retrieved by trace_fetch are also cleared from the accumulated
        trace data for 'token'
        """

        return self._transaction("trace/fetch", {"token":token})

    #------------------- Tree variable API (not tree parameters).

    def treevariable_list(self):
        """ Not supported in rustogramer
        Returns information about all of the defined tree variables.
        Name, value, and units are returned for each variable.
        """
        return self._transaction("treevariable/list", {})

    def treevariable_set(self, name, value, units=None):
        """ Not suported in rustogramer.  Set the value and units
        of a tree variable.

        *   name -name of the variable.
        *   value - new value to give to the variavble.
        *   [units] - defaults to None  for unit-less.  Sets the units of
        measure associated with the variable.
        """
        params = {"name": name, "value": value}
        if units is not None:
            params['units'] = units
        return self._transaction("treevariable/set", params)

    def treevariable_check(self, name):
        """ not supported by rustogamer - check the changed flag for
        treevariable 'name'.
        """
        return self._transaction("treevariable/check", {"name":name})
    
    def treevariable_setchanged(self, name):
        """ Not supported by rustogramer
         Set the changed flag for tree variable 'name'.
        """
        return self._transaction("treevariable/setchanged", {"name":name})
    
    def treevariable_firetraces(self, pattern="*"):
        """ not suported by rustogramer:
        Fires any pending variable traces for the tree variables whose
        names match the glob 'pattern'  If 'pattern' is omitted it defaults
        to '*' which matches al lnames.

        firing traces may be needed to update user interface elements that
        monitor a tree variable value that is set programmatically.
        """
        return self._transaction("treevariable/firetraces", {"pattern": pattern})
    #----------------------------- Version:

    def get_version(self):
        """ Returns information about the version of the server.
        """
        return self._transaction("/version", {})

    
    


    
