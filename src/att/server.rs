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

/// The Reserved Handle
///
/// The first handle (value of '0') is reserved for future use. This is used to represent that
/// handle when creating a new Attribute Bearer
struct ReservedHandle;

impl super::AnyAttribute for ReservedHandle {
    fn get_type(&self) -> crate::UUID { crate::UUID::from(0u128) }

    fn get_permissions(&self) -> Box<[super::AttributePermissions]> {
        alloc::vec!(super::AttributePermissions::Read).into_boxed_slice()
    }

    fn get_handle(&self) -> u16 { 0 }

    fn get_val_as_transfer_format(&self) -> &dyn TransferFormat { &() }

    fn set_val_from_raw(&mut self, _: &[u8]) -> Result<(), pdu::Error> {
        Err(pdu::Error::InvalidHandle)
    }
}

struct ServerReceiver<'a, C>
where C: crate::gap::ConnectionChannel
{
    server: &'a mut Server<C>
}

impl<'a,C> ServerReceiver<'a,C>
where C: crate::gap::ConnectionChannel
{
    fn process_client_pdu(
        &mut self,
        pdu_name: client::ClientPduName,
        data: &[u8])
    -> Result<(), pdu::Error>
    {
        match pdu_name {
            super::client::ClientPduName::ExchangeMtuRequest =>
                self.server.process_exchange_mtu_request( TransferFormat::from(data)? ),
                super::client::ClientPduName::WriteRequest =>
                self.server.process_write_request( data ),
                super::client::ClientPduName::ReadRequest =>
                self.server.process_read_request( TransferFormat::from(data)? ),
            super::client::ClientPduName::FindInformationRequest |
            super::client::ClientPduName::FindByTypeValueRequest |
            super::client::ClientPduName::ReadByTypeRequest |
            super::client::ClientPduName::ReadBlobRequest |
            super::client::ClientPduName::ReadMultipleRequest |
            super::client::ClientPduName::ReadByGroupTypeRequest |
            super::client::ClientPduName::WriteCommand |
            super::client::ClientPduName::PrepareWriteRequest |
            super::client::ClientPduName::ExecuteWriteRequest |
            super::client::ClientPduName::HandleValueConfirmation |
            super::client::ClientPduName::SignedWriteCommand => {
                let opcode = if let Some(oc) = data.get(0) { *oc } else { 0 };

                self.server.send_pdu_error(0, opcode, pdu::Error::RequestNotSupported);
            },
        };

        Ok(())
    }
}

impl<'a, C> Future for ServerReceiver<'a,C>
where C: crate::gap::ConnectionChannel
{
    type Output = Result<(), super::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {

        let this = self.get_mut();

        let data_opt = this.server.connection.0.receive(cx.waker().clone());

        if let Some(data) = data_opt {
            if data.len() > 1 {
                match super::client::ClientPduName::try_from(data[0]) {
                    Ok(pdu_name) => {
                        match this.process_client_pdu(pdu_name, &data) {
                            Ok(_) => Poll::Ready(Ok(())),
                            Err(e) => Poll::Ready(Err(e.into()))
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
where C: crate::gap::ConnectionChannel
{
    /// The maximum mtu that this server can handle. This is also the mtu sent in a MTU response
    /// PDU.
    max_mtu: u16,
    /// TODO: It is temporary that only one connection can be made to this server, capability needs
    /// to be added
    ///
    /// `connection` is compromised of the connection channel and the maximum transfer unit
    connection: (C, Option<u16>),
    attributes: Vec<Box<dyn super::AnyAttribute + Unpin>>
}

impl<C> Server<C>
where C: crate::gap::ConnectionChannel
{

    /// Create a new Server
    ///
    /// The maximum transfer unit is set here, it cannot be smaller then the minimum MTU as
    /// specified by the DEFAULT_ATT_MTU constant in trait `ConnectionChannel`. If the provided MTU
    /// value is smaller than DEFAULT_ATT_MTU or none is passed, then the MTU will be set to
    /// DEFAULT_ATT_MTU.
    ///
    /// # WARNING
    /// This function will change in the future. The imput "connection" will be removed when
    /// multiple connections are supported in the future.
    pub fn new<Mtu>( connection: C, max_mtu: Mtu) -> Self where Mtu: Into<Option<u16>> {
        let mtu = if let Some(val) = max_mtu.into() {
            if val >= C::DEFAULT_ATT_MTU {
                val
            } else {
                C::DEFAULT_ATT_MTU
            }
        } else {
            C::DEFAULT_ATT_MTU
        };

        Self {
            max_mtu: mtu,
            connection: (connection, None),
            attributes: alloc::vec![ Box::new(ReservedHandle) ],
        }
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

    pub fn on_receive<'a>(&'a mut self)
    -> impl Future<Output=Result<(), super::Error>> + 'a
    {
        ServerReceiver{ server: self }
    }

    /// Deref the handle, and if the handle is valid, do F, otherwise return an error.
    ///
    /// If the handle is invalid, then this function sends the error PDU with the error InvalidHandle
    #[inline]
    fn send_invalid_handle_error(&mut self, handle: u16, received_opcode: u8)
    {
        self.send_pdu_error(handle, received_opcode, pdu::Error::InvalidHandle);
    }

    #[inline]
    fn send_pdu_error(&mut self, handle: u16, received_opcode: u8, pdu_error: pdu::Error) {

        let err_pdu = pdu::error_response(
            received_opcode,
            handle,
            pdu_error
        );

        self.connection.0.send( &TransferFormat::into(&err_pdu) );
    }

    fn process_exchange_mtu_request(&mut self, pdu: pdu::Pdu<u16>) {
        let client_mtu = pdu.into_parameters();

        if client_mtu < self.max_mtu {
            self.connection.1 = client_mtu.into();
        }

        let response_pdu = pdu::exchange_mtu_response(self.max_mtu);

        let data = TransferFormat::into(&response_pdu);

        self.connection.0.send( &data )
    }

    fn process_read_request(&mut self, pdu: pdu::Pdu<u16>) {
        let handle = *pdu.get_parameters();

        if let Some(attribute) = self.attributes.get_mut( handle as usize) {

            let mut data = alloc::vec!( super::server::ServerPduName::ReadResponse.into() );

            data.extend_from_slice( &attribute.get_val_as_transfer_format().into() );

            self.connection.0.send( &data );
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

            if let Some(data) = self.attributes.get_mut( handle as usize) {
                match data.set_val_from_raw( raw_data ) {
                    Ok(_) =>
                        self.connection.0.send( &TransferFormat::into(&pdu::write_response()) ),
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

            self.connection.0.send( &TransferFormat::into(&err_pdu) );
        }
    }
}
