use alloc::{
    vec::Vec,
    boxed::Box,
};
use core::{
    future::Future,
    pin::Pin,
    task::{Poll, Context},
};
use super::{
    client,
    pdu,
    TransferFormat,
};
use crate::l2cap;

#[derive(Debug,Clone,Copy,PartialEq,PartialOrd,Eq)]
pub enum ServerPduName {
    ErrorResponse,
    ExchangeMTUResponse,
    FindInformationResponse,
    FindByTypeValueResponse,
    ReadByTypeResponse,
    ReadResponse,
    ReadBlobResponse,
    ReadMultipleResponse,
    ReadByGroupTypeResponse,
    WriteResponse,
    PrepareWriteResponse,
    ExecuteWriteResponse,
    HandleValueNotification,
    HandleValueIndication,
}

impl core::convert::TryFrom<super::pdu::PduOpCode> for ServerPduName {
    type Error = ();

    fn try_from(opcode: super::pdu::PduOpCode) -> Result<Self, Self::Error> {
        Self::try_from(opcode.into_raw())
    }
}

impl From<ServerPduName> for pdu::PduOpCode {
    fn from(pdu_name: ServerPduName) -> pdu::PduOpCode {
        let raw: u8 = From::from(pdu_name);

        From::from(raw)
    }
}

impl From<ServerPduName> for u8 {
    fn from(name: ServerPduName) -> Self {
        match name {
            ServerPduName::ErrorResponse => 0x1,
            ServerPduName::ExchangeMTUResponse => 0x3,
            ServerPduName::FindInformationResponse => 0x5,
            ServerPduName::FindByTypeValueResponse => 0x7,
            ServerPduName::ReadByTypeResponse => 0x9,
            ServerPduName::ReadResponse => 0xB,
            ServerPduName::ReadBlobResponse => 0xD,
            ServerPduName::ReadMultipleResponse => 0xF,
            ServerPduName::ReadByGroupTypeResponse => 0x11,
            ServerPduName::WriteResponse => 0x13,
            ServerPduName::PrepareWriteResponse => 0x17,
            ServerPduName::ExecuteWriteResponse => 0x19,
            ServerPduName::HandleValueNotification => 0x1B,
            ServerPduName::HandleValueIndication => 0x1D,
        }
    }
}

impl core::convert::TryFrom<u8> for ServerPduName {
    type Error = ();

    fn try_from(val: u8) -> Result<Self, Self::Error> {
        match val {
            0x1  => Ok(ServerPduName::ErrorResponse),
            0x3  => Ok(ServerPduName::ExchangeMTUResponse),
            0x5  => Ok(ServerPduName::FindInformationResponse),
            0x7  => Ok(ServerPduName::FindByTypeValueResponse),
            0x9  => Ok(ServerPduName::ReadByTypeResponse),
            0xB  => Ok(ServerPduName::ReadResponse),
            0xD  => Ok(ServerPduName::ReadBlobResponse),
            0xF  => Ok(ServerPduName::ReadMultipleResponse),
            0x11 => Ok(ServerPduName::ReadByGroupTypeResponse),
            0x13 => Ok(ServerPduName::WriteResponse),
            0x17 => Ok(ServerPduName::PrepareWriteResponse),
            0x19 => Ok(ServerPduName::ExecuteWriteResponse),
            0x1B => Ok(ServerPduName::HandleValueNotification),
            0x1D => Ok(ServerPduName::HandleValueIndication),
            _ => Err(()),
        }
    }
}

impl ServerPduName {

