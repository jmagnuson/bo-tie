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

pub mod pdu;
pub mod client;
pub mod server;

/// Attribute premission restriction
///
/// Some attributes permissions are restrictions regarding reading and writing permissions. For
/// those permissions this enum will be used to specify whether the restriction is for reading,
/// writing, or both.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum AttributeRestriction {
    Read,
    Write,
    ReadAndWrite
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum AttributePermissions {
    /// Readable attributes
    Read,
    /// Writeable attributes
    Write,
    /// Encryption Requirement
    ///
    /// Encryption is required to access the specified attribute permission(s)
    Encryption(AttributeRestriction),
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
    pub fn new( attribute_type: crate::UUID, permissions: Box<[AttributePermissions]>, value: V)
    -> Self
    {
        Attribute {
            ty: attribute_type,
            handle: None,
            permissions: permissions,
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
    UnexpectedPdu
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

impl<T> TransferFormat for Box<[T]> where T: TransferFormat {

    fn from( raw: &[u8] ) -> Result<Self, pdu::Error> {
        use alloc::vec::Vec;
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

    fn get_val_as_transfer_format<'a>(&'a self) -> &'a TransferFormat;
}

impl<V> AnyAttribute for Attribute<V> where V: TransferFormat + Sized + Unpin {

    fn get_type(&self) -> crate::UUID { self.ty }

    fn get_permissions(&self) -> Box<[AttributePermissions]> { self.permissions.clone() }

    /// This will panic if the handle value hasn't been set yet
    fn get_handle(&self) -> u16 { self.handle.unwrap() }

    fn get_val_as_transfer_format<'a>(&'a self) -> &'a TransferFormat {
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

    struct TestChannel {
        data: Option<Vec<u8>>,
        waker: Option<Waker>,
    }

    impl crate::gap::ConnectionChannel for Mutex<TestChannel> {
        const DEFAULT_ATT_MTU: u16 = crate::gap::MIN_ATT_MTU_LE;

        fn send(&self, data: &[u8]) {
            let mut gaurd = *self.lock().expect("Failed to acquire lock");

            gaurd.data = Some(data.into_vec());

            if let Some(waker) = gaurd.waker.take() {
                waker.wake();
            }
        }

        fn receive(&self, waker: Waker) -> Option<Box<[u8]>>;
    }
}
