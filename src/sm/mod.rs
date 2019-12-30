//! Bluetooth Security Manager
//!
//! The Security Manager is used to manage the pairing process and key distribution between two
//! connected devices. A [`SecurityManager`] is used to contain the keys generated and used for
//! encrypting messages between this device and the devices it is currently or was connected to.
//!
//! For each connection either a ['MasterSecurityManager'] or a ['SlaveSecurityManager'] is created
//! based on the role of this device in the connection. The `MasterSecurityManager` can be used for
//! initializing the pairing process and for re-establishing encryption to the slave device.
//! `SlaveSecurityManger` is used by the slave device as the responder to pairing requests.
//!
//! # Async
//! Both the `Async` and non-`Async` prepended Managers utilize asynchronous operations for I/O
//! to the Bluetooth Radio. What the `Async` versions do is further use the Bluetooth Controller for
//! the encryption calculations that require either AES or the generation of a elliptic curve
//! Diffie-Hellman key pair.
//!
//! The ['AsyncMasterSecurityManager'] and ['AsyncSlaveSecurityManager'] are versions of
//! ['MasterSecurityManager'] or a ['SlaveSecurityManager'] which can be used when it desired for
//! the controller to perform the encryption of the cleartext and to generate the Diffie-Hellman
//! Key, but make sure that the controller supports both of these Host Controller Interface commands
//! ( See the Bluetooth Specification v5.0 | Vol 2, Part E, sections 7.8.22-26 and 7.8.37). These
//! may not
//!
//! # Note
//! This module uses the following crates for parts of the encryption process.
//! * ['aes'](https://crates.io/crates/aes)
//! * ['ring'](https://crates.io/crates/ring)
//!
//! The assumption was made that these crates are adequate for their required usage within this
//! module, but no formal process was used to validate them for use with this library.
//! ['MasterSecurityManagerAsync'] and ['AsyncSlaveSecurityManager'] can be used if you don't trust
//! these crates, but they do require that the adequate functionality be present on the Bluetooth
//! Controller.
//!
//! # Temporary Note
//! For now passkey pairing is not supported. Only Numeric Comparison and Out Of Band are supported

use alloc::vec::Vec;
use core::sync::atomic;
use serde::{Serialize, Deserialize};

use crate::l2cap::ConnectionChannel;

pub mod toolbox;
pub mod pairing;
pub mod encrypt_info;
pub mod responder;
pub mod initiator;

const L2CAP_LEGACY_MTU: usize = 23;
const L2CAP_SECURE_CONNECTIONS_MTU: usize = 65;

const ENCRYPTION_KEY_MIN_SIZE: usize = 7;
const ENCRYPTION_KEY_MAX_SIZE: usize = 16;

const SECURITY_MANAGER_L2CAP_CHANNEL_ID: crate::l2cap::ChannelIdentifier =
    crate::l2cap::ChannelIdentifier::LE(crate::l2cap::LeUserChannelIdentifier::SecurityManagerProtocol);

#[derive(Debug)]
pub enum Error {
    Size,
    Format,
    IncorrectValue,
    IncorrectCommand(CommandType),
    UnsupportedFeature,
    PairingFailed(pairing::PairingFailedReason),
    EncryptionFailed(alloc::boxed::Box<dyn core::fmt::Debug>),
}

#[derive(Debug,PartialEq,Eq,Clone,Copy)]
pub enum CommandType {
    PairingRequest,
    PairingResponse,
    PairingConfirm,
    PairingRandom,
    PairingFailed,
    EncryptionInformation,
    MasterIdentification,
    IdentityInformation,
    IdentityAddressInformation,
    SigningInformation,
    SecurityRequest,
    PairingPublicKey,
    PairingDHKeyCheck,
    PairingKeyPressNotification,
}

impl CommandType {
    fn into_val(self) -> u8 {
        match self {
            CommandType::PairingRequest => 0x1,
            CommandType::PairingResponse => 0x2,
            CommandType::PairingConfirm => 0x3,
            CommandType::PairingRandom => 0x4,
            CommandType::PairingFailed => 0x5,
            CommandType::EncryptionInformation => 0x6,
            CommandType::MasterIdentification => 0x7,
            CommandType::IdentityInformation => 0x8,
            CommandType::IdentityAddressInformation => 0x9,
            CommandType::SigningInformation => 0xa,
            CommandType::SecurityRequest => 0xb,
            CommandType::PairingPublicKey => 0xc,
            CommandType::PairingDHKeyCheck => 0xd,
            CommandType::PairingKeyPressNotification => 0xe,
        }
    }

