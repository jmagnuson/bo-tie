//! This is the module for anything part of advertising
//! Advertising data
//!
//! This contains the advertising data types use d for the advertising packet. See Vol 3, Part C
//! section 11 for more details on this.
use core::fmt;

pub enum AssignedTypes {
    Flags,
    IncompleteListOf16bitServiceClassUUIDs,
    CompleteListOf16bitServiceClassUUIDs,
    IncompleteListOf32bitServiceClassUUIDs,
    CompleteListOf32bitServiceClassUUIDs,
    IncompleteListOf128bitServiceClassUUIDs,
    CompleteListOf128bitServiceClassUUIDs,
    ShortenedLocalName,
    CompleteLocalName,
    TxPowerLevel,
    ClassOfDevice,
    SimplePairingHashC,
    SimplePairingHashC192,
    SimplePairingRandomizerR,
    SimplePairingRandomizerR192,
    DeviceID,
    SecurityManagerTKValue,
    SecurityManagerOutOfBandFlags,
    SlaveConnectionIntervalRange,
    ListOf16bitServiceSolicitationUUIDs,
    ListOf128bitServiceSolicitationUUIDs,
    ServiceData,
    ServiceData16BitUUID,
    PublicTargetAddress,
    RandomTargetAddress,
    Appearance,
    AdvertisingInterval,
    LEBluetoothDeviceAddress,
    LERole,
    SimplePairingHashC256,
    SimplePairingRandomizerR256,
    ListOf32bitServiceSolicitationUUIDs,
    ServiceData32BitUUID,
    ServiceData128BitUUID,
    LESecureConnectionsConfirmationValue,
    LESecureConnectionsRandomValue,
    URI,
    IndoorPositioning,
    TransportDiscoveryData,
    LESupportedFeatures,
    ChannelMapUpdateIndication,
    PBADV,
    MeshMessage,
    MeshBeacon,
    _3DInformationData,
    ManufacturerSpecificData,
}

impl AssignedTypes {
    fn val(&self) -> u8 {
        match *self {
            AssignedTypes::Flags => 0x01,
            AssignedTypes::IncompleteListOf16bitServiceClassUUIDs => 0x02,
            AssignedTypes::CompleteListOf16bitServiceClassUUIDs => 0x03,
            AssignedTypes::IncompleteListOf32bitServiceClassUUIDs => 0x04,
            AssignedTypes::CompleteListOf32bitServiceClassUUIDs => 0x05,
            AssignedTypes::IncompleteListOf128bitServiceClassUUIDs => 0x06,
            AssignedTypes::CompleteListOf128bitServiceClassUUIDs => 0x07,
            AssignedTypes::ShortenedLocalName => 0x08,
            AssignedTypes::CompleteLocalName => 0x09,
            AssignedTypes::TxPowerLevel => 0x0A,
            AssignedTypes::ClassOfDevice => 0x0D,
            AssignedTypes::SimplePairingHashC => 0x0E,
            AssignedTypes::SimplePairingHashC192 => 0x0E,
            AssignedTypes::SimplePairingRandomizerR => 0x0F,
            AssignedTypes::SimplePairingRandomizerR192 => 0x0F,
            AssignedTypes::DeviceID => 0x10,
            AssignedTypes::SecurityManagerTKValue => 0x10,
            AssignedTypes::SecurityManagerOutOfBandFlags => 0x11,
            AssignedTypes::SlaveConnectionIntervalRange => 0x12,
            AssignedTypes::ListOf16bitServiceSolicitationUUIDs => 0x14,
            AssignedTypes::ListOf128bitServiceSolicitationUUIDs => 0x15,
            AssignedTypes::ServiceData => 0x16,
            AssignedTypes::ServiceData16BitUUID => 0x16,
            AssignedTypes::PublicTargetAddress => 0x17,
            AssignedTypes::RandomTargetAddress => 0x18,
            AssignedTypes::Appearance => 0x19,
            AssignedTypes::AdvertisingInterval => 0x1A,
            AssignedTypes::LEBluetoothDeviceAddress => 0x1B,
            AssignedTypes::LERole => 0x1C,
            AssignedTypes::SimplePairingHashC256 => 0x1D,
            AssignedTypes::SimplePairingRandomizerR256 => 0x1E,
            AssignedTypes::ListOf32bitServiceSolicitationUUIDs => 0x1F,
            AssignedTypes::ServiceData32BitUUID => 0x20,
            AssignedTypes::ServiceData128BitUUID => 0x21,
            AssignedTypes::LESecureConnectionsConfirmationValue => 0x22,
            AssignedTypes::LESecureConnectionsRandomValue => 0x23,
            AssignedTypes::URI => 0x24,
            AssignedTypes::IndoorPositioning => 0x25,
            AssignedTypes::TransportDiscoveryData => 0x26,
            AssignedTypes::LESupportedFeatures => 0x27,
            AssignedTypes::ChannelMapUpdateIndication => 0x28,
            AssignedTypes::PBADV => 0x29,
            AssignedTypes::MeshMessage => 0x2A,
            AssignedTypes::MeshBeacon => 0x2B,
            AssignedTypes::_3DInformationData => 0x3D,
            AssignedTypes::ManufacturerSpecificData => 0xFF,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    IncorrectDataType,
    IncorrectLength,
    RawTooSmall,
    UTF8Error(::alloc::str::Utf8Error),
    LeBytesConversionError,
}

impl fmt::Display for Error where {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::IncorrectDataType => write!(f, "Incorrect Data Type Field"),
            Error::IncorrectLength => write!(f, "The length of this type is larger than the remaining bytes in the packet"),
            Error::RawTooSmall => write!(f, "Raw data length is too small"),
            Error::UTF8Error(utf8_err) => write!(f, "UTF-8 conversion error, valid up to {}: '{}'",
                utf8_err.valid_up_to(), utf8_err.to_string()),
            Error::LeBytesConversionError => write!(f, "Error converting bytes from le")
        }
    }
}

