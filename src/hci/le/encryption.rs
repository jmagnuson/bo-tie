/// Encrypt 16 bytes of plain text with the provided key
///
/// The controller uses AES-128 to encrypt the data. Once the controller is done encrypting
/// the plain text, the [`Command Complete`](crate::hci::events::Events::CommandComplete) event will
/// return with the cypher text generated.
pub mod encrypt {

    use crate::hci::*;

    const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::Encrypt);

    #[repr(packed)]
    struct CommandReturn {
        status: u8,
        cypher_text: [u8;16]
    }

    #[repr(packed)]
    #[derive(Clone)]
    struct Parameter {
        key: [u8;16],
        plain_text: [u8;16],
    }

    impl CommandParameter for Parameter {
        type Parameter = Self;
        const COMMAND: opcodes::HCICommand = COMMAND;
        fn get_parameter(&self) -> Self::Parameter { self.clone() }
    }

    pub struct Cypher {
        pub cypher_text: [u8;16]
    }

    impl Cypher {
        fn try_from(packed: CommandReturn) -> Result<Self, error::Error > {
            let status = error::Error::from(packed.status);

            if let error::Error::NoError = status {
                Ok( Self { cypher_text: packed.cypher_text })
            }
            else {
                Err(status)
            }
        }
    }

    impl_get_data_for_command!(
            COMMAND,
            CommandReturn,
            Cypher,
            error::Error
        );

    impl_command_data_future!(Cypher, error::Error);

    /// Send the command to start encrypting the `plain_text`
    ///
    /// Both the `key` and `plain-text` should be in native endian.
    pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, key: u128, plain_text: [u8;16])
                                 -> impl Future<Output=Result<Cypher, impl Display + Debug>> + 'a
        where T: HostControllerInterface
    {
        let parameter = Parameter {
            key: key.to_be_bytes(),
            plain_text,
        };

        ReturnedFuture( hci.send_command(
            parameter,
            events::Events::CommandComplete,
            Duration::from_secs(1)
        ))
    }
}

pub mod long_term_key_request_reply {
    use crate::hci::*;
    use crate::hci::common::ConnectionHandle;

    const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::LongTermKeyRequestReply);

    #[repr(packed)]
    struct CommandReturn {
        status: u8,
        handle: u16,
    }

    struct Parameter {
        handle: ConnectionHandle,
        /// Long Term Key
        ltk: u128,
    }

    #[repr(packed)]
    #[allow(dead_code)]
    struct CmdParameter {
        handle: u16,
        ltk: u128,
    }

    impl CommandParameter for Parameter {
        type Parameter = CmdParameter;
        const COMMAND: opcodes::HCICommand = COMMAND;
        fn get_parameter(&self) -> Self::Parameter {
            CmdParameter {
                handle: self.handle.get_raw_handle(),
                ltk: self.ltk
            }
        }
    }

    pub struct Return {
        pub connection_handle: ConnectionHandle,
    }

    impl Return {
        fn try_from(packed: CommandReturn) -> Result<Self, error::Error > {
            let status = error::Error::from(packed.status);

            if let error::Error::NoError = status {
                Ok( Self { connection_handle: ConnectionHandle::try_from(packed.handle)? })
            }
            else {
                Err(status)
            }
        }
    }

    impl_get_data_for_command!(
            COMMAND,
            CommandReturn,
            Return,
            error::Error
        );

    impl_command_data_future!(Return, error::Error);

    pub fn send<'a, T: 'static>(
        hci: &'a HostInterface<T>,
        connection_handle: ConnectionHandle,
        long_term_key: u128,
    ) -> impl Future<Output=Result<Return, impl Display + Debug>> + 'a
        where T: HostControllerInterface
    {
        let parameter = Parameter {
            handle: connection_handle,
            ltk: long_term_key,
        };

        ReturnedFuture( hci.send_command(
            parameter,
            events::Events::CommandComplete,
            Duration::from_secs(1)
        ))
    }
}

pub mod long_term_key_request_negative_reply {
    use crate::hci::*;
    use crate::hci::common::ConnectionHandle;