    /// Check that the given raw pdu is this response pdu
    ///
    /// This will loosly check that the size of the pdu is correct and that the opcode value
    /// matches this response. The size of the packet will only be checked for the minimum possible
    /// size and not the maximum allowable size by the connection's ATT_MTU.
    pub(super) fn is_convertable_from( &self, raw_pdu: &[u8] ) -> bool {

        // Each of these check that the size of the packet is correct and the opcode matches
        match self {
            ServerPduName::ErrorResponse => {
                ( raw_pdu.len() == 5 ) && ( raw_pdu[0] == ServerPduName::ErrorResponse.into() )
            },
            ServerPduName::ExchangeMTUResponse => {
                ( raw_pdu.len() == 3 ) && ( raw_pdu[0] == ServerPduName::ExchangeMTUResponse.into() )
            },
            ServerPduName::FindInformationResponse => {
                ( raw_pdu.len() >= 6 ) && ( raw_pdu[0] == ServerPduName::FindInformationResponse.into() )
            },
            ServerPduName::FindByTypeValueResponse => {
                ( raw_pdu.len() >= 5 ) && ( raw_pdu[0] == ServerPduName::FindByTypeValueResponse.into() )
            },
            ServerPduName::ReadByTypeResponse => {
                ( raw_pdu.len() >= 4 ) && ( raw_pdu[0] == ServerPduName::ReadByTypeResponse.into() )
            },
            ServerPduName::ReadResponse => {
                ( raw_pdu.len() >= 1 ) && ( raw_pdu[0] == ServerPduName::ReadResponse.into() )
            },
            ServerPduName::ReadBlobResponse => {
                ( raw_pdu.len() >= 1 ) && ( raw_pdu[0] == ServerPduName::ReadBlobResponse.into() )
            },
            ServerPduName::ReadMultipleResponse => {
                ( raw_pdu.len() >= 1 ) && ( raw_pdu[0] == ServerPduName::ReadMultipleResponse.into() )
            },
            ServerPduName::ReadByGroupTypeResponse => {
                ( raw_pdu.len() >= 6 ) && ( raw_pdu[0] == ServerPduName::ReadByGroupTypeResponse.into() )
            },
            ServerPduName::WriteResponse => {
                ( raw_pdu.len() == 1 ) && ( raw_pdu[0] == ServerPduName::WriteResponse.into() )
            },
            ServerPduName::PrepareWriteResponse => {
                ( raw_pdu.len() >= 5 ) && ( raw_pdu[0] == ServerPduName::PrepareWriteResponse.into() )
            },
            ServerPduName::ExecuteWriteResponse => {
                ( raw_pdu.len() == 1 ) && ( raw_pdu[0] == ServerPduName::ExecuteWriteResponse.into() )
            },
            ServerPduName::HandleValueNotification => {
                ( raw_pdu.len() >= 3 ) && ( raw_pdu[0] == ServerPduName::HandleValueNotification.into() )
            },
            ServerPduName::HandleValueIndication => {
                ( raw_pdu.len() >= 3 ) && ( raw_pdu[0] == ServerPduName::HandleValueIndication.into() )
            },
        }
    }
}

/// The Reserved Handle
///
/// The first handle (value of '0') is reserved for future use. This is used to represent that
/// handle when creating a new Attribute Bearer
struct ReservedHandle;

impl super::AnyAttribute for ReservedHandle {
    fn get_type(&self) -> crate::UUID { Into::<crate::UUID>::into(0u128) }

    fn get_permissions(&self) -> Box<[super::AttributePermissions]> {
        alloc::vec!(super::AttributePermissions::Read).into_boxed_slice()
    }

    fn get_handle(&self) -> u16 { 0 }

    fn get_val_as_transfer_format(&self) -> &dyn TransferFormat { &() }

    fn set_val_from_raw(&mut self, _: &[u8]) -> Result<(), pdu::Error> {
        Err(pdu::Error::InvalidHandle)
    }
}

pub struct RequestProcessor<'a, C>
where C: l2cap::ConnectionChannel
{
    pdu_name: client::ClientPduName,
    pdu_raw_data: Vec<u8>,
    server: &'a mut Server<C>,
}