/// Create a new raw buffer for a data type
///
/// This method is use d for initialize a raw vector with the length & type fields
fn new_raw_type( ad_type: u8 ) -> Vec<u8> {
    vec![0, ad_type]
}

fn set_len( buf: &mut [u8] ) {
    buf[0] = (buf.len() as u8) - 1
}

/// A trait for converting the Advertising Data Structure into its raw (byte slice) form
pub trait IntoRaw where Self: core::marker::Sized {
    /// Convert the data into a vector of bytes
    ///
    /// This converts the data into the form that will be passed from devices over the air
    fn into_raw(&self) -> Vec<u8>;
}

/// A trait for attempting to convert a slice of bytes into an Advertising Data Structure
pub trait TryFromRaw where Self: core::marker::Sized {
    /// Attempt to convert the data from its raw form into this type
    ///
    /// Takes the data protion of one raw advertising or extended inquiry struct and converts
    /// it into this data type.  An error will be returned if the raw data cannot be converted
    /// into this type.
    ///
    /// The passed parameter `raw` needs to refer to a slice to a single data portion *without* the
    /// length byte. The slice should start with the type id. Refer to the Core specification
    /// (Version 5.0 | Vol 3, Part C Section 11) for raw data format details.
    fn try_from_raw(raw: &[u8]) -> Result<Self, Error>;
}

/// $arr is assumed to be an array/slice where the first byte is the ad type.
macro_rules! from_raw {
    ($arr:expr, $( $ad:path, )* $to_do:block) => {
        if $arr.len() > 0 && $($arr[0] == $ad.val())||* {
            Ok($to_do)
        }
        else {
            if $arr.len() == 0 {
                Err(crate::gap::advertise::Error::RawTooSmall)
            }
            else {
                Err(crate::gap::advertise::Error::IncorrectDataType)
            }
        }
    };
}

pub mod flags {

    use core::cell::Cell;
    use alloc::collections::BTreeSet;
    use alloc::rc::Rc;
    use super::*;

    pub enum CoreFlags {
        /// LE limited discoverable mode
        LELimitedDiscoverableMode,
        /// LE general discoverable mode
        LEGeneralDiscoverableMode,
        /// BR/EDR not supported
        BREDRNotSupported,
        /// The controller supports simultanious BR/EDR and LE to the same device
        ControllerSupportsSimultaniousLEAndBREDR,
        /// The host supports simultanious BR/EDR and LE to the same device.
        HostSupportsSimultaniousLEAndBREDR,
    }

    impl CoreFlags {
        /// The number of bits that are required for the core flags & reserved flags
        #[inline]
        fn get_bit_cnt() -> usize { 8 }

