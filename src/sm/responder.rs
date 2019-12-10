
use super::*;

pub struct SlaveSecurityManagerBuilder<'a, C> {
    sm: &'a SecurityManager,
    connection_channel: &'a C,
    io_capabilities: pairing::IOCapability,
    oob_data: Option<u128>,
    encryption_key_min: usize,
    encryption_key_max: usize,
    remote_address: &'a crate::BluetoothDeviceAddress,
    this_address: &'a crate::BluetoothDeviceAddress,
    remote_address_is_random: bool,
    this_address_is_random: bool,
}

impl<'a,C> SlaveSecurityManagerBuilder<'a,C>
where C: ConnectionChannel
{
    pub(super) fn new(
        sm: &'a SecurityManager,
        connection_channel: &'a C,
        connected_device_address: &'a crate::BluetoothDeviceAddress,
        this_device_address: &'a crate::BluetoothDeviceAddress,
        is_connected_devices_address_random: bool,
        is_this_device_address_random: bool,
    ) -> Self {
        Self {
            sm,
            connection_channel,
            io_capabilities: pairing::IOCapability::NoInputNoOutput,
            oob_data: None,
            encryption_key_min: ENCRYPTION_KEY_MIN_SIZE,
            encryption_key_max: ENCRYPTION_KEY_MAX_SIZE,
            remote_address: connected_device_address,
            this_address: this_device_address,
            remote_address_is_random: is_connected_devices_address_random,
            this_address_is_random: is_this_device_address_random,
        }
    }

    /// Set the minimum and maximum encryption key size
    ///
    /// min >= 7, max <= 16, and `min` <= `max`
    pub fn set_min_and_max_encryption_key_size(mut self, min: usize, max: usize) -> Result<Self, ()> {
        if (min >= 7) && (max <= 16) && (min <= max) {
            self.encryption_key_min = min;
            self.encryption_key_max = max;
            Ok(self)
        } else {
            Err(())
        }
    }

    /// Set the 128 bit random number that was shared between two devices over some 'out of band'
    /// way.
    pub fn set_oob_data(mut self, temp_key: u128) -> Self {
        self.oob_data = temp_key.into();
        self
    }

    pub fn create_security_manager(&self) -> SlaveSecurityManager<'a, C> {

        let auth_req = alloc::vec![
            encrypt_info::AuthRequirements::Bonding,
            encrypt_info::AuthRequirements::ManInTheMiddleProtection,
            encrypt_info::AuthRequirements::Sc,
        ];

        let key_dist = alloc::vec![
            pairing::KeyDistributions::IdKey,
        ];

        SlaveSecurityManager {
            sm: self.sm,
            connection_channel: self.connection_channel,
            io_capability: self.io_capabilities,
            oob_data: self.oob_data,
            // passkey: None,
            encryption_key_size_min: self.encryption_key_min,
            encryption_key_size_max: self.encryption_key_max,
            auth_req,
            initiator_key_distribution: key_dist.clone(),
            responder_key_distribution: key_dist,
            initiator_address: self.remote_address,
            responder_address: self.this_address,
            initiator_address_is_random: self.remote_address_is_random,
            responder_address_is_random: self.this_address_is_random,
            pairing_data: None,
        }
    }
}

/// Data that is gathered in the process of pairing
///
/// This data is unique for each pairing attempt and must be dropped after a successful or failed
/// pairing attempt.
struct PairingData {
    /// The current pairing method
    key_gen_method: KeyGenerationMethod,
    /// The public key generated by this device
    public_key: toolbox::PubKey,
    /// The private key generated by this device
    private_key: Option<toolbox::PriKey>,
    /// initiator IO Capabilities information
    remote_io_cap: [u8;3],
    /// Nonce value
    ///
    /// This will change multiple times for passkey, but is static for just works or number
    /// comparison
    nonce: u128,
    /// Calculated Diffie-Hellman shared secret Key
    dh_key: Option<toolbox::DHSecret>,
    /// The public key received from the remote device
    peer_public_key: Option<toolbox::PeerKey>,
    /// The secret key
    secret_key: Option<[u8;32]>,
    /// The remote nonce
    ///
    /// This will change multiple times for passkey, but is static for just works or number
    /// comparison
    remote_nonce: Option<u128>,
    /// The generated LTK
    ltk: Option<u128>,
    /// The generated MacKey
    mac_key: Option<u128>,
}