    fn try_from_val(val: u8) -> Result<Self, Error> {
        match val {
            0x1 => Ok( CommandType::PairingRequest ),
            0x2 => Ok( CommandType::PairingResponse ),
            0x3 => Ok( CommandType::PairingConfirm ),
            0x4 => Ok( CommandType::PairingRandom ),
            0x5 => Ok( CommandType::PairingFailed ),
            0x6 => Ok( CommandType::EncryptionInformation ),
            0x7 => Ok( CommandType::MasterIdentification ),
            0x8 => Ok( CommandType::IdentityInformation ),
            0x9 => Ok( CommandType::IdentityAddressInformation ),
            0xa => Ok( CommandType::SigningInformation ),
            0xb => Ok( CommandType::SecurityRequest ),
            0xc => Ok( CommandType::PairingPublicKey ),
            0xd => Ok( CommandType::PairingDHKeyCheck ),
            0xe => Ok( CommandType::PairingKeyPressNotification ),
            _   => Err( Error::IncorrectValue )
        }
    }
}

/// Command Data
///
/// A trait for converting to or from the data format sent over the radio as specified in the
/// Bluetooth Specification Security Manager Protocol (V.5.0 | Vol 3, Part H
trait CommandData where Self: Sized {

    /// Convert into the interface formatted command data
    fn into_icd(self) -> Vec<u8>;

    /// Convert from the interface formatted command data
    ///
    /// If `icd` is incorrectly formatted or sized an `Err` is returned.
    fn try_from_icd(icd: &[u8]) -> Result<Self, Error>;
}

struct Command<D> {
    command_type: CommandType,
    data: D,
}

impl<D> Command<D> {
    fn new( command_type: CommandType, data: D) -> Self {
        Command { command_type, data }
    }
}

impl<D> CommandData for Command<D> where D: CommandData {

    fn into_icd(self) -> Vec<u8> {
        let mut data_v = self.data.into_icd();

        let mut rec = Vec::with_capacity(1 + data_v.len());

        rec.push(self.command_type.into_val());

        rec.append(&mut data_v);

        rec
    }

    fn try_from_icd(icd : &[u8] ) -> Result<Self, Error> {
        if icd.len() == 0 {
            Err(Error::Size)
        } else {
            Ok( Command {
                command_type: CommandType::try_from_val(icd[0])?,
                data: D::try_from_icd(&icd[1..])?
            } )
        }
    }
}

/// The Encryption Key "Database"
///
/// This contains the keys that were previously generated. However calling this a true DataBase is a
/// little overdone. In reality there are just two `HashMaps` for the three different types of
/// keys stored.
///
/// Keys are 'queried' by either an 'ER' value or an 'IR' value. An 'ER' is used to get an Identity
/// Resolving Key (IRK) and 'IR' is use to get a Long Term Key (LTK) or a Connection Signature
/// Resolving Key (CSRK)
///
/// # Usage
/// Please only use the functions labeled with the `pub` keyword. All other functions
struct KeyDB {
    keys: atomic::AtomicPtr<KeyDBEntry>,
    keys_len: usize,
    keys_cap: usize,
}

impl KeyDB {

    /// Create a new `KeyDB` from a vector of *sorted* `KeyDBEntry`s
    pub fn new(mut v: Vec<KeyDBEntry>) -> Self {

        let len = v.len();
        let cap = v.capacity();
        let v_mut_ptr = v.as_mut_ptr();

        core::mem::forget(v);

        KeyDB {
            keys: atomic::AtomicPtr::new(v_mut_ptr),
            keys_len: len,
            keys_cap: cap,
        }
    }

