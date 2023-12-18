''' This package contains code that
1.  Determines the program type (currently SpecTcl or Rustogramer)
2.  Provides capability information about the current program.
For example; rustogramer only implements a subset of the spectrum and
gate types that SpecTcl implements.  The application can ask questions like
'does the server program implement strip chart spectra?'

Normally this is used to disable/enable parts of the user interface appropriate
to the program.

To minimize the (expensive) interactions with the server, 
'''

from enum import Enum, auto
class Program(Enum) :
    @staticmethod
    def _generate_next_value(name, start, count, last_values) :
        return count+1
    
    Rustogramer = auto()
    SpecTcl = auto()
    Unknown = auto()    # Could not determine server type

server_program = None
major_version = None
minor_version = None
edit_level = None
client = None
'''
   If the program is not known get it after that, return it:
   Note set_client must have been called.
'''
def get_program():
    global server_program
    global major_version
    global minor_version
    global edit_level
    global client
    if server_program is None:
        info = client.get_version()
        info = info['detail']
        major_version = info['major']
        minor_version = info['minor']
        edit_level = info['editlevel']
        #  Get the program name.. note version of SpecTcl may
        # not return a program_name key:

        if 'program_name' in info.keys():
            name = info['program_name']
        else:
            name = 'SpecTcl'

        
        # Compue the value of server_program

        if name == 'SpecTcl':
            server_program = Program.SpecTcl
        elif name == 'rustogramer':
            server_program = Program.Rustogramer
        else:
            server_program = Program.Unknown

    return server_program


'''  This must be called first to provide a client object to the
     package:
'''

def set_client(client_obj): 
    global client
    client = client_obj

def get_client():
    global client
    return client

#  Spectrum Type capabilities:

class SpectrumTypes(Enum):
    @staticmethod
    def _generate_next_value(name, start, count, last_values) :
        return count+1
    
    Oned = auto()
    Twod = auto()
    Summary = auto()
    Gamma1D = auto()
    Gamma2D = auto()
    TwodSum = auto()
    GammaDeluxe = auto()
    Projection = auto()

    #   Not implemented in client API (yet)
    #  But SpecTcl has them :

    StripChart = auto()
    Bitmask    = auto()
    GammaSummary = auto()

# Supported by each type.  This is a map of sets
# OF types supported by both the API and the 
# server program indexed by server program.

supported_spectrum_types = {
    Program.Rustogramer: {
        SpectrumTypes.Oned, SpectrumTypes.Twod,
        SpectrumTypes.Summary, SpectrumTypes.Gamma1D,
        SpectrumTypes.Gamma2D, SpectrumTypes.TwodSum,
        SpectrumTypes.GammaDeluxe,
        SpectrumTypes.Projection
    },
    Program.SpecTcl: {
        SpectrumTypes.Oned, SpectrumTypes.Twod,
        SpectrumTypes.Summary, SpectrumTypes.Gamma1D,
        SpectrumTypes.Gamma2D, SpectrumTypes.TwodSum,
        SpectrumTypes.GammaDeluxe,
        SpectrumTypes.Projection,
        SpectrumTypes.GammaSummary,
        SpectrumTypes.Bitmask,
        SpectrumTypes.StripChart
    },
    Program.Unknown: {}
}

def _has_stype(type_sel):
    global supported_spectrum_types
    server = get_program() 
    return type_sel in supported_spectrum_types[server]

def has_1d():
    return _has_stype(SpectrumTypes.Oned)
def has_2d():
    return _has_stype(SpectrumTypes.Twod)
def has_summary():
    return _has_stype(SpectrumTypes.Summary)
def has_gamma1d():
    return _has_stype(SpectrumTypes.Gamma1D)
def has_gamma2d():
    return _has_stype(SpectrumTypes.Gamma2D)
def has_twod_sum():
    return _has_stype(SpectrumTypes.TwodSum)
def has_pgamma():
    return _has_stype(SpectrumTypes.GammaDeluxe)
def has_stripchart():
    return _has_stype(SpectrumTypes.StripChart)
def has_bitmask():
    return _has_stype(SpectrumTypes.Bitmask)
def has_projection():
    return _has_stype(SpectrumTypes.Projection)
def get_supported_spectrumTypes():
    global supported_spectrum_types
    server_program = get_program()
    return supported_spectrum_types[server_program]


#  Spectrum data types supported:

class ChannelTypes(Enum):
    Double = auto()
    Long = auto()
    Short = auto()
    Byte = auto()

supported_channel_types = {
    Program.Rustogramer : {
        ChannelTypes.Double
    },
    Program.SpecTcl : {
        ChannelTypes.Long, ChannelTypes.Short, ChannelTypes.Byte
    },
    Program.Unknown: {}
}

def _has_channel_type(data_type) :
    global supported_channel_types
    return data_type in supported_channel_types[get_program()]

def has_double_channels():
    return _has_channel_type(ChannelTypes.Double)
def has_long_channels():
    return _has_channel_type(ChannelTypes.Long)
def has_short_channels():
    return _has_channel_type(ChannelTypes.Short)
def has_byte_channels():
    return _has_channel_type(ChannelTypes.Byte)
def get_supported_channelTypes():
    global supported_channel_types
    program = get_program()
    return supported_channel_types[program]