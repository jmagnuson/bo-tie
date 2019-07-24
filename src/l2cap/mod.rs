/// Logical Link Control and Adaption protocol (L2CAP)

use alloc::{
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
    NullIdentifier,
    /// LE User (Logical link) identifiers
    LE(LeUserChannelIdentifier)
}

impl ChannelIdentifier {
    /// Convert to the numerical value
    ///
    /// The returned value is in *native byte order*
    pub fn to_val(&self) -> u16 {
        match self {
            ChannelIdentifier::NullIdentifier => 0,
            ChannelIdentifier::LE(ci) => ci.to_val(),
        }
    }

    /// Try to convert a raw value into a LeUserChannelIdentifier
    ///
    /// This function expects the input to be in *native byte order*
    pub fn try_from_raw(val: u16) -> Result<Self, ()> {

        // TODO add BR/EDR CI check

        Ok(ChannelIdentifier::LE(LeUserChannelIdentifier::try_from_raw(val)?))
    }
}

/// Dynamicly created l2cap channel
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
    /// [`DynamicallyAllocated`](../enum.LeUserChannelIdentifier.html#variant.DynamicallyAllocated)
    /// with the `channel_id` if the id is within the bounds of
    /// [`LE_LOWER`](#const.LE_LOWER) and
    /// [`LE_UPPER`](#const.LE_UPPER). If the input is not between those bounds, then an error is
    /// returned containing the infringing input value.
    pub fn new_le( channel_id: u16 ) -> Result< LeUserChannelIdentifier, u16 > {
        if Self::LE_BOUNDS.contains(&channel_id) {
            Ok( LeUserChannelIdentifier::DynamicallyAllocated( DynChannelId::new(channel_id) ) )
        } else {
            Err( channel_id )
        }
    }
}

/// LE User (LE-U) Channel Identifiers
///
/// These are the channel identifiers for a LE
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum LeUserChannelIdentifier {
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

impl LeUserChannelIdentifier {

    fn to_val(&self) -> u16 {
        match self {
            LeUserChannelIdentifier::AttributeProtocol => 0x4,
            LeUserChannelIdentifier::LowEnergyL2CAPSignalingChannel => 0x5,
            LeUserChannelIdentifier::SecurityManagerProtocl => 0x6,
            LeUserChannelIdentifier::DynamicallyAllocated(dyn_id) => dyn_id.channel_id,
        }
    }

    fn try_from_raw(val: u16) -> Result<Self, ()>  {
        match val {
            0x4 => Ok( LeUserChannelIdentifier::AttributeProtocol ),
            0x5 => Ok( LeUserChannelIdentifier::LowEnergyL2CAPSignalingChannel ),
            0x6 => Ok( LeUserChannelIdentifier::SecurityManagerProtocl ),
            _ if DynChannelId::LE_BOUNDS.contains(&val) =>
                Ok( LeUserChannelIdentifier::DynamicallyAllocated( DynChannelId::new(val) ) ),
            _ => Err(()),
        }
    }
}

/// Acl Data Errors
#[derive(Debug)]
pub enum AclDataError {
    /// Raw data is too small for an ACL frame
    RawDataTooSmall,
    /// Specified payload length didn't match the actual payload length
    PayloadLengthIncorrect,
    /// Invalid Channel Id
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
    data: Vec<u8>
}

impl AclData {
    pub fn new( payload: Vec<u8>, channel_id: ChannelIdentifier ) -> Self {
        AclData {
            channel_id: channel_id,
            data: payload,
        }
    }

    pub fn get_channel_id(&self) -> ChannelIdentifier { self.channel_id }

    pub fn get_payload(&self) -> &[u8] { &self.data }

    pub fn into_raw_data(&self) -> Vec<u8> {
        use core::convert::TryInto;

        let mut v = Vec::new();

        let len: u16 = self.data.len().try_into().expect("Couldn't convert into u16");

        v.extend_from_slice( &len.to_le_bytes() );

        v.extend_from_slice( &self.channel_id.to_val().to_le_bytes() );

        v.extend_from_slice( &self.data );

        v
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
                        LeUserChannelIdentifier::try_from_raw(raw_channel_id).or(Err(AclDataError::InvalidChannelId))?
                    ),
                    data: Vec::from(payload),
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
    fn receive(&self, waker: core::task::Waker) -> Option<Vec<AclData>>;
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

    /// Create a new LE ACL channel from the
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
            data.into_raw_data().into()
        );

        self.hi_ref.send_data(hci_acl_data).expect("Failed to send hci acl data");
    }

    fn receive(&self, waker: core::task::Waker) -> Option<Vec<AclData>> {
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
                            .into()
                    },
                }
            })
    }
}