        fn get_position(&self) -> usize {
            match *self {
                CoreFlags::LELimitedDiscoverableMode => 0,
                CoreFlags::LEGeneralDiscoverableMode => 1,
                CoreFlags::BREDRNotSupported => 2,
                CoreFlags::ControllerSupportsSimultaniousLEAndBREDR => 3,
                CoreFlags::HostSupportsSimultaniousLEAndBREDR => 4,
            }
        }

        fn from_position(raw: usize) -> Self {
            match raw {
                0 => CoreFlags::LELimitedDiscoverableMode,
                1 => CoreFlags::LEGeneralDiscoverableMode,
                2 => CoreFlags::BREDRNotSupported,
                3 => CoreFlags::ControllerSupportsSimultaniousLEAndBREDR,
                4 => CoreFlags::HostSupportsSimultaniousLEAndBREDR,
                _ => panic!("Position beyond core flags"),
            }
        }
    }

    pub enum FlagType {
        Core(CoreFlags),
        User(usize),
    }

    /// A flag in the `Flags` structure
    ///
    /// This is use d to enable/disable flags retreived from a `Flags` data type. By default
    /// a newly created flag is false, but calling `get` on a flags instance doesn't
    /// gaurentee that the flag is newly created. `enable`, `disable`, or `set` should be
    /// called to explicitly set the state of the flag.
    ///
    /// The highest position *enabled* flag will determine the actual length of the data
    /// for the resulting transmission of Flags data.
    ///
    /// ```rust
    /// # use bo_tie::gap::advertise::flags;
    /// let flags = Flags::new();
    ///
    /// // enable the bluetooth specified flag *LE limited discoverable mode*
    /// flags.get(CoreFlags::LELimitedDiscoverableMode).enable();
    ///
    /// // enable a use r specific flag
    /// flags.get(0).enable();
    /// ```
    #[derive(Eq,Debug)]
    pub struct Flag {
        position: usize,
        val: Cell<bool>,
    }

    impl Flag {

        fn new( position: usize, state: bool ) -> Flag {
            Flag {
                position: position,
                val: Cell::new(state),
            }
        }

        /// Set the state of the flag to enabled
        pub fn enable(self: Rc<Self>) where { self.val.set(true); }

        /// Set the state of the flag to disabled
        pub fn disable(self: Rc<Self>) where { self.val.set(false); }

        /// Set the state of the flag to `state`
        pub fn set(self: Rc<Self>, state: bool) where { self.val.set(state) }

        /// Get the state of the flag
        pub fn get(self: Rc<Self>) -> bool where { self.val.get() }

        pub fn pos(self: Rc<Self>) -> FlagType {
            if self.position < CoreFlags::get_bit_cnt() {
                FlagType::Core(CoreFlags::from_position(self.position))
            }
            else {
                FlagType::User(self.position - CoreFlags::get_bit_cnt())
            }
        }
    }

    impl Ord for Flag {
        fn cmp(&self, other: &Flag) -> ::core::cmp::Ordering {
            self.position.cmp(&other.position)
        }
    }

    impl PartialOrd for Flag {
        fn partial_cmp(&self, other: &Flag) -> Option<::core::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl PartialEq for Flag {
        fn eq(&self, other: &Flag) -> bool {
            self.position == other.position
        }
    }

    #[derive(Debug)]
    pub struct Flags {
        set: BTreeSet<Rc<Flag>>,
    }

    impl Flags {
        const AD_TYPE:AssignedTypes = AssignedTypes::Flags;

        /// Creates a flags object with no enabled flag
        pub fn new() -> Self {
            Flags {
                set: BTreeSet::new(),
            }
        }

        fn get(&mut self, flag: Rc<Flag>) -> Rc<Flag> {
            if self.set.contains(&flag) {
                self.set.get(&flag).unwrap().clone()
            }
            else {
                self.set.insert(flag.clone());

                flag
            }
        }

        /// Get a use r flag for a given position
        ///
        /// Get a flag in the use r defined region after the core flags. A value of zero is the
        /// first use r defined flag. Positions are the relative bit position in the flags data
        /// type after the Bluetooth Supplement specifed flags (and reserved flags). Try to
        /// keep the flag positions stacked towards zero as `pos` / 8 is the number of
        /// bytes for the use r flags that will need to be allocated for this flags data when
        /// transmitting.
        pub fn get_user(&mut self, pos: usize) -> Rc<Flag> {
            self.get(Rc::new(Flag {
                position: pos + CoreFlags::get_bit_cnt(),
                val: Cell::new(false),
            }))
        }

