//! This file implements the memory transfer part of the
//! shared memory mirror server.   The mirror server is
//! responsible for providing the contents of the shared memory
//! to remote (and local) clients.  These clients are, typically,
//! but not exclusively, viewers such as CutiePie or even Xamine
//! run via the Xamine Mirror client.
//!
//! A simple client server binary protocol is used to communicate
//! between the client and server. The message structure uses
//! a header (MessageHeader struct), and a variable length body
//! whose contents depend on the message type.
//!
//!
use super::*;
use memmap;
use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::mem;
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::ptr;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use md5;

/// Here are the message type codes for the MessageHeader:
///
/// ### Client request message types:
///
/// * SHM_INFO - provides information to the server about the
/// shared memory key for any local shared memory region it creates.
/// to mirror the Rustogramer shared memory.  This key can be used
/// by clients to ensure only one mirror per host is used
/// (see rest/mirror.rs).  No reply message is sent for this
/// request.  *mut
/// *  REQUEST_UPDATE - Requests updated information for the
/// shared memory.  The server determines both, based on the
/// history of what's been sent and the state of the shared memory
/// region what type of reply to send.
pub const SHM_INFO: u32 = 1;
pub const REQUEST_UPDATE: u32 = 2;

/// ### Server reply message types:
///  
/// *   FULL_UPDATE - the shared memory region header and
/// all of the used data region are sent.  Used data region size
/// is determined by examining the extent of the largest used
/// offset.
/// *   PARTIAL_UPDATE - only the used data region is sent.
///
/// The use of these two reply types is intended to deal with
/// the fact that changes in bindings are infrequent so the header
/// does not have to be sent out very often.   Changes, to the
/// spectrum contents, however can be frequent - if analysis is
/// in progress.
///
pub const FULL_UPDATE: u32 = 3;
pub const PARTIAL_UPDATE: u32 = 4;

///
/// MessageHeader is the fixed part of the messages sent betweeen
/// client and server.  The fields are:
///
/// *  size - the entire message size in bytes.
/// *  type - the message type (should be one of the
/// message types above).  
///
/// #### Note
///  The struct can be private since we format the messages ourselves
/// and, at present, there are no RUST clients and hence no RUST
/// client crate.

#[derive(Debug)]
#[repr(C)]
struct MessageHeader {
    msg_size: u32,
    msg_type: u32,
}

impl MessageHeader {
    /// Validate a message type:

    fn validate_type(h: Self) -> Result<MessageHeader, String> {
        match h.msg_type {
            FULL_UPDATE => Ok(h),
            PARTIAL_UPDATE => Ok(h),
            REQUEST_UPDATE => Ok(h),
            SHM_INFO => Ok(h),
            _ => Err(format!("Invalid message type: {}", h.msg_type)),
        }
    }
    /// Read a message header from a readable:

    fn read<T: Read>(f: &mut T) -> Result<MessageHeader, String> {
        let mut buf: [u8; mem::size_of::<MessageHeader>()] = [0; mem::size_of::<MessageHeader>()];

        match f.read_exact(&mut buf) {
            Ok(_) => {
                let mut result = MessageHeader {
                    msg_size: 0,
                    msg_type: 0,
                };
                let l: [u8; 4] = buf[0..mem::size_of::<u32>()].try_into().unwrap();
                result.msg_size = u32::from_ne_bytes(l);

                let l: [u8; 4] = buf[mem::size_of::<u32>()..].try_into().unwrap();
                result.msg_type = u32::from_ne_bytes(l);
                Self::validate_type(result)
            }
            Err(reason) => Err(format!(
                "Unable to complete message Header read: {}",
                reason
            )),
        }
    }
    /// write a messgae header to a writeable.

    fn write<T: Write>(&self, f: &mut T) -> Result<usize, String> {
        let mut buf: [u8; mem::size_of::<MessageHeader>()] = [0; mem::size_of::<MessageHeader>()];
        buf[0..4].copy_from_slice(&self.msg_size.to_ne_bytes()[0..]);
        buf[4..].copy_from_slice(&self.msg_type.to_ne_bytes()[0..]);

        match f.write_all(&buf) {
            Ok(_) => Ok(mem::size_of::<MessageHeader>()),
            Err(e) => Err(format!("Header write failed: {}", e)),
        }
    }
    // Given this header, return the body size:
    ///
    fn body_size(&self) -> usize {
        self.msg_size as usize - mem::size_of::<MessageHeader>()
    }
}

///
/// The software maintains a directory of hosts and
/// shared memory keys.  Unlike the
/// SpecTcl implementation, shared memory keys can be
/// arbitrary strings.  This allows mirrors to be e.g. file
/// or posix backed.  
/// The director is used to let clients know of mirrors that
/// might already be running in their host so that they
/// can simply leverage off existing mirrors rather than
/// chewing up bandwidth with additional mirrors.
/// Here are entries in the mirror directory:
///
#[derive(Clone)] // Needed for tests will warn about dead code.
pub struct DirectoryEntry {
    host: String,
    key: String,
}

impl DirectoryEntry {
    pub fn new(host: &str, key: &str) -> DirectoryEntry {
        DirectoryEntry {
            host: String::from(host),
            key: String::from(key),
        }
    }
    pub fn host(&self) -> String {
        self.host.clone()
    }
    pub fn key(&self) -> String {
        self.key.clone()
    }
}

///  In defining the mirror directory, we assume that
///
/// *  Additions are not frequent.
/// *  There won't be many (on the scaler of computer storage) entries
/// in a directory.  We therefore have a hash map of directory
/// entries indexed by host.key.  This at least has the
/// virtue of making it easy to detect double use of
/// the same host/key pair:
///
pub struct Directory {
    items: HashMap<String, DirectoryEntry>,
}

