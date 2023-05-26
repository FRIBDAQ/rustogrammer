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

    def _transaction(self, request, queryparams):
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
        REST interface.  THis connection by each service request to that
        port.
        """
        self.port = connection["port"]
        self.host = connection["host"]
        if "service" in connection:
            self.port = self._service_port(connection['host'], self.port, connection["service"])

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