    /// Insert, overwrite, or erase the LTK associated with the provided EDIV
    pub fn set_ltk<K>(&mut self, ediv: u16, ltk: K)
    where K: Into<Option<u128>>
    {
        match ltk.into() {
            ltk @ Some(_) => self.change_or_insert_key_entry(
                    ediv,
                    |keys, idx| keys[idx].ltk = ltk,
                    |keys, idx| keys.insert(idx, KeyDBEntry {
                        ediv,
                        ltk,
                        csrk: None,
                        irk: None,
                    })
                ),
            None => self.change_or_insert_key_entry(
                    ediv,
                    |keys, idx| {
                        if keys[idx].csrk == None && keys[idx].irk == None {
                            keys.remove(idx);
                        } else {
                            keys[idx].ltk = None;
                        }
                    },
                    |_, _| {}
                ),
        }
    }

    /// Insert, overwrite, or erase the CSRK associated with the provided EDIV
    pub fn set_csrk<K>(&mut self, ediv: u16, csrk: K)
        where K: Into<Option<u128>>
    {
        match csrk.into() {
            csrk @ Some(_) => self.change_or_insert_key_entry(
                ediv,
                |keys, idx| keys[idx].csrk = csrk,
                |keys, idx| keys.insert(idx, KeyDBEntry {
                    ediv,
                    ltk: None,
                    csrk,
                    irk: None,
                })
            ),
            None => self.change_or_insert_key_entry(
                ediv,
                |keys, idx| {
                    if keys[idx].ltk == None && keys[idx].irk == None {
                        keys.remove(idx);
                    } else {
                        keys[idx].csrk = None;
                    }
                },
                |_, _| {}
            ),
        }
    }

    /// Insert, overwrite, or erase the IRK associated with the provided EDIV
    pub fn set_irk<K>(&mut self, ediv: u16, irk: K)
        where K: Into<Option<u128>>
    {
        match irk.into() {
            irk @ Some(_) => self.change_or_insert_key_entry(
                ediv,
                |keys, idx| keys[idx].irk = irk,
                |keys, idx| keys.insert(idx, KeyDBEntry {
                    ediv,
                    ltk: None,
                    csrk: None,
                    irk,
                })
            ),
            None => self.change_or_insert_key_entry(
                ediv,
                |keys, idx| {
                    if keys[idx].ltk == None && keys[idx].csrk == None {
                        keys.remove(idx);
                    } else {
                        keys[idx].irk = None;
                    }
                },
                |_, _| {}
            ),
        }
    }

    /// Get the keys associated with the provided encryption diversifier (EDIV)
    ///
    /// The returned keys will be in the order of Long Term Key (LTK), Connection Signature
    /// Resolving Key (CSRK), and Identity Resolving Key. A `None` is returned for any key that is
    /// not associated with the provided `ediv`.
    pub fn get_keys<K>(&mut self, ediv: u16) -> (Option<u128>, Option<u128>, Option<u128>) {
        self.use_keys( |keys| {
            keys.binary_search_by(|entry| entry.ediv.cmp(&ediv) )
                .ok()
                .and_then(|idx| keys.get(idx) )
                .map_or( (None, None, None), |entry| (entry.ltk, entry.csrk, entry.irk))
        })
    }

    /// Safely use member `keys`
    ///
    /// This function uses a spinlock to try to acquire the database of keys, so it should be used
    /// relatively sparingly. Most usages for this should be for getting, creating, and deleting
    /// keys. And since keys are for a connection, unless the keys are changed, getting keys from
    /// the database should happen only when re-establishing a connection.
    ///
    /// This function takes
    /// Performs a backoff using
    /// [crossbeam](https://docs.rs/crossbeam-utils/0.7.0/crossbeam_utils/struct.Backoff.html)
    /// if the vector cannot be acquired.
    fn use_keys<F,R>(&mut self, to_do: F) -> R
    where F: FnOnce(&mut Vec<KeyDBEntry>) -> R
    {
        use core::ptr::null_mut;

        let backoff = crossbeam_utils::Backoff::new();

        loop {
            // Since keys is a vector, the pointer value must always be loaded and cannot be stored
            // for multiple loops as possible reallocation of the vector would make the pointer
            // invalid.
            match match self.keys.load(atomic::Ordering::Acquire) {
                x if x == null_mut() => { backoff.spin(); continue; },
                mut_ptr => self.keys.compare_and_swap(mut_ptr, null_mut(), atomic::Ordering::Acquire)
            } {
                x if x == null_mut() => { backoff.spin(); continue; },
                mut_ptr => {

                    let mut v = unsafe {
                        Vec::from_raw_parts(mut_ptr, self.keys_len, self.keys_cap)
                    };

                    let r = to_do( &mut v );

                    self.keys_len = v.len();
                    self.keys_cap = v.capacity();

                    let mut_vec_ptr = v.as_mut_ptr();

                    self.keys.store(mut_vec_ptr, atomic::Ordering::Release);

                    core::mem::forget(v);

                    break r;
                }
            }
        }
    }

