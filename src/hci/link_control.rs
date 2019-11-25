//! Link Control Commands

pub mod read_remote_version_information {

    use crate::hci::*;
    use crate::hci::common::ConnectionHandle;

    const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LinkControl(opcodes::LinkControl::ReadRemoteVersionInformation);

    #[repr(packed)]
    #[derive( Clone, Copy)]
    struct CmdParameter {
        _connection_handle: u16
    }

    impl CommandParameter for CmdParameter {
        type Parameter = Self;
        const COMMAND: opcodes::HCICommand = COMMAND;
        fn get_parameter(&self) -> Self::Parameter { *self }
    }

    impl_command_status_future!();

    pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, handle: ConnectionHandle)
                                 -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
        where T: HostControllerInterface
    {

        let parameter = CmdParameter {
            _connection_handle: handle.get_raw_handle()
        };

        ReturnedFuture( hci.send_command(parameter, events::Events::CommandStatus, Duration::from_secs(1) ) )
    }
}