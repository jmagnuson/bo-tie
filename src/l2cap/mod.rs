/// Logical Link Control and Adaption protocol (L2CAP)

use alloc::{
    vec::Vec,
};

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

impl From<LeUserChannelIdentifier> for ChannelIdentifier {
    fn from( le: LeUserChannelIdentifier ) -> Self {
        ChannelIdentifier::LE(le)
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
    SecurityManagerProtocol,
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
            LeUserChannelIdentifier::SecurityManagerProtocol => 0x6,
            LeUserChannelIdentifier::DynamicallyAllocated(dyn_id) => dyn_id.channel_id,
        }
    }

    fn try_from_raw(val: u16) -> Result<Self, ()>  {
        match val {
            0x4 => Ok( LeUserChannelIdentifier::AttributeProtocol ),
            0x5 => Ok( LeUserChannelIdentifier::LowEnergyL2CAPSignalingChannel ),
            0x6 => Ok( LeUserChannelIdentifier::SecurityManagerProtocol),
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
            AclDataError::InvalidChannelId => write!(f, "Invalid Channel Id"),
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
            channel_id,
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

    /// Create an AclData struct from a non-fragmented raw l2cap acl data
    ///
    /// # Errors
    /// * The length of the raw data must be >= 4
    /// * The length value in the raw data must be less than or equal to the length of the payload
    ///   portion of the raw data
    /// * The channel id must be valid
    pub fn from_raw_data(data: &[u8]) -> Result<Self, AclDataError> {
        if data.len() >= 4 {
            let len = <u16>::from_le_bytes( [data[0], data[1]] ) as usize;
            let raw_channel_id = <u16>::from_le_bytes( [data[2], data[3]] );
            let payload = &data[4..];

            if len <= payload.len() {
                Ok( Self {
                    channel_id: ChannelIdentifier::LE(
                        LeUserChannelIdentifier::try_from_raw(raw_channel_id).or(Err(AclDataError::InvalidChannelId))?
                    ),
                    data: payload[..len].to_vec(),
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

/// A Complete or Fragmented Acl Data
///
/// Packets sent between the Master and Slave may be fragmented and need to be combined into a
/// complete [`AclData`]. Multiple AclDataFragments, when in order and complete, can be combined
/// into a single 'AclData' through the use of 'FromIterator' for AclData.
pub struct AclDataFragment {
    start_fragment: bool,
    data: Vec<u8>
}

impl AclDataFragment {

    /// Crate a 'AclDataFragment'
    pub(crate) fn new(start_fragment: bool, data: Vec<u8>) -> Self {
        Self {start_fragment, data}
    }

    /// Get the length of the payload as specified in the ACL data
    ///
    /// This returns None if this packet doesn't contain the length parameter
    pub fn get_acl_len(&self) -> Option<usize> {
        if self.start_fragment && self.data.len() > 2 {
            Some( <u16>::from_le_bytes([ self.data[0], self.data[1] ]) as usize )
        } else {
            None
        }
    }

    pub fn is_start_fragment(&self) -> bool { self.start_fragment }

    pub fn fragment_data(&self) -> &[u8] { &self.data }
}

pub struct L2capPdu {
    data: alloc::vec::Vec<u8>,
    mtu: Option<usize>,
}

impl L2capPdu {
    pub(crate) fn into_data(self) -> alloc::vec::Vec<u8> { self.data }

    pub(crate) fn get_mtu(&self) -> Option<usize> { self.mtu }
}

/// Create a `L2capPdu` from an `AclData` when the packet length is know to be less than or equal
/// to the maximum transmission unit (MTU) .
impl From<AclData> for L2capPdu {
    fn from(acl_data: AclData) -> Self {
        L2capPdu {
            data: acl_data.into_raw_data(),
            mtu: None
        }
    }
}

/// Create a `L2capPdu` from an `AclData` where the size may be larger then the MTU.
///
/// # Note
/// If mtu is smaller then 
/// [`MINIMUM_LE_U_FRAGMENT_START_SIZE`](crate::hci::HciAclData::MINIMUM_LE_U_FRAGMENT_START_SIZE)
/// then it is sent as that size to the controller over the Host Controller Interface for LE-U.
impl<Mtu> From<(AclData, Mtu)> for L2capPdu where Mtu: Into<Option<usize>>
{
    fn from((acl_data, mtu): (AclData, Mtu)) -> Self {
        Self {
            data: acl_data.into_raw_data(),
            mtu: mtu.into()
        }
    }
}

/// A Connection channel
///
/// A connection channel is used for sending and receiving Asynchronous Connection-oriented (ACL)
/// data packets between the Host and Bluetooth Controller.
pub trait ConnectionChannel {
    fn send<Pdu>(&self, data: Pdu) where Pdu: Into<L2capPdu>;

    fn receive(&self, waker: &core::task::Waker) -> Option<Vec<AclDataFragment>>;

    fn future_receiver(&self) -> ConChanFutureRx<'_, Self> {
        ConChanFutureRx {
            cc: self,
            full_acl_data: Vec::new(),
            carryover_fragments: Vec::new(),
            length: None,
        }
    }
}

/// A future for asynchronously waiting for received packets from the connected device
///
/// This struct is created via the function [`future_receiver`](ConnectionChannel::future_receiver)
/// in the trait [`ConnectionChannel`].
///
/// This implements [`Future`](https://doc.rust-lang.org/core/future/trait.Future.html) for polling
/// the Bluetooth Controller to obtain complete [`AclData`] (L2CAP data packets). `ConChanFutureRx`
/// is effectively a packet defragmenter for packets received by the controller.
///
/// # How It Works
/// When poll is called, the function will receive all the available ACL data fragments from the
/// backend driver and try to assemble the packets into complete ACL data.
///
/// If all fragments received can be converted into complete L2CAP packets, then `Poll::Ready` is
/// returned will all the packets.
///
/// When the all fragments cannot be converted into complete ACL Packets, then `Poll::Pending` is
/// returned, and the completed packets along with the incomplete fragments are saved for the next
/// poll. Upon polling again, if the newly received fragments can be assembled with the saved
/// fragments to make complete L2CAP packets then `Poll::Ready` is returned with all the L2CAP
/// packets (saved and newly assembled).  Otherwise `Poll::Pending` is returned and the process
/// repeats itself.
pub struct ConChanFutureRx<'a, C> where C: ?Sized {
    cc: &'a C,
    full_acl_data: Vec<AclData>,
    carryover_fragments: Vec<u8>,
    length: Option<usize>,
}

impl<'a, C> ConChanFutureRx<'a, C> where C: ?Sized {

    /// Get the complete, de-fragmented, received ACL Data
    ///
    /// This is useful when resulting `poll` may contain many complete packets, but still returns
    /// `Poll::Pending` because there were also incomplete fragments received.
    pub fn get_received_packets(&mut self) -> Vec<AclData> {
        core::mem::replace(&mut self.full_acl_data, Vec::new() )
    }
}

impl<'a,C> core::future::Future for ConChanFutureRx<'a,C>
where C: ConnectionChannel
{
    type Output = Result<Vec<AclData>, AclDataError>;

    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> core::task::Poll<Self::Output> {

        // The size of the L2CAP data header
        const HEADER_SIZE:usize = 4;

        use core::task::Poll;

        let this = self.get_mut();

        loop {
            if let Some(ret) = match this.cc.receive(cx.waker()) {
                None => return Poll::Pending,
                Some(fragments) => {
                    match fragments.into_iter().try_for_each( |mut f| {
        
                        // Continue if f is just an empty fragment, there is nothing to do
                        if f.data.len() == 0 { return Ok(()) }
        
                        if this.carryover_fragments.is_empty()
                        {
                            match f.get_acl_len() {
                                Some(l) if (l + HEADER_SIZE) <= f.data.len()  => {
                                    match AclData::from_raw_data(&f.data) {
                                        Ok(data) => this.full_acl_data.push(data),
                                        Err(e) => return Err(e)
                                    }
                                },
                                len @ Some(_) => {
                                    this.carryover_fragments.append(&mut f.data);
                                    this.length = len;
                                },
                                None => {
                                    this.carryover_fragments.append(&mut f.data);
                                },
                            }
                        } else {
                            this.carryover_fragments.append(&mut f.data);
        
                            let acl_len = match this.length {
                                None => {
                                    // There will always be at least 2 items to take because a starting
                                    // fragment and a proceeding fragment have been received and empty
                                    // fragments are not added to `self.carryover_fragments`.
                                    let len_bytes = this.carryover_fragments.iter()
                                        .take(2)
                                        .enumerate()
                                        .fold([0u8;2], |mut a, (i, &v)| { a[i] = v; a });
        
                                    let len = <u16>::from_le_bytes(len_bytes) as usize;
        
                                    this.length = Some(len);
        
                                    len
                                },
                                Some(len) => len,
                            };
        
                            if (acl_len + HEADER_SIZE) <= this.carryover_fragments.len() {
                                match AclData::from_raw_data(&this.carryover_fragments) {
                                    Ok(data) => {
                                        this.full_acl_data.push(data);
                                        this.carryover_fragments.clear();
                                    },
                                    Err(e) => return Err(e)
                                }
                            }
                        }
        
                        Ok(())
                    }) {
                        // Body of match statement

                        Ok(_) => {
                            if this.carryover_fragments.is_empty() &&
                                !this.full_acl_data.is_empty()
                            {
                                Some( Ok(core::mem::replace(&mut this.full_acl_data, Vec::new())) )
                            } else {
                                None
                            }
                        },
                        Err(e) => Some( Err(e) )
                    }
                }
            } { 
                // Body of "if Some(ret) = ... "
                return Poll::Ready(ret);

                // Loop continues if None is returned by match statement
            }
        }
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
