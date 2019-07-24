//! The Attribute Protocol
//!
//! The Attribute Protocol is used to expose the attributes of a device through Bluetooth.
//!
//! The Attribute Protocol is the base for the
//! `[Generic Attribute Profile](../gatt/index.html)
//!
//! This is implementation of the Attribute Protocol as defined in the Bluetooth Specification
//! (version 5.0), Vol. 3, Part F.

use alloc::boxed::Box;
use alloc::vec::Vec;

pub mod pdu;
pub mod client;
pub mod server;

use crate::l2cap;

const L2CAP_CHANNEL_ID: l2cap::ChannelIdentifier =
    l2cap::ChannelIdentifier::LE(l2cap::LeUserChannelIdentifier::AttributeProtocol);

/// Avanced Encryption Standard (AES) key sizes
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum EncryptionKeySize {
    Bits128,
    Bits192,
    Bits256,
}

impl EncryptionKeySize {
    /// Used to force an ordering such that Bits128 < Bits192 < Bits256
    fn forced_order_val(&self) -> usize {
        match self {
            EncryptionKeySize::Bits128 => 0,
            EncryptionKeySize::Bits192 => 1,
            EncryptionKeySize::Bits256 => 2,
        }
    }
}

impl PartialOrd for EncryptionKeySize {
    fn partial_cmp(&self, other: &EncryptionKeySize) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EncryptionKeySize {
    fn cmp(&self, other: &EncryptionKeySize) -> core::cmp::Ordering {
        self.forced_order_val().cmp(&other.forced_order_val())
    }
}

/// Attribute premission restriction
///
/// Some attributes permissions are restrictions regarding reading and writing permissions. For
/// those permissions this enum will be used to specify whether the restriction is for reading,
/// writing, or both.
#[derive(Clone,Copy,Debug,PartialEq,Eq,PartialOrd,Ord)]
pub enum AttributeRestriction {
    Read,
    Write
}

#[derive(Clone,Copy,Debug,PartialEq,Eq,PartialOrd,Ord)]
pub enum AttributePermissions {
    /// Readable attributes
    Read,
    /// Writeable attributes
    Write,
    /// Encryption Requirement
    ///
    /// Encryption is required to access the specified attribute permission(s)
    /// A minimum key size may also be requred to access the specified
    Encryption(AttributeRestriction, EncryptionKeySize),
    /// Authentication Requirement
    ///
    /// Authentication is required to access the specified attribute permission(s)
    Authentication(AttributeRestriction),
    /// Authorization Requirement
    ///
    /// Authorization is required to access the specified attribute permission(s)
    Authorization(AttributeRestriction),
}

/// An Attribute
///
/// Attributes contain the information required for a client to get data from a server device. Each
/// attribute contains an attribute type, an attribute handle, and permissions for accessing the
/// attribute data.
///
/// # Attribute Type
/// An attribute type is a UUID used for labeling what the attribute is. It is essentially a
/// 'common noun' for the attribute, so that the client can gather a basic understanding of what
/// the attribute refers too.
///
/// # Handle
/// A reference to the attribute on the server. The client can access specific attributes through
/// the handle value as all handle values on a server are gaurenteed to be unique. This can be
/// handy or required to refer to different attributes (e.g. multiple attributes with the same
/// types ).
///
/// # Permissions
/// Permissions define the accessability and requirements for accessability of the Attribute. The
/// permissions `Read` and `Write` define how the user can access the data, where as the
/// permissions `Encryption`, `Authentication`, and `Authorization` define the conditions where
/// `Read` and `Write` permissions are available to the client.
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Attribute<V> {

    /// The Attribute type
    ty: crate::UUID,

    /// The attribute handle
    ///
    /// The handle is like an address to an attribute. Its how a client refers to and accesses
    /// a specific attribute on a server.
    handle: Option<u16>,

    /// Access Permissions
    permissions: Box<[AttributePermissions]>,

    /// Attribute value
    value: V
}

impl<V> Attribute<V> {