impl Directory {
    fn compute_index(host: &str, key: &str) -> String {
        format!("{}.{}", host, key)
    }
    /// Create a new directory.
    pub fn new() -> Directory {
        Directory {
            items: HashMap::new(),
        }
    }
    /// adds a new directory entry.
    /// Computes the key and:
    /// *  If it's a duplicate, Errs indicating that.
    /// *  If it's not a duplicate, constructs a DirectoryEntry
    /// and inserts it into the items.
    pub fn add(&mut self, host: &str, key: &str) -> Result<(), String> {
        let index = Self::compute_index(host, key);

        if let std::collections::hash_map::Entry::Vacant(e) = self.items.entry(index) {
            e.insert(DirectoryEntry::new(host, key));
            Ok(())
        } else {
            Err(format!(
                "The host/key pair {} {} are already registered",
                host, key
            ))
        }
    }
    /// Iterate over the DirectoryEntry -s in the directory.
    pub fn iter(&self) -> Values<'_, String, DirectoryEntry> {
        self.items.values()
    }
    /// Remove an entry from the directory:

    fn remove(&mut self, host: &str, key: &str) -> Result<(), String> {
        let index = Self::compute_index(host, key);
        if self.items.remove(&index).is_some() {
            Ok(())
        } else {
            Err(format!("No such entry for {} {}", host, key))
        }
    }
}
pub type SharedMirrorDirectory = Arc<Mutex<Directory>>;

/// MirrorServerInstance represents an instance of the
/// mirror server.  Each mirror server makes its own map to the
/// shared memory region.  This bypasses the lifetime/Send trait
/// issues encountered when trying to maintain one map and pass
/// pointers/references to threads.
///
/// Prior to sending the contents of the shared memory to the
/// client on request, an md5 hash is done of the header to
/// determine if we can get away with a partial transfer or if
/// a full transfer is required.  The digest is encapsulated
/// in an option which is initially None.
/// Thus the logic for doing upates is like this:
///
/// ```
///    compute the digest of the header
///    If the digest  is None 
///      set the digest to Some(digest of the header).
///      do a full update.
///    else 
///      let current_digest = digest of the header.
///      if the current digest == digest of header
///         Do a partial update
///      else 
///         Update the digest to Some(digest of the header)
///         Do a full update.
///    endif
/// ```

struct MirrorServerInstance {
    #[allow(dead_code)]
    shared_memory_map: memmap::Mmap,
    shared_memory: *const XamineSharedMemory,
    socket: TcpStream,
    peer: SocketAddr,
    mirror_directory: SharedMirrorDirectory,
    shm_info: Option<String>,
    digest: Option<md5::Digest>,
}

impl MirrorServerInstance {
    // Provide what RUST considers a safe way to look at the
    // shared memory region header.
    // THe map should hold for the lifetime of the server instance
    // so this really is safe.
    //
    fn memory(&self) -> &XamineSharedMemory {
        unsafe { self.shared_memory.as_ref().unwrap() }
    }
    // Provide a memory soup reference for the shared memory region with
    // a given size.  This is suitable to be written to the socket
    // for an e.g. FULL_UPDATE
    //
    fn make_update_pointer(&self, size: usize) -> *const [u8] {
        let p8 = self.shared_memory as *const u8;
        ptr::slice_from_raw_parts(p8, size)
    }
    // Find the defined spectrum definition with the largest offset.
    // note that it's possible there are no defined spectra in which case,
    // None is returned:
    fn find_largest_index(header: &XamineSharedMemory) -> Option<usize> {
        let mut biggest_offset = 0;
        let mut result = None;
        // Note the >= on the offset compare deals with the fact that
        // the 'lowest' spectrum can have an offset of 0...and might be the
        // only one.
        for i in 0..XAMINE_MAXSPEC {
            if (header.dsp_types[i] != SpectrumTypes::Undefined)
                && (header.dsp_offsets[i] >= biggest_offset)
            {
                result = Some(i);
                biggest_offset = header.dsp_offsets[i];
            }
        }

        result
    }
    // Return the number of bytes used in the spectrum region.
    // This is done by finding the spectrum with the largest offset,
    // computing its size and converting that to bytes (offsets and sizes
    // for rustogramer bound spectra are in u32's).
    //  Note it's possible that there are no defined spectra in which case
    // we need to return 0.
    fn size_spectrum_region(&self) -> usize {
        let header = self.memory();
        if let Some(largest_idx) = Self::find_largest_index(header) {
            let offset = header.dsp_offsets[largest_idx];
            let size = header.dsp_xy[largest_idx].xchans * header.dsp_xy[largest_idx].ychans;
            ((offset + size) as usize) * mem::size_of::<u32>()
        } else {
            0
        }
    }
    // Handle an SHM_INFO request.
    // No reply is needed:
    // *  There must be a non-zero body
    // *  The body, assumed to be a shared memory designator (e.g. file:path)
    //   and peer rendered as a string must not yet exist in the
    //   directory.
    // If these conditions are met the peer/body string are added to the
    // directory and OK(()) is returned.  Otherwise, an Appropriate Err(msg) is
    // returned.

    fn process_shminfo(&mut self, body_size: usize) -> Result<(), String> {
        if body_size > 0 {
            // Read the body:
            let mut byte_buf = Vec::<u8>::new();
            byte_buf.resize(body_size, 0);
            match self.socket.read_exact(&mut byte_buf) {
                Err(reason) => Err(format!("Body read failed: {}", reason)),
                Ok(()) => {
                    // Make a string from the buffer contents:

                    match std::str::from_utf8(&byte_buf) {
                        Ok(body) => {
                            if let Err(s) = self
                                .mirror_directory
                                .lock()
                                .unwrap()
                                .add(&format!("{}", self.peer.ip()), body)
                            {
                                Err(format!("Failed to make directory entry {}", s))
                            } else {
                                self.shm_info = Some(String::from(body));
                                Ok(())
                            }
                        }
                        Err(err) => Err(format!("shm info was not valid UTF-8 {}", err)),
                    }
                }
            }
        } else {
            Err(String::from("SHM_INFO requires a non-null body"))
        }
    }
    // Process a full update request.
    // In this version of MirrorServerInstance only FULL_UPDATE replies are given.
    // A later version will support caching information about the shared memory
    // header so that we can determine if a partial u pdate is possible.
    //
    