        /// Get a core flag for a given position
        ///
        /// Get a flag in the core defined region before the use r flags.
        pub fn get_core(&mut self, core: CoreFlags) -> Rc<Flag> {
            self.get(Rc::new(Flag {
                position: core.get_position(),
                val: Cell::new(false),
            }))
        }

        /// Get an iterator over the flags in Flags
        pub fn iter(&self) -> ::alloc::collections::btree_set::Iter<Rc<Flag>> {
            self.set.iter()
        }
    }

    impl IntoRaw for Flags {
        fn into_raw(&self) -> Vec<u8> {
            let mut raw = new_raw_type(Self::AD_TYPE.val());

            for ref flag in &self.set {
                // only add flags that are currently enabled
                if flag.val.get() {
                    let octet = flag.position / 8;
                    let bit   = flag.position % 8;

                    // fillout the vec until the octet is reached
                    while raw.len() <= octet {
                        raw.push(0);
                    }

                    raw[octet] |= 1 << bit;
                }
            }

            // Now set the length. One less because length is only for data type + flags. The
            // first byte contains
           set_len(&mut raw);

            raw
        }
    }

    impl TryFromRaw for Flags {

        fn try_from_raw(raw: &[u8]) -> Result<Flags,Error> {
            let mut set = BTreeSet::new();

            from_raw!{ raw, AssignedTypes::Flags, {
                let data = &raw[1..];

                for octet in 0..data.len() {
                    for bit in 0..8 {
                        if 0 != data[octet] & (1 << bit) {
                            set.insert(Rc::new(Flag::new( octet * 8 + (bit as usize), true )));
                        }
                    }
                }

                Flags {
                    set: set
                }
            }}
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn into_raw_test() {
            let mut flags = Flags::new();

            flags.get_core(CoreFlags::LELimitedDiscoverableMode).enable();
            flags.get_user(0).enable();

            let raw = flags.into_raw();

            assert_eq!(vec![1u8,1u8], raw);
        }

        #[test]
        fn from_raw_test() {
            let d_type = AssignedTypes::Flags.val();

            let packet = [4u8, d_type, 3u8, 8u8, 7u8];

            let mut flags = Flags::try_from_raw(&packet[1..]).unwrap();

            assert!(flags.get_core(CoreFlags::LELimitedDiscoverableMode).get());
            assert!(flags.get_core(CoreFlags::LEGeneralDiscoverableMode).get());
            assert!(flags.get_user(3).get());
            assert!(flags.get_user(8).get());
            assert!(flags.get_user(9).get());
            assert!(flags.get_user(10).get());
        }
    }
}

pub mod service_class_uuid {
    //! Service Class UUID Data Type
    //!
    //! The struct Services is the data type for the list of service class UUIDs.

    use alloc::collections::BTreeSet;
    use core::convert::{AsRef, AsMut};
    use core::iter::{IntoIterator, FromIterator};
    use super::*;

    /// Internal trait for specifying the Data Type Value
    ///
    /// For UUIDs there is a complete and an incomplete list version for each UUID type (16,
    /// 32, 128 bit).
    trait DataType {
        const INCOMPLETE: AssignedTypes;
        const COMPLETE: AssignedTypes;
    }

    impl DataType for Services<u16> {
        const COMPLETE: AssignedTypes = AssignedTypes::CompleteListOf16bitServiceClassUUIDs;
        const INCOMPLETE: AssignedTypes = AssignedTypes::IncompleteListOf16bitServiceClassUUIDs;
    }

    impl DataType for Services<u32> {
        const COMPLETE: AssignedTypes = AssignedTypes::CompleteListOf32bitServiceClassUUIDs;
        const INCOMPLETE: AssignedTypes = AssignedTypes::IncompleteListOf32bitServiceClassUUIDs;
    }

    impl DataType for Services<u128> {
        const COMPLETE: AssignedTypes = AssignedTypes::CompleteListOf128bitServiceClassUUIDs;
        const INCOMPLETE: AssignedTypes = AssignedTypes::IncompleteListOf128bitServiceClassUUIDs;
    }

    /// Create a Services data type for 16-bit UUIDs
    ///
    /// This takes one input to indicate if the service list is to be a complete or incomplete
    /// list of service id's.
    pub fn new_16( complete: bool ) -> Services<u16> {
        Services::new(complete)
    }

