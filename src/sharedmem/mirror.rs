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
use std::net::{SocketAddr, TcpStream};

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

/// MirrorServerInstance represents an instance of the
/// mirror server.  Each mirror server makes its own map to the
/// shared memory region.  This bypasses the lifetime/Send trait
/// issues encountered when trying to maintain one map and pass
/// pointers/references to threads.
///
/// This initial version of the MirrorServerInstance is stupid
/// and only does full updates.  A later version needs to understand
/// how to do partial updates based on when things change in the
/// shared memory header.

struct MirrorServerInstance {
    shared_memory_map: memmap::Mmap,
    shared_memory: *const XamineSharedMemory,
    socket: TcpStream,
    peer: SocketAddr,
}

impl MirrorServerInstance {
    fn memory(&self) -> &XamineSharedMemory {
        unsafe {self.shared_memory.as_ref().unwrap()}
    }

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
    pub fn new(shm_name: &str, shm_size: usize, sock: TcpStream) -> MirrorServerInstance {
        // Map the shared memory.

        let f = File::open(shm_name).expect("MirrorServerInstance failed to open map file");
        let map = unsafe {
            memmap::Mmap::map(&f).expect("MirrorServerInstance failed to map backing file")
        };
        let p = unsafe { map.as_ptr() as *const XamineSharedMemory };

        let peer = sock.peer_addr().expect("MirrorServerInstance getting peer addr");
        MirrorServerInstance {
            shared_memory_map: map,
            shared_memory: p,
            socket: sock,
            peer: peer
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
