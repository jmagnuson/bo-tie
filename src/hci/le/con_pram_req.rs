//! Connect Parameter Request Procedure response
//!
//! These are the commands that are used in response to a request from either a master or slave to
//! change the connection parameters.

pub mod remote_connection_parameter_request_reply {

    use crate::hci::*;
    use crate::hci::common::{
        ConnectionHandle,
        ConnectionInterval,
        ConnectionLatency,
        SupervisionTimeout,
    };
    use crate::hci::le::common::ConnectionEventLength;

    const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::ReadConnectionParameterRequestReply);

    #[repr(packed)]
    struct CommandReturn {
        status: u8,
        connection_handle: u16,
    }

    #[repr(packed)]
    #[doc(hidden)]
    pub struct Parameter {
        _connection_handle: u16,
        _interval_min: u16,
        _interval_max: u16,
        _latency: u16,
        _timeout: u16,
        _minimum_ce_len: u16,
        _maximum_ce_len: u16,
    }

    pub struct CommandParameters {
        /// The handle for the connection
        pub handle: ConnectionHandle,
        /// The minimum connection interval
        pub interval_min: ConnectionInterval,
        /// The maximum connection interval
        pub interval_max: ConnectionInterval,
        /// The slave latency
        pub latency: ConnectionLatency,
        /// The link supervision timeout
        pub timeout: SupervisionTimeout,
        /// The minimum connection event length
        pub ce_len: ConnectionEventLength,
    }

    impl CommandParameter for CommandParameters {
        type Parameter = Parameter;
        const COMMAND: opcodes::HCICommand = COMMAND;
        fn get_parameter(&self) -> Self::Parameter {
            Parameter {
                _connection_handle: self.handle.get_raw_handle().to_le(),
                _interval_min: self.interval_min.get_interval().to_le(),
                _interval_max: self.interval_max.get_interval().to_le(),
                _latency: self.latency.get_latency().to_le(),
                _timeout: self.timeout.get_timeout().to_le(),
                _minimum_ce_len: self.ce_len.minimum.to_le(),
                _maximum_ce_len: self.ce_len.maximum.to_le(),
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
                Ok( Self { connection_handle: ConnectionHandle::try_from(packed.connection_handle)? })
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

    pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, parameter: CommandParameters)
    -> impl Future<Output=Result<Return, impl Display + Debug>> + 'a
    where T: HostControllerInterface
    {
        ReturnedFuture( hci.send_command(
            parameter,
            events::Events::CommandComplete,
            core::time::Duration::from_secs(1)
        ))
    }
}

/// Send the negative reply to a connection parameter request
///
/// This sends a reason as to why the the request is rejected
///
/// # Note
/// That reason cannot be
/// [`NoError`](crate::hci::error::Error::NoError) nor
/// [`Message`](crate::hci::error::Error::Message)
/// as they are translated into the value of 0 on the interface.
pub mod remote_connection_parameter_request_negative_reply {

    use crate::hci::*;
    use crate::hci::common::ConnectionHandle;

    struct CommandReturn {
        status: u8,
        connection_handle: u16,
    }

    const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::ReadConnectionParameterRequestNegativeReply);

    pub struct Return {
        pub connection_handle: ConnectionHandle,
    }

    impl Return {
        fn try_from(packed: CommandReturn) -> Result<Self, error::Error > {
            let status = error::Error::from(packed.status);

            if let error::Error::NoError = status {
                Ok( Self { connection_handle: ConnectionHandle::try_from(packed.connection_handle)? })
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

    #[repr(packed)]
    #[derive(Clone, Copy)]
    struct Parameter {
        _handle: u16,
        _reason: u8,
    }

    impl CommandParameter for Parameter {
        type Parameter = Parameter;
        const COMMAND: opcodes::HCICommand = COMMAND;
        fn get_parameter(&self) -> Self::Parameter { *self }
    }

    pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>,
        handle: ConnectionHandle,
        reason: error::Error
    ) -> impl Future<Output=Result<Return, impl Display + Debug>> + 'a
    where T: HostControllerInterface
    {
        let parameter = Parameter {
            _handle: handle.get_raw_handle().to_le(),
            _reason: reason.into(),
        };

        ReturnedFuture( hci.send_command(
            parameter,
            events::Events::CommandComplete,
            core::time::Duration::from_secs(1)
        ))
    }
}