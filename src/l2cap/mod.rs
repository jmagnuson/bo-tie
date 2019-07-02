/// Logical Link Control and Adaption protocol (L2CAP)

use alloc::{
    boxed::Box,
    vec::Vec,
};
use crate::hci;

/// Channel Identifier
///
/// Channel Identifiers are used by the L2CAP to associate the data with a given channel. Channels
/// are a numeric identifer for a protocol or an association of protocols that are part of L2CAP or
/// a higher layer (such as the Attribute (ATT) protocl).
///
/// # Specification Reference
/// See Bluetooth Specification V5 | Vol 3, Part A Section 2.1
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum ChannelIdentifier {
    /// LE User (Logical link) identifiers
    LE(LeUChannelIdentifier)
}

impl ChannelIdentifier {
    fn to_val(&self) -> u16 {
        match self {
            ChannelIdentifier::LE(ci) => ci.to_val()
        }
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub struct DynChannelId {
    channel_id: u16
}

impl DynChannelId {

    pub const LE_BOUNDS: core::ops::RangeInclusive<u16> = 0x0040..=0x007F;

    fn new( channel_id: u16 ) -> Self {
        DynChannelId { channel_id }
    }

    /// Create a new Dynamic Channel identifer for the LE-U CID name space
    ///
    /// This will return the enum
    /// [`DynamicallyAllocated`](../enum.LeUChannelIdentifier.html#variant.DynamicallyAllocated)
    /// with the `channel_id` if the id is within the bounds of
    /// [`LE_LOWER`](#const.LE_LOWER) and
    /// [`LE_UPPER`](#const.LE_UPPER). If the input is not between those bounds, then an error is
    /// returned containing the infringing input value.
    pub fn new_le( channel_id: u16 ) -> Result< LeUChannelIdentifier, u16 > {
        if Self::LE_BOUNDS.contains(&channel_id) {
            Ok( LeUChannelIdentifier::DynamicallyAllocated( DynChannelId::new(channel_id) ) )
        } else {
            Err( channel_id )
        }
    }
}

/// LE-U Channel Identifiers
///
/// These are the channel identifiers for a LE
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum LeUChannelIdentifier {
    /// Channel for the Attribute Protocol
    ///
    /// This channel is used for the attribute protocol, which also means that all GATT data will
    /// be sent through this channel.
    AttributeProtocol,
    /// Channel signaling
    ///
    /// See the Bluetooth Specification V5 | Vol 3, Part A Section 4
    LowEnergyL2CAPSignalingChannel,
    /// Security Manager Protocol
    SecurityManagerProtocl,
    /// Dynamically allocated channel identifiers
    ///
    /// These are channels that are dynamically allocated through the "Credit Based Connection
    /// Request" procedure defined in See Bluetooth Specification V5 | Vol 3, Part A Section 4.22
    ///
    /// To make a `DynamicallyAllocated` variant, use the function
    /// [`new_le`](../DynChannelId/index.html)
    /// of the struct `DynChannelId`
    DynamicallyAllocated(DynChannelId)
}

impl LeUChannelIdentifier {
    fn to_val(&self) -> u16 {
        match self {
            LeUChannelIdentifier::AttributeProtocol => 0x4,
            LeUChannelIdentifier::LowEnergyL2CAPSignalingChannel => 0x5,
            LeUChannelIdentifier::SecurityManagerProtocl => 0x6,
            LeUChannelIdentifier::DynamicallyAllocated(dyn_id) => dyn_id.channel_id,
        }
    }

    fn from_raw(val: u16) -> Result<Self, ()>  {
        match val {
            0x4 => Ok( LeUChannelIdentifier::AttributeProtocol ),
            0x5 => Ok( LeUChannelIdentifier::LowEnergyL2CAPSignalingChannel ),
            0x6 => Ok( LeUChannelIdentifier::SecurityManagerProtocl ),
            _ if DynChannelId::LE_BOUNDS.contains(&val) =>
                Ok( LeUChannelIdentifier::DynamicallyAllocated( DynChannelId::new(val) ) ),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum AclDataError {
    RawDataTooSmall,
    PayloadLengthIncorrect,
    InvalidChannelId,
}

impl core::fmt::Display for AclDataError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            AclDataError::RawDataTooSmall => write!(f, "Raw data is too small for an ACL frame"),
            AclDataError::PayloadLengthIncorrect =>
                write!(f, "Specified payload length didn't match the actual payload length"),
            AclDataError::InvalidChannelId => write!(f, "Invalid Channel Id")
        }
    }
}

/// Connection-oriented channel data
#[derive(Debug,Clone)]
pub struct AclData {
    channel_id: ChannelIdentifier,
    data: Box<[u8]>
}

impl AclData {
    pub fn new( payload: Box<[u8]>, channel_id: ChannelIdentifier ) -> Self {
        AclData {
            channel_id: channel_id,
            data: payload,
        }
    }

    pub fn get_channel_id(&self) -> ChannelIdentifier { self.channel_id }

    pub fn get_payload(&self) -> &[u8] { &self.data }

    pub fn into_raw_data(&self) -> Box<[u8]> {
        use core::convert::TryInto;

        let mut v = Vec::new();

        let len: u16 = self.data.len().try_into().expect("Couldn't convert into u16");

        v.extend_from_slice( &len.to_le_bytes() );

        v.extend_from_slice( &self.channel_id.to_val().to_le_bytes() );

        v.extend_from_slice( &self.data );

        v.into_boxed_slice()
    }

    /// Create an AclData struct from raw l2cap acl data
    ///
    /// # Errors
    /// * The length of the raw data must be >= 4
    /// * The length value of the raw data must be equal to the length of the payload portion of
    ///   the raw data
    /// * The channel id must be valid
    pub fn from_raw_data(data: &[u8]) -> Result<Self, AclDataError> {
        use core::convert::TryInto;

        if data.len() >= 4 {
            let len = <u16>::from_le_bytes( [data[0], data[1]] );
            let raw_channel_id = <u16>::from_le_bytes( [data[2], data[3]] );
            let payload = &data[4..];

            if payload.len() == len.try_into().expect("Cannot convert len to usize") {
                Ok( Self {
                    channel_id: ChannelIdentifier::LE(
                        LeUChannelIdentifier::from_raw(raw_channel_id).or(Err(AclDataError::InvalidChannelId))?
                    ),
                    data: Box::from(payload),
                })
            } else {
                Err( AclDataError::PayloadLengthIncorrect )
            }
        }
        else {
            Err( AclDataError::RawDataTooSmall )
        }
    }
}

/// The minimum number of data bytes in an attribute protocol based packet for bluetooth le
pub const MIN_ATT_MTU_LE: u16 = 23;

/// The minimum number of data bytes in an attribute protocol based packet for bluetooth BR/EDR
pub const MIN_ATT_MTU_BR_EDR: u16 = 48;

pub trait ConnectionChannel {
    const DEFAULT_ATT_MTU: u16;

    fn send(&self, data: AclData);
    fn receive(&self, waker: core::task::Waker) -> Option<Box<[AclData]>>;
}

/// A channel constructed via the Acl HCI interface
///
/// This channel is for use with Acl data sent through the host controller interface.
pub struct LeAclHciChannel<'a, I> where I: hci::HciAclDataInterface {
    connection_handle: hci::common::ConnectionHandle,
    connection_interval: hci::common::ConnectionInterval,
    hi_ref: &'a hci::HostInterface<I>,
    buf_recv: hci::HciAclDataReceiver<'a, I>
}

impl<'a, I> LeAclHciChannel<'a, I> where I: hci::HciAclDataInterface {