    /// Create a Services data type for 32-bit UUIDs
    ///
    /// This takes one input to indicate if the service list is to be a complete or incomplete
    /// list of service id's.
    pub fn new_32( complete: bool ) -> Services<u32> {
        Services::new(complete)
    }

    /// Create a Services data type for 128-bit UUIDs
    ///
    /// This takes one input to indicate if the service list is to be a complete or incomplete
    /// list of service id's.
    pub fn new_128( complete: bool ) -> Services<u128> {
        Services::new(complete)
    }

    /// Service UUIDs
    ///
    /// Use the module level functions
    /// `[new_16]`(../fn.new_16.html),
    /// `[new_32]`(../fn.new_32.html), or
    /// `[new_128]` (../fn.new_128.html)
    /// to crunstruct a new, empty `Services` (of 16, 32, or 128 bit UUIDs, respectively).
    ///
    /// This is a set of services uuids with sizes of u16, u32, or u128. `Services` can either
    /// be set as a complete or incomplete list
    ///
    /// `Services` is a set of uuids, so duplicate uuids cannot exist within an instance of
    /// `Services`
    ///
    /// Services implements `AsRef` for `BTreeSet` so use the methods of `BTreeSet` for editing
    /// the UUIDs in the instance
    #[derive(Clone, Debug)]
    pub struct Services<T> where T: Ord {
        set: BTreeSet<T>,
        complete: bool,
    }

    impl<T> Services<T> where T: Ord {

        fn new( complete: bool ) -> Self {
            Self {
                set: BTreeSet::new(),
                complete: complete
            }
        }

        pub fn is_complete(&self) -> bool {
            self.complete
        }

        /// Add uuids to the list of uuids
        ///
        /// Returns true if the uuid is added, otherwise returns false if the uuid is already in
        /// the set.
        pub fn insert(&mut self, uuid: T) -> bool {
            self.set.insert(uuid)
        }
    }

    impl<T> AsRef<BTreeSet<T>> for Services<T> where T: Ord
    {
        fn as_ref(&self) -> &BTreeSet<T> {
            &self.set
        }
    }

    impl<T> AsMut<BTreeSet<T>> for Services<T> where T: Ord
    {
        fn as_mut(&mut self) -> &mut BTreeSet<T> {
            &mut self.set
        }
    }

    impl<T> core::ops::Deref for Services<T> where T: Ord
    {
        type Target = BTreeSet<T>;

        fn deref(&self) -> &Self::Target {
            self.as_ref()
        }
    }

    impl<T> IntoIterator for Services<T> where T: ::std::cmp::Ord {
        type Item = T;
        type IntoIter = <BTreeSet<T> as IntoIterator>::IntoIter;

        /// Usefull for iterating over the contained UUIDs, but after this is done you obviously
        /// cannot tell if the list is complete or not.
        fn into_iter(self) -> Self::IntoIter {
            self.set.into_iter()
        }
    }

    /// Using this constructs a new *complete* Services from the provided iterator
    impl<T,B> FromIterator<T> for Services<B> where B: Ord, T: Into<B> {
        fn from_iter<Iter>(iter: Iter) -> Self where Iter: IntoIterator<Item = T> {
            let mut services = Self::new(true);

            for i in iter {
                services.insert(i.into());
            }

            services
        }
    }

    macro_rules! impl_from_services {
        ( $uuid_type_to:ty, $( $uuid_type_from:ty),+ ) => {
            $( impl<'a> From<Services<$uuid_type_from>> for Services<$uuid_type_to> {

                fn from( services: Services<$uuid_type_from> ) -> Self {
                    services.into_iter().map( |uuid| uuid.clone() as $uuid_type_to ).collect()
                }
            } )*
        };
    }

    impl_from_services!{u128,u32,u16}
    impl_from_services!{u32,u16,&u16} // todo double check that this is correct

    macro_rules! impl_from_for_slice_with_complete {
        ( $type: ty ) => {
            impl<'a> From<( &'a [$type], bool)> for Services<$type> {

                fn from((uuids, complete): ( &[$type], bool)) -> Self {
                    let mut services = Self::new(complete);

                    for uuid in uuids {
                        services.set.insert(*uuid);
                    }

                    services
                }

            }
        }
    }

