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
use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::mem;
use std::net::{Shutdown, TcpStream};
use std::slice::from_raw_parts;

use std::sync::{Arc, Mutex};

/// Here are the message type codes for the MessageHeader:
///
/// ### Client request message types:
///
/// * SHM_INFO - provides information to the server about the
/// shared memory key for any local shared memory region it creates.
/// to mirror the Rustogramer shared memory.  This key can be used
/// by clients to ensure only one mirror per host is used
/// (see rest/mirror.rs).  No reply message is sent for this
/// request.  
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

        if let Ok(_) = f.read_exact(&mut buf) {
            let mut result = MessageHeader {
                msg_size: 0,
                msg_type: 0,
            };
            let l: [u8; 4] = buf[0..mem::size_of::<u32>()].try_into().unwrap();
            result.msg_size = u32::from_ne_bytes(l);

            let l: [u8; 4] = buf[mem::size_of::<u32>()..].try_into().unwrap();
            result.msg_type = u32::from_ne_bytes(l);
            Self::validate_type(result)
        } else {
            Err(String::from("Unable to complete message Header read"))
        }
    }
    /// write a messgae header to a writeable.

    fn write<T: Write>(&self, f: &mut T) -> Result<usize, String> {
        let mut buf: [u8; mem::size_of::<MessageHeader>()] = [0; mem::size_of::<MessageHeader>()];
        buf[0..4].copy_from_slice(&self.msg_size.to_ne_bytes()[0..]);
        buf[4..].copy_from_slice(&self.msg_type.to_ne_bytes()[0..]);
        ();
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
#[derive(Clone)]
struct DirectoryEntry {
    host: String,
    key: String,
}

impl DirectoryEntry {
    fn new(host: &str, key: &str) -> DirectoryEntry {
        DirectoryEntry {
            host: String::from(host),
            key: String::from(key),
        }
    }
    fn host(&self) -> String {
        self.host.clone()
    }
    fn key(&self) -> String {
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
struct Directory {
    items: HashMap<String, DirectoryEntry>,
}

impl Directory {
    fn compute_index(host: &str, key: &str) -> String {
        format!("{}.{}", host, key)
    }
    fn new() -> Directory {
        Directory {
            items: HashMap::new(),
        }
    }
    /// adds a new directory entry.
    /// Computes the key and:
    /// *  If it's a duplicate, Errs indicating that.
    /// *  If it's not a duplicate, constructs a DirectoryEntry
    /// and inserts it into the items.
    fn add(&mut self, host: &str, key: &str) -> Result<(), String> {
        let index = Self::compute_index(host, key);
        if self.items.contains_key(&index) {
            Err(format!(
                "The host/key pair {} {} are already registered",
                host, key
            ))
        } else {
            self.items.insert(index, DirectoryEntry::new(host, key));
            Ok(())
        }
    }
    /// Iterate over the DirectoryEntry -s in the directory.
    fn iter(&self) -> Values<'_, String, DirectoryEntry> {
        self.items.values()
    }
    /// Remove an entry from the directory:

    fn remove(&mut self, host: &str, key: &str) -> Result<(), String> {
        let index = Self::compute_index(host, key);
        if let Some(_) = self.items.remove(&index) {
            Ok(())
        } else {
            Err(format!("No such entry for {} {}", host, key))
        }
    }
}

/// In order to maintain a common Directory between threads we
/// need to define a data type which is a mutex locked directory.
/// We'll wrap that in an API that automatically locks and
/// unlocks the directory.
///
type SharedDirectory = Arc<Mutex<Directory>>;

/// The mirror server instance needs:
/// * A TcpStream used to communicate with the
///   client.
/// * A reference to the shared memory header.
/// * A reference to the soup of spectra
///   This will be a reference to an array of u32s.
/// * A SharedDirectory which will be used to register mirrors.
///
struct MirrorServerInstance {
    socket: TcpStream,
    header: *const XamineSharedMemory,
    directory: SharedDirectory,
    partner: Option<DirectoryEntry>,
}

impl MirrorServerInstance {
    // Return a reference to the XamineSharedMemory
    // encapsulates unsafe stuff.
    //
    fn get_header(&self) -> &XamineSharedMemory {
        unsafe { self.header.as_ref().unwrap() }
    }
    // Returns the spectrum shared memory as a reference
    // of the specified size (preparatory to writing it).
    //
    fn get_storage(&self, size: usize) -> &[u8] {
        unsafe { from_raw_parts(self.header as *const u8, size) }
    }
    // really reads an arbitrarily sized buffer.
    fn read_body(&mut self, buffer: &mut [u8]) -> Result<(), String> {
        let result = self.socket.read_exact(buffer);
        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(String::from("Failed to read message body")),
        }
    }
    // Called on thread exit to ensure that the shared directory no longer
    // has our key etc.
    //
    fn clean_shm_info(&mut self) {
        if let Some(partner) = &self.partner {
            let _ = self
                .directory
                .lock()
                .unwrap()
                .remove(&partner.host, &partner.key);
        }
    }
    //
    // Given a pointer to the header, return the index with the biggest
    // offset value.
    // Note:  All rustogrammer offsets are in u32 units so straight compares
    //       are ok.
    // Note:
    //      None is returned if no spectra are in use:
    //
    fn last_index(header: &XamineSharedMemory) -> Option<usize> {
        let mut result = None;

        // Remember it's the _index_ we want not the value.
        for i in 0..XAMINE_MAXSPEC {
            // Only care about defined spectra.
            if header.dsp_types[i] != SpectrumTypes::Undefined {
                let offset = header.dsp_offsets[i];

                if let None = result {
                    // No prior undefined so this is biggest.
                    result = Some(i);
                } else {
                    if offset > header.dsp_offsets[result.unwrap()] {
                        // Otherwise we must be bigger tosave.
                        result = Some(i);
                    }
                }
            }
        }

        result
    }
    // Given the index of a defined spectrum with the biggest offset,
    // determine the _byte_ offset to the end of the spectrum.  Note
    // that dsp_offsets are in units of u32.
    // We assume (valid) that the bind operation sets y size of 1 if
    // the spectrum is one-dimensional
    //
    fn end_of(header: &XamineSharedMemory, index: usize) -> usize {
        let base = header.dsp_offsets[index];
        let size = header.dsp_xy[index].xchans * header.dsp_xy[index].ychans;

        (base + size) as usize * mem::size_of::<u32>()
    }
    // Compute the used size of the shared memory.
    // This is  the size of the header = the biggest used offset (in byts) + the
    // size of that spectrum in bytes.
    //  Note that there's really storage on the back end of the an XamineSharedMemory
    // struct to hold the spectrum channels.
    //

    fn compute_used_size(&self) -> usize {
        let header = self.get_header();
        let header_size = mem::size_of::<XamineSharedMemory>();

        // Figure out the Spectrum with the largest offset, and it's size:
        // It's possible there are no spectra

        let spectrum_usage = if let Some(i) = Self::last_index(header) {
            Self::end_of(header, i)
        } else {
            0
        };

        header_size + spectrum_usage
    }

    // Process SHM_INFO message
    //
    fn enter_shm_info(&mut self, key_size: usize) -> bool {
        // Read the body and make it into a string...

        let mut body = Vec::<u8>::new();
        body.resize(key_size, 0);

        if let Err(s) = self.read_body(&mut body) {
            eprintln!("Badly formed SHM_INFO message: {}", s);
            return false;
        }
        // Conver the peer into a string as required by the directory:

        let shm_key = std::str::from_utf8(&body);
        if let Err(reason) = shm_key {
            eprintln!("Badly formed SHM_INFO body must be valid utf-8: {}", reason);
            return false;
        }
        let shm_key = shm_key.unwrap();

        // figure out our peer:

        let peer = self.socket.peer_addr();
        if let Err(err) = peer {
            eprintln!(
                "Could not determine peer address in enter_shm_info: {}",
                err
            );
            return false;
        }
        let addr_string = format!("{}", peer.unwrap());

        // add this entry to the directory

        match self.directory.lock().unwrap().add(&addr_string, &shm_key) {
            Ok(()) => {
                self.partner = Some(DirectoryEntry::new(&addr_string, &shm_key));
                true
            }
            Err(s) => {
                eprintln!("Failed to make directory entry for {} : {}", addr_string, s);
                false
            }
        }
    }
    // Process REQUEST_UPDATE message:
    //
    // SpecTcl is smart enough to know when bindings change.
    // It can then send either full updates, or, if the client already knows the
    // most recent bindings, full updates.
    // Initially, we'll just always send full updates.
    // Note that even full updates don't send the entire memory block unless
    // the entire shared memory spectrum region is full.
    //
    // At this point we already know we don't have a body so just figure out
    // how much data to send and send it.
    //
    // This encapsulates some unsafe code as we need to figure out the size
    // of the reference to the data to send in order to create a reference to the
    // equivalently sized [u8].
    //
    fn send_update(&mut self) -> bool {
        let num_bytes = self.compute_used_size();

        // Make the header:

        let header = MessageHeader {
            msg_size: (num_bytes + std::mem::size_of::<MessageHeader>()) as u32,
            msg_type: FULL_UPDATE,
        };
        // Send the header and the data:

        if let Err(s) = header.write(&mut self.socket) {
            eprintln!("Failed to send peer header update {}", s);
            return false;
        }
        // Send the body:

        if let Err(s) = self
            .socket
            .try_clone() // needed to drop the self mutable borrow.
            .unwrap()
            .write_all(self.get_storage(num_bytes))
        {
            eprintln!("Failed to send perr shared memory contents: {}", s);
            false
        } else {
            true
        }
    }
    // Public entries:

    /// Create a new server instance.
    /// normal use case is that ther's a server listener which, when it accepts
    /// a connection will create a server instance (using new) and then
    /// spawn a thread with that instance moved into it and the thread will just
    /// do e.g. instance.run() to become a functional Mirror Server instance.
    ///
    pub fn new(
        sock: &TcpStream,
        header: &mut SharedMemory,
        directory: &SharedDirectory,
    ) -> MirrorServerInstance {
        MirrorServerInstance {
            socket: sock.try_clone().expect("Could not clone socket"),
            header: header.get_header(),
            directory: directory.clone(),
            partner: None,
        }
    }
    /// Call this in a thread to run the server instance's logic.
    ///
    pub fn run(&mut self) {
        loop {
            let header = MessageHeader::read(&mut self.socket);
            if header.is_err() {
                break;
            }
            let header = header.unwrap();
            let body_size = header.body_size();

            // The value of the match determines if we keep looping
            // or close the connection and return.
            //
            if !match header.msg_type {
                SHM_INFO => self.enter_shm_info(body_size),
                REQUEST_UPDATE => {
                    // Updates should not have a body:

                    if body_size > 0 {
                        false
                    } else {
                        self.send_update()
                    }
                }
                _ => {
                    // If there's an illegal request punish by
                    // exiting the server.

                    false
                }
            } {
                // If the match value was false break causing a return.
                break;
            }
        }
        // Clean up the shared memory key so a new peer won't get
        // rejected.

        self.clean_shm_info();

        // not much we can really do if shutdown fails -- presumably, if it does,
        // dropping the socket when this struct is destroyed will fix all that.
        let _ = self.socket.shutdown(Shutdown::Both); // close the connection before returning.
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