/// Protocol and Service Multiplexers
///
/// This is a wrapper around the numerical number of the PSM. There are two ways to create a `Psm`.
/// One way is to convert one of the enumerations of
/// [`PsmAssignedNum`](PsmAssignedNum)
/// into this, the other way is to create a dynamic PSM with the function
/// [`new_dyn`](#method.new_dyn).
pub struct Psm { val: u16 }

impl Psm {

    /// Get the value of the PSM
    ///
    /// The returned value is in *native byte order*
    pub fn to_val(&self) -> u16 {
        self.val
    }

    /// Create a new *dynamic* PSM
    ///
    /// This will create a dynamic PSM if the input `dyn_psm` is within the acceptable range of
    /// dynamically allocated PSM values (see the Bluetooth core spec v 5.0 | Vol 3, Part A).
    ///
    /// # Note
    /// For now extended dynamic PSM's are not supported as I do not know how to support them (
    /// see
    /// [`DynPsmIssue`](DynPsmIssue) for why)
    pub fn new_dyn( dyn_psm: u16 ) -> Result<Self, DynPsmIssue> {
        match dyn_psm {
            _ if dyn_psm <= 0x1000 => Err( DynPsmIssue::NotDynamicRange ),
            _ if dyn_psm & 0x1 == 0 => Err( DynPsmIssue::NotOdd ),
            _ if dyn_psm & 0x100 != 0 => Err( DynPsmIssue::Extended ),
            _ => Ok( Psm { val: dyn_psm } )
        }
    }
}

impl From<PsmAssignedNum> for Psm {
    fn from( pan: PsmAssignedNum ) -> Psm {

        let val = match pan {
            PsmAssignedNum::Sdp => 0x1,
            PsmAssignedNum::Rfcomm => 0x3,
            PsmAssignedNum::TcsBin => 0x5,
            PsmAssignedNum::TcsBinCordless => 0x7,
            PsmAssignedNum::Bnep => 0xf,
            PsmAssignedNum::HidControl => 0x11,
            PsmAssignedNum::HidInterrupt => 0x13,
            PsmAssignedNum::Upnp => 0x15,
            PsmAssignedNum::Avctp => 0x17,
            PsmAssignedNum::Avdtp => 0x19,
            PsmAssignedNum::AvctpBrowsing => 0x1b,
            PsmAssignedNum::UdiCPlane => 0x1d,
            PsmAssignedNum::Att => 0x1f,
            PsmAssignedNum::ThreeDsp => 0x21,
            PsmAssignedNum::LePsmIpsp => 0x23,
            PsmAssignedNum::Ots => 0x25,
        };

        Psm { val }
    }
}

/// Protocol and Service Multiplexers assigned numbers
///
/// The enumartions defined in `PsmAssignedNum` are those listed in the Bluetooth SIG assigned
/// numbers.
pub enum PsmAssignedNum {
    /// Service Disconvery Protocol
    Sdp,
    /// RFCOMM
    Rfcomm,
    /// Telephony Control Specification
    TcsBin,
    /// Telephony Control Specification ( Dordless )
    TcsBinCordless,
    /// Network Encapsulation Protocol
    Bnep,
    /// Human Interface Device ( Control )
    HidControl,
    /// Human Interface Device ( Interrupt )
    HidInterrupt,
    /// ESDP(?)
    Upnp,
    /// Audio/Video Control Transport Protocol
    Avctp,
    /// Audio/Video Distribution Transport Protocol
    Avdtp,
    /// Audio/Video Remote Control Profile
    AvctpBrowsing,
    /// Unrestricted Digital Information Profile
    UdiCPlane,
    /// Attribute Protocol
    Att,
    /// 3D Synchronization Profile
    ThreeDsp,
    /// Internet Protocol Support Profile
    LePsmIpsp,
    /// Object Transfer Service
    Ots,
}

/// The issue with the provided PSM value
///
/// ### NotDynamicRange
/// Returned when the PSM is within the assigned number range of values. Dynamic values need to be
/// larger then 0x1000.
///
/// ### NotOdd
/// All PSM values must be odd, the value provided was even
///
/// ### Extended
/// The least signaficant bit of the most significant byte (aka bit 8) must be 0 unless you want
/// an extended PSM (but I don't know what that is as I don't want to pay 200 sweedish dubloons
/// for ISO 3309 to find out what that is). For now extended PSM is not supported.
pub enum DynPsmIssue {
    NotDynamicRange,
    NotOdd,
    Extended,
}

impl core::fmt::Display for DynPsmIssue {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            DynPsmIssue::NotDynamicRange =>
                write!(f, "Dynamic PSM not within allocated range"),
            DynPsmIssue::NotOdd =>
                write!(f, "Dynamic PSM value is not odd"),
            DynPsmIssue::Extended =>
                write!(f, "Dynamic PSM has extended bit set"),
        }
    }
}