    fn process_full_update(&mut self) -> Result<(), String> {
    
        let shm_header_size = mem::size_of::<XamineSharedMemory>();
        let shm_spectrum_size = self.size_spectrum_region();
        let shm_ptr = self.make_update_pointer(shm_header_size + shm_spectrum_size);
        let msg_header = MessageHeader {
            msg_size: (mem::size_of::<MessageHeader>() + shm_header_size + shm_spectrum_size)
                as u32,
            msg_type: FULL_UPDATE,
        };
        let msg_body = unsafe { shm_ptr.as_ref().unwrap() };
        if let Err(s) = msg_header.write(&mut self.socket) {
            return Err(format!("Failed to write update header: {}", s));
        }
        if let Err(reason) = self.socket.write_all(msg_body) {
            return Err(format!("Failed to write update body: {}", reason));
        }
        self.socket.flush().expect("Failed toflush socket");

        Ok(())
    
    }
    // Process a partial update request:
    //
    
    // Process and update request:
    //
    // * There must be no body in the request message.
    // * We determien how large the used part of the shared memory region is.
    // * We get a pointer to the shared memory region cast as a [u8]
    // * Turning that into a reference we can then write the header and it
    // to the socket.
    //
    fn process_update(&mut self, body_size: usize) -> Result<(), String> {
        if body_size == 0 {
            self.process_full_update()
        } else {
            Err(String::from("REQUEST_UPDATES must not have a body"))
        }
    }

    ///
    /// Create a new server instance.
    /// Normally a MirrorServerInstance will accept a connection
    /// request by spawning a thread that wille
    /// invoke new for MirrorServerInstance to create the server instance state
    /// and then run on the instance to execute that server.
    /// The server runs until:
    ///
    /// * The peer closes the socket.
    /// * The peer sends a blatantly illegal request.
    ///
    pub fn new(
        shm_name: &str,
        sock: TcpStream,
        dir: SharedMirrorDirectory,
    ) -> MirrorServerInstance {
        // Map the shared memory.

        match File::open(shm_name) {
            Ok(f) => {
                if let Ok(map) = unsafe { memmap::Mmap::map(&f) } {
                    let p = map.as_ptr() as *const XamineSharedMemory;

                    let peer = sock
                        .peer_addr()
                        .expect("MirrorServerInstance getting peer addr");
                    MirrorServerInstance {
                        shared_memory_map: map,
                        shared_memory: p,
                        socket: sock.try_clone().unwrap(),
                        peer,
                        mirror_directory: dir.clone(),
                        shm_info: None,
                        digest: None
                    }
                } else {
                    sock.shutdown(Shutdown::Both)
                        .expect("Shutting down socket in new");
                    panic!("Map of shared memory failed");
                }
            }
            Err(reason) => {
                sock.shutdown(Shutdown::Both)
                    .expect("Shutting down socket in new");
                panic!("Failed to map shared memory in server instance: {}", reason);
            }
        }
        // The shared image backing file will close here when f is droppted
        // but the mapping will be retained.
    }
    /// Run the server.  Once a MirrorServerInstance has been
    /// created via new, run should be called to allow that instance to
    /// process client requests.
    ///
    /// This function will only exit when the client disconnects or if
    /// it sends us a patently bad request.
    /// Error messages will be sent to stderr as there's not really much of
    /// a better place to send them.
    ///
    fn run(&mut self) {
        loop {
            match MessageHeader::read(&mut self.socket) {
                Ok(header) => match header.msg_type {
                    SHM_INFO => {
                        if let Err(s) = self.process_shminfo(header.body_size()) {
                            eprintln!(
                                "MirrorServerInstance Invalid SHM_INFO request from {} : {}",
                                self.peer, s
                            );
                            break;
                        }
                    }
                    REQUEST_UPDATE => {
                        if let Err(s) = self.process_update(header.body_size()) {
                            eprintln!(
                                "MirrorServerInstance - invalid REQUEST_UPDATE from {} : {}",
                                self.peer, s
                            );
                            break;
                        }
                    }
                    _ => {
                        eprintln!(
                            "MirrorServerInstance invalid request type from {} : {}",
                            self.peer, header.msg_type
                        );
                        break;
                    }
                },
                Err(s) => {
                    eprintln!(
                        "MirrorServerInstance failed to read header from {} : {}",
                        self.peer, s
                    );
                    break;
                }
            }
        }
        eprintln!("Shutting down MirrorServerInstance");
        // Remove our mirror entry if possible but ignore errors cause there's
        // not much we can do if there is one:
        if let Some(shm) = &self.shm_info {
            let _ = self
                .mirror_directory
                .lock()
                .unwrap()
                .remove(&format!("{}", self.peer.ip()), shm);
        }

        // Shutdown the socket rather than letting it linger.
        let _ = self.socket.shutdown(Shutdown::Both); // Ignore shutdown errors.
    }
}
/// MirrorServer listens for connections and, spawns off a MirrorServerInstance thread
/// to handle requests by the connected client.
/// The server is the owner of the initial copy of the shared mirror directory
/// it also is given the patht ot the shared memory backing file.  These
/// are all passed to the thread.
///
///  As with all server objects, running the server is a two step process:
///
///  * new is invoked to pass the server any data it must store in its struct.
///  * run is invoked to actually run the server.  
///
/// Normally this is done in a thread
/// One interesting design point - we want the server listener to be able to
/// exit but Rust's TcpListener doesn't really give us a mechanism to do that.
/// What we do is use a tricky combination to exit this server:
/// *  The server is instantiated given a receiver of bools.
/// *  After each connection is processe, a recv_timeout is called on the
///  channel with a very short timeout, and the listener exits if data are
///  received.  Therefor to force an exit of the server:
/// *   Send a bool, any bool, to the Sender side of the channel,
/// *   Make a connection to the server.
/// *   Close the TCP/IP connection - that will force the server instance to exit.
///
/// What we can't do with this method, sadly, is to force the server instance
/// threads to exit.
///
pub struct MirrorServer {
    port: u16,                               // Listener port.
    shm_name: String,                        // Path to the shared memory region.
    mirror_directory: SharedMirrorDirectory, // Registered mirrors.
    exit_req: Receiver<bool>,                // Send here to request exit after next connection.
}
impl MirrorServer {
    // handle a new client:
    // Start a thread that creates a new MirrorServerInstance and runs it:

    fn start_server_instance(&mut self, socket: TcpStream) {
        let shm_name = self.shm_name.clone();
        let dir = self.mirror_directory.clone();
        thread::spawn(move || {
            let mut instance = MirrorServerInstance::new(&shm_name, socket, dir);
            instance.run();
        });
    }

    /// Create the instance of the MirrorServer - run must still be called
    /// to execute the server code.

    pub fn new(
        listen_port: u16,
        shm_file: &str,
        exit_req: Receiver<bool>,
        mirror_dir: SharedMirrorDirectory,
    ) -> MirrorServer {
        MirrorServer {
            port: listen_port,
            shm_name: String::from(shm_file),
            mirror_directory: mirror_dir,
            exit_req,
        }
    }
    /// Called to run the server.  The typical game is to spawn a thread
    /// Which
    /// * Instantiates the listener giving the the port and the receiver
    /// side of a channel pair that was created as well as the shared memory
    /// backing file path.
    /// *  Invokes run() to actually run the server.
    ///
    pub fn run(&mut self) {
        let listener = TcpListener::bind(&format!("0.0.0.0:{}", self.port))
            .expect("Unable to listen on server port");
        let timeout = Duration::from_micros(100); // Suitably short.

        for client in listener.incoming() {
            if let Ok(client) = client {
                self.start_server_instance(client);
            }
            match self.exit_req.recv_timeout(timeout) {
                Ok(_) => break,
                Err(reason) => {
                    match reason {
                        RecvTimeoutError::Disconnected => break, // no senders left...
                        RecvTimeoutError::Timeout => {}          // Keep accepting connections.
                    }
                }
            }
        }
    }
}

//------------------- unit tests ---------------------------

#[cfg(test)]
mod header_tests {
    use super::*;
    use std::ptr;

    #[test]
    fn write_1() {
        let mut buffer: Vec<u8> = Vec::with_capacity(mem::size_of::<MessageHeader>());
        let header = MessageHeader {
            msg_size: mem::size_of::<MessageHeader>() as u32,
            msg_type: SHM_INFO,
        };
        header.write(&mut buffer).expect("Failed write");
        assert_eq!(mem::size_of::<MessageHeader>(), buffer.len());
        assert_eq!(
            header.msg_size,
            u32::from_ne_bytes(buffer.as_slice()[0..4].try_into().unwrap())
        );
        assert_eq!(
            header.msg_type,
            u32::from_ne_bytes(buffer.as_slice()[4..].try_into().unwrap())
        );
    }
    #[test]
    fn read_1() {
        // NOrmal read:
        // Make a message and a byte buffer into which it wil be put.
        let hdr_size = mem::size_of::<MessageHeader>();
        let header = MessageHeader {
            msg_size: hdr_size as u32,
            msg_type: SHM_INFO,
        };
        let mut buffer = Vec::<u8>::new();
        buffer.resize(hdr_size, 0);
        // Now do the rust magic of copying it into the buffer:

        let hdr_ptr: *const MessageHeader = &header;
        let hdr_as_bytes: *const u8 = hdr_ptr as *const u8;
        let pbuffer = buffer.as_mut_ptr();
        unsafe {
            ptr::copy(hdr_as_bytes, pbuffer, hdr_size);
        }
        // Do the read from the buffer:

        let read_header =
            MessageHeader::read(&mut buffer.as_slice()).expect("Failed to read header");
        assert_eq!(header.msg_size, read_header.msg_size);
        assert_eq!(header.msg_type, read_header.msg_type);
    }
    #[test]
    fn read_2() {
        // Read _but_ invalid message type:

        let hdr_size = mem::size_of::<MessageHeader>();
        let header = MessageHeader {
            msg_size: hdr_size as u32,
            msg_type: 1000,
        };

        let mut buffer = Vec::<u8>::new();
        buffer.resize(hdr_size, 0);
        // Now do the rust magic of copying it into the buffer:

        let hdr_ptr: *const MessageHeader = &header;
        let hdr_as_bytes: *const u8 = hdr_ptr as *const u8;
        let pbuffer = buffer.as_mut_ptr();
        unsafe {
            ptr::copy(hdr_as_bytes, pbuffer, hdr_size);
        }
        // Do the read from the buffer:

        let read_header = MessageHeader::read(&mut buffer.as_slice());
        assert!(read_header.is_err());
    }
    #[test]
    fn bodysize_1() {
        // No residual after the header:

        let hdr_size = mem::size_of::<MessageHeader>();
        let header = MessageHeader {
            msg_size: hdr_size as u32,
            msg_type: SHM_INFO,
        };
        assert_eq!(0, header.body_size());
    }
    #[test]
    fn bodysize_2() {
        // Non zero body:
        let hdr_size = mem::size_of::<MessageHeader>();
        let body_size: u32 = 100;
        let header = MessageHeader {
            msg_size: hdr_size as u32 + body_size,
            msg_type: SHM_INFO,
        };
        assert_eq!(body_size as usize, header.body_size());
    }
}

#[cfg(test)]
mod dentry_tests {
    use super::*;