    /// Gets or inserts a key entry matching the provided Encryption Diversifier (EDIV).
    fn change_or_insert_key_entry<Ch,Cr>(&mut self, ediv: u16, on_change: Ch, on_insert: Cr)
    where Ch: FnOnce(&mut Vec<KeyDBEntry>, usize),
          Cr: FnOnce(&mut Vec<KeyDBEntry>, usize),
    {
        self.use_keys(|keys|{
            match keys.binary_search_by(|entry| entry.ediv.cmp(&ediv) ) {
                Ok(idx) => on_change(keys, idx),
                Err(idx) => on_insert(keys, idx)
            }
        });
    }
}

#[derive(Clone,Copy,Serialize,Deserialize)]
pub struct KeyDBEntry {
    /// encryption diversifier
    ///
    /// The encryption diversifier is used for sorting the entries in the 'database' KeyDB
    ediv: u16,

    /// Associated LTK
    ltk: Option<u128>,

    /// Associated CSRK
    csrk: Option<u128>,

    /// Associated IRK
    irk: Option<u128>,
}

enum KeyGenerationMethod {
    /// Out of Bound
    Oob,
    PassKeyEntry,
    JustWorks,
    /// Numeric comparison
    NumbComp,
}

impl KeyGenerationMethod {

    /// Used to determine the pairing method to be executed between the initiator and responder
    fn determine_method (
        initiator_oob_data: pairing::OOBDataFlag,
        responder_oob_data: pairing::OOBDataFlag,
        initiator_io_capability: pairing::IOCapability,
        responder_io_capability: pairing::IOCapability,
        is_legacy: bool,
    ) -> Self
    {
        use pairing::{IOCapability, OOBDataFlag};

        // This match should match Table 2.8 in the Bluetooth Specification v5.0 | Vol 3, Part H,
        // section 2.3.5.1
        match (initiator_oob_data, responder_oob_data) {

            ( OOBDataFlag::AuthenticationDataFromRemoteDevicePresent,
              OOBDataFlag::AuthenticationDataFromRemoteDevicePresent) =>
                KeyGenerationMethod::Oob,

            (_,_) => match (initiator_io_capability, responder_io_capability) {

                (IOCapability::DisplayOnly, IOCapability::KeyboardOnly) |
                (IOCapability::DisplayOnly, IOCapability::KeyboardDisplay) =>
                    KeyGenerationMethod::PassKeyEntry,

                (IOCapability::DisplayWithYesOrNo, IOCapability::DisplayWithYesOrNo) if !is_legacy =>
                    KeyGenerationMethod::NumbComp,

                (IOCapability::DisplayWithYesOrNo, IOCapability::KeyboardOnly) =>
                    KeyGenerationMethod::PassKeyEntry,

                (IOCapability::DisplayWithYesOrNo, IOCapability::KeyboardDisplay) =>
                    if is_legacy {
                        KeyGenerationMethod::PassKeyEntry
                    } else {
                        KeyGenerationMethod::NumbComp
                    }

                (IOCapability::KeyboardOnly, IOCapability::DisplayOnly) |
                (IOCapability::KeyboardOnly, IOCapability::DisplayWithYesOrNo) |
                (IOCapability::KeyboardOnly, IOCapability::KeyboardOnly) |
                (IOCapability::KeyboardOnly, IOCapability::KeyboardDisplay) =>
                    KeyGenerationMethod::PassKeyEntry,

                (IOCapability::KeyboardDisplay, IOCapability::DisplayOnly) |
                (IOCapability::KeyboardDisplay, IOCapability::KeyboardOnly) =>
                    KeyGenerationMethod::PassKeyEntry,

                (IOCapability::KeyboardDisplay, IOCapability::DisplayWithYesOrNo) |
                (IOCapability::KeyboardDisplay, IOCapability::KeyboardDisplay) =>
                    if is_legacy {
                        KeyGenerationMethod::PassKeyEntry
                    } else {
                        KeyGenerationMethod::NumbComp
                    }

                (_,_) => KeyGenerationMethod::JustWorks
            }
        }
    }
}