    impl_from_for_slice_with_complete!{u16}
    impl_from_for_slice_with_complete!{u32}
    impl_from_for_slice_with_complete!{u128}

    /// Implementation for pimitive type numbers
    ///
    /// Requires `$type` to implement method to_le
    macro_rules! impl_raw {
        ( $type:tt ) => {
            impl IntoRaw for Services<$type> {

                fn into_raw(&self) -> Vec<u8> {

                    let data_type = if self.set.is_empty() || self.complete {
                        Self::COMPLETE
                    } else {
                        Self::INCOMPLETE
                    };

                    let mut raw = self.set.iter()
                    .map(|v| $type::to_le_bytes(*v) )
                    .fold(new_raw_type(data_type.val()), |mut raw, slice| {
                        raw.extend_from_slice(&slice);
                        raw }
                    );

                    set_len(&mut raw);

                    raw

                }
            }

            impl TryFromRaw for Services<$type> {

                fn try_from_raw( raw: &[u8] ) -> Result<Services<$type>,Error> {
                    from_raw!{raw, Self::COMPLETE, Self::INCOMPLETE, {
                        use core::mem::size_of;

                        Services::<$type> {
                            set: raw[1..].chunks_exact(size_of::<$type>())
                                .map( |raw_uuid| {
                                    unsafe{ $type::from_le(*(raw_uuid.as_ptr() as *const $type)) }
                                })
                                .collect::<BTreeSet<$type>>(),
                            // from_raw does the check to see if the data is Self::COMPLETE or
                            // Self::INCOMPLETE. All that needs to be done here is to check
                            // if this is the complete one or not.
                            complete: Self::COMPLETE.val() == raw[1],
                        }
                    }}
                }
            }
        }
    }

    impl_raw!{u16}
    impl_raw!{u32}
    impl_raw!{u128}
}

pub mod service_data {
    //! Service Class UUID Data Type
    //!
    //! The struct Services is the data type for the list of service class UUIDs paired with
    //! service data. It is implemented for the three types of UUIDs (16, 32, and 128 bit)
    //! and to create an instance of it use the functions `new_16`, `new_32`, or
    //! `new_128` at the module level.

    use super::*;

    /// Create service data for 16-bit UUID's
    pub fn new_16<Data>(uuid: u16, data: &Data) -> crate::serializer::Result<ServiceData<u16>>
        where Data: ::serde::Serialize
    {
        ServiceData::new(uuid, data)
    }

    /// Create service data for 32-bit UUID's
    pub fn new_32<Data>(uuid: u32, data: &Data) -> crate::serializer::Result<ServiceData<u32>>
        where Data: ::serde::Serialize
    {
        ServiceData::new(uuid, data)
    }

    /// Create service data for 64-bit UUID's
    pub fn new_128<Data>(uuid: u128, data: &Data) -> crate::serializer::Result<ServiceData<u128>>
        where Data: ::serde::Serialize
    {
        ServiceData::new(uuid, data)
    }

    /// Service Data
    ///
    /// Contains a UUID along with the coresponding data for that UUID
    ///
    /// Use the module level functions
    /// `[new_16]`(../fn.new_16.html),
    /// `[new_32]`(../fn.new_32.html), or
    /// `[new_128]` (../fn.new_128.html)
    /// to crunstruct a new, empty `ServiceData` (of 16, 32, or 128 bit UUIDs, respectively).
    #[derive(Clone, Debug)]
    pub struct ServiceData<UuidType> {
        uuid: UuidType,
        pub(crate) serialized_data: Vec<u8>,
    }

    impl<UuidType> ServiceData<UuidType>
    {
        const AD_TYPE: AssignedTypes = AssignedTypes::ServiceData;

        fn new<Data>(uuid: UuidType, data: &Data) -> crate::serializer::Result<Self>
            where Data: ::serde::Serialize
        {
            Ok(ServiceData {
                uuid: uuid,
                serialized_data: crate::serializer::serialize(&data)?,
            })
        }

        pub fn get_uuid(&self) -> UuidType where UuidType: Copy {
            self.uuid
        }

        /// Attemp to get the service data as `Data`
        pub fn get_data<'d, Data>(&'d self) -> crate::serializer::Result<Data>
            where Data: ::serde::Deserialize<'d>
        {
            crate::serializer::deserialize(&self.serialized_data)
        }