    #[test]
    fn new_1() {
        let entry = DirectoryEntry::new("localhost", "file:/some/path");
        assert_eq!("localhost", entry.host);
        assert_eq!("file:/some/path", entry.key);
    }
    #[test]
    fn host_1() {
        let entry = DirectoryEntry::new("localhost", "file:/some/path");
        assert_eq!("localhost", entry.host().as_str());
    }
    #[test]
    fn key_1() {
        let entry = DirectoryEntry::new("localhost", "file:/some/path");
        assert_eq!("file:/some/path", entry.key().as_str());
    }
}
#[cfg(test)]
mod directory_tests {
    use super::*;
    #[test]
    fn new_1() {
        let dir = Directory::new();
        assert_eq!(0, dir.items.len());
    }
    #[test]
    fn add_1() {
        // add no failure:

        let mut dir = Directory::new();
        let result = dir.add("localhost", "file:/test/path");
        assert!(result.is_ok());
        let key = Directory::compute_index("localhost", "file:/test/path");
        assert!(dir.items.contains_key(&key));
        let contents = dir.items.get(&key).expect("Didn't find item");
        assert_eq!("localhost", contents.host());
        assert_eq!("file:/test/path", contents.key());
    }
    #[test]
    fn add_2() {
        // duplicates fail to add:

        let mut dir = Directory::new();
        let host = "localhost";
        let key = "file:/some/path";

        dir.add(host, key).expect("added ok");
        let result = dir.add(host, key); // Should be err:
        assert!(result.is_err());
        assert_eq!(
            format!("The host/key pair {} {} are already registered", host, key),
            result.unwrap_err().as_str()
        );
    }
    #[test]
    fn iterator_1() {
        // Be able to iterate over values..

        let mut dir = Directory::new();
        // stock it:

        let hosts = vec!["host1", "host2", "host3"];
        let keys = vec!["file1", "file2", "file3"];
        for (i, h) in hosts.iter().enumerate() {
            dir.add(h, &keys[i]).expect("add failed");
        }

        let mut contents = Vec::<DirectoryEntry>::new();
        for e in dir.iter() {
            contents.push(e.clone());
        }
        // contents will be in random order so:

        contents.sort_by_key(|e| e.host.clone());
        assert_eq!(3, contents.len());
        for (i, e) in contents.iter().enumerate() {
            assert_eq!(hosts[i], e.host(), "Failed host compare on {}", i);
            assert_eq!(keys[i], e.key(), "Failed key compare on {}", i);
        }
    }
    #[test]
    fn remove_1() {
        // Remove a nonexistent item:

        let host = "localhost";
        let key = "file:/some/key";
        let mut dir = Directory::new();

        let r = dir.remove(&host, &key);
        assert!(r.is_err());

        assert_eq!(
            format!("No such entry for {} {}", host, key),
            r.unwrap_err()
        );
    }
    #[test]
    fn remove_2() {
        // Remove from amongst some.

        let mut dir = Directory::new();
        let hosts = vec!["host1", "host2", "host3"];
        let keys = vec!["file1", "file2", "file3"];
        for (i, h) in hosts.iter().enumerate() {
            dir.add(h, &keys[i]).expect("add failed");
        }

        // remove the first one:

        dir.remove(&hosts[0], &keys[0])
            .expect("Remove should have worked.");
        let lookup = Directory::compute_index(&hosts[0], &keys[0]);
        assert!(!dir.items.contains_key(&lookup));
    }
}
// Tests of the mirror server, if I can figure those out.

// ************************ IMPORTANT NOTE: ******************************
// A bit about these tests:
// cargo test, by default, runs several tests in threaded parallel.  In this
// case, since there's a TCP/IP server involved, it's necessary to be sure that each
// server listens on a different port.  I don't know how to force this to happen
// as again, data shared between those tests might not work properly to allocate
// a port.
// So
// Suggested form of tests is literals in {} are intended to be substituted:
//
// ```
//    #[test]
//    fn a_test()  {
//       let off = {some integer unique across the tests};
//       let (mem_name, sender) = setup(SERVER_PORT + off, {xamine-spectrumsize});
//       let mut connection = connect_server(off);
//       ...
///      teardown(&sender, off)
//    }
//
// Each test's server then listens on the unique port SERVER_PORT + off and the
// connections get made to the test's server which is also torn down properly.
//
// Convention is that each 'textually next' test uses an offset one larger
// than the textually previous test.

#[cfg(test)]
mod mirror_server_tests {
    use super::*;
    use memmap;
    use std::mem;
    use std::net::{Shutdown, TcpStream};
    use std::sync::mpsc::{channel, Sender};
    use tempfile;
    use thread;

    const SERVER_PORT: u16 = 10000;

    fn init_memory(mem: &mut XamineSharedMemory) {
        for i in 0..XAMINE_MAXSPEC {
            mem.dsp_types[i] = SpectrumTypes::Undefined;
        }
    }
    fn create_shared_memory(spec_bytes: usize) -> tempfile::NamedTempFile {
        let total_size = mem::size_of::<XamineSharedMemory>() + spec_bytes;
        let file = tempfile::NamedTempFile::new().expect("Creating shared mem tempfile");
        file.as_file()
            .set_len(total_size as u64)
            .expect("Failed to set file size");

        let map = unsafe { memmap::MmapMut::map_mut(file.as_file()) };
        let mut map = map.expect("Mapping shared memory");

        let pmap = map.as_mut_ptr() as *mut XamineSharedMemory;
        let memory = unsafe { pmap.as_mut().unwrap() };

        init_memory(memory);

        file
    }
    // INitialize shared memory as expectedy by mirror_2:

    fn init_mirror_2shm(mem_file: &tempfile::NamedTempFile) {
        let mut map =
            unsafe { memmap::MmapMut::map_mut(mem_file.as_file()) }.expect("mapping shared memory");

        let header = unsafe {
            (map.as_mut_ptr() as *mut XamineSharedMemory)
                .as_mut()
                .unwrap()
        };
        let mut psoup =
            unsafe { (map.as_mut_ptr() as *mut XamineSharedMemory).offset(1) as *mut u32 };
        // define slot 0 as a u32 1d spectrum with 1024 channels:

        header.dsp_xy[0].xchans = 1024;
        header.dsp_xy[0].ychans = 1;
        header.dsp_offsets[0] = 0;
        header.dsp_types[0] = SpectrumTypes::OnedLong;

        for i in 0..1024 {
            unsafe {
                *psoup = i;
                psoup = psoup.add(1);
            }
        }
    }
    // initialize the shared memory for the mirror_3 test:
    // spectrum slot 3 is 10x10 TwodLong at offset 0.
    // spectrum slot 1 is a 1024 OnedLong at offset 1024.

    fn init_mirror_3shm(mem_file: &tempfile::NamedTempFile) {
        let mut map =
            unsafe { memmap::MmapMut::map_mut(mem_file.as_file()) }.expect("mapping shared memory");

        let header = unsafe {
            (map.as_mut_ptr() as *mut XamineSharedMemory)
                .as_mut()
                .unwrap()
        };

        // Define slot 1 as 1024 Onedlong offset 1024.
        // This determines how much of the spectrum soup is sent.
        header.dsp_xy[1].xchans = 1024;
        header.dsp_xy[1].ychans = 1;
        header.dsp_offsets[1] = 1024;
        header.dsp_types[1] = SpectrumTypes::OnedLong;

        // define slot 3 as 10x10 TwodLong at offset 0.

        header.dsp_xy[3].xchans = 10;
        header.dsp_xy[3].ychans = 10;
        header.dsp_offsets[3] = 0;
        header.dsp_types[3] = SpectrumTypes::TwodLong;

        // Fill in spectrum1.
        // (offset 1024)

        let mut psoup =
            unsafe { (map.as_mut_ptr() as *mut XamineSharedMemory).offset(1) as *mut u32 };
        unsafe {
            psoup = psoup.add(1024);
        }
        for i in 0..1024 {
            unsafe {
                *psoup = i;
                psoup = psoup.add(1);
            }
        }

        // Fill in Spectrum3.

        let mut psoup =
            unsafe { (map.as_mut_ptr() as *mut XamineSharedMemory).offset(1) as *mut u32 };
        for x in 0..10 {
            for y in 0..10 {
                unsafe {
                    *psoup = x + y * 10;
                    psoup = psoup.add(1);
                }
            }
        }
    }

    // Common set up code.
    // We need to:
    // - Make a Shared memory file.
    // - Initialize the header.
    // - Start a mirror server on that file.
    // - Return the send side of the exit request channel.
    fn setup(port: u16, spectrum_size: usize) -> (tempfile::NamedTempFile, Sender<bool>) {
        let (sender, receiver) = channel::<bool>();
        let shm = create_shared_memory(spectrum_size);

        let thread_shm = format!("{}", shm.path().display());
        let dir = Arc::new(Mutex::new(Directory::new()));
        thread::spawn(move || {
            let mut server = MirrorServer::new(port, &thread_shm, receiver, dir);
            server.run();
        });
        thread::sleep(Duration::from_millis(500)); // so the thread can listen.
        (shm, sender)
    }
    fn connect_server(port_offset: u16) -> TcpStream {
        TcpStream::connect(&format!("127.0.0.1:{}", SERVER_PORT + port_offset))
            .expect("Connecting to mirror server")
    }