    const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::LongTermKeyRequestNegativeReply);

    #[repr(packed)]
    struct CommandReturn {
        status: u8,
        handle: u16,
    }

    struct Parameter {
        handle: ConnectionHandle,
    }

    #[repr(packed)]
    #[allow(dead_code)]
    struct CmdParameter {
        handle: u16,
    }

    impl CommandParameter for Parameter {
        type Parameter = CmdParameter;
        const COMMAND: opcodes::HCICommand = COMMAND;
        fn get_parameter(&self) -> Self::Parameter {
            CmdParameter {
                handle: self.handle.get_raw_handle(),
            }
        }
    }

    pub struct Return {
        pub connection_handle: ConnectionHandle,
    }

    impl Return {
        fn try_from(packed: CommandReturn) -> Result<Self, error::Error > {
            let status = error::Error::from(packed.status);

            if let error::Error::NoError = status {
                Ok( Self { connection_handle: ConnectionHandle::try_from(packed.handle)? })
            }
            else {
                Err(status)
            }
        }
    }

    impl_get_data_for_command!(
            COMMAND,
            CommandReturn,
            Return,
            error::Error
        );

    impl_command_data_future!(Return, error::Error);

    pub fn send<'a, T: 'static>(
        hci: &'a HostInterface<T>,
        connection_handle: ConnectionHandle,
    ) -> impl Future<Output=Result<Return, impl Display + Debug>> + 'a
        where T: HostControllerInterface
    {
        let parameter = Parameter {
            handle: connection_handle,
        };

        ReturnedFuture( hci.send_command(
            parameter,
            events::Events::CommandComplete,
            Duration::from_secs(1)
        ))
    }
}

pub mod rand {
    use crate::hci::*;

    const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::Rand);

    #[repr(packed)]
    struct CommandReturn {
        status: u8,
        random: u64,
    }

    struct Parameter;

    impl CommandParameter for Parameter {
        type Parameter = ();
        const COMMAND: opcodes::HCICommand = COMMAND;
        fn get_parameter(&self) -> Self::Parameter { () }
    }

    pub struct Return {
        pub random_number: u64,
    }

    impl From<Return> for u64 {
        fn from(ret: Return) -> Self { ret.random_number }
    }

    impl Return {
        fn try_from(packed: CommandReturn) -> Result<Self, error::Error > {
            let status = error::Error::from(packed.status);

            if let error::Error::NoError = status {
                Ok( Self { random_number: packed.random })
            }
            else {
                Err(status)
            }
        }
    }

    impl_get_data_for_command!(
            COMMAND,
            CommandReturn,
            Return,
            error::Error
        );

    impl_command_data_future!(Return, error::Error);

    pub fn send<'a, T: 'static>(hci: &'a HostInterface<T>)
    -> impl Future<Output=Result<Return, impl Display + Debug>> + 'a
    where T: HostControllerInterface
    {
        ReturnedFuture( hci.send_command(
            Parameter,
            events::Events::CommandComplete,
            Duration::from_secs(1)
        ))
    }
}

/// Start or restart encryption of the data
///
/// This will either start encryption or restart the encryption of data by the controller.
///
/// # Events
/// When encryption has been started, the event
/// [Encryption Change](crate::hci::events::Events::EncryptionChange) will be sent from the controller
/// to indicate that data will now be encrypted. If the connection was already encrypted,
/// sending this command will instead cause the controller to issue the
/// [Encryption Key Refresh](crate::hci::events::Events::EncryptionKeyRefreshComplete) event once the
/// encryption is updated.
pub mod start_encryption {
    use crate::hci::*;
    use crate::hci::common::ConnectionHandle;

    const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::StartEncryption);

    #[derive(Debug,Clone,Copy)]
    pub struct Parameter {
        pub handle: ConnectionHandle,
        pub random_number: u64,
        pub encrypted_diversifier: u16,
        pub long_term_key: u128,
    }

    #[repr(packed)]
    #[doc(hidden)]
    #[allow(dead_code)]
    pub struct CmdParameter {
        handle: u16,
        rand: u64,
        ediv: u16,
        ltk: u128,
    }

    impl CommandParameter for Parameter {
        type Parameter = CmdParameter;
        const COMMAND: opcodes::HCICommand = COMMAND;
        fn get_parameter(&self) -> Self::Parameter {
            CmdParameter {
                handle: self.handle.get_raw_handle(),
                rand: self.random_number,
                ediv: self.encrypted_diversifier,
                ltk: self.long_term_key,
            }
        }
    }

    impl_command_status_future!();

    pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, parameter: Parameter)
    -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
    where T: HostControllerInterface
    {
        ReturnedFuture( hci.send_command(
            parameter,
            events::Events::CommandStatus,
            Duration::from_secs(1)
        ))
    }
}