/// The Security Manager
///
/// The security manager is the top level container of keys
pub struct SecurityManager {
    key_db: KeyDB,
}

impl SecurityManager {

    const SMALLEST_PACKET_SIZE: usize = 1;

    pub fn new(keys: Vec<KeyDBEntry>) -> Self {
        SecurityManager {
            key_db: KeyDB::new(keys),
        }
    }

    pub fn new_slave_builder<'a, C>(
        &'a self,
        channel: &'a C,
        master_address: &'a crate::BluetoothDeviceAddress,
        is_master_address_random: bool,
        this_address: &'a crate::BluetoothDeviceAddress,
        is_this_address_random: bool,
    )
    -> responder::SlaveSecurityManagerBuilder<'a, C>
    where C: ConnectionChannel
    {
        responder::SlaveSecurityManagerBuilder::new(
            self,
            channel,
            master_address,
            this_address,
            is_master_address_random,
            is_this_address_random,
        )
    }

    pub fn new_master_security_manager_builder<C>(&self, _channel: &C)
    -> initiator::MasterSecurityManagerBuilder<'_, C>
    where C: ConnectionChannel
    {
        unimplemented!()
    }
}

trait GetXOfP256Key {
    fn x(&self) -> [u8;32];
}

impl GetXOfP256Key for [u8;64] {
    fn x(&self) -> [u8;32] {
        let mut x = [0u8; 32];

        x.copy_from_slice(&self[..32]);

        x
    }
}

/// Lazy Encryption
///
/// This can be used to encrypt a connection between a master and slave.
pub struct LazyEncrypt {
    ltk: u128,
    _irk: u128,
    _csrk: u128,
}

impl LazyEncrypt {

    /// Use the Host Controller Interface to start encrypting the Bluetooth LE connection
    ///
    /// This returns a future that will setup encryption of the connection channel associated with
    /// the provided `connection_handle` once it is polled to completion.
    ///
    /// Only the AES cypher is considered "encrypted" by this procedure. An error will be returned
    /// if the controller decides to use the E0 cypher for encryption. A timeout can also be
    /// provided for waiting on the encryption events to be sent from the controller to the host.
    ///
    /// # Note
    /// This will set the event mask for the Host Controller Interface to the events
    /// ['DisconnectionComplete'](crate::hci::events::Events::DisconnectionComplete),
    /// ['EncryptionChange'](crate::hci::events::Events::EncryptionChange),
    /// and
    /// ['EncryptionKeyRefreshComplete'](crate::hci::events::Events::EncryptionKeyRefreshComplete),
    /// These events are needed by the returned future, and can only be masked away once it is
    /// polled to completion.
    pub fn le_start_encryption_with_hci<'a, HCI, D>(
        self,
        hci: &'a crate::hci::HostInterface<HCI>,
        connection_handle: crate::hci::common::ConnectionHandle,
        encryption_timeout: D,
    ) -> impl core::future::Future<Output=Result<(), Error>> + 'a
    where HCI: crate::hci::HostControllerInterface + 'static,
            D: Into<Option<core::time::Duration>> + 'a
    {
        use crate::hci::cb::set_event_mask::EventMask;

        let event_mask = [
            EventMask::DisconnectionComplete,
            EventMask::EncryptionChange,
            EventMask::EncryptionKeyRefreshComplete,
        ];

        lazy_encrypt::new_lazy_encrypt_future(event_mask, self.ltk, hci, connection_handle, encryption_timeout)
    }
}