    // Common tear down code:
    //  - Send a request to stop the server.
    //  - Send a connection request to the server.
    //  - Close our connection.
    //  - Delete the shared memory image file.
    fn teardown(sender: &Sender<bool>, offset: u16) {
        // this sleep is in case tests are fast enough that the send below gets processed
        // before the connection:

        thread::sleep(Duration::from_millis(100));
        sender.send(false).expect("Sending halt request to server");
        let stream = connect_server(offset);
        stream
            .shutdown(Shutdown::Both)
            .expect("Shutting down client stream");

        // Let the server instance start and exit before allowing the
        // shared memory file to drop:

        thread::sleep(Duration::from_millis(100));
    }
    #[test]
    fn infrastructure_1() {
        let (_, sender) = setup(SERVER_PORT, 0);
        teardown(&sender, 0);
    }
    #[test]
    fn connect_1() {
        // I canconnect to the server:

        let (_, sender) = setup(SERVER_PORT + 1, 0);

        let stream = connect_server(1);

        stream
            .shutdown(Shutdown::Both)
            .expect("Failed to shutdown client test stream");

        teardown(&sender, 1);
    }
    #[test]
    fn shm_info_1() {
        // A new shared memory name works fine.

        let (mem_name, sender) = setup(SERVER_PORT + 2, 0);

        let mut stream = connect_server(2);
        let mut msg_body = String::from("file:");
        msg_body.push_str(&format!("{}", mem_name.path().display()));

        let msg_len = mem::size_of::<MessageHeader>() + msg_body.len();
        let header = MessageHeader {
            msg_size: msg_len as u32,
            msg_type: SHM_INFO,
        };
        header
            .write(&mut stream)
            .expect("Failed to write SHM_INFO header");
        stream
            .write_all(msg_body.as_bytes())
            .expect("Failed to write SHM_INFO body");

        // Stream should still be open...test by trying to write a byte
        //
        let byte: [u8; 1] = [0; 1];
        let result = stream.write_all(&byte);
        assert!(result.is_ok());

        teardown(&sender, 2);
    }
    #[test]
    fn shm_info_2() {
        // Duplicate shared memory region on same sever should fail:

        let (mem_name, sender) = setup(SERVER_PORT + 3, 0);

        let mut stream = connect_server(3);
        let mut msg_body = String::from("file:");
        msg_body.push_str(&format!("{}", mem_name.path().display()));

        let msg_len = mem::size_of::<MessageHeader>() + msg_body.len();
        let header = MessageHeader {
            msg_size: msg_len as u32,
            msg_type: SHM_INFO,
        };
        header
            .write(&mut stream)
            .expect("Failed to write SHM_INFO header");
        stream
            .write_all(msg_body.as_bytes())
            .expect("Failed to write SHM_INFO body");

        // Write it again and the stream will get closed:

        header
            .write(&mut stream)
            .expect("Failed to write SHM_INFO header");
        stream
            .write_all(msg_body.as_bytes())
            .expect("Failed to write SHM_INFO body");

        // Stream should have closed because this is not allowed:

        let mut byte = [0; 1];
        let result = stream.read_exact(&mut byte);
        assert!(result.is_err());

        teardown(&sender, 3);
    }
    #[test]
    fn shm_info_3() {
        // Duplicate shm info between server instances should fail
        // truly a shared directory:

        // Duplicate shared memory region on same sever should fail:

        let (mem_name, sender) = setup(SERVER_PORT + 4, 0);

        let mut stream = connect_server(4);
        let mut msg_body = String::from("file:");
        msg_body.push_str(&format!("{}", mem_name.path().display()));

        let msg_len = mem::size_of::<MessageHeader>() + msg_body.len();
        let header = MessageHeader {
            msg_size: msg_len as u32,
            msg_type: SHM_INFO,
        };

        header
            .write(&mut stream)
            .expect("Failed to write SHM_INFO header");
        stream
            .write_all(msg_body.as_bytes())
            .expect("Failed to write SHM_INFO body");

        // Write it again and the stream will get closed:

        thread::sleep(Duration::from_millis(250)); // be sure it's processed second.
        let mut stream1 = connect_server(4);
        header
            .write(&mut stream1)
            .expect("Failed to write SHM_INFO header");
        stream1
            .write_all(msg_body.as_bytes())
            .expect("Failed to write SHM_INFO body");

        // Peer should have disconnected:

        let mut buf = [0; 1];
        let result = stream1.read_exact(&mut buf);
        assert!(result.is_err());

        teardown(&sender, 4);
    }
    #[test]
    fn shm_info_4() {
        // Once a shared memory client has closed its connection,
        // it's shminfo gets released:

        let (mem_name, sender) = setup(SERVER_PORT + 5, 0);

        let mut stream = connect_server(5);
        let mut msg_body = String::from("file:");
        msg_body.push_str(&format!("{}", mem_name.path().display()));

        let msg_len = mem::size_of::<MessageHeader>() + msg_body.len();
        let header = MessageHeader {
            msg_size: msg_len as u32,
            msg_type: SHM_INFO,
        };

        header
            .write(&mut stream)
            .expect("Failed to write SHM_INFO header");
        stream
            .write_all(msg_body.as_bytes())
            .expect("Failed to write SHM_INFO body");

        stream
            .shutdown(Shutdown::Both)
            .expect("Failed to shutdown stream");

        // This shared memory region should be registerable:

        let mut stream = connect_server(5);
        header
            .write(&mut stream)
            .expect("Failed to write SHM_INFO header");
        stream
            .write_all(msg_body.as_bytes())
            .expect("Failed to write shm info body");

        // If I've lost it I can't write another shm header e.g.:

        let result = header.write(&mut stream);
        let result1 = stream.write_all(msg_body.as_bytes());

        // While the server should (and shm_info_2 says it does) reject this
        // request both results should be ok:

        assert!(result.is_ok());
        assert!(result1.is_ok());

        teardown(&sender, 5);
    }
    //------------------------------------------------------------------------
    // Tests for the actual mirroring operation.

