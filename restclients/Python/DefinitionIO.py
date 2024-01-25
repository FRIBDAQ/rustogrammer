''' 
This module provides code to save/restore various definitions to an Sqlite3 database file.
One would create one of a DefinitionWriter or DefinitionReader object and then invoke the various
methods  on that to do the I/O.  The database files use the same schema as the SpecTcl
sqlite3 data store; See https://docs.nscl.msu.edu/daq/newsite/spectcldb/index.html

Note that while that schema supports storing more than one 'save set' in a single database file,
this capability is not used and when a database file is initialized, the save set created is 
called 'rustogramer_gui'. 
'''

import sqlite3
import time

save_set_name = 'rustogramer_gui'

class DefinitionWriter:
    ''' Writer for definitions.  Insantiating the writer creates the initial schema if
       needed.  Note that at present, we only create a single save set, see the 
       save_set_name variable.  Given that, creating one of these on a non empty 
       database file can result in undefined consequences.
       
       Note as well that while the SpecTcl data store includes scheme components to store
       e.g. runs and spectrum contents, we don't create those elements.
    '''
    def __init__(self, filename):
        self._sqlite = sqlite3.connect(filename)
        self._create_schema()
        self._saveid = self.open_saveset(save_set_name)
    def __del__(self):
        print('closing database')
        self._sqlite.close()
        
    def open_saveset(self, name):
        '''
        Opens a save set in the databse.  If the save set does not exist, it is created.
        Any definitions saved until the next open_saveset operation will be done on that
        saveset
        
        Parameters:
        *  name   - name of the saveset.
        
        Returns integer save-set id.
        
        '''
        
        cur = self._sqlite.execute('''
            SELECT id FROM save_sets WHERE name = ?
                             ''', (name,))
        id = cur.fetchone()
        if id is None:
            print('inserting')
            cur = self._sqlite.execute('INSERT INTO save_sets (name, timestamp) VALUES (?,?)', (name, time.time()))
            id = cur.lastrowid
            print('id = ', id)
            self._saveid = id
            self._sqlite.commit()
            return id
        else:
            self._saveid = id[0]
            return id[0]
        
    def save_parameter_definitions(self, defs):
        '''
        Save the parameter definitions into the current save set.  
        
        Parameters:
        * defs - parameter definitions as gotten from e.g. 
                 rustogramer_client.rustogramer.paramter_list()['detail']
        
        '''  
        
        cur = self._sqlite.cursor()
        cur.executemany(f'''
            INSERT INTO parameter_defs (save_id, name, number, low, high, bins, units)
                VALUES({self._saveid}, :name, :id, :low, :hi, :bins, :units)
                        ''', defs)
        self._sqlite.commit()
        
    # Private methods    
    def _create_schema(self):
        # Create the databas schema; again see 
        # https://docs.nscl.msu.edu/daq/newsite/spectcldb/index.html
        # 'Database schema' appendix.
        #  Note:  In this implementation we don't worry about making the
        #         indices.. that provides for fastest inserts (I think).
        #  Note:  This schema is pretty much assured to be correct since
        #         it's literally copy/pasted from the SpecTcl main/db/SpecTclDatabase.cpp  module
        #  This method is rather lengthy but straightforward so I don't bother to break it up
        #
        
        cursor = self._sqlite.cursor()
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS  save_sets 
                (id  INTEGER PRIMARY KEY,
                name TEXT UNIQUE,
                timestamp INTEGER)
                             ''')
        
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS parameter_defs
                (id      INTEGER PRIMARY KEY,                    
                save_id INTEGER NOT NULL,  -- foreign key to save_sets.id
                name    TEXT NOT NULL,
                number  INTEGER NOT NULL,
                low     REAL,
                high    REAL,
                bins    INTEGER,
                units   TEXT)
                                ''')
        
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS spectrum_defs
                (id      INTEGER PRIMARY KEY,
                save_id INTEGER NOT NULL,     -- Foreign key to save_sets.id
                name    TEXT NOT NULL,
                type    TEXT NOT NULL,
                datatype TEXT NOT NULL
            )
                             ''')
        
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS axis_defs
            (
                id           INTEGER PRIMARY KEY,
                spectrum_id  INTEGER NOT NULL,  -- FK to spectrum_defs.id
                low          REAL NOT NULL,
                high         REAL NOT NULL,
                bins         INTEGER NOT NULL
            )
                             ''')
        
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS spectrum_params   
            (   id          INTEGER PRIMARY KEY,          
                spectrum_id INTEGER NOT NULL,             
                parameter_id INTEGER NOT NULL             
            )
                            ''')
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS gate_defs       
                (   id          INTEGER PRIMARY KEY,   
                    saveset_id  INTEGER NOT NULL,      
                    name        TEXT NOT NULL,         
                    type        TEXT NOT NULL          
                )
                             ''')
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS gate_points  
            (   id          INTEGER PRIMARY KEY,   
                gate_id     INTEGER NOT NULL,      
                x           REAL,                  
                y           REAL                   
            )
                              ''')
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS gate_parameters 
            (   id   INTEGER PRIMARY KEY,           
                parent_gate INTEGER NOT NULL,       
                parameter_id INTEGER NOT NULL       
            )
                             ''')
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS component_gates       
                (                                            
                    id          INTEGER PRIMARY KEY,         
                    parent_gate INTEGER NOT NULL,           
                    child_gate  INTEGER NOT NULL            
                )
                            ''')
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS gate_masks    
            (   id          INTEGER PRIMARY KEY,     
                parent_gate INTEGER NOT NULL,        
                mask        INTEGER NOT NULL         
            )
                             ''')
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS gate_applications (
                id                INTEGER PRIMARY KEY,  
                spectrum_id       INTEGER NOT NULL,     
                gate_id           INTEGER NOT NULL      
            )
                             ''')
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS treevariables (   
                id             INTEGER PRIMARY KEY,   
                save_id        INTEGER NOT NULL,      
                name           TEXT NOT NULL,         
                value          DOUBLE NOT NULL,       
                units          TEXT                   
            )
                             ''')
        
        
