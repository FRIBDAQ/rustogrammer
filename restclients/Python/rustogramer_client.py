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
        """
        return self._transaction("exit")