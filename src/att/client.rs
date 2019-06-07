use super::{
    pdu,
    TransferFormat,
};
use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::task::{Poll, Context};

#[derive(Debug,Clone,Copy,PartialEq,PartialOrd,Eq)]
pub enum ClientPduName {
    ExchangeMtuRequest,
    FindInformationRequest,
    FindByTypeValueRequest,
    ReadByTypeRequest,
    ReadRequest,
    ReadBlobRequest,
    ReadMultipleRequest,
    ReadByGroupTypeRequest,
    WriteRequest,
    WriteCommand,
    PrepareWriteRequest,
    ExecuteWriteRequest,
    HandleValueConfirmation,
    SignedWriteCommand,
}

impl ClientPduName {
    /// Convert a u8 opcode to a client pdu name
    ///
    /// If the opcode is not part of the Attribute protocol, then the an error is returned.
    pub(crate) fn try_from(val: u8) -> Result<Self, ()> {
        match val {
            0x02 => Ok(ClientPduName::ExchangeMtuRequest),
            0x04 => Ok(ClientPduName::FindInformationRequest),
            0x06 => Ok(ClientPduName::FindByTypeValueRequest),
            0x08 => Ok(ClientPduName::ReadByTypeRequest),
            0x0A => Ok(ClientPduName::ReadRequest),
            0x0C => Ok(ClientPduName::ReadBlobRequest),
            0x0E => Ok(ClientPduName::ReadMultipleRequest),
            0x11 => Ok(ClientPduName::ReadByGroupTypeRequest),
            0x12 => Ok(ClientPduName::WriteRequest),
            0x52 => Ok(ClientPduName::WriteCommand),
            0x16 => Ok(ClientPduName::PrepareWriteRequest),
            0x18 => Ok(ClientPduName::ExecuteWriteRequest),
            0x1E => Ok(ClientPduName::HandleValueConfirmation),
            0xD2 => Ok(ClientPduName::SignedWriteCommand),
            _    => Err(()),
        }
    }
}

impl From<ClientPduName> for pdu::PduOpCode {
    fn from(pdu_name: ClientPduName) -> pdu::PduOpCode {
        let raw: u8 = From::from(pdu_name);

        From::from(raw)
    }
}

impl From<ClientPduName> for u8 {
    fn from(pdu_name: ClientPduName) -> u8 {
        match pdu_name {
            ClientPduName::ExchangeMtuRequest => 0x02,
            ClientPduName::FindInformationRequest => 0x04,
            ClientPduName::FindByTypeValueRequest => 0x06,
            ClientPduName::ReadByTypeRequest => 0x08,
            ClientPduName::ReadRequest => 0x0A,
            ClientPduName::ReadBlobRequest => 0x0C,
            ClientPduName::ReadMultipleRequest => 0x0E,
            ClientPduName::ReadByGroupTypeRequest => 0x11,
            ClientPduName::WriteRequest => 0x12,
            ClientPduName::WriteCommand => 0x52,
            ClientPduName::PrepareWriteRequest => 0x16,
            ClientPduName::ExecuteWriteRequest => 0x18,
            ClientPduName::HandleValueConfirmation => 0x1E,
            ClientPduName::SignedWriteCommand => 0xD2,
        }
    }
}

struct MtuFuture<Ch> where Ch: crate::gap::ConnectionChannel + Unpin {
    mtu_size: u16,
    mtu_pdu: Option<pdu::Pdu<u16>>,
    channel: Option<Ch>,
}

impl<Ch> Future for MtuFuture<Ch> where Ch: crate::gap::ConnectionChannel + Unpin
{
    type Output = Result< Client<Ch>, super::Error >;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        use super::server::ServerPduName;

        let this = self.get_mut();

        if let Some(pdu) = this.mtu_pdu.take() {

            // Return an error if the mtu is too small
            if Ch::DEFAULT_ATT_MTU > this.mtu_size {
                return Poll::Ready(Err(super::Error::TooSmallMtu));
            }

            // The channel must exist at this point
            this.channel.as_ref().expect("Channel doesn't exist").send( &TransferFormat::into(&pdu) );
        }