fn convert_io_cap(
    auth_req: &[encrypt_info::AuthRequirements],
    oob_flag: pairing::OOBDataFlag,
    io_cap: pairing::IOCapability
)
-> [u8;3]
{
    [
        io_cap.into_val(),
        oob_flag.into_val(),
        encrypt_info::AuthRequirements::make_auth_req_val(auth_req),
    ]
}

pub struct SlaveSecurityManager<'a,  C> {
    sm: &'a SecurityManager,
    connection_channel: &'a C,
    io_capability: pairing::IOCapability,
    oob_data: Option<u128>,
    // passkey: Option<u32>,
    auth_req: Vec<encrypt_info::AuthRequirements>,
    encryption_key_size_min: usize,
    encryption_key_size_max: usize,
    initiator_key_distribution: Vec<pairing::KeyDistributions>,
    responder_key_distribution: Vec<pairing::KeyDistributions>,
    initiator_address: &'a crate::BluetoothDeviceAddress,
    responder_address: &'a crate::BluetoothDeviceAddress,
    initiator_address_is_random: bool,
    responder_address_is_random: bool,
    pairing_data: Option<PairingData>
}

impl<'a, C> SlaveSecurityManager<'a, C>
where C: ConnectionChannel,
{
    pub fn set_oob_data(&mut self, val: u128) { self.oob_data = Some(val) }

    /// Process a request from a MasterSecurityManager
    ///
    /// This will return a response to a valid request that can be sent to the Master device.
    /// Errors will be returned if the request is not something that can be processed by the slave
    /// or there was something wrong with the request message.
    ///
    /// When this function returns `true`, it indicates that the keys have been verified *on this
    /// end* and the host can indicate to the controller to
    /// [`start_encryption`](crate::hci::le::encryption::start_encryption), however the initiator
    /// still needs to perform one more verify before it is ready to start encryption. If the verify
    /// fails, then the initiator will return a `PairingFailed` to this.
    ///
    /// It is recommended to always keep processing Bluetooth Security Manager packets as the
    /// responder. The host can at any point decide to restart encryption using different keys or
    /// send a `PairingFailed` to indicate that the prior pairing process failed.
    pub fn process_command(&mut self, received_data: &[u8] ) -> Result<bool, Error>
    {
        if received_data.len() > SecurityManager::SMALLEST_PACKET_SIZE {

            let (d_type, payload) = received_data.split_at(1);

            match CommandType::try_from_val(d_type[0]) {
                Ok( CommandType::PairingRequest ) => self.p_pairing_request(payload),
                Ok( CommandType::PairingConfirm ) => self.p_pairing_confirm(payload) ,
                Ok( CommandType::PairingPublicKey ) => self.p_pairing_public_key(payload),
                Ok( CommandType::PairingRandom ) => self.p_pairing_random(payload),
                Ok( CommandType::PairingFailed ) => self.p_pairing_failed(payload),
                Ok( CommandType::PairingDHKeyCheck ) => self.p_pairing_dh_key_check(payload),
//                Ok( CommandType::EncryptionInformation ) => Ok(false),
//                Ok( CommandType::MasterIdentification ) => Ok(false),
//                Ok( CommandType::IdentityInformation ) => Ok(false),
//                Ok( CommandType::IdentityAddressInformation ) => Ok(false),
//                Ok( CommandType::SigningInformation ) => Ok(false),
                Ok( cmd ) => self.p_command_not_supported(cmd),
                Err( cmd ) => self.p_unknown_command(cmd),
            }

        } else {
            self.p_bad_data_len()
        }
    }

    fn send<Cmd,P>(&self, command: Cmd)
        where Cmd: Into<Command<P>>,
              P: CommandData
    {
        use crate::l2cap::AclData;

        let acl_data = AclData::new( command.into().into_icd(), SECURITY_MANAGER_L2CAP_CHANNEL_ID);

        self.connection_channel.send(acl_data);
    }

    fn send_err(&mut self, fail_reason: pairing::PairingFailedReason) {
        self.pairing_data = None;

        self.send(pairing::PairingFailed::new(fail_reason));
    }

    fn p_bad_data_len(&mut self) -> Result<bool, Error> {
        self.send_err(pairing::PairingFailedReason::UnspecifiedReason);

        Err( Error::Size )
    }

    fn p_unknown_command(&mut self, err: Error) -> Result<bool, Error> {
        self.send_err(pairing::PairingFailedReason::CommandNotSupported);

        Err(err)
    }

    fn p_command_not_supported(&mut self, cmd: CommandType) -> Result<bool, Error> {
        self.send_err(pairing::PairingFailedReason::CommandNotSupported);

        Err(Error::IncorrectCommand(cmd))
    }

    fn p_pairing_request<'z>(&'z mut self, data: &'z [u8]) -> Result<bool, Error> {

        log::trace!("(SM) Processing pairing request");

        let request = match pairing::PairingRequest::try_from_icd(data) {
            Ok(request) => request,
            Err(_) => {
                self.send_err(pairing::PairingFailedReason::UnspecifiedReason);

                return Err(Error::IncorrectCommand(CommandType::PairingPublicKey))
            }
        };

        if request.get_max_encryption_size() < self.encryption_key_size_min {
            self.send_err(pairing::PairingFailedReason::EncryptionKeySize);

            Err(Error::PairingFailed(pairing::PairingFailedReason::EncryptionKeySize))
        } else {

            let response = pairing::PairingResponse::new(
                self.io_capability,
                if self.oob_data.is_some() {
                    pairing::OOBDataFlag::AuthenticationDataFromRemoteDevicePresent
                } else {
                    pairing::OOBDataFlag::AuthenticationDataNotPresent
                },
                self.auth_req.clone(),
                self.encryption_key_size_max,
                self.initiator_key_distribution.clone(),
                self.responder_key_distribution.clone(),
            );

            let pairing_method = KeyGenerationMethod::determine_method(
                request.get_oob_data_flag(),
                response.get_oob_data_flag(),
                request.get_io_capability(),
                response.get_io_capability(),
                false
            );

            let remote_io_cap = convert_io_cap(
                request.get_auth_req(),
                request.get_oob_data_flag(),
                request.get_io_capability(),
            );

            self.send(response);

            let (private_key, public_key) = toolbox::ecc_gen()
                .expect("Failed to fill bytes for generated random");

            self.pairing_data = Some(PairingData {
                key_gen_method: pairing_method,
                public_key,
                private_key: Some(private_key),
                remote_io_cap,
                nonce: toolbox::nonce(),
                dh_key: None,
                peer_public_key: None,
                secret_key: None,
                remote_nonce: None,
                ltk: None,
                mac_key: None,
            });

            Ok(false)
        }
    }

    fn p_pairing_public_key(&mut self, data: &[u8]) -> Result<bool, Error> {

        log::trace!("(SM) Processing pairing public Key");

        let initiator_pub_key = match pairing::PairingPubKey::try_from_icd(data) {
            Ok(request) => request,
            Err(e) => {
                self.send_err(pairing::PairingFailedReason::UnspecifiedReason);

                return Err(e)
            }
        };

        if let Some(mut pairing_data) = self.pairing_data.take() {

            let raw_pub_key = {
                let key_bytes = pairing_data.public_key.clone().into_icd();

                let mut raw_key = [0u8;64];

                raw_key.copy_from_slice(&key_bytes);

                raw_key
            };

            // Send the public key of this device
            self.send(pairing::PairingPubKey::new(raw_pub_key));

            let remote_public_key = initiator_pub_key.get_key();

            log::trace!("remote public key: {:x?}", remote_public_key.as_ref());

            let peer_key = toolbox::PeerKey::try_from_icd(&remote_public_key)
                .or(Err(super::Error::IncorrectValue))?;

            let secret_key = pairing_data.private_key.take().expect("Secret key must exist");

            // Calculate the shared secret key
            let secret_key = toolbox::ecdh(secret_key, &peer_key);

            pairing_data.peer_public_key = Some(peer_key);

            match secret_key {
                Ok(key) => {
                    pairing_data.secret_key = Some(key);

                    self.pairing_data = Some(pairing_data);

                    Ok(false)
                },
                Err(e) => {
                    // Generating the dh key failed

                    log::error!("(SM) Secret Key failed, '{:?}'", e);

                    self.send_err(pairing::PairingFailedReason::UnspecifiedReason);
                    Err(Error::IncorrectValue)
                }
            }

        } else {
            self.send_err(pairing::PairingFailedReason::UnspecifiedReason);

            Err(Error::IncorrectCommand(CommandType::PairingPublicKey))
        }
    }

    fn p_pairing_confirm(&mut self, payload: &[u8]) -> Result<bool, Error> {

        log::trace!("(SM) Processing pairing confirm");

        let initiator_confirm = match pairing::PairingConfirm::try_from_icd(payload) {
            Ok(request) => request,
            Err(e) => {
                self.send_err(pairing::PairingFailedReason::UnspecifiedReason);

                return Err(e)
            }
        };

        match self.pairing_data.as_ref() {
            Some( PairingData{
                key_gen_method: KeyGenerationMethod::JustWorks,
                public_key: this_pk,
                peer_public_key: Some( initiator_pk ),
                nonce,
                ..
            }) |
            Some( PairingData{
                key_gen_method: KeyGenerationMethod::NumbComp,
                public_key: this_pk,
                peer_public_key: Some( initiator_pk ),
                nonce,
                ..
            }) => /* Legacy Just Works or LE Secure Connection Number Comparison */
            {
                let confirm_value = toolbox::f4(this_pk.x(), initiator_pk.x(), *nonce, 0);

                if confirm_value == initiator_confirm.get_value() {
                    self.send_err(pairing::PairingFailedReason::ConfirmValueFailed);

                    Err(Error::PairingFailed(pairing::PairingFailedReason::ConfirmValueFailed))
                } else {
                    self.send(pairing::PairingConfirm::new(confirm_value));

                    Ok(false)
                }
            },
            // The pairing methods OOB and Passkey are not supported yet
            //
            // This is normally here for catching when the protocol is issued this command out
            // of order
            _ => {
                self.send_err(pairing::PairingFailedReason::PairingNotSupported);

                Err(Error::UnsupportedFeature)
            },
        }
    }

    fn p_pairing_random(&mut self, payload: &[u8]) -> Result<bool, Error> {

        log::trace!("(SM) Processing pairing random");

        let initiator_random = match pairing::PairingRandom::try_from_icd(payload) {
            Ok(request) => request,
            Err(e) => {
                self.send_err(pairing::PairingFailedReason::UnspecifiedReason);

                return Err(e)
            }
        };

        if self.pairing_data.is_some() {

            self.pairing_data.as_mut().unwrap().remote_nonce = Some(initiator_random.get_value());

            self.send( pairing::PairingRandom::new(self.pairing_data.as_ref().unwrap().nonce) );

            Ok(false)

        } else {
            self.send_err(pairing::PairingFailedReason::UnspecifiedReason);

            Err(Error::UnsupportedFeature)
        }
    }

    fn p_pairing_failed(&mut self, payload: &[u8]) -> Result<bool, Error> {
        log::trace!("(SM) Processing pairing failed");

        let initiator_fail = match pairing::PairingFailed::try_from_icd(payload) {
            Ok(request) => request,
            Err(e) => {
                self.send_err(pairing::PairingFailedReason::UnspecifiedReason);

                return Err(e)
            }
        };

        self.pairing_data = None;

        Err(Error::PairingFailed(initiator_fail.get_reason()))
    }

    fn p_pairing_dh_key_check(&mut self, payload: &[u8]) -> Result<bool, Error> {

        log::trace!("(SM) Processing pairing dh key check");

        let initiator_dh_key_check = match pairing::PairingDHKeyCheck::try_from_icd(payload) {
            Ok(request) => request,
            Err(e) => {
                self.send_err(pairing::PairingFailedReason::UnspecifiedReason);

                return Err(e)
            }
        };

        match self.pairing_data {
            Some( PairingData {
                dh_key: Some( dh_key ),
                nonce,
                remote_nonce: Some( remote_nonce ),
                remote_io_cap,
                ..
            }) => {

                let init_msb_addr_byte: u8 = if self.initiator_address_is_random {1} else {0};
                let this_msb_addr_byte: u8 = if self.responder_address_is_random {1} else {0};

                let mut a_addr = [init_msb_addr_byte, 0, 0, 0, 0, 0, 0];
                let mut b_addr= [this_msb_addr_byte, 0, 0, 0, 0, 0, 0];

                a_addr[1..].copy_from_slice(self.initiator_address);
                b_addr[1..].copy_from_slice(self.responder_address);

                let (mac_key, ltk) = toolbox::f5(
                    dh_key,
                    remote_nonce,
                    nonce,
                    a_addr.clone(),
                    b_addr.clone(),
                );

                let ea = toolbox::f6(
                    mac_key,
                    remote_nonce,
                    nonce,
                    0,
                    remote_io_cap,
                    a_addr,
                    b_addr,
                );

                let received_ea = initiator_dh_key_check.get_key_check();

                if received_ea == ea {

                    self.pairing_data.as_mut().unwrap().ltk = Some(ltk);

                    let eb = toolbox::f6(
                        mac_key,
                        nonce,
                        remote_nonce,
                        0,
                        convert_io_cap(
                            &self.auth_req,
                            if self.oob_data.is_some() {
                                pairing::OOBDataFlag::AuthenticationDataFromRemoteDevicePresent
                            } else {
                                pairing::OOBDataFlag::AuthenticationDataNotPresent
                            },
                            self.io_capability,
                        ),
                        b_addr.clone(),
                        a_addr.clone(),
                    );

                    self.send(pairing::PairingDHKeyCheck::new(eb));

                    // TODO send these to the initiator (along with the device address)
                    let irk = toolbox::rand_u128();
                    let csrk = toolbox::rand_u128();

                    Ok(true)
                } else {
                    self.send_err(pairing::PairingFailedReason::DHKeyCheckFailed);

                    Err(Error::PairingFailed(pairing::PairingFailedReason::DHKeyCheckFailed))
                }
            }
            _ => {
                self.send_err(pairing::PairingFailedReason::UnspecifiedReason);

                Err(Error::UnsupportedFeature)
            }
        }
    }
}

// pub struct AsyncMasterSecurityManager<'a, HCI, C> {
//     sm: &'a SecurityManager,
//     hci: &'a HostInterface<HCI>,
//     connection_channel: &'a C,
// }
// 
// impl<'a, HCI, C> AsyncMasterSecurityManager<'a, HCI, C> {
//     fn new( sm: &'a SecurityManager, hci: &'a HostInterface<HCI>, connection_channel: &'a C ) -> Self {
//         Self { sm, hci, connection_channel }
//     }
// }
// 
// pub struct AsyncSlaveSecurityManager<'a, HCI, C> {
//     sm: &'a SecurityManager,
//     hci: &'a HostInterface<HCI>,
//     connection_channel: &'a C,
// }
// 
// impl<'a, HCI, C> AsyncSlaveSecurityManager<'a, HCI, C> {
//     fn new( sm: &'a SecurityManager, hci: &'a HostInterface<HCI>, connection_channel: &'a C ) -> Self {
//         Self { sm, hci, connection_channel }
//     }
// }
