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
        