impl<'a, C> RequestProcessor<'a, C>
where C: l2cap::ConnectionChannel
{
    fn new( pdu_name: client::ClientPduName, pdu_raw_data: Vec<u8>, server: &'a mut Server<C>) -> Self {
        RequestProcessor { pdu_name, pdu_raw_data, server }
    }

    /// Attribute Protocol Process
    ///
    /// This is the processor of client requests **at the Attribute protocol level**. This employs
    /// algorithms that take each attribute weighted equally, meaning there is no relationships
    /// between attributes that are definied in this protocol. Consequently this function is the
    /// usually the slowest way to process an attribute request of the *find* or *by type* kind and
    /// higher layer protocls may have a faster way to process those requests.
    pub fn process_request( &mut self )-> Result<(), pdu::Error>
    {
        match self.pdu_name {
            super::client::ClientPduName::ExchangeMtuRequest =>
                self.server.process_exchange_mtu_request( TransferFormat::from( &self.pdu_raw_data)? ),
            super::client::ClientPduName::WriteRequest =>
                self.server.process_write_request( &self.pdu_raw_data ),
            super::client::ClientPduName::ReadRequest =>
                self.server.process_read_request( TransferFormat::from(&self.pdu_raw_data)? ),
            super::client::ClientPduName::FindInformationRequest =>
                self.server.process_find_information_request( TransferFormat::from(&self.pdu_raw_data)? ),
            super::client::ClientPduName::FindByTypeValueRequest =>
                self.server.process_find_by_type_value_request( &self.pdu_raw_data ),
            super::client::ClientPduName::ReadByTypeRequest =>
                self.server.process_read_by_type_request( TransferFormat::from(&self.pdu_raw_data)? ),
            super::client::ClientPduName::ReadBlobRequest |
            super::client::ClientPduName::ReadMultipleRequest |
            super::client::ClientPduName::ReadByGroupTypeRequest |
            super::client::ClientPduName::WriteCommand |
            super::client::ClientPduName::PrepareWriteRequest |
            super::client::ClientPduName::ExecuteWriteRequest |
            super::client::ClientPduName::HandleValueConfirmation |
            super::client::ClientPduName::SignedWriteCommand =>
                self.server.send_pdu_error(0, self.pdu_name.into(), pdu::Error::RequestNotSupported),
        };

        Ok(())
    }

    /// Get the request type
    pub fn get_request_type(&self) -> client::ClientPduName { self.pdu_name }

    /// Get the received payload
    pub fn get_request_raw_data(&self) -> &[u8] { &self.pdu_raw_data }
}

struct ServerReceiver<'a, C>
where C: l2cap::ConnectionChannel
{
    /// Reference to the attribute server that created this `ServerReceiver`
    server: Option<&'a mut Server<C>>,

}

impl<'a,C> ServerReceiver<'a,C>
where C: l2cap::ConnectionChannel
{
    fn new( server: &'a mut Server<C> ) -> Self {
        ServerReceiver {
            server: Some( server ),
        }
    }
}

impl<'a,C> Future for ServerReceiver<'a,C>
where C: l2cap::ConnectionChannel
{
    type Output = Result<RequestProcessor<'a, C>, super::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {

        let acl_packets_opt = self.server.as_ref().and_then(
            |server| server.connection.receive(cx.waker().clone())
        );

        if let Some(acl_packets) = acl_packets_opt {

            let data = acl_packets.iter()
                .map( |packet| packet.get_payload() )
                .fold( Vec::new(), |mut vec, data| { vec.extend_from_slice(data); vec } );

            if data.len() > 1 {
                match super::client::ClientPduName::try_from(data[0]) {
                    Ok(pdu_name) => {
                        if let Some(server) = self.get_mut().server.take() {
                            Poll::Ready( Ok( RequestProcessor::new(pdu_name, data, server) ) )
                        } else {
                            Poll::Pending
                        }
                    }
                    Err(_) => Poll::Ready(Err(pdu::Error::InvalidPDU.into())),
                }
            } else {
                Poll::Ready(Err(pdu::Error::InvalidPDU.into()))
            }
        } else {
            Poll::Pending
        }
    }
}

/// An Attribute server
///
/// For now a server can only handle one client. It will be updated to handle multiple clients
/// as soon as possible.
pub struct Server<C>
where C: l2cap::ConnectionChannel
{
    /// The maximum mtu that this server can handle. This is also the mtu sent in a MTU response
    /// PDU. This is not the mtu that is decided as the maximum transmit size between the server
    /// and client, that is `set_mtu`.
    max_mtu: u16,
    /// The set mtu between the client and server. If this value is ever None, then the default
    /// value as defined in the connection channel will be used.
    set_mtu: Option<u16>,
    /// The set maximum mtu
    /// TODO: It is temporary that only one connection can be made to this server, capability needs
    /// to be added
    ///
    /// `connection` is compromised of the connection channel and the maximum transfer unit
    connection: (C),
    attributes: Vec<Box<dyn super::AnyAttribute + Unpin>>,
    /// The permissions the client currently has
    given_permissions: Vec<super::AttributePermissions>,
}