    #[test]
    fn mirror_1() {
        // Initially I'll get a shared memory header with nothing else
        // and no spectra defined:

        let offset = 7;
        let (mem, sender) = setup(SERVER_PORT + offset, 0);

        // Registering the region isn't strictly required but we do it:

        let mut stream = connect_server(offset);
        let mut msg_body = String::from("file:");
        msg_body.push_str(&format!("{}", mem.path().display()));
        // msg_body.shrink_to_fit();
        let msg_len = mem::size_of::<MessageHeader>() + msg_body.len();
        let header = MessageHeader {
            msg_size: msg_len as u32,
            msg_type: SHM_INFO,
        };

        header
            .write(&mut stream)
            .expect("Failed to write SHM_INFO header");
        stream
            .write_all(msg_body.as_bytes())
            .expect("Failed to write SHM_INFO body");

        // now request the update, read the header and be sure it's right:
        // full update and the size is the size of a message header + XamineSharedMemory size.

        thread::sleep(Duration::from_millis(500));
        let header = MessageHeader {
            msg_size: mem::size_of::<MessageHeader>() as u32,
            msg_type: REQUEST_UPDATE,
        };

        header
            .write(&mut stream)
            .expect("Failed to request an update");
        stream.flush().expect("Flushing stream failed");
        let reply_header = MessageHeader::read(&mut stream).expect("Failed to read update header");
        assert_eq!(
            mem::size_of::<MessageHeader>() + mem::size_of::<XamineSharedMemory>(),
            reply_header.msg_size as usize
        );
        assert_eq!(FULL_UPDATE, reply_header.msg_type);

        // Read the data - this requires some skulduggery to get a reference
        // to [u8]:

        let mut mirror_bytes = Vec::<u8>::new();
        mirror_bytes.resize(mem::size_of::<XamineSharedMemory>(), 0);
        stream
            .read_exact(&mut mirror_bytes)
            .expect("Reading mirror.");

        let pmirror_bytes = mirror_bytes.as_ptr(); // ptr to u8
                                                   // All spectra should be undefined...that's the only other thing we can check:
        let pmirror = pmirror_bytes as *const XamineSharedMemory;
        let mirror = unsafe { pmirror.as_ref().expect("pmirror nonzero") };

        for i in 0..XAMINE_MAXSPEC {
            assert_eq!(
                SpectrumTypes::Undefined,
                mirror.dsp_types[i],
                "Spectrum {} is not undefined",
                i
            );
        }

        teardown(&sender, offset);
    }
    #[test]
    fn mirror_2() {
        // Define a spectrum and see if the mirror returns

        let offset = 8;
        let (mem, sender) = setup(SERVER_PORT + offset, 1024 * 1024); //1mb of spectrum.

        // Initialize 1, 1-d spectrum - slot 0, offset 0, contains a counting pattern.
        // 1024 u32's long.

        init_mirror_2shm(&mem);
        let mut stream = connect_server(offset);

        // Send the upate request:

        let header = MessageHeader {
            msg_size: mem::size_of::<MessageHeader>() as u32,
            msg_type: REQUEST_UPDATE,
        };

        header
            .write(&mut stream)
            .expect("Failed to request an update");
        stream.flush().expect("Flushing stream failed");

        // Request the reply:

        let reply_header = MessageHeader::read(&mut stream).expect("Failed to read update header");
        assert_eq!(
            mem::size_of::<MessageHeader>()
                + mem::size_of::<XamineSharedMemory>()
                + 1024 * mem::size_of::<u32>(),
            reply_header.msg_size as usize
        );
        assert_eq!(FULL_UPDATE, reply_header.msg_type);

        // Now the mirror data:

        let mut mirror_bytes = Vec::<u8>::new();
        mirror_bytes.resize(
            mem::size_of::<XamineSharedMemory>() + 1024 * mem::size_of::<u32>(),
            0,
        );

        stream
            .read_exact(&mut mirror_bytes)
            .expect("Reading mirror_2 data");

        // Make a pointer to the header and the soup:

        let pmirror_bytes = mirror_bytes.as_ptr(); // ptr to u8
                                                   // All spectra should be undefined...that's the only other thing we can check:
        let pmirror = pmirror_bytes as *const XamineSharedMemory;
        let mirror = unsafe { pmirror.as_ref().expect("pmirror nonzero") };
        let mut psoup = unsafe { pmirror.offset(1) as *const u32 };

        // Slot 0 defines a 1d spectrum.

        assert_eq!(mirror.dsp_xy[0].xchans, 1024);
        assert_eq!(mirror.dsp_xy[0].ychans, 1);
        assert_eq!(mirror.dsp_offsets[0], 0);
        assert_eq!(SpectrumTypes::OnedLong, mirror.dsp_types[0]);

        for i in 0..1024 {
            assert_eq!(i, unsafe { *psoup }, "Mismatch on {}", i);
            unsafe { psoup = psoup.add(1) };
        }

        teardown(&sender, offset);
    }
    #[test]
    fn mirror_3() {
        // Spectrum slots 1 and 3 are filled, 3 has offset 0 and is a 10x10 long
        // 2d 1 has offset 1024 and is a 1024 oned.  Both have counting patterns
        //
        let offset = 9;
        let (mem, sender) = setup(SERVER_PORT + offset, 1024 * 1024);

        init_mirror_3shm(&mem);

        let mut stream = connect_server(offset);

        // Send the upate request:

        let header = MessageHeader {
            msg_size: mem::size_of::<MessageHeader>() as u32,
            msg_type: REQUEST_UPDATE,
        };

        header
            .write(&mut stream)
            .expect("Failed to request an update");
        stream.flush().expect("Flushing stream failed");

        // Request the reply:

        let reply_header = MessageHeader::read(&mut stream).expect("Failed to read update header");
        assert_eq!(
            mem::size_of::<MessageHeader>()
                + mem::size_of::<XamineSharedMemory>()
                + 2 * 1024 * mem::size_of::<u32>(), //1024 offset + 1024 channels
            reply_header.msg_size as usize
        );
        assert_eq!(FULL_UPDATE, reply_header.msg_type);

        // Let's get the mirror image now:

        let mut mirror_bytes = Vec::<u8>::new();
        mirror_bytes.resize(
            mem::size_of::<XamineSharedMemory>() + 2 * 1024 * mem::size_of::<u32>(),
            0,
        );
        stream
            .read_exact(&mut mirror_bytes)
            .expect("Reading mirror_image data");

        // Make reference tothe header and check spectrum defs

        let pmirror_bytes = mirror_bytes.as_ptr(); // ptr to u8
                                                   // All spectra should be undefined...that's the only other thing we can check:
        let pmirror = pmirror_bytes as *const XamineSharedMemory;
        let mirror = unsafe { pmirror.as_ref().expect("pmirror nonzero") };
        let psoup = unsafe { pmirror.offset(1) as *const u32 };

        assert_eq!(mirror.dsp_xy[1].xchans, 1024);
        assert_eq!(mirror.dsp_xy[1].ychans, 1);
        assert_eq!(mirror.dsp_offsets[1], 1024);
        assert_eq!(mirror.dsp_types[1], SpectrumTypes::OnedLong);

        assert_eq!(mirror.dsp_xy[3].xchans, 10);
        assert_eq!(mirror.dsp_xy[3].ychans, 10);
        assert_eq!(mirror.dsp_offsets[3], 0);
        assert_eq!(mirror.dsp_types[3], SpectrumTypes::TwodLong);

        // Data for spectrum 1:

        let mut sp1 = unsafe { psoup.offset(1024) };
        for i in 0..1024 {
            assert_eq!(i, unsafe { *sp1 });
            sp1 = unsafe { sp1.add(1) };
        }

        // Data for spectrum 3:

        let mut sp3 = psoup;
        for x in 0..10 {
            for y in 0..10 {
                assert_eq!(x + y * 10, unsafe { *sp3 });
                sp3 = unsafe { sp3.add(1) };
            }
        }

        teardown(&sender, offset);
    }
}