        /// Get a reference to the serialized data
        pub fn get_serialized_data<'a>(&'a self) -> &'a [u8] {
            self.serialized_data.as_ref()
        }

        /// Consume self and get the serialized data
        pub fn into_serialized_data(self) -> Box<[u8]> {
            self.serialized_data.into_boxed_slice()
        }
    }

    macro_rules! impl_raw {
        ( $type:tt, $ad_type:path ) => {
            impl IntoRaw for ServiceData<$type> {

                fn into_raw(&self) -> Vec<u8> {
                    let mut raw = new_raw_type(Self::AD_TYPE.val());

                    raw.extend_from_slice(&self.uuid.to_le_bytes());

                    raw.extend(self.serialized_data.clone());

                    set_len(&mut raw);

                    raw
                }
            }

            impl TryFromRaw for ServiceData<$type> {

                fn try_from_raw( raw: &[u8] ) -> Result<ServiceData<$type>,Error> {
                    let ad_type = $ad_type;
                    from_raw!{raw, ad_type, {
                        use std::convert::TryInto;

                        if raw.len() >= 3 {
                            let (uuid_raw, data) = raw.split_at(std::mem::size_of::<$type>());
                            let err = crate::gap::advertise::Error::LeBytesConversionError;

                            ServiceData {
                                uuid: $type::from_le_bytes(uuid_raw.try_into().or(Err(err))?),
                                serialized_data: Vec::from(data),
                            }
                        }
                        else {
                            return Err(crate::gap::advertise::Error::RawTooSmall)
                        }
                    }}
                }
            }
        }
    }

    impl_raw!{u16, AssignedTypes::ServiceData16BitUUID }
    impl_raw!{u32, AssignedTypes::ServiceData32BitUUID }
    impl_raw!{u128, AssignedTypes::ServiceData128BitUUID }
}

pub mod local_name {
    //! Local name data type
    use super::*;

    pub struct LocalName {
        name: String,
        is_short: bool,
    }

    impl LocalName {
        const SHORTENED_TYPE: AssignedTypes = AssignedTypes::ShortenedLocalName;
        const COMPLETE_TYPE: AssignedTypes = AssignedTypes::CompleteLocalName;

        /// Create a new loca name data type
        ///
        /// If the name is 'short' then set the `short` parameter to true.
        /// See the Bluetooth Core Supplement Spec. section 1.2.1 for more details.
        pub fn new<T>(name: T, short: bool) -> Self where T: Into<String>{
            Self {
                name: name.into(),
                is_short: short,
            }
        }

        pub fn is_short(&self) -> bool {
            self.is_short
        }
    }

    impl AsRef<str> for LocalName {
        fn as_ref(&self) -> &str {
            &self.name
        }
    }

    impl From<LocalName> for String {
        fn from( ln: LocalName) -> String {
            ln.name
        }
    }

    impl IntoRaw for LocalName {
        fn into_raw(&self) -> Vec<u8> {


            let data_type = if self.is_short {
                Self::SHORTENED_TYPE
            }
            else {
                Self::COMPLETE_TYPE
            };

            let mut val = new_raw_type(data_type.val());

            val.extend(self.name.clone().bytes());

            set_len(&mut val);

            val
        }
    }

    impl TryFromRaw for LocalName {

        fn try_from_raw(raw: &[u8]) -> Result<Self,Error> {
            from_raw!(raw, Self::SHORTENED_TYPE, Self::COMPLETE_TYPE, {
                use core::str::from_utf8;

                let ref_name = if raw.len() > 1 {
                    from_utf8(&raw[1..]).map_err(|e| super::Error::UTF8Error(e) )?
                } else {
                    ""
                };

                Self {
                    name: ref_name.to_string(),
                    is_short: raw[0] == Self::SHORTENED_TYPE.val(),
                }
            })
        }
    }
}

#[derive(Debug)]
pub struct DataTooLargeError {
    pub(crate) overflow: usize,
    pub(crate) remaining: usize,
}

impl DataTooLargeError {
    /// Return the number of bytes that would overflow the advertising packet buffer
    pub fn overflow(&self) -> usize {
        self.overflow
    }

    /// The number of bytes remaining in the advertising buffer at the time that this error was
    /// generated.
    pub fn remaining(&self) -> usize {
        self.remaining
    }
}

impl ::core::fmt::Display for DataTooLargeError {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) ->core::fmt::Result {
        write!(f, "Advertising Data Too Large")
    }
}
