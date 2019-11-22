use super::*;

pub struct MasterSecurityManagerBuilder<'a, HCI, C> {
    sm: &'a SecurityManager,
    hci: &'a HostInterface<HCI>,
    connection_channel: &'a C,
}

pub struct MasterSecurityManager<'a, HCI, C> {
    sm: &'a SecurityManager,
    hci: &'a HostInterface<HCI>,
    connection_channel: &'a C,
}

impl<'a, HCI, C> MasterSecurityManager<'a, HCI, C> {
    fn new( sm: &'a SecurityManager, hci: &'a HostInterface<HCI>, connection_channel: &'a C ) -> Self {
        Self { sm, hci, connection_channel }
    }

    pub fn process_command(&self, received_data: &[u8]) -> Result<Vec<u8>, Error> {
        if received_data.len() > SecurityManager::SMALLEST_PACKET_SIZE {
            match CommandType::try_from_val(received_data[0])? {
                CommandType::PairingResponse => unimplemented!(),
                CommandType::PairingConfirm => unimplemented!(),
                CommandType::PairingPublicKey => unimplemented!(),
                CommandType::PairingRandom => unimplemented!(),
                CommandType::PairingFailed => unimplemented!(),
                CommandType::SecurityRequest => unimplemented!(),
                CommandType::EncryptionInformation => unimplemented!(),
                CommandType::MasterIdentification => unimplemented!(),
                CommandType::IdentityInformation => unimplemented!(),
                CommandType::IdentityAddressInformation => unimplemented!(),
                CommandType::SigningInformation => unimplemented!(),
                cmd => Err( Error::IncorrectCommand(cmd) )
            }
        } else {
            Err( Error::Size )
        }
    }
}