    /// Create an Attribute
    ///
    /// There are four components to an attribute, the type of the attribute, the handle of the
    /// attribute, the access permissions of the attribute, and the value of it. Every part except
    /// for the handle is assigned with the inputs. The handle will be set once the attribute is
    /// pushed on to the server.
    ///
    /// Ihe input 'permissions' will have all duplicates removed.
    pub fn new( attribute_type: crate::UUID, mut permissions: Vec<AttributePermissions>, value: V)
    -> Self
    {
        permissions.sort();
        permissions.dedup();

        Attribute {
            ty: attribute_type,
            handle: None,
            permissions: permissions.into_boxed_slice(),
            value: value,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    /// Returned when there is no connection to the bluetooth controller
    NotConnected,
    /// A PDU exceeds the MTU set between the client and server
    MtuExceeded,
    /// The desired MTU is smaller then the minimum value
    TooSmallMtu,
    /// An Error PDU is received
    Pdu(pdu::Pdu<pdu::ErrorAttributeParameter>),
    /// The only error information gathered is the PDU Error type
    Only(pdu::Error),
    /// A different pdu was expected
    ///
    /// This contains the opcode value of the unexpectedly received pdu
    UnexpectedPdu(u8)
}

impl From<pdu::Error> for Error {
    fn from(err: pdu::Error) -> Error {
        Error::Only(err)
    }
}

impl From<pdu::Pdu<pdu::ErrorAttributeParameter>> for Error {
    fn from(err: pdu::Pdu<pdu::ErrorAttributeParameter>) -> Error {
        Error::Pdu(err)
    }
}

/// ATT Protocol Transmission format
///
/// Structures that implement `TransferFormat` can be converted into the transmitted format
/// (between the server and client) or be constructed from the raw transmitted data.
pub trait TransferFormat {
    /// Make Self from the attribute parameter
    ///
    /// This will attempt to take the passed byte slice and convert it into Self. The byte slice
    /// needs to only be the attribute parameter, it cannot contain either the attribute opcode
    /// or the attribute signature.
    fn from( raw: &[u8] ) -> Result<Self, pdu::Error> where Self: Sized;

    /// Convert Self into the attribute parameter
    fn into(&self) -> Box<[u8]>;
}

macro_rules! impl_transfer_format_for_number {
    ( $num: ty ) => {
        impl TransferFormat for $num {
            fn from( raw: &[u8]) -> Result<Self, pdu::Error> {
                if raw.len() == core::mem::size_of::<$num>() {
                    let mut bytes = <[u8;core::mem::size_of::<$num>()]>::default();

                    bytes.clone_from_slice(raw);

                    Ok(Self::from_le_bytes(bytes))
                } else {
                    Err(pdu::Error::InvalidPDU)
                }
            }

            fn into(&self) -> Box<[u8]> {
                From::<&'_ [u8]>::from(&self.to_le_bytes())
            }
        }
    }
}

impl_transfer_format_for_number!{i8}
impl_transfer_format_for_number!{u8}
impl_transfer_format_for_number!{i16}
impl_transfer_format_for_number!{u16}
impl_transfer_format_for_number!{i32}
impl_transfer_format_for_number!{u32}
impl_transfer_format_for_number!{i64}
impl_transfer_format_for_number!{u64}
impl_transfer_format_for_number!{isize}
impl_transfer_format_for_number!{usize}
impl_transfer_format_for_number!{i128}
impl_transfer_format_for_number!{u128}

impl TransferFormat for alloc::string::String {
    fn from( raw: &[u8] ) -> Result<Self, pdu::Error> {
        alloc::string::String::from_utf8(raw.to_vec()).map_err(|_| pdu::Error::InvalidPDU)
    }

    fn into( &self ) -> Box<[u8]> {
        From::from(self.as_bytes())
    }
}

impl TransferFormat for crate::UUID {
    fn from(raw: &[u8]) -> Result<Self, pdu::Error> {
        use core::mem::size_of;

        if raw.len() == size_of::<u16>() {
            Ok( crate::UUID::from_u16( TransferFormat::from(raw)? ) )
        } else if raw.len() == size_of::<u128>() {
            Ok( crate::UUID::from_u128( TransferFormat::from(raw)? ) )
        } else {
            Err( pdu::Error::InvalidPDU )
        }
    }

    fn into(&self) -> Box<[u8]> {
        match core::convert::TryInto::<u16>::try_into( *self ) {
            Ok(raw) => TransferFormat::into( &raw ),
            Err(_) => TransferFormat::into( &Into::<u128>::into(*self) ),
        }
    }
}

impl<T> TransferFormat for Box<[T]> where T: TransferFormat {

    fn from( raw: &[u8] ) -> Result<Self, pdu::Error> {
        use core::mem::size_of;

        let mut chunks = raw.chunks_exact(size_of::<T>());

        if chunks.remainder().len() == 0 {
            Ok( chunks.try_fold( Vec::new(), |mut v,c| {
                    v.push(TransferFormat::from(&c)?);
                    Ok(v)
                })?
                .into_boxed_slice()
            )
        } else {
            Err(pdu::Error::InvalidPDU)
        }
    }

    fn into(&self) -> Box<[u8]> {
        let mut v = alloc::vec::Vec::new();

        self.iter().for_each(|t| v.extend_from_slice(&TransferFormat::into(t)) );

        v.into_boxed_slice()
    }
}

impl<T> TransferFormat for Box<T> where T: TransferFormat {
    fn from( raw: &[u8] ) -> Result<Self, pdu::Error> {
        <T as TransferFormat>::from(raw).and_then( |v| Ok(Box::new(v)) )
    }

    fn into(&self) -> Box<[u8]> {
        TransferFormat::into( self.as_ref() )
    }
}

impl TransferFormat for () {

    fn from( raw: &[u8] ) -> Result<Self, pdu::Error> {
        if raw.len() == 0 {
            Ok(())
        } else {
            Err(pdu::Error::InvalidPDU)
        }
    }

    fn into(&self) -> Box<[u8]> {
        From::<&[u8]>::from(&[])
    }
}

impl TransferFormat for Box<dyn TransferFormat> {
    /// It is impossible to convert data from its raw form into a Box<dyn TransferFormat>. This
    /// will always return Err(pdu::Error::InvalidPDU)
    fn from( _: &[u8]) -> Result<Self, pdu::Error> {
        Err(pdu::Error::InvalidPDU)
    }

    fn into(&self) -> Box<[u8]> {
        TransferFormat::into(self.as_ref())
    }
}

trait AnyAttribute {

    fn get_type(&self) -> crate::UUID;

    fn get_permissions(&self) -> Box<[AttributePermissions]>;

    fn get_handle(&self) -> u16;

    fn set_val_from_raw(&mut self, raw: &[u8]) -> Result<(), pdu::Error>;

    fn get_val_as_transfer_format<'a>(&'a self) -> &'a dyn TransferFormat;
}

impl<V> AnyAttribute for Attribute<V> where V: TransferFormat + Sized + Unpin {

