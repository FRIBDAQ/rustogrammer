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
        
        # Note that I believe this creates a transaction that encapsulates all of the
        # INSERTs below but it's not clear from the docs.
        cur = self._sqlite.cursor()
        cur.executemany(f'''
            INSERT INTO parameter_defs (save_id, name, number, low, high, bins, units)
                VALUES({self._saveid}, :name, :id, :low, :hi, :bins, :units)
                        ''', defs)
        self._sqlite.commit()
    
    def save_spectrum_definitions(self, defs):
        '''
            Save the definitions of all spectra to the database file.  Note that since
            inserting a spectrum is not an atomic database operation,  everything is done
            in a transaction.  This implies the spectrum save is an all or nothing thing.
            
            *   defs - the specturm definitions from e.g. 
                rustogramer_client.rustogramer.spectrum_list()['detail']
            
            If the save fails, the exception raised is passed onward to the caller with the
            transation rolled back.
        '''   
        c = self._sqlite.cursor()
        # I want expclicit control over the transaction and I'm not sure when/what needs
        # committing if I just use the auto so we'll use a save point for that:
        
        c.execute('SAVEPOINT spectrum_save')
        try :
            for s in defs:
                self._save_specdef(c, s)
        except:
            #  If there are any errors rollback the save point and any
            #  tansaction and re-raise.
            c.execute('ROLLBACK TRANSACTION TO SAVEPOINT spectrum_save')
            c.execute('RELEASE SAVEPOINT spectrum_save')       # Save points are tricky this way.
            self._sqlite.rollback()
            raise
        # Success so commit:
        
        c.execute('RELEASE SAVEPOINT spectrum_save')
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
        
        #  Table for savesets.
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS  save_sets 
                (id  INTEGER PRIMARY KEY,
                name TEXT UNIQUE,
                timestamp INTEGER)
                             ''')
        
        #  Table for parameter and metadata definitions.
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
        
        # Tables for spectrum definitions
        
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
        #     Note this definition means that gd spectra can't be recovered.
        #     we really need spectrum_x_params and spetrum_y_params tables.
        #     That also means reworking the SpecTcl part of the equation.
        #     We'll generate those tables, and stock them with an issue in SpecTcl
        #     to fix.
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS spectrum_params   
            (   id          INTEGER PRIMARY KEY,          
                spectrum_id INTEGER NOT NULL,             
                parameter_id INTEGER NOT NULL             
            )
                            ''')
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS spectrum_x_params   -- Rustogramer.
            (   id          INTEGER PRIMARY KEY,          
                spectrum_id INTEGER NOT NULL,             
                parameter_id INTEGER NOT NULL             
            )
        ''')
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS spectrum_y_params   -- Rustogramer.
            (   id          INTEGER PRIMARY KEY,          
                spectrum_id INTEGER NOT NULL,             
                parameter_id INTEGER NOT NULL             
            )
        ''')
        # Tables for condition (gate) definitions
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
        
        # Join table defining which conditions are applied to which 
        # spectra.
        
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS gate_applications (
                id                INTEGER PRIMARY KEY,  
                spectrum_id       INTEGER NOT NULL,     
                gate_id           INTEGER NOT NULL      
            )
        
                             ''')
        # Define tree variables (only used by SpecTcl).
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS treevariables (   
                id             INTEGER PRIMARY KEY,   
                save_id        INTEGER NOT NULL,      
                name           TEXT NOT NULL,         
                value          DOUBLE NOT NULL,       
                units          TEXT                   
            )
                             ''')
    def _save_specdef(self, cursor, d):
        # Given a database cursor 'cursor' and spectrum definition 'd', performs the
        # SQL to save that defintiion to file.  Note that it is best if there's a transaction
        # or savepoint active on the database so that the non-atomic save of the spectrum
        # becomes atomic over some timeline.
        
        # Write the root record and save it's id for foreign keys in the child records:
        
        cursor.execute('''INSERT INTO spectrum_defs 
            (save_id, name, type, datatype) 
            VALUES (:sid, :name, :type, :dtype)
        ''', 
        {
            'sid': self._saveid, 'name': d['name'], 'type': d['type'], 'dtype': d['chantype']
        })
        specid = cursor.lastrowid
        for axis in d['axes']:
            cursor.execute('''
                INSERT INTO axis_defs (spectrum_id, low, high, bins)
                    VALUES (:sid, :low, :high, :bins)
            ''', {
                'sid': specid, 'low': axis['low'], 'high': axis['high'], 'bins': axis['bins']
            }
        )
        #   Inserts are done  in a tricky way using a subselect that gets both the 
        #   spectrum id and parameter ids we have to do it in this tricky way because
        #   we can't mix VALUES and subselects on one INSERT Alternatives:
        #    Do an INSERT and an UPDATE
        #    Do a separate SELEC\T to get the parameter id from the name then do the insert
        #
        #  For non SQL fluent people.  the INSERT will insert as many rows as will match the
        #  query in the subselect.   The columns in parentheses will be inserted with values pulled 
        #  from the query in order:  spectrum_id stocked with the spectrum_defs.id value and
        #  parmaeter_id with parameter_defs.id
        #  
        #   The INNER JOIN joins the spectrum_defs table with the parameter_defs table wherever
        #   the save_id field on each matches (for the same save set id).
        #   The WHERE clause requires a match with the id of the spectrum we're saving,
        #           and a match for the parameter name as well as a match for our saveset.
        #    this should result in one match that will have our spectrum id, our saveset an the
        #    parameter id we're looking up for parameter name in that saveset.
        #
        for p in d['parameters']:
            cursor.execute('''
                INSERT INTO spectrum_params (spectrum_id, parameter_id)
                SELECT spectrum_defs.id AS spectrum_id, 
                        parameter_defs.id AS param_id FROM spectrum_defs 
                        INNER JOIN parameter_defs ON spectrum_defs.save_id = parameter_defs.save_id   
                    WHERE spectrum_defs.id = :specid 
                        AND parameter_defs.name = :paramname 
                        AND spectrum_defs.save_id = :saveid
            ''', {
                'specid': specid, 'paramname': p, 'saveid': self._saveid
            })
        # x parameters, same subselect trick, different target table:
        
        for p in d['xparameters']:
            cursor.execute('''
                INSERT INTO spectrum_x_params (spectrum_id, parameter_id)
                SELECT spectrum_defs.id AS spectrum_id, 
                        parameter_defs.id AS param_id FROM spectrum_defs 
                        INNER JOIN parameter_defs ON spectrum_defs.save_id = parameter_defs.save_id   
                    WHERE spectrum_defs.id = :specid 
                        AND parameter_defs.name = :paramname 
                        AND spectrum_defs.save_id = :saveid
            ''', {
                'specid': specid, 'paramname': p, 'saveid': self._saveid
            })
        
        # y parameters
        
        for p in d['yparameters']:
            cursor.execute('''
                INSERT INTO spectrum_y_params (spectrum_id, parameter_id)
                SELECT spectrum_defs.id AS spectrum_id, 
                        parameter_defs.id AS param_id FROM spectrum_defs 
                        INNER JOIN parameter_defs ON spectrum_defs.save_id = parameter_defs.save_id   
                    WHERE spectrum_defs.id = :specid 
                        AND parameter_defs.name = :paramname 
                        AND spectrum_defs.save_id = :saveid
            ''', {
                'specid': specid, 'paramname': p, 'saveid': self._saveid
            })
        
        
            
        
