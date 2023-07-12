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
use std::io::Read;
use std::io::Write;
use std::mem;

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

    fn read_header<T: Read>(f: &mut T) -> Result<MessageHeader, String> {
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

    fn write_header<T: Write>(f: &mut T, hdr: &MessageHeader) -> Result<usize, String> {
        let mut buf: [u8; mem::size_of::<MessageHeader>()] = [0; mem::size_of::<MessageHeader>()];
        buf[0..3].copy_from_slice(&hdr.msg_size.to_ne_bytes()[0..]);
        buf[4..].copy_from_slice(&hdr.msg_type.to_ne_bytes()[0..]);
        ();
        match f.write_all(&buf) {
            Ok(n) => Ok(mem::size_of::<MessageHeader>()),
            Err(e) => Err(format!("Header write failed: {}", e)),
        }
    }
    // Given this header, return the body size:
    ///
    fn body_size(&self) -> usize {
        self.msg_size as usize - mem::size_of::<MessageHeader>()
    }
}