        if let Some(bytes) = this.channel.as_ref().and_then( |c| c.receive(cx.waker().clone()) ) {

            // Check for a ExchangeMTUResponse PDU
            if bytes.len() == 3 && bytes[0] == From::from(ServerPduName::ExchangeMTUResponse) {

                match TransferFormat::from( &bytes[1..] ) {
                    Ok( received_mtu ) => {

                        let client = Client {
                            mtu: core::cmp::min( this.mtu_size,  received_mtu) as usize,

                            // The channel must always be `Some` here
                            channel: this.channel.take().expect("No channel to take"),
                        };

                        Poll::Ready(Ok(client))
                    },
                    Err(e) => {
                        Poll::Ready(Err(e.into()))
                    }
                }

            } else if bytes.len() == 5 && bytes[1] == From::from(ServerPduName::ErrorResponse) {

                // Return error code if received error PDU
                Poll::Ready(Err( pdu::Error::from_raw(bytes[4]).into() ))

            } else {

                Poll::Ready(Err( pdu::Error::InvalidPDU.into() ))

            }

        } else {

            Poll::Pending

        }
    }
}

struct ResponseFuture<'a, Ch, Rd>
where Ch: crate::gap::ConnectionChannel,
      Rd: TransferFormat,
{
    channel: &'a Ch,
    send_data: Option<Box<[u8]>>,
    pd: core::marker::PhantomData<Rd>,
    exp_resp: super::server::ServerPduName,
}

impl<Ch, Rd> Future for ResponseFuture<'_, Ch, Rd>
where Ch: crate::gap::ConnectionChannel,
      Rd: TransferFormat + Unpin,
{
    type Output = Result<pdu::Pdu<Rd>, super::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();

        if let Some(data) = this.send_data.take() {
            this.channel.send(&data)
        }

        if let Some(bytes) = this.channel.receive(cx.waker().clone()) {
            
            if bytes.len() >= 1 {
                use core::convert::TryFrom;

                match super::server::ServerPduName::try_from(bytes[0]) {

                    Ok(super::server::ServerPduName::ErrorResponse) => {

                        let err_pdu_rslt: Result<pdu::Pdu<pdu::ErrorAttributeParameter>, pdu::Error> =
                            TransferFormat::from(&bytes);

                        match err_pdu_rslt {
                            Ok(err_pdu) => Poll::Ready( Err(err_pdu.into()) ),
                            Err(e) => Poll::Ready( Err(e.into()) ),
                        }

                    },
                    Ok(name) if name == this.exp_resp => {

                        let pdu_rslt = TransferFormat::from(&bytes);

                        match pdu_rslt {
                            Ok(pdu) => Poll::Ready( Ok(pdu) ),
                            Err(e)  => Poll::Ready( Err(e.into()) ),
                        }
                    },
                    Ok(_) => Poll::Ready( Err( super::Error::UnexpectedPdu )),
                    Err(_) => Poll::Ready( Err( pdu::Error::InvalidPDU.into() )),
                }
            } else {
                Poll::Ready(Err( pdu::Error::InvalidPDU.into() ))
            }
        } else {
            Poll::Pending
        }
    }
}

struct ReturnedResponse<'a, Ch, Rd>
where Ch: crate::gap::ConnectionChannel,
      Rd: TransferFormat,
{
    mtu: usize,
    rf: ResponseFuture<'a, Ch, Rd>,
}