    fn get_type(&self) -> crate::UUID { self.ty }

    fn get_permissions(&self) -> Box<[AttributePermissions]> { self.permissions.clone() }

    /// This will panic if the handle value hasn't been set yet
    fn get_handle(&self) -> u16 { self.handle.unwrap() }

    fn get_val_as_transfer_format<'a>(&'a self) -> &'a dyn TransferFormat {
        &self.value
    }

    fn set_val_from_raw(&mut self, raw: &[u8]) -> Result<(), pdu::Error> {

        self.value = TransferFormat::from(raw)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::sync::{Arc, Mutex};
    use std::task::Waker;

    struct TwoWayChannel {
        b1: Option<Vec<u8>>,
        w1: Option<Waker>,

        b2: Option<Vec<u8>>,
        w2: Option<Waker>,
    }

    /// Channel 1 sends to b1 and receives from b2
    struct Channel1 {
        two_way: Arc<Mutex<TwoWayChannel>>
    }

    /// Channel 2 sends to b2 and receives from b1
    struct Channel2 {
        two_way: Arc<Mutex<TwoWayChannel>>
    }

    impl TwoWayChannel {
        fn new() -> (Channel1, Channel2) {
            let tc = TwoWayChannel {
                b1: None,
                w1: None,
                b2: None,
                w2: None,
            };

            let am_tc = Arc::new(Mutex::new(tc));

            let c1 = Channel1 { two_way: am_tc.clone() };
            let c2 = Channel2 { two_way: am_tc.clone() };

            (c1, c2)
        }
    }

    impl l2cap::ConnectionChannel for Channel1 {

        const DEFAULT_ATT_MTU: u16 = crate::gap::MIN_ATT_MTU_LE;

        fn send(&self, data: &[u8]) {
            let mut gaurd = self.two_way.lock().expect("Failed to acquire lock");

            gaurd.b1 = Some(data.to_vec());

            if let Some(waker) = gaurd.w1.take() {
                waker.wake();
            }
        }

        fn receive(&self, waker: Waker) -> Option<Box<[u8]>> {
            let mut gaurd = self.two_way.lock().expect("Failed to acquire lock");

            if let Some(data) = gaurd.b2.take() {
                Some(data.into_boxed_slice())
            } else {
                gaurd.w2 = Some(waker);
                None
            }
        }
    }

    impl l2cap::ConnectionChannel for Channel2 {

        const DEFAULT_ATT_MTU: u16 = crate::gap::MIN_ATT_MTU_LE;

        fn send(&self, data: &[u8]) {
            let mut gaurd = self.two_way.lock().expect("Failed to acquire lock");

            gaurd.b2 = Some(data.to_vec());

            if let Some(waker) = gaurd.w2.take() {
                waker.wake();
            }
        }

        fn receive(&self, waker: Waker) -> Option<Box<[u8]>> {
            let mut gaurd = self.two_way.lock().expect("Failed to acquire lock");

            if let Some(data) = gaurd.b1.take() {
                Some(data.into_boxed_slice())
            } else {
                gaurd.w1 = Some(waker);
                None
            }
        }
    }

    #[test]
    fn test_att_connection() {
        use std::thread;

        const UUID_1: u16 = 1;
        const UUID_2: u16 = 2;
        const UUID_3: u16 = 3;

        let test_val_1 = 33usize;
        let test_val_2 = 64u64;
        let test_val_3 = -11i8;

        let (c1,c2) = TwoWayChannel::new();

        thread::spawn( move || {
            use AttributePermissions::*;

            let mut server = server::Server::new( c2, 256, None );

            let attribute_0 = Attribute::new(
                From::from(UUID_1),
                [Read, Write].to_vec().into_boxed_slice(),
                0usize
            );

            let attribute_1 = Attribute::new(
                From::from(UUID_2),
                [Read, Write].to_vec().into_boxed_slice(),
                0u64
            );

            let attribute_3 = Attribute::new(
                From::from(UUID_3),
                [Read, Write].to_vec().into_boxed_slice(),
                0i8
            );

            server.push(attribute_0); // has handle value of 1
            server.push(attribute_1); // has handle value of 2
            server.push(attribute_3); // has handle value of 3

            loop {
                if let Err(e) = futures::executor::block_on( server.on_receive() ) {
                    panic!("Pdu error: {:?}", e);
                }
            }
        });

        let client = futures::executor::block_on(client::Client::connect(c1, 512))
            .expect("Failed to connect attribute client");

        // writing to handle 0
        futures::executor::block_on(client.write_request(1, test_val_1))
            .expect("Failed to write to server for handle 0");

        // writing to handle 1
        futures::executor::block_on(client.write_request(2, test_val_2))
            .expect("Failed to write to server for handle 1");

        // writing to handle 2
        futures::executor::block_on(client.write_request(3, test_val_3))
            .expect("Failed to write to server for handle 2");

        let read_val_1: usize = futures::executor::block_on(client.read_request(1))
            .expect("Failed to read at handle 0 from the server");

        let read_val_2 = futures::executor::block_on(client.read_request(2))
            .expect("Failed to read at handle 1 from the server");

        let read_val_3 = futures::executor::block_on(client.read_request(3))
            .expect("Failed to read at handle 2 from the server");

        assert_eq!(test_val_1, read_val_1);
        assert_eq!(test_val_2, read_val_2);
        assert_eq!(test_val_3, read_val_3);
    }
}