mod lazy_encrypt {
    use alloc::boxed::Box;
    use super::Error;
    use core::fmt::{Display,Debug};
    use core::future::Future;
    use core::marker::Unpin;
    use core::pin::Pin;
    use core::task::{Context,Poll};
    use core::time::Duration;
    use crate::hci::{
        cb::set_event_mask::EventMask,
        common::ConnectionHandle,
        events::{Events, EventsData},
        HostInterface,
        HostControllerInterface,
        le::encryption::start_encryption::Parameter as EncryptionParameter,
    };

    type EventMasks = [EventMask; 3];

    pub fn new_lazy_encrypt_future<'a, HCI, D>(
        event_mask: EventMasks,
        ltk: u128,
        hci: &'a HostInterface<HCI>,
        connection_handle: ConnectionHandle,
        encryption_timeout: D,
    ) -> impl Future<Output = Result<(), Error>> + 'a
    where HCI: HostControllerInterface + 'static,
            D: Into<Option<Duration>>
    {
        LazyEncryptFuture::new(
            event_mask,
            ltk,
            hci,
            connection_handle,
            crate::hci::cb::set_event_mask::send,
            crate::hci::le::encryption::start_encryption::send,
            crate::hci::HostInterface::wait_for_event_with_matcher,
            encryption_timeout,
        )
    }

    struct LazyEncryptFuture<'a, HCI, F1, SMFn, F2, SEFn, WFEFn, F3> {
        event_mask: EventMasks,
        ltk: u128,
        hci: &'a HostInterface<HCI>,
        connection_handle: ConnectionHandle,
        current: LazyEncryptCurrent<F1,F2,F3,F3>,
        set_mask_fn: SMFn,
        start_encryption_fn: SEFn,
        wait_for_event_with_matcher_fn: WFEFn,
        encrypt_timeout: Option<Duration>,
    }

    impl<'a, HCI, F1, SMFn, F2, SEFn, WFEFn, F3>
    LazyEncryptFuture<'a, HCI, F1, SMFn, F2, SEFn, WFEFn, F3>
    where HCI: HostControllerInterface
    {
        fn new<D: Into<Option<Duration>>>(
            event_mask: EventMasks,
            ltk: u128,
            hci: &'a HostInterface<HCI>,
            connection_handle: ConnectionHandle,
            set_mask_fn: SMFn,
            start_encryption_fn: SEFn,
            wait_for_event_with_matcher_fn: WFEFn,
            encrypt_timeout: D,
        ) -> Self {

            LazyEncryptFuture {
                event_mask,
                ltk,
                hci,
                connection_handle,
                current: LazyEncryptCurrent::None,
                set_mask_fn,
                start_encryption_fn,
                wait_for_event_with_matcher_fn,
                encrypt_timeout: encrypt_timeout.into()
            }
        }
    }

    impl<'a, HCI, F1, FER1, SMFn, F2, FER2, SEFn, WFEFn, F3> core::future::Future
    for LazyEncryptFuture<'a, HCI, F1, SMFn, F2, SEFn, WFEFn, F3,>
    where  HCI: HostControllerInterface,
            F1: Future<Output = Result<(), FER1>> + Unpin + 'a,
          FER1: Display + Debug + 'static,
          SMFn: Fn(&'a HostInterface<HCI>, &[EventMask]) -> F1 + Unpin,
            F2: Future<Output = Result<(), FER2>> + Unpin + 'a,
          FER2: Display + Debug + 'static,
          SEFn: Fn(&'a HostInterface<HCI>, EncryptionParameter) -> F2 + Unpin,
            F3: Future<Output = Result<EventsData, <HCI as HostControllerInterface>::ReceiveEventError>> + Unpin + 'a,
         WFEFn: Fn(&'a HostInterface<HCI>, Events, Option<Duration>, EncryptEventMatcher) -> F3 + Unpin,
         <HCI as HostControllerInterface>::ReceiveEventError: 'static
    {
        type Output = Result<(), Error>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output>
        {
            let this = self.get_mut();

            loop {
                match &mut this.current {
                    LazyEncryptCurrent::None => {
                        log::trace!("None");

                        let future = (this.set_mask_fn)(this.hci, &this.event_mask);

                        this.current = LazyEncryptCurrent::SetMask(future);
                    },
                    LazyEncryptCurrent::SetMask(future) => {
                        log::trace!("SetMask");

                        match Pin::new(future).poll(cx).map_err(err_map) {
                            Poll::Pending => break Poll::Pending,
                            err @ Poll::Ready(Err(_)) => break err,
                            Poll::Ready(Ok(_)) => {

                                let encrypt_pram = EncryptionParameter {
                                    handle: this.connection_handle,
                                    random_number: 0,
                                    encrypted_diversifier: 0,
                                    long_term_key: this.ltk
                                };

                                let start_encrypt_fut = (this.start_encryption_fn)(this.hci, encrypt_pram);

                                let encrypt_change_fut = (this.wait_for_event_with_matcher_fn)(
                                    this.hci,
                                    Events::EncryptionChange,
                                    this.encrypt_timeout,
                                    EncryptEventMatcher(this.connection_handle),
                                );

                                let encrypt_key_refresh_fut = (this.wait_for_event_with_matcher_fn)(
                                    this.hci,
                                    Events::EncryptionKeyRefreshComplete,
                                    this.encrypt_timeout,
                                    EncryptEventMatcher(this.connection_handle),
                                );

                                this.current = LazyEncryptCurrent::StartEncryption(
                                    start_encrypt_fut,
                                    encrypt_change_fut,
                                    encrypt_key_refresh_fut,
                                );
                            },
                        }
                    },
                    LazyEncryptCurrent::StartEncryption(cmd_fut, _, _) => {
                        log::trace!("StartEncryption");

                        match Pin::new(cmd_fut).poll(cx).map_err(err_map) {
                            Poll::Pending => break Poll::Pending,
                            err @ Poll::Ready(Err(_)) => break err,
                            Poll::Ready(Ok(_)) => {

                                let start_encrypt = core::mem::replace(
                                    &mut this.current,
                                    LazyEncryptCurrent::None
                                );

                                match start_encrypt {
                                    LazyEncryptCurrent::StartEncryption(_,f1, f2) =>
                                        this.current = LazyEncryptCurrent::AwaitEncryptFinish(f1, f2),
                                    _ => panic!("Expected StartEncryption")
                                }

                            },
                        }
                    },
                    LazyEncryptCurrent::AwaitEncryptFinish(change_fut, refresh_fut) => {
                        log::trace!("AwaitEncryptFinish");

                        match Pin::new(change_fut).poll(cx).map_err(err_map) {
                            Poll::Pending => match Pin::new(refresh_fut).poll(cx).map_err(err_map) {
                                Poll::Pending => break Poll::Pending,
                                Poll::Ready(Err(e)) => break Poll::Ready(Err(e)),
                                Poll::Ready(Ok(refresh_data)) => break Poll::Ready(Ok(())),
                            },
                            Poll::Ready(Err(e)) => break Poll::Ready(Err(e)),
                            Poll::Ready(Ok(change_data)) => {
                                match change_data {
                                    EventsData::EncryptionChange(e_data) => {

                                        break match e_data.encryption_enabled.get_for_le() {
                                            crate::hci::common::EncryptionLevel::AESCCM =>
                                                Poll::Ready(Ok(())),
                                            crate::hci::common::EncryptionLevel::E0 =>
                                                Poll::Ready(Err(err_map("E0 cypher used"))),
                                            crate::hci::common::EncryptionLevel::Off =>
                                                Poll::Ready(Err(err_map("Encryption not enabled"))),
                                        };
                                    }
                                    ed => panic!("Received unexpected event data: '{:?}'", ed),
                                }
                            },
                        }
                    }
                }
            }
        }
    }

    fn err_map<E: 'static>(e: E) -> super::Error where E: Debug {
        super::Error::EncryptionFailed(Box::new(e))
    }

    enum LazyEncryptCurrent<F1,F2,F3,F4>
    {
        None,
        SetMask(F1),
        StartEncryption(F2,F3,F4),
        AwaitEncryptFinish(F3, F4)
    }

    struct EncryptEventMatcher(ConnectionHandle);

    impl crate::hci::EventMatcher for EncryptEventMatcher {

        fn match_event(&self, event_data: &EventsData) -> bool {
            match event_data {
                EventsData::EncryptionKeyRefreshComplete(data) => data.connection_handle == self.0,
                EventsData::EncryptionChange(data) => data.connection_handle == self.0,
                _ => false,
            }
        }
    }
}