impl<C> Server<C>
where C: l2cap::ConnectionChannel
{

    /// Create a new Server
    ///
    /// The maximum transfer unit is set here, it cannot be smaller then the minimum MTU as
    /// specified by the DEFAULT_ATT_MTU constant in trait `l2cap::ConnectionChannel`. If the provided MTU
    /// value is smaller than DEFAULT_ATT_MTU or none is passed, then the MTU will be set to
    /// DEFAULT_ATT_MTU.
    pub fn new<Mtu, A>( connection: C, max_mtu: Mtu, server_attributes: A) -> Self
    where Mtu: Into<Option<u16>>,
          A: Into<Option<ServerAttributes>>
    {
        use alloc::vec;

        let actual_max_mtu = if let Some(val) = max_mtu.into() {
            if val >= C::DEFAULT_ATT_MTU {
                val
            } else {
                C::DEFAULT_ATT_MTU
            }
        } else {
            C::DEFAULT_ATT_MTU
        };

        let attributes: Vec<Box<dyn super::AnyAttribute + Unpin>> = match server_attributes.into()
        {
            Some(a) => a.attributes,
            None => vec!(Box::new(ReservedHandle)),
        };

        Self {
            max_mtu: actual_max_mtu,
            set_mtu: None,
            connection: connection,
            attributes: attributes,
            given_permissions: Vec::new(),
        }
    }

    #[inline]
    fn get_connection_mtu(&self) -> u16 {
        match self.set_mtu { Some(mtu) => mtu, None => C::DEFAULT_ATT_MTU }
    }

    /// Push an attribute onto the handle stack
    ///
    /// This function will return the handle to the attribute.
    ///
    /// # Panic
    /// If you manage to push 65535 attributes onto this server, the next pushed attribute will
    /// cause this function to panic.
    pub fn push<V>(&mut self, attribute: super::Attribute<V>) -> u16
    where V: TransferFormat + Sized + Unpin + 'static
    {
        use core::convert::TryInto;

        let ret = self.attributes.len().try_into().expect("Exceeded attribute handle limit");

        self.attributes.push( Box::new(attribute) );

        ret
    }

    /// Return the next unused handle
    pub fn next_handle(&self) -> u16 { self.attributes.len() as u16 }

    /// Give a permission to the client
    ///
    /// This doesn't check that the client is qualified to receive the permission, it just adds an
    /// indication on the server that the client has it.
    pub fn give_permission_to_clinet(&mut self, permission: super::AttributePermissions) {
        if !self.given_permissions.contains(&permission) {
            self.given_permissions.push(permission);
        }
    }

    /// Validate the permissions of the attribute
    ///
    /// There are two types of permissions that are checked for
    /// * `required` - permissions that *both* the attribute and client must have
    /// * `restricted` - each permission that the client must have if *and only if* the the
    ///   attribute has it.
    ///
    /// If there is an offending permision, that permission is returned, otherwise 'None' is
    /// returned by this function.
    fn validate_permissions(&self,
        att: &dyn super::AnyAttribute,
        required: &[super::AttributePermissions],
        restricted: &[super::AttributePermissions])
    -> Option<pdu::Error>
    {
        let attribute_permissions = att.get_permissions();

        // Both filter closures return true when a permission is not valid
        required.iter()
        .filter(|&&p| {
            attribute_permissions.iter().find(|&&x| x == p).is_none() ||
            self.given_permissions.iter().find(|&&x| x == p).is_none()
        })
        .chain(
            restricted.iter()
            .filter(|&&p| {
                attribute_permissions.iter().find(|&&x| x == p).is_some() &&
                self.given_permissions.iter().find(|&&x| x == p).is_none()
            })
        )
        .map(|p| {
            match p {
                super::AttributePermissions::Read => pdu::Error::ReadNotPermitted,
                super::AttributePermissions::Write => pdu::Error::WriteNotPermitted,
                super::AttributePermissions::Encryption(rest, _) => {
                    attribute_permissions.iter().find(|&&x| {
                        match x {
                            super::AttributePermissions::Encryption(x_rest, _) => *rest == x_rest,
                            _ => false
                        }
                    })
                    .and_then(|_| Some(pdu::Error::InsufficientEncryptionKeySize) )
                    .or_else( || Some(pdu::Error::InsufficientEncryption) )
                    .unwrap()
                }
                super::AttributePermissions::Authentication(_) => pdu::Error::InsufficientAuthentication,
                super::AttributePermissions::Authorization(_) => pdu::Error::InsufficientAuthorization,
            }
        })
        .nth(0) // return the first offending permission, if any.
    }

    /// Get a future to receive ACL packets
    ///
    /// This will return a future that will return
    /// [`Poll::Ready`](https://doc.rust-lang.org/nightly/std/task/enum.Poll.html#variant.Ready)
    /// when there is an ACL packet to process. This assumes that the packet received is part of
    /// the Attribute (ATT) protocol.
    ///
    /// # temporary
    /// For now this will automatically process the request command from the Attribute Client. This
    /// is
    pub fn receiver<'a>(&'a mut self)
    -> impl Future<Output=Result<RequestProcessor<'a,C>, super::Error>> + 'a
    {
        ServerReceiver::new(self)
    }

    /// Send out notification
    ///
    /// The attribute at the given handle will be sent out in the notification.
    ///
    /// If the handle doesn't exist, then the notification isn't sent and false is returned
    pub fn send_notification(&self, handle: u16) -> bool {
        self.attributes.get(handle as usize).and_then( | attribute | {

            let val = attribute.get_val_as_transfer_format().into();

            let pdu = pdu::handle_value_notification( handle, val );

            let data = TransferFormat::into(&pdu);

            self.connection.send( l2cap::AclData::new(data, super::L2CAP_CHANNEL_ID) );

            Some(())
        } )
        .is_some()
    }

    /// Deref the handle, and if the handle is valid, do F, otherwise return an error.
    ///
    /// If the handle is invalid, then this function sends the error PDU with the error InvalidHandle
    #[inline]
    fn send_invalid_handle_error(&self, handle: u16, received_opcode: u8) {
        self.send_pdu_error(handle, received_opcode, pdu::Error::InvalidHandle);
    }

    #[inline]
    fn send_pdu_error(&self, handle: u16, received_opcode: u8, pdu_error: pdu::Error) {

        let err_pdu = pdu::error_response(
            received_opcode,
            handle,
            pdu_error
        );

        let data = TransferFormat::into(&err_pdu);

        self.connection.send( l2cap::AclData::new(data, super::L2CAP_CHANNEL_ID) );
    }

    fn process_exchange_mtu_request(&mut self, pdu: pdu::Pdu<u16>) {
        let client_mtu = pdu.into_parameters();

        if (C::DEFAULT_ATT_MTU..=self.max_mtu).contains(&client_mtu)  {
            self.set_mtu = Some(client_mtu.into());
        }

        let response_pdu = pdu::exchange_mtu_response(self.get_connection_mtu());

        let data = TransferFormat::into(&response_pdu);

        self.connection.send( l2cap::AclData::new( data, super::L2CAP_CHANNEL_ID) );
    }

    fn process_read_request(&mut self, pdu: pdu::Pdu<u16>) {
        let handle = *pdu.get_parameters();

        if let Some(attribute) = self.attributes.get( handle as usize) {

            let mut data = alloc::vec!( super::server::ServerPduName::ReadResponse.into() );

            data.extend_from_slice( &attribute.get_val_as_transfer_format().into() );

            let acl_data = l2cap::AclData::new( data.into_boxed_slice(), super::L2CAP_CHANNEL_ID );

            self.connection.send( acl_data );
        } else {
            self.send_invalid_handle_error(handle, super::client::ClientPduName::ReadRequest.into());
        }
    }

    /// # Note
    /// This cannot accept a PDU because the data size isn't know at compile time, thus the method
    /// determines the data type based on the handle.
    fn process_write_request(&mut self, raw_pdu: &[u8]) {
        let received_opcode = super::client::ClientPduName::WriteRequest.into();

        if raw_pdu.len() >= 3 {
            let raw_handle = &raw_pdu[1..3];
            let raw_data = &raw_pdu[3..];

            let handle = TransferFormat::from( &raw_handle ).unwrap();

            if let Some(data) = self.attributes.get_mut( handle as usize ) {
                match data.set_val_from_raw( raw_data ) {
                    Ok(_) => {
                        let data = TransferFormat::into(&pdu::write_response());
                        self.connection.send( l2cap::AclData::new( data, super::L2CAP_CHANNEL_ID ) );
                    },
                    Err(pdu_err) =>
                        self.send_pdu_error(handle, received_opcode, pdu_err),
                };
            } else {
                self.send_invalid_handle_error(handle, received_opcode);
            }
        } else {
            let err_pdu = pdu::error_response(
                received_opcode,
                0,
                pdu::Error::InvalidPDU
            );

            let data = TransferFormat::into(&err_pdu);

            self.connection.send( l2cap::AclData::new( data, super::L2CAP_CHANNEL_ID ) );
        }
    }

    fn process_find_information_request(&mut self, pdu: pdu::Pdu<pdu::HandleRange>) {
        use pdu::HandleRange;

        match pdu.get_parameters() {
            HandleRange { starting_handle: 0, .. } => {
                let oc = super::client::ClientPduName::FindInformationRequest.into();

                log::error!("Received 'find information request' with a handle range starting \
                    at 0");

                self.send_invalid_handle_error(0, oc);
            },
            HandleRange { starting_handle: sh @ _, ending_handle: eh @ _ } if sh <= eh => {
                use core::cmp::min;
                use core::convert::TryInto;
                use core::mem::size_of;

                enum Format {
                    Uuid16Bit,
                    Uuid128Bit,
                }

                let start = min( *sh as usize, self.attributes.len() );
                let stop  = min( *eh as usize, self.attributes.len() );

                let attributes = &self.attributes[start..stop];

                attributes
                .first()
                .and_then(|first| {

                    // See if the first attribute's type can be made into a 16 bit UUID
                    TryInto::<u16>::try_into(first.get_type()).and_then( | uuid | {

                        let handle = first.get_handle();

                        Ok( (handle, uuid) )
                    })
                    .and_then( | (first_handle, first_uuid) | {

                        // The type of the starting attribute is convertable to a 16 bit UUID.
                        //
                        // The find information response PDU will contain the the attributes after
                        // the starting attribute until:
                        // * an attribute's type cannot be converted to a 16 bit uuid
                        // * the end of `attributes`
                        // * the packet MTU was reached

                        let mtu = self.get_connection_mtu() as usize;

                        let vec_cap = min( (stop - start) * (2 + size_of::<u16>()), mtu - 2 );

                        let mut v = Vec::with_capacity( vec_cap );

                        v.extend_from_slice( &TransferFormat::into(&first_handle) );
                        v.extend_from_slice( &TransferFormat::into(&first_uuid) );

                        for attribute in attributes[1..].iter() {
                            if (v.len() + 2 + size_of::<u16>()) < v.capacity() {
                                if let Ok(uuid_16) = TryInto::<u16>::try_into(attribute.get_type()) {
                                    let handle =  attribute.get_handle();

                                    v.extend_from_slice( &TransferFormat::into(&handle) );
                                    v.extend_from_slice( &TransferFormat::into(&uuid_16) );

                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }

                        log::debug!("Sending a 'find information response' with 16 bit UUIDs");

                        Ok( (Format::Uuid16Bit, v) )
                    })
                    .or_else( |_| -> Result<(Format,Vec<u8>), ()> {

                        // The type of the first attribute cannot be made into a 16 bit UUID.
                        //
                        // The find information response PDU will contain the the attributes after
                        // the starting attribute until:
                        // * the end of `attributes`
                        // * the packet MTU was reached
                        //
                        // This is greedy, attributes that have a type that can be converted into a
                        // 16 bit UUID are included.

                        let mtu = self.get_connection_mtu() as usize;

                        let vec_cap = min( (stop - start) * (2 + size_of::<u128>()), mtu - 2 );

                        let mut v = Vec::with_capacity( vec_cap );

                        for attribute in attributes[1..].iter() {
                            if (v.len() + 2 + size_of::<u128>()) < v.capacity() {
                                if let Ok(uuid_128) = TryInto::<u128>::try_into(attribute.get_type()) {
                                    let handle =  attribute.get_handle();

                                    v.extend_from_slice( &TransferFormat::into(&handle) );
                                    v.extend_from_slice( &TransferFormat::into(&uuid_128) );

                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }

                        log::debug!("Sending a 'find information response' with 128 bit UUIDs");

                        Ok( (Format::Uuid128Bit, v) )
                    })
                    .and_then( | (format, information_data) | {

                        let mut vec = Vec::new();

                        vec.push( ServerPduName::FindInformationResponse.into() );

                        vec.push( match format { Format::Uuid16Bit => 0x1, Format::Uuid128Bit => 0x2 } );

                        vec.extend_from_slice(&information_data);

                        let data = vec.into_boxed_slice();

                        self.connection.send( l2cap::AclData::new( data, super::L2CAP_CHANNEL_ID ));

                        Ok(())
                    })
                    .ok()
                })
                .or_else( || {

                    // When there is no first attribute, that means there is no attributes
                    // within the specified range

                    let starting_handle = *sh;

                    let oc = super::client::ClientPduName::FindInformationRequest.into();

                    log::info!("Sending Error response for 'find information request' as no
                        attributes were found within the requested handle range");

                    self.send_pdu_error(starting_handle, oc, pdu::Error::AttributeNotFound);

                    None
                });
            },
            HandleRange { starting_handle: h @ _, ..} => {

                let oc = super::client::ClientPduName::FindInformationRequest.into();

                log::error!("Received invalid handle range in 'find information request'");

                self.send_invalid_handle_error(*h, oc);
            }
        }


    }

    fn process_find_by_type_value_request(&mut self, data: &[u8] ) {

        if data.len() >= 7 {

            let starting_handle: u16 = TransferFormat::from( &data[1..2] ).unwrap();

            let ending_handle: u16 = TransferFormat::from( &data[3..4] ).unwrap();

            let att_type: crate::UUID = TransferFormat::from( &data[5..6] ).unwrap();

            let raw_value = &data[7..];

            if starting_handle == 0u16 {
                let oc = super::client::ClientPduName::FindByTypeValueRequest.into();

                log::error!("Received 'find type by value request' with a handle range starting \
                    at 0");

                self.send_invalid_handle_error(0, oc);

            } else if starting_handle <= ending_handle {
                use core::cmp::min;
                use core::mem::size_of;

                let start = min( starting_handle as usize, self.attributes.len() );
                let end   = min( ending_handle   as usize, self.attributes.len() );

                let mtu = self.get_connection_mtu() as usize;

                let attributes = &self.attributes[start..end];

                let vec_cap = min( (end - start) * (2 + size_of::<u16>()), mtu - 1 );

                let mut size_counter = 0;

                let mut handles_information_list = Vec::with_capacity(vec_cap + 1);

                handles_information_list.push( ServerPduName::FindByTypeValueResponse.into() );

                attributes
                .iter()
                .filter(|attribute| {
                    (attribute.get_type() == att_type) &&
                    (attribute.get_val_as_transfer_format().into().as_ref() == raw_value)
                })
                .try_for_each(|a| {

                    handles_information_list.extend_from_slice( &TransferFormat::into( &a.get_handle() ));
                    handles_information_list.extend_from_slice( &TransferFormat::into( &a.get_type() ));

                    size_counter += 2 + size_of::<u16>();

                    if size_counter < vec_cap { Ok(()) } else { Err(()) }
                })
                .ok();

                if handles_information_list.len() > 1 {

                    log::info!("Sending 'find by type value response'");

                    self.connection.send(
                        l2cap::AclData::new(
                            handles_information_list.into_boxed_slice(),
                            super::L2CAP_CHANNEL_ID
                        )
                    );

                } else {
                    let oc = super::client::ClientPduName::FindByTypeValueRequest.into();

                    log::info!("No attributes were found within the provided range with the given \
                        type and value in the received 'find by type value request'");

                    self.send_pdu_error(starting_handle, oc, pdu::Error::AttributeNotFound);
                }

            } else {
                let oc = super::client::ClientPduName::FindByTypeValueRequest.into();

                log::error!("Sending Error response to 'find by type value request', invalid
                    handles receive");

                self.send_invalid_handle_error(starting_handle, oc);
            }
        } else {

            log::error!("Invalid 'find by type value request' received");

            self.send_pdu_error(0, 0, pdu::Error::InvalidPDU);
        }
    }

    fn process_read_by_type_request(&self, pdu: pdu::Pdu<pdu::TypeRequest> ) {

        use pdu::HandleRange;

        let handle_range = &pdu.get_parameters().handle_range;

        let desired_att_type = pdu.get_parameters().attr_type;

        // required permissions
        let required = &[
            super::AttributePermissions::Read
        ];

        let restricted = &[
            super::AttributePermissions::Encryption(super::AttributeRestriction::Read, super::EncryptionKeySize::Bits128),
            super::AttributePermissions::Authentication(super::AttributeRestriction::Read),
            super::AttributePermissions::Authorization(super::AttributeRestriction::Read),
        ];

        match handle_range {
            HandleRange{ starting_handle: 0, .. } => {
                let oc = super::client::ClientPduName::ReadByTypeRequest.into();

                log::error!("Received 'read by type request' with a handle range starting with 0");

                self.send_invalid_handle_error(0, oc);
            },
            HandleRange{ starting_handle: sh @ _, ending_handle: eh @ _ } if eh > sh => {
                use core::cmp::min;

                let start = min( *sh as usize, self.attributes.len() );
                let end   = min( *eh as usize, self.attributes.len() );

                let vec_cap = min(self.get_connection_mtu() as usize, 256);

                let mut read_by_type_response = Vec::with_capacity(vec_cap);

                read_by_type_response.push( ServerPduName::ReadByTypeResponse.into() );

                let first_match = self.attributes[start..end].iter()
                    .enumerate()
                    .filter( |(_,att)| att.get_type() == desired_att_type )
                    .next();

                first_match.and_then( |(cnt, att)| {

                    if let Some(permission) = self.validate_permissions(att.as_ref(), required, restricted) {

                        let oc = super::client::ClientPduName::ReadByTypeRequest.into();

                        log::info!("Client doesn't requires permission {:?} to access attribute",
                            permission);

                        self.send_pdu_error(*sh, oc, permission.into());

                        None

                    } else {

                        let transfer_value = att.get_val_as_transfer_format().into();

                        let handle = att.get_handle();

                        let length = min(transfer_value.len() + 2, vec_cap - 2);

                        // Add the length of each handle value pair
                        read_by_type_response.push( length as u8 );

                        // Add the first handle
                        read_by_type_response.extend_from_slice( &TransferFormat::into(&handle) );

                        // Add the first value
                        read_by_type_response.extend_from_slice( &transfer_value );

                        // Return the next attribute to be check and the length of the matched
                        // attribute
                        Some( (start + cnt + 1, length) )
                    }
                })
                .and_then( | (next_attr, handle_value_len) | {

                    let mut remaining = vec_cap - (2 + handle_value_len);

                    let attr_iter = self.attributes[next_attr..end].iter()
                        .filter(|att| att.get_type() == desired_att_type);

                    // Scour the rest of the attributes for any more values that can be added
                    for attribute in attr_iter {
                        let transfer_value = attribute.get_val_as_transfer_format().into();

                        if (2 + transfer_value.len()) <= remaining &&
                           None == self.validate_permissions(attribute.as_ref(), required, restricted)
                        {
                            remaining -= transfer_value.len();

                            let handle = attribute.get_handle();

                            read_by_type_response.extend_from_slice( &TransferFormat::into(&handle) );

                            read_by_type_response.extend_from_slice( &transfer_value );

                        } else {
                            break;
                        }
                    }

                    Some(())
                });

                // Check that at least one handle-value was added to the response otherwise return
                // the error that no attributes were found.
                if read_by_type_response.len() > 1 {

                    self.connection.send( l2cap::AclData::new(
                        read_by_type_response.into_boxed_slice(),
                        super::L2CAP_CHANNEL_ID
                    ));

                } else {
                    let oc = super::client::ClientPduName::ReadByTypeRequest.into();

                    log::info!("No attributes were found within the provided range with the given \
                        type and value in the received 'find by type value request'");

                    self.send_invalid_handle_error(*sh, oc);
                }
            },
            HandleRange{ starting_handle: sh @ _, .. } => {
                let oc = super::client::ClientPduName::ReadByTypeRequest.into();

                log::error!("Sending Error response to 'read by type request', invalid
                    handles receive");

                self.send_invalid_handle_error(*sh, oc);
            }
        }
    }
}

pub struct ServerAttributes {
    attributes: Vec<Box<dyn super::AnyAttribute + Unpin>>
}

impl ServerAttributes {
    pub fn new() -> Self { Self { attributes: Vec::new() } }

    pub fn push<V>(&mut self, attribute: super::Attribute<V>) -> u16
    where V: TransferFormat + Sized + Unpin + 'static
    {
        use core::convert::TryInto;

        let ret = self.attributes.len().try_into().expect("Exceeded attribute handle limit");

        self.attributes.push( Box::new(attribute) );

        ret
    }

    /// Get the next handle to push an attribute into
    pub fn next_handle(&self) -> u16 {
        self.attributes.len() as u16
    }
}

// impl AsRef<[Box<dyn Any>]> for ServerAttributes {
//     fn as_ref(&self) -> &[Box<dyn TransferFormat>] {
//         self.attributes.as_slice()
//     }
// }