    pub fn new( hi: &'a hci::HostInterface<I>, connection_event: hci::events::LEConnectionCompleteData )
    -> Self
    {
        LeAclHciChannel {
            connection_handle: connection_event.connection_handle,
            connection_interval: connection_event.connection_interval,
            hi_ref: hi,
            buf_recv: hi.buffered_receiver(connection_event.connection_handle)
        }
    }
}

impl<'a,I> ConnectionChannel for LeAclHciChannel<'a,I> where I: hci::HciAclDataInterface {
    const DEFAULT_ATT_MTU: u16 = MIN_ATT_MTU_LE;

    fn send(&self, data: AclData ) {

        let hci_acl_data = hci::HciAclData::new(
            self.connection_handle,
            hci::AclPacketBoundry::FirstNonFlushable,
            hci::AclBroadcastFlag::NoBroadcast,
            data.into_raw_data()
        );

        self.hi_ref.send_data(hci_acl_data).expect("Failed to send hci acl data");
    }

    fn receive(&self, waker: core::task::Waker) -> Option<Box<[AclData]>> {
        self.buf_recv.now(waker).and_then(
            |r| {
                match r {
                    Err(e) => {
                        log::error!("{}", e);
                        None
                    },
                    Ok(hci_acl_data) => {
                        hci_acl_data.iter()
                            .try_fold( Vec::new(), |mut vec, hci_data| -> Result<Vec<AclData>, AclDataError> {
                                let raw_data = AclData::from_raw_data(hci_data.get_data())?;

                                vec.push( raw_data );
                                Ok( vec )
                            })
                            .ok()?
                            .into_boxed_slice()
                            .into()
                    },
                }
            })
    }
}