impl<Ch, Rd> Future for ReturnedResponse<'_, Ch, Rd>
where Ch: crate::gap::ConnectionChannel,
      Rd: TransferFormat + Unpin,
{
    type Output = Result<Rd, super::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {

        // mtu check
        if let Some(ref data) = self.rf.send_data {
            if data.len() > self.mtu {
                return Poll::Ready(Err( super::Error::MtuExceeded ))
            }
        }

        match Pin::new(&mut self.get_mut().rf).poll(cx) {
            Poll::Ready(Ok(pdu)) => Poll::Ready(Ok(pdu.into_parameters())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct Client<C>
where C: crate::gap::ConnectionChannel,
{
    mtu: usize,
    channel: C,
}

impl<C> Client<C> where C: crate::gap::ConnectionChannel + Unpin {

    /// Connect a client to a attribute server
    ///
    /// This performs the initial setup between the client and the server required for establishing
    /// the attribute protocol. An optional input is used to determine the maximum size of each
    /// attribute packet. If maximum_transfer_unit is `None` the the minimum MTU is used.
    ///
    /// The bluetooth connection must already be established
    pub fn connect<Mtu>( channel: C, maximum_transfer_unit: Mtu )
    -> impl Future<Output=Result<Self, super::Error>>
    where Mtu: Into<Option<u16>>
    {
        let mtu = if let Some(mtu) = maximum_transfer_unit.into() {mtu} else {C::DEFAULT_ATT_MTU};

        MtuFuture {
            mtu_size: mtu,
            mtu_pdu: Some(pdu::exchange_mtu_request(mtu)),
            channel: Some(channel),
        }
    }

    /// Send the mtu request
    ///
    /// The maximum transfer size is part of connecting the client to the server, but if you want
    /// to try to change the mtu, then this will resend the exchange mtu request PDU to the server.
    ///
    /// The new MTU is returned by the future
    pub fn exchange_mtu_request<'a>(&'a mut self, mtu: u16 )
    -> impl Future<Output=Result< u16, super::Error>> + 'a
    {
        let data = TransferFormat::into( &pdu::exchange_mtu_request(mtu) );

        struct MtuResponse<'a, Channel> where Channel: crate::gap::ConnectionChannel {
            wanted_mtu: u16,
            current_mtu: &'a mut usize,
            rf: ResponseFuture<'a, Channel, u16>,
        }

        impl<Channel> Future for MtuResponse<'_, Channel>
        where Channel: crate::gap::ConnectionChannel
        {
            type Output = Result<u16, super::Error>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {

                // Return an error if the mtu is too small
                if Channel::DEFAULT_ATT_MTU > self.wanted_mtu {
                    return Poll::Ready( Err(super::Error::TooSmallMtu) )
                }

                let this = self.get_mut();
                match Pin::new(&mut this.rf).poll(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(pdu_rslt) => {
                        match pdu_rslt {
                            Ok(pdu) => {
                                *this.current_mtu = core::cmp::min(
                                    this.wanted_mtu,
                                    *pdu.get_parameters()
                                ).into();

                                Poll::Ready( Ok(*pdu.get_parameters()) )
                            }
                            Err(err) => Poll::Ready (Err(err) )
                        }
                    }
                }
            }
        }

        MtuResponse {
            wanted_mtu: mtu,
            current_mtu: &mut self.mtu,
            rf: ResponseFuture {
                send_data: Some(data),
                channel: &self.channel,
                exp_resp: super::server::ServerPduName::ExchangeMTUResponse,
                pd: core::marker::PhantomData,
            },
        }
    }

    pub fn find_information_request<'a, R>(&'a self, handle_range: R)
    -> impl Future<Output=Result<pdu::FormattedHandlesWithType, super::Error>> + 'a
    where R: Into<pdu::HandleRange>
    {
        let pdu = pdu::find_information_request(handle_range);

        ReturnedResponse {
            mtu: self.mtu,
            rf: ResponseFuture {
                send_data: Some(TransferFormat::into(&pdu)),
                channel: &self.channel,
                exp_resp: super::server::ServerPduName::FindInformationResponse,
                pd: core::marker::PhantomData,
            }
        }
    }

    /// Find by type and value request
    ///
    /// The attribute type, labeled as the input `uuid`, is a 16 bit assigned number type. If the
    /// type cannot be converted into a 16 bit UUID, then this function will return an error
    /// containing the incorrect type.
    pub fn find_by_type_value_request<'a, R, D>(&'a self, handle_range: R, uuid: crate::UUID, value: D)
    -> Result<impl Future<Output=Result<pdu::TypeValueRequest<D>, super::Error>> + 'a , crate::UUID>
    where R: Into<pdu::HandleRange>,
          D: TransferFormat + Unpin + 'a,
    {
        let pdu_rslt = pdu::find_by_type_value_request(handle_range, uuid, value);

        match pdu_rslt {
            Ok(pdu) => Ok (
                ReturnedResponse  {
                    mtu: self.mtu,
                    rf: ResponseFuture {
                        send_data: Some(TransferFormat::into(&pdu)),
                        channel: &self.channel,
                        exp_resp: super::server::ServerPduName::FindByTypeValueResponse,
                        pd: core::marker::PhantomData,
                    }
                }
            ),
            Err(_) => Err( uuid )
        }
    }

    pub fn read_by_type_request<'a, R>(&'a self, handle_range: R, attr_type: crate::UUID)
    -> impl Future<Output=Result<pdu::TypeRequest, super::Error>> + 'a
    where R: Into<pdu::HandleRange>
    {
        let pdu = pdu::read_by_type_request(handle_range, attr_type);

        ReturnedResponse {
            mtu: self.mtu,
            rf: ResponseFuture {
                send_data: Some(TransferFormat::into(&pdu)),
                channel: &self.channel,
                exp_resp: super::server::ServerPduName::ReadByTypeResponse,
                pd: core::marker::PhantomData,
            }
        }
    }

    pub fn read_request<'a,D>(&'a self, handle: u16 )
    -> impl Future<Output=Result<D, super::Error>> + 'a
    where D: TransferFormat + Unpin + 'a
    {
        let pdu = pdu::read_request(handle);

        ReturnedResponse {
            mtu: self.mtu,
            rf: ResponseFuture {
                send_data: Some(TransferFormat::into(&pdu)),
                channel: &self.channel,
                exp_resp: super::server::ServerPduName::ReadResponse,
                pd: core::marker::PhantomData,
            }
        }

    }

    pub fn read_blob_request<'a,D>(&'a self, handle: u16, offset: u16)
    -> impl Future<Output=Result<D, super::Error>> + 'a
    where D: TransferFormat + Unpin + 'a
    {
        let pdu = pdu::read_blob_request(handle, offset);

        ReturnedResponse {
            mtu: self.mtu,
            rf: ResponseFuture {
                send_data: Some(TransferFormat::into(&pdu)),
                channel: &self.channel,
                exp_resp: super::server::ServerPduName::ReadBlobResponse,
                pd: core::marker::PhantomData,
            }
        }
    }

    /// Read multiple handles
    ///
    /// If handles has length of 0 an error is returned
    pub fn read_multiple_request<'a>(&'a self, handles: Box<[u16]> )
    -> Result< impl Future<Output=Result<Box<[Box<dyn TransferFormat>]>, super::Error>> + 'a, ()>
    {
        let pdu = pdu::read_multiple_request( handles )?;

        Ok( ReturnedResponse {
            mtu: self.mtu,
            rf: ResponseFuture {
                send_data: Some(TransferFormat::into(&pdu)),
                channel: &self.channel,
                exp_resp: super::server::ServerPduName::ReadMultipleResponse,
                pd: core::marker::PhantomData,
            }
        } )
    }

    pub fn read_by_group_type_request<'a,R,D>(&'a self, handle_range: R, group_type: crate::UUID)
    -> impl Future<Output = Result<Box<[pdu::ReadGroupTypeResponse<D>]>, super::Error>> + 'a
    where R: Into<pdu::HandleRange>,
          D: TransferFormat + Unpin + 'a
    {
        let pdu = pdu::read_by_group_type_request(handle_range, group_type);

        ReturnedResponse {
            mtu: self.mtu,
            rf: ResponseFuture {
                send_data: Some(TransferFormat::into(&pdu)),
                channel: &self.channel,
                exp_resp: super::server::ServerPduName::ReadByGroupTypeResponse,
                pd: core::marker::PhantomData,
            }
        }
    }

    pub fn write_request<'a,D>(&'a self, handle: u16, data: D)
    -> impl Future<Output = Result<(), super::Error>> + 'a
    where D: TransferFormat
    {
        let pdu = pdu::write_request(handle, data);

        ReturnedResponse {
            mtu: self.mtu,
            rf: ResponseFuture {
                send_data: Some(TransferFormat::into(&pdu)),
                channel: &self.channel,
                exp_resp: super::server::ServerPduName::WriteResponse,
                pd: core::marker::PhantomData,
            }
        }
    }

    pub fn write_command<D>(&self, handle: u16, data: D) -> Result<(), super::Error>
    where D: TransferFormat
    {
        let pdu = pdu::write_command(handle, data);

        let data = TransferFormat::into(&pdu);

        if self.mtu < data.len() {
            self.channel.send(&data);
            Ok(())
        } else {
            Err(super::Error::MtuExceeded)
        }
    }

    pub fn prepare_write_request<'a, D>(&'a self, handle: u16, offset: u16, data: D)
    -> impl Future<Output=Result<pdu::PrepareWriteRequest<D>, super::Error>> + 'a
    where D: TransferFormat + Unpin + 'a
    {
        let pdu = pdu::prepare_write_request(handle, offset, data);

        ReturnedResponse {
            mtu: self.mtu,
            rf: ResponseFuture {
                send_data: Some(TransferFormat::into(&pdu)),
                channel: &self.channel,
                exp_resp: super::server::ServerPduName::PrepareWriteResponse,
                pd: core::marker::PhantomData,
            }
        }
    }

    pub fn execute_write_request<'a>(&'a self, execute: bool )
    -> impl Future<Output=Result<u8, super::Error>> + 'a
    {
        let pdu = pdu::execute_write_request(execute);

        ReturnedResponse {
            mtu: self.mtu,
            rf: ResponseFuture {
                send_data: Some(TransferFormat::into(&pdu)),
                channel: &self.channel,
                exp_resp: super::server::ServerPduName::ExecuteWriteResponse,
                pd: core::marker::PhantomData,
            }
        }
    }
}