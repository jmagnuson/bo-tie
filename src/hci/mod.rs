//! In Bluez there is a "manager" for the interface to the bluetooth hardware
//!
//! This is really the api to the HCI (host controller interface) of the bluetooth device (as per
//! the bluetooth specification). This module is further broken up into modules for OGFs (OpCode
//! group field(s)).

mod opcodes;
pub mod common;
pub mod error;
#[macro_use] pub mod events;

use alloc::sync::Arc;
use core::fmt::Debug;
use core::fmt::Display;
use core::future::Future;
use core::pin::Pin;
use core::time::Duration;
use core::task::{ Poll, Waker };

/// Used to get the information required for sending a command from the host to the controller
///
/// The type Parameter should be a packed structure of the command's parameters
pub trait CommandParameter {
    /// Data for the parameter as specified by the Bluetooth Specification.
    type Parameter;

    /// The command to send to the Bluetooth Controller.
    ///
    /// This is the OGF & OCF pair.
    const COMMAND: opcodes::HCICommand;

    /// Convert Self into the parameter form
    ///
    /// The returned parameter is the structure defined as the parameter part of the command packet
    /// for the specific HCI command.
    fn get_parameter(&self) -> Self::Parameter;

    /// Get the command packet to be sent to the controller
    ///
    /// # Note
    /// This is not the entire packet sent to the interface as there may be additional information
    /// that needs to be sent for the HCI transport layer (such as the HCI packet indicator).
    fn as_command_packet<'a>(&self) -> alloc::boxed::Box<[u8]> {
        use core::mem::size_of;

        let parameter_size = size_of::<Self::Parameter>();

        // Allocating a vector to the exact size of the packet. The 3 bytes come from the opcode
        // field (2 bytes) and the length field (1 byte)
        let mut buffer:alloc::vec::Vec<u8> = alloc::vec::Vec::with_capacity( parameter_size + 3);

        let parameter = self.get_parameter();

        let p_bytes_p = &parameter as *const Self::Parameter as *const u8;

        let parm_bytes = unsafe { core::slice::from_raw_parts( p_bytes_p, parameter_size ) };

        let opcode_bytes = Self::COMMAND.as_opcode_pair().as_opcode().to_le();

        buffer.extend_from_slice(&opcode_bytes.to_le_bytes());

        buffer.push(parm_bytes.len() as u8);

        buffer.extend_from_slice(parm_bytes);

        buffer.into_boxed_slice()
    }
}

/// A trait for matching received events
///
/// When receiving an event in a concurrent system, it can be unknown which context a received
/// event should be propigated to. The event must be matched to determine this.
pub trait EventMatcher: Sync + Send {
    /// Match the event data
    fn match_event(&self, event_data: &events::EventsData ) -> bool;
}

impl<F> EventMatcher for F where F: Fn( &events::EventsData ) -> bool + Sized + Sync + Send {
    fn match_event(&self, event_data: &events::EventsData) -> bool {
        self(event_data)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AclPacketBoundry {
    FirstNonFlushable,
    ContinuingFragment,
    FirstAutoFlushable,
    CompleteL2capPdu,
}

impl AclPacketBoundry {

    /// Get the value shifted into the correct place of the Packet Boundary Flag in the HCI ACL
    /// data packet. The returned value is in host byte order.
    fn get_shifted_val(&self) -> u16 {
        ( match self {
            AclPacketBoundry::FirstNonFlushable => 0x0,
            AclPacketBoundry::ContinuingFragment => 0x1,
            AclPacketBoundry::FirstAutoFlushable => 0x2,
            AclPacketBoundry::CompleteL2capPdu => 0x3,
        } ) >> 12
    }

    /// Get the `AclPacketBoundry` from the first 16 bits of a HCI ACL data packet. The input
    /// `val` does not need to be masked to only include the Packet Boundary Flag, however it does
    /// need to be in host byte order.
    fn from_shifted_val(val: u16) -> Self {
        match (val << 12) & 3  {
            0x0 => AclPacketBoundry::FirstNonFlushable,
            0x1 => AclPacketBoundry::ContinuingFragment,
            0x2 => AclPacketBoundry::FirstAutoFlushable,
            0x3 => AclPacketBoundry::CompleteL2capPdu,
            _ => panic!("This cannot happen"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AclBroadcastFlag {
    // Point-to-point message
    NoBroadcast,
    // Broadcast to all active slaves
    ActiveSlaveBroadcast,
}

impl AclBroadcastFlag {

    /// Get the value shifted into the correct place of the Packet Boundary Flag in the HCI ACL
    /// data packet. The returned value is in host byte order.
    fn get_shifted_val(&self) -> u16 {
        ( match self {
            AclBroadcastFlag::NoBroadcast => 0x0,
            AclBroadcastFlag::ActiveSlaveBroadcast => 0x1,
        } ) >> 14
    }

    /// Get the `AclPacketBoundry` from the first 16 bits of a HCI ACL data packet. The input
    /// `val` does not need to be masked to only include the Packet Boundary Flag, however it does
    /// need to be in host byte order.
    fn try_from_shifted_val(val: u16) -> Result<Self, ()> {
        match (val << 14) & 1  {
            0x0 => Ok(AclBroadcastFlag::NoBroadcast),
            0x1 => Ok(AclBroadcastFlag::ActiveSlaveBroadcast),
            0x2 | 0x3 => Err( () ),
            _ => panic!("This cannot happen"),
        }
    }
}

#[derive(Debug)]
pub enum HciAclPacketConvertError {
    PacketTooSmall,
    InvalidBroadcastFlag,
    InvalidConnectionHandle( &'static str ),
}

impl Display for HciAclPacketConvertError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            HciAclPacketConvertError::PacketTooSmall =>
                write!(f, "Packet is too small to be a valid HCI ACL Data"),
            HciAclPacketConvertError::InvalidBroadcastFlag =>
                write!(f, "Packet has invalid broadcast Flag"),
            HciAclPacketConvertError::InvalidConnectionHandle(reason) =>
                write!(f, "Invalid connection handle, {}", reason),
        }
    }
}

#[derive(Debug)]
pub struct HciAclData {
    connection_handle: common::ConnectionHandle,
    packet_boundry_flag: AclPacketBoundry,
    broadcast_flag: AclBroadcastFlag,
    data: alloc::boxed::Box<[u8]>,
}

impl HciAclData {

    pub fn new(
        connection_handle: common::ConnectionHandle,
        packet_boundry_flag: AclPacketBoundry,
        broadcast_flag: AclBroadcastFlag,
        data: alloc::boxed::Box<[u8]>
    ) -> Self
    {
        HciAclData { connection_handle, packet_boundry_flag, broadcast_flag, data }
    }

    pub fn get_handle(&self) -> &common::ConnectionHandle {
        &self.connection_handle
    }

    pub fn get_data(&self) -> &[u8] { &self.data }

    pub fn get_packet_boundry_flag(&self) -> AclPacketBoundry { self.packet_boundry_flag }

    pub fn get_broadcast_flag(&self) -> AclBroadcastFlag { self.broadcast_flag }

    /// Convert the HciAclData into a packet
    ///
    /// This will convert HciAclData into a packet that can be sent between the host and controller.
    ///
    /// # Panics (TODO to remove)
    /// For now this panics if the length of data is greater then 2^16 because this library only
    /// supports LE.
    pub fn into_packet(&self) -> alloc::boxed::Box<[u8]> {
        let mut v = alloc::vec::Vec::with_capacity( self.data.len() + 4 );

        let first_2_bytes = self.connection_handle.get_raw_handle()
            | self.packet_boundry_flag.get_shifted_val()
            | self.broadcast_flag.get_shifted_val();

        v.extend_from_slice( &first_2_bytes.to_le_bytes() );

        v.extend_from_slice( &(self.data.len() as u16).to_le_bytes() );

        v.extend_from_slice(&self.data);

        v.into_boxed_slice()
    }


    /// Attempt to create a `HciAclData`
    ///
    /// A `HciAclData` is created if the packet is in the correct HCI ACL data packet format. If
    /// not, then an error is returned.
    pub fn from_packet(packet: &[u8]) -> Result<Self, HciAclPacketConvertError> {
        if packet.len() >= 4 {
            let first_2_bytes = <u16>::from_le_bytes( [ packet[0], packet[1] ] );

            let connection_handle = match common::ConnectionHandle::try_from( first_2_bytes & 0xFFF) {
                Ok(handle) => handle,
                Err(e) => return Err( HciAclPacketConvertError::InvalidConnectionHandle(e) ),
            };

            let packet_boundry_flag = AclPacketBoundry::from_shifted_val( first_2_bytes );

            let broadcast_flag = match AclBroadcastFlag::try_from_shifted_val( first_2_bytes ) {
                Ok(flag) => flag,
                Err(_) => return Err( HciAclPacketConvertError::InvalidBroadcastFlag ),
            };

            let length = <u16>::from_le_bytes( [ packet[0], packet[1] ] ) as usize;

            Ok(
                HciAclData {
                    connection_handle: connection_handle,
                    packet_boundry_flag: packet_boundry_flag,
                    broadcast_flag: broadcast_flag,
                    data: alloc::boxed::Box::from( &packet[2..length] ),
                }
            )

        } else {
            Err( HciAclPacketConvertError::PacketTooSmall )
        }
    }
}

/// Trait for interfacing with the controller
///
///
/// # Implemenation
///
/// ## [send_command](#send_command)
/// This is used for sending the command to the Bluetooth controller by the HostInterface object.
/// It is provided with a input that implementes the
/// `[CommandParameter](../index.html#CommandParameter)` which contains all the information required
/// for sending the command packet to the Bluetooth controller. This information is not in the
/// packet format and needs to be implemented as such.
///
/// The funciton should return Ok if there were no errors sending the command.
///
/// ## [receive_event](#receive_event)
/// receive_event is used for implementing a future around the controller's event process. When
/// called it needs to check if the event is available to the Host or not. If the event is not not
/// immediately available, the implementation of receive_event needs to call wake on the provided
/// Waker input when the event is accepted by the Host.
///
/// It is suggested, but not nessicary, for the implementor to provide a means of timing out while
/// waiting for the event to be received by the host. The duration of the timeout shall be the
/// input `timeout` if set to some value, otherwise there is no timeout when the value is None.
/// When the timeout occurs, the wake function of the provided Waker input will be called. When
/// receive_event is called the next time (with the same event), it will return an error to
/// indicate a timeout.
///
/// If the timeout functionality isn't imeplemented, then the only value accepted should be None
/// and any Duration value provided should cause the function to return an Error stating that
/// timeouts are not available for this implementation.
///
/// Events need to be correctly propigated to the right context that is currently waiting for the
/// requested event. Some events can be differeniated from themselves through the data passed with
/// the event, but most do not have any discernable way to tell which context should receive which
/// event. Its the responsibility of the implementor of `HostControllerInterface` to determine
/// what event goes with what waker, along with matching events to a waker based on the provided
/// matcher.
pub trait HostControllerInterface
{
    type SendCommandError: Debug + Display;
    type ReceiveEventError: Debug + Display;

    /// Send a command from the Host to the Bluetooth Controller
    ///
    /// This will return true if the command was sent to the bluetooth controller, and false if
    /// the command couldn't be transferred to the controller yet. This doesn't mean that an error
    /// occured (it generally means that the bluetooth controller buffer is full), but it does mean
    /// that the command must be resent. If an error does occur then an Error will be returned.
    ///
    /// The `cmd_data` input contains all the HCI command information, where as the `waker` input
    /// is used to wake the context for the command to be resent.
    fn send_command<D,W>(&self, cmd_data: &D, waker: W) -> Result<bool, Self::SendCommandError>
    where D: CommandParameter,
          W: Into<Option<Waker>>;

    /// Receive an event from the Bluetooth controller
    ///
    /// This is implemented as a non-blocking operation, the host has either received the event or
    /// the event hasn't been send sent (or will never be sent) to the host. The function will
    /// return the data associated with the event (or an error if it occurs) if the event has been
    /// received or it will return None.
    ///
    /// If None is returned, the waker will be used to indicate that the event was received. But to
    /// get the events data, the exact same event and matcher reference (the matcher may be cloned)
    /// must be used to gaurentee that the event data is returned.
    ///
    /// The function requires a
    /// `[Waker](https://doc.rust-lang.org/nightly/core/task/struct.Waker.html)` object because
    /// it will call wake when the event has been received after the method is called or a timeout
    /// occurs (if available). At which point the function must be called again to receive the
    /// EventData.
    fn receive_event<P>(
        &self,
        event: events::Events,
        waker: Waker,
        matcher: Pin<Arc<P>>,
        timeout: Option<Duration>
    ) -> Option<Result<events::EventsData, Self::ReceiveEventError>>
    where P: EventMatcher + Send + Sync + 'static;
}

/// HCI ACL Data interface
///
/// This is the trait that must be implemented by the platform specific HCI structure.
pub trait HciAclDataInterface {
    type SendACLDataError: Debug + Display;
    type ReceiveACLDataError: Debug + Display;

    /// Send ACL data
    ///
    /// This will send ACL data to the controller for sending to the connected bluetooth device
    fn send(
        &self,
        data: HciAclData,
    ) -> Result<(), Self::SendACLDataError>;

    /// Register a handle for receiving ACL packets
    ///
    /// Unlike events, it can be unpredictable if data will be received by the controller while
    /// this API is waiting for it. There may be times where data sent from the controller
    /// to the host and there is nothing to receive it. Lower level implementations should utilize
    /// this function to enable buffers for each connection handle.
    ///
    /// The `receive_acl_data` function will be called afterwards to acquire the buffered data,
    /// however the buffer needs to still exist
    fn start_receiver(&self, handle: common::ConnectionHandle);

    /// Unregister a handle for receiving ACL packets
    ///
    /// This will be called once there will be no more ACL packets to be received or the user no
    /// longer cares about receiving ACL packets. Once this is called any buffers can be dropped
    /// that are associated with the given handle.
    fn stop_receiver(&self, handle: &common::ConnectionHandle);

    /// Receive ACL data
    ///
    /// Receive data from the controller for the given connection handle. If no data is available
    /// to be received then None will be returned and the provided waker will be used when the next
    /// ACL data is received.
    fn receive(
        &self,
        handle: &common::ConnectionHandle,
        waker: Waker,
    ) -> Result<alloc::boxed::Box<[HciAclData]>, Self::ReceiveACLDataError>;
}

enum SendCommandError<I> where I: HostControllerInterface {
    Send(<I as HostControllerInterface>::SendCommandError),
    Recv(<I as HostControllerInterface>::ReceiveEventError),
}

impl<I> Debug for SendCommandError<I> where I: HostControllerInterface {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            SendCommandError::Send(err) => Debug::fmt(err, f),
            SendCommandError::Recv(err) => Debug::fmt(err, f),
        }
    }
}

impl<I> Display for SendCommandError<I> where I: HostControllerInterface {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            SendCommandError::Send(err) => Display::fmt(err, f),
            SendCommandError::Recv(err) => Display::fmt(err, f),
        }
    }
}

struct CommandFutureReturn<'a, I, CD, P>
where I: HostControllerInterface,
      CD: CommandParameter,
      P: EventMatcher + Send + Sync + 'static,
{
    interface: &'a I,
    /// Parameter data sent with the command packet
    ///
    /// This must be set to Some(*data*) for a command to be sent to the controller. No command
    /// will be sent to the controller if the `command_data` isn't set to Some.
    command_data: Option<CD>,
    event: events::Events,
    matcher: Pin<Arc<P>>,
    timeout: Option<Duration>,
}

impl<'a, I, CD, P> CommandFutureReturn<'a, I, CD, P>
where I: HostControllerInterface,
      CD: CommandParameter + Unpin,
      P: EventMatcher + Send + Sync + 'static,
{

    /// This is just called within an implemenation of future created by the macro
    /// `[impl_returned_future]`(../index.html#impl_returned_future)
    fn fut_poll(&mut self, cx: &mut core::task::Context) -> Poll<Result<events::EventsData, SendCommandError<I>>> {

        if let Some(ref data) = self.command_data {
            match self.interface.send_command(data, cx.waker().clone() ) {
                Err(e) => return Poll::Ready(Err(SendCommandError::Send(e))),
                // False means the command wasn't sent
                Ok(false) => return Poll::Pending,
                Ok(true) => { self.command_data.take(); },
            }
        }

        match self.interface.receive_event(self.event, cx.waker().clone(), self.matcher.clone(), self.timeout) {
            None => Poll::Pending,
            Some(result) => Poll::Ready(result.map_err(|e| SendCommandError::Recv(e)))
        }
    }
}

struct EventReturnFuture<'a, I, P>
where I: HostControllerInterface,
      P: EventMatcher + Sync + Send + 'static
{
    interface: &'a I,
    event: events::Events,
    matcher: Pin<Arc<P>>,
    timeout: Option<Duration>,
}

impl<'a, I, P> Future for EventReturnFuture<'a, I, P>
where I: HostControllerInterface,
P: EventMatcher + Send + Sync + 'static
{
    type Output = Result<events::EventsData, I::ReceiveEventError>;

    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> Poll<Self::Output> {
        match self.interface.receive_event(self.event, cx.waker().clone(), self.matcher.clone(), self.timeout) {
            Some(evnt_rspn) => Poll::Ready(evnt_rspn),
            None => Poll::Pending,
        }
    }
}

/// The host interface
///
/// This is used by the host to interact with the interface between itself and the Bluetooth
/// Controller.
#[derive(Clone)]
pub struct HostInterface<I>
{
    interface: I
}

impl<I> From<I> for HostInterface<I>
{
    fn from(interface: I) -> Self {
        HostInterface { interface }
    }
}

impl<T> ::core::default::Default for HostInterface<T> where T: Default {

    fn default() -> Self {
        HostInterface { interface: T::default() }
    }
}

impl<I> HostInterface<I>
where I: HostControllerInterface
{
    /// Get the native interface to the Bluetooth HCI
    ///
    /// This is the same interface that was used to create this HostInterface instance.
    pub fn get_native_interface(&self) -> &I {
        &self.interface
    }

    /// Send a command to the controller
    ///
    /// The command data will be used in the command packet to determine what HCI command is sent
    /// to the controller. The events specified should be the events directly returned by the
    /// controller in response to the command, they should not be events that will come later as
    /// a timeout may occur before the event is sent from the controller.
    ///
    /// A future is returned for waiting on the event generated from the controller in *direct*
    /// response to the sent command.
    fn send_command<'a, CD, D>( &'a self, cmd_data: CD, event: events::Events, timeout: D )
    -> CommandFutureReturn<'a, I, CD, impl EventMatcher + Send + Sync + 'static>
    where CD: CommandParameter + Unpin + 'static,
          D: Into<Option<Duration>>,
    {
        let cmd_matcher = | ed: &events::EventsData | {

            fn match_opcode<CD: CommandParameter>(opcode: Option<u16>) -> bool {
                match opcode {
                    Some(opcode) => {
                        use core::convert::TryFrom;

                        let expected_op_code =
                            opcodes::HCICommand::try_from(<CD as CommandParameter>::COMMAND)
                            .unwrap();

                        let recv_oc_code = opcodes::HCICommand::try_from(
                            opcodes::OpCodePair::from_opcode(opcode)
                        );

                        match recv_oc_code {
                            Ok(code) => {
                                expected_op_code == code
                            },
                            Err(reason) => {
                                log::error!("{}", reason);
                                false
                            }
                        }
                    }
                    None => false,
                }
            }

            match ed {
                events::EventsData::CommandComplete(data) => match_opcode::<CD>(data.command_opcode),
                events::EventsData::CommandStatus(data)   => match_opcode::<CD>(data.command_opcode),
                _ => false
            }
        };

        CommandFutureReturn {
            interface: &self.interface,
            command_data: Some(cmd_data),
            event: event,
            matcher: Arc::pin(cmd_matcher),
            timeout: timeout.into(),
        }
    }

    /// Get a future for a Bluetooth Event
    ///
    /// The event provided to the method will be the event to waited upon, and an optional timeout
    /// can be provided for the case were the event isn't gaurenteed to be returned to the Host.
    ///
    /// # Limitations
    /// Calling this multiple times with the same event is OK only when the returned future is
    /// polled to completion before the next call to this method *with the same event* is made.
    /// Not doing this with multiple events results in undefined behavior.
    ///
    /// If multiple of the same event need to be made, use
    /// [`wait_for_event_with_matcher`](#method.wait_for_event_with_matcher)
    /// to match the data returned with the event.
    pub fn wait_for_event<'a,D>(&'a self, event: events::Events, timeout: D)
    -> impl Future<Output=Result<events::EventsData, <I as HostControllerInterface>::ReceiveEventError >> + 'a
    where D: Into<Option<Duration>>,
    {
        fn default_matcher(_: &events::EventsData) -> bool { true }

        EventReturnFuture {
            interface: &self.interface,
            event: event,
            matcher: Arc::pin(default_matcher),
            timeout: timeout.into(),
        }
    }

    /// Get a future for a *more* specific Bluetooth Event
    ///
    /// This is the same as the function
    /// `[wait_for_event](#wait_for_event)` except an additional matcher is used to filter same
    /// events based on the data sent with the event. See
    /// `[EventMatcher](../EventMatcher/index.html)`
    /// for information on implementing a matcher, but you can use a closure that borrows
    /// `[EventsData](/hci/events/EventsData)`
    /// as an input and returns a `bool` as a matcher.
    ///
    /// # Limitations
    /// While this can be used to further specify what event data gets returned by a future, its
    /// really just further filters the event data. The same exact limitation scenerio happens when
    /// muliple futures are created to wait upon the same event with matchers that *possibly* have
    /// the same functionality. **Two matchers will have the same functionality if for a given
    /// event data, the method `match_event` returns true.**
    ///
    /// Using a matcher that always returns true results in `wait_for_event_with_matcher`
    /// functioning the same way as
    /// `[wait_for_event](wait_for_event)`
    pub fn wait_for_event_with_matcher<'a,P,D>(&'a self, event: events::Events, timeout: D, matcher: P)
    -> impl Future<Output=Result<events::EventsData, <I as HostControllerInterface>::ReceiveEventError >> + 'a
    where P: EventMatcher + Send + Sync + 'static,
          D: Into<Option<Duration>>,
    {
        EventReturnFuture {
            interface: &self.interface,
            event: event,
            matcher: Arc::pin(matcher),
            timeout: timeout.into(),
        }
    }
}

pub struct HciAclDataReceiver<'a, I> where I: HciAclDataInterface {
    handle: common::ConnectionHandle,
    interface: &'a I
}

impl<'a, I> HciAclDataReceiver<'a, I> where I: HciAclDataInterface {

    fn new(interface: &'a I, handle: common::ConnectionHandle) -> Self {

        interface.start_receiver(handle);

        HciAclDataReceiver { handle, interface }
    }

    /// Get the data (if any) right now
    ///
    /// This is used by the
    /// [`AclHciChannel`](./bo_tie/gap/AclHciChannel)
    /// If there is no data to receive (and no errors occured), then `None` is returned
    pub(crate) fn now(&self, waker: Waker)
    -> Option<Result<alloc::boxed::Box<[HciAclData]>, I::ReceiveACLDataError>>
    {
        match self.interface.receive( &self.handle, waker ) {
            Ok( data ) => if data.len() != 0 { Some( Ok( data ) ) } else { None },
            Err(e) => Some(Err(e)),
        }
    }

    /// Get a future receivier for acquiring the next received ACL data.
    ///
    /// This returns a future that can be used to get the received data.
    pub fn future_receive(&self)
    -> impl Future<Output=Result<alloc::boxed::Box<[HciAclData]>, I::ReceiveACLDataError>> + 'a {
        struct FutureReturn<'a, HI> where HI: HciAclDataInterface {
            handle: common::ConnectionHandle,
            interface: &'a HI
        }

        impl<'a, HI> Future for FutureReturn<'a, HI> where HI: HciAclDataInterface {
            type Output = Result<alloc::boxed::Box<[HciAclData]>, HI::ReceiveACLDataError>;

            fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context)
            -> Poll<Self::Output>
            {
                match self.interface.receive( &self.handle, cx.waker().clone() ) {
                    Ok(buffers) => if buffers.len() != 0 {
                            Poll::Ready(Ok(buffers))
                        } else {
                            Poll::Pending
                        },
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
        }

        FutureReturn {
            handle: self.handle,
            interface: self.interface,
        }
    }
}

impl<'a,I> core::ops::Drop for HciAclDataReceiver<'a,I> where I: HciAclDataInterface {
    fn drop(&mut self) {
        self.interface.stop_receiver(&self.handle)
    }
}

impl<I> HostInterface<I> where I: HciAclDataInterface {

    /// Send ACL data
    pub fn send_data<D>(&self, data: D )
    -> Result<(), I::SendACLDataError>
    where D: Into<HciAclData>
    {
        self.interface.send( data.into() )
    }

    /// Create a Buffered Receiver
    ///
    /// A buffered receiver will queue received ACL data in the order in which they are received.
    /// The buffer will exist, and continue to queue received ACL packets, for the lifetime of the
    /// return.
    pub fn buffered_receiver<'a>(&'a self, connection_handle: common::ConnectionHandle)
    -> HciAclDataReceiver<'a,I>
    {
        HciAclDataReceiver::new(&self.interface, connection_handle)
    }
}

/// For commands that only return a status
macro_rules! impl_status_return {
    ($command:expr) => {
        pub struct Return;

        impl Return {
            fn try_from( raw: u8 ) -> Result<(), error::Error> {
                let status = error::Error::from(raw);

                if let error::Error::NoError = status {
                    Ok(())
                }
                else {
                    Err(status)
                }
            }
        }

        impl_get_data_for_command!($command, u8, Return, (), error::Error);

        impl_command_data_future!(Return, (), error::Error);
    }
}

 #[derive(Debug)]
enum OutputErr<TargErr, CmdErr>
where TargErr: Display + Debug,
      CmdErr: Display + Debug,
{
    /// An error occured at the target specific HCI implementation
    TargetSpecificErr(TargErr),
    /// Cannot convert the data from the HCI packed form into its useable form.
    CommandDataConversionError(CmdErr),
    /// The first item is the received event and the second item is the event expected
    ReceivedIncorrectEvent(crate::hci::events::Events, crate::hci::events::Events),
    /// This is used when either the 'command complete' or 'command status' events contain no data
    /// and are used to indicate the maximum number of HCI command packets that can be queued by
    /// the controller.
    ResponseHasNoAssociatedCommand,
    /// The command status event returned with this error
    CommandStatusErr(error::Error),
}

impl<TargErr, CmdErr> Display for OutputErr<TargErr, CmdErr>
where TargErr: Display + Debug,
      CmdErr: Display + Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            OutputErr::TargetSpecificErr(reason) => {
                core::write!(f, "{}", reason)
            },
            OutputErr::CommandDataConversionError(reason) => {
                core::write!(f, "{}", reason)
            },
            OutputErr::ReceivedIncorrectEvent(received_event, expected_event) => {
                core::write!(f, "Received unexpected event '{:?}' from the bluetooth controller, \
                    expected event {:?}", received_event, expected_event )
            },
            OutputErr::ResponseHasNoAssociatedCommand => {
                core::write!(f,"Event Response contains no data and is not associated with \
                    a HCI command. This should have been handled by the driver and not received \
                    here")
            },
            OutputErr::CommandStatusErr(reason) => {
                core::write!(f, "{}", reason)
            }
        }
    }
}

macro_rules! event_pattern_creator {
    ( $event_path:path, $( $data:pat ),+ ) => { $event_path ( $($data),+ ) };
    ( $event_path:path ) => { $event_path };
}

macro_rules! impl_returned_future {
    // these inputs match the inputs from crate::hci::events::impl_get_data_for_command
    ($return_type: ty, $event: path, $data:pat, $error:ty, $to_do: block) => {

        struct ReturnedFuture<'a, I, CD, P>( CommandFutureReturn<'a, I, CD, P> )
        where I: HostControllerInterface,
              CD: CommandParameter + Unpin,
              P: EventMatcher + Send + Sync + 'static;

        impl<'a, I, CD, P> core::future::Future for ReturnedFuture<'a, I, CD, P>
        where I: HostControllerInterface,
              CD: CommandParameter + Unpin,
              P: EventMatcher + Send + Sync + 'static,
        {
            type Output = core::result::Result< $return_type, crate::hci::OutputErr<SendCommandError<I>,$error>>;

            fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> core::task::Poll<Self::Output> {
                if let core::task::Poll::Ready(result) = self.get_mut().0.fut_poll(cx) {
                    match result {
                        Ok( event_pattern_creator!($event, $data) ) => $to_do,
                        Ok(event @ _) => {
                            let expected_event = crate::hci::events::Events::CommandComplete;
                            let received_event = event.get_enum_name();

                            let ret = Err(crate::hci::OutputErr::ReceivedIncorrectEvent(expected_event, received_event));

                            core::task::Poll::Ready(ret)
                        },
                        Err(reason) =>
                            core::task::Poll::Ready(Err(crate::hci::OutputErr::TargetSpecificErr(reason))),
                    }
                } else {
                    core::task::Poll::Pending
                }
            }
        }
    };

}

macro_rules! impl_command_data_future {
    ($data_type: ty, $return_type: ty, $try_from_err_ty:ty) => {
        impl_returned_future!(
            $return_type,
            crate::hci::events::EventsData::CommandComplete,
            data,
            crate::hci::events::CommandDataErr<$try_from_err_ty>,
            {
                use crate::hci::OutputErr::{
                    ResponseHasNoAssociatedCommand,
                    CommandDataConversionError
                };

                match unsafe {
                    (&data as &dyn crate::hci::events::GetDataForCommand<$data_type>)
                        .get_return()
                } {
                    Ok(Some(ret_val)) => core::task::Poll::Ready(Ok(ret_val)),
                    Ok(None) =>
                        core::task::Poll::Ready(Err(ResponseHasNoAssociatedCommand)),
                    Err(reason) =>
                        core::task::Poll::Ready(Err(CommandDataConversionError(reason))),
                }
            }
        );
    };
    ($data: ty, $try_from_err_ty:ty) => { impl_command_data_future!($data, $data, $try_from_err_ty); };
}

macro_rules! impl_command_status_future {
    () => {
        impl_returned_future!{
            (),
            crate::hci::events::EventsData::CommandStatus,
            data,
            &'static str,
            {
                use crate::hci::OutputErr::CommandStatusErr;

                if let crate::hci::error::Error::NoError = data.status {
                    core::task::Poll::Ready(Ok(()))
                } else {
                    core::task::Poll::Ready(Err(CommandStatusErr(data.status)))
                }
            }
        }
    };
}

pub mod le {

    #[macro_use]
    pub mod common {

        use core::convert::From;
        use core::time::Duration;

        /// The valid address types for this HCI command
        ///
        /// - PublicDeviceAddress
        ///     A bluetooth public address
        /// - RandomDeviceAddress
        ///     A bluetooth random address
        /// - DevicesSendingAnonymousAdvertisements
        ///     A device sending advertisment packets without an address
        pub enum AddressType {
            PublicDeviceAddress,
            RandomDeviceAddress,
            #[cfg(bluetooth_5_0)] DevicesSendingAnonymousAdvertisements,
        }

        impl AddressType {
            pub fn to_value(&self) -> u8 {
                match *self {
                    AddressType::PublicDeviceAddress => 0x00u8,
                    AddressType::RandomDeviceAddress => 0x01u8,
                    #[cfg(bluetooth_5_0)]
                    AddressType::DevicesSendingAnonymousAdvertisements => 0xFFu8,
                }
            }
        }

        /// Own Address Type
        ///
        /// Default is a Public Address.
        ///
        /// # Notes
        /// These are the full explanation for the last two enumerations (as copied from
        /// the core 5.0 specification):
        /// - RPAFromLocalIRKPA -> Controller generates Resolvable Private Address based on
        ///     the local IRK from the resolving list. If the resolving list contains no
        ///     matching entry, use the public address.
        /// - RPAFromLocalIRKRA -> Controller generates Resolvable Private Address based on
        ///     the local IRK from the resolving list. If the resolving list contains no
        ///     matching entry, use the random address from LE_Set_Random_Address.
        #[cfg_attr(test,derive(Debug))]
        pub enum OwnAddressType {
            PublicDeviceAddress,
            RandomDeviceAddress,
            RPAFromLocalIRKPA,
            RPAFromLocalIRKRA,
        }

        impl OwnAddressType {
            pub(super) fn into_val(&self) -> u8 {
                match *self {
                    OwnAddressType::PublicDeviceAddress => 0x00,
                    OwnAddressType::RandomDeviceAddress => 0x01,
                    OwnAddressType::RPAFromLocalIRKPA => 0x02,
                    OwnAddressType::RPAFromLocalIRKRA => 0x03,
                }
            }
        }

        impl Default for OwnAddressType {
            fn default() -> Self {
                OwnAddressType::PublicDeviceAddress
            }
        }

        #[cfg_attr(test,derive(Debug))]
        pub struct Frequency {
            val: u8
        }

        impl Frequency {
            /// Maximum frequency value
            pub const MAX: usize = 2480;

            /// Minimum frequency value
            pub const MIN: usize = 2402;

            /// Creates a new Frequency object
            ///
            /// The value (N) passed to the adapter follows the following equation:
            ///
            /// # Error
            /// The value is less then MIN or greater than MAX. MIN or MAX is returned
            /// depending on which bound is violated.
            pub fn new( mega_hz: usize ) -> Result<Frequency, usize> {
                if mega_hz < Frequency::MIN {
                    Err(Frequency::MIN)
                }
                else if mega_hz > Frequency::MAX {
                    Err(Frequency::MAX)
                }
                else {
                    Ok(Frequency{ val: ((mega_hz - 2402) / 2) as u8})
                }
            }

            pub(in super::super) fn get_val(&self) -> u8 { self.val }
        }

        pub struct IntervalRange<T> where T: PartialEq + PartialOrd {
            pub low: T,
            pub hi: T,
            pub micro_sec_conv: u64,
        }

        impl<T> IntervalRange<T> where T: PartialEq + PartialOrd {

            pub fn contains(&self, val: &T ) -> bool {
                self.low <= *val && *val <= self.hi
            }
        }

        impl From<IntervalRange<u16>> for IntervalRange<Duration> {
            fn from( raw: IntervalRange<u16> ) -> Self {
                IntervalRange {
                    low: Duration::from_micros( raw.low as u64 * raw.micro_sec_conv  ),
                    hi:  Duration::from_micros( raw.hi as u64 * raw.micro_sec_conv  ),
                    micro_sec_conv: raw.micro_sec_conv,
                }
            }
        }

        macro_rules! interval {
            ( $(#[ $expl:meta ])* $name:ident, $raw_low:expr, $raw_hi:expr,
                SpecDef, $raw_default:expr, $micro_sec_conv:expr ) =>
            {
                make_interval!(
                    $(#[ $expl ])*
                    $name,
                    $raw_low,
                    $raw_hi,
                    #[doc("This is a Bluetooth Specification defined default value")],
                    $raw_default,
                    $micro_sec_conv
                );
            };
            ( $(#[ $expl:meta ])* $name:ident, $raw_low:expr, $raw_hi:expr,
                ApiDef, $raw_default:expr, $micro_sec_conv:expr ) =>
            {
                make_interval!(
                    $(#[ $expl ])*
                    $name,
                    $raw_low,
                    $raw_hi,
                    #[doc("This is a default value defined by the API, the Bluetooth Specification")]
                    #[doc("does not specify a default for this interval")],
                    $raw_default,
                    $micro_sec_conv
                );
            }
        }

        macro_rules! make_interval {
            ( $(#[ $expl:meta ])*
                $name:ident,
                $raw_low:expr,
                $raw_hi:expr,
                $(#[ $raw_default_note:meta ])*,
                $raw_default:expr,
                $micro_sec_conv:expr) =>
            {
                $(#[ $expl ])*
                #[cfg_attr(test,derive(Debug))]
                pub struct $name {
                    interval: u16,
                }

                impl $name {

                    const RAW_RANGE: crate::hci::le::common::IntervalRange<u16> = crate::hci::le::common::IntervalRange{
                        low: $raw_low,
                        hi: $raw_hi,
                        micro_sec_conv: $micro_sec_conv,
                    };

                    /// Create an interval from a raw value
                    ///
                    /// # Error
                    /// The value is out of bounds.
                    pub fn try_from_raw( raw: u16 ) -> Result<Self, &'static str> {
                        if $name::RAW_RANGE.contains(&raw) {
                            Ok($name{
                                interval: raw,
                            })
                        }
                        else {
                            Err(concat!("Raw value out of range: ", $raw_low, "..=", $raw_hi))
                        }
                    }

                    /// Create an advertising interval from a Duration
                    ///
                    /// # Error
                    /// the value is out of bounds.
                    pub fn try_from_duration( duration: ::core::time::Duration ) -> Result<Self, &'static str>
                    {
                        let duration_range = crate::hci::le::common::IntervalRange::<::core::time::Duration>::from($name::RAW_RANGE);

                        if duration_range.contains(&duration) {
                            Ok( $name {
                                interval: (duration.as_secs() * (1000000 / $micro_sec_conv)) as u16 +
                                    (duration.subsec_micros() / $micro_sec_conv as u32) as u16,
                            })
                        }
                        else {
                            Err(concat!("Duration out of range: ",
                                stringify!( ($raw_low * $micro_sec_conv) ),
                                "us..=",
                                stringify!( ($raw_hi * $micro_sec_conv) ),
                                "us"))
                        }
                    }

                    /// Get the raw value of the interval
                    pub fn get_raw_val(&self) -> u16 { self.interval }

                    /// Get the value of the interval as a `Duration`
                    pub fn get_duration(&self) -> ::core::time::Duration {
                        ::core::time::Duration::from_micros(
                            (self.interval as u64) * $micro_sec_conv
                        )
                    }
                }

                impl Default for $name {

                    /// Creates an Interval with the default value for the interval
                    ///
                    $(#[ $raw_default_note ])*
                    fn default() -> Self {
                        $name{
                            interval: $raw_default,
                        }
                    }
                }
            };
        }
    }

    /// Manditory commands for a device that implements lE
    ///
    /// Some of these functions are not specific to Bluetooth LE, but they are here to be noted
    /// that they are associated with LE.
    ///
    /// Vol2 Part E 3.1 of the Bluetooth spec
    pub mod mandatory {

        macro_rules! add_remove_white_list_setup {
            ( $command: ident ) => {
                use crate::hci::*;
                use crate::hci::events::Events;
                use crate::hci::le::common::AddressType;

                /// Command parameter data for both add and remove whitelist commands.
                ///
                /// Not using bluez becasue there are different parameter structs for the
                /// two commands even though they are the same in structure.
                #[repr(packed)]
                #[derive(Clone, Copy)]
                struct CommandPrameter {
                    _address_type: u8,
                    _address: [u8;6],
                }

                impl_status_return!( $command );

                pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>,
                    at: AddressType,
                    addr: crate::BluetoothDeviceAddress )
                -> impl core::future::Future<Output=Result<(), impl Display + Debug>> + 'a
                where T: HostControllerInterface
                {
                    let parameter = CommandPrameter {
                        _address_type: at.to_value(),
                        _address: addr,
                    };

                    ReturnedFuture( hci.send_command(parameter, Events::CommandComplete, Duration::from_secs(1) ) )
                }

                impl CommandParameter for CommandPrameter {
                    type Parameter = Self;
                    const COMMAND: opcodes::HCICommand = $command;
                    fn get_parameter(&self) -> Self::Parameter { *self }
                }
            };
        }

        pub mod add_device_to_white_list {
            const COMMAND: crate::hci::opcodes::HCICommand = crate::hci::opcodes::HCICommand::LEController(crate::hci::opcodes::LEController::AddDeviceToWhiteList);

            add_remove_white_list_setup!(COMMAND);
        }

        pub mod clear_white_list {

            use crate::hci::*;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::ClearWhiteList);

            #[derive(Clone, Copy)]
            struct Prameter;

            impl CommandParameter for Prameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter { *self }
            }

            impl_status_return!(COMMAND);

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T> ) -> impl Future<Output=Result<(), impl Display + Debug>> + 'a where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(Prameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }
        }

        pub mod read_buffer_size {

            use crate::hci::*;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::ReadBufferSize);

            #[repr(packed)]
            pub(crate) struct CmdReturn {
                status: u8,
                packet_length: u16,
                maximum_packet_cnt: u8,
            }

            #[derive(Clone,Copy)]
            struct Parameter;

            impl CommandParameter for Parameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter { *self }
            }

            /// This type consists of the ACL packet data length and total number of ACL data
            /// packets the Bluetooth device (controller portion) can store.
            ///
            /// If either member of BufferSize is None (they are either both None or both Some),
            /// then the Read Buffer Size (v5 | vol2, part E, sec 7.4.5) command should be used
            /// instead.
            #[derive(Debug)]
            pub struct BufferSize {
                /// The maximum size of each packet
                pub packet_len: Option<u16>,
                /// The maximum number of packets that the controller can hold
                pub packet_cnt: Option<u8>,
            }

            impl BufferSize {
                fn try_from(packed: CmdReturn) -> Result<Self, error::Error >{
                    let err_val = error::Error::from(packed.status);

                    match err_val {
                        error::Error::NoError => {
                            let len = if packed.packet_length != 0 {
                                Some(packed.packet_length)
                            } else {
                                None
                            };

                            let cnt = if packed.maximum_packet_cnt != 0 {
                                Some(packed.maximum_packet_cnt)
                            } else {
                                None
                            };

                            Ok(BufferSize {
                                packet_len: len,
                                packet_cnt: cnt,
                            })
                        },
                        _ => Err(err_val),
                    }
                }
            }

            impl_get_data_for_command!(
                COMMAND,
                CmdReturn,
                BufferSize,
                error::Error);

            impl_command_data_future!(BufferSize, error::Error);

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T> ) -> impl Future<Output=Result<BufferSize,impl Display + Debug>> + 'a where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(Parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }

        pub mod read_local_supported_features {

            use crate::hci::common::EnabledLEFeaturesItr;
            use crate::hci::*;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::ReadLocalSupportedFeatures);

            #[repr(packed)]
            pub(crate) struct CmdReturn {
                status: u8,
                features: [u8;8]
            }

            impl EnabledLEFeaturesItr {
                fn try_from( packed: CmdReturn ) -> Result<Self,error::Error> {
                    let status = error::Error::from(packed.status);

                    if let error::Error::NoError = status {
                        Ok(EnabledLEFeaturesItr::from(packed.features))
                    }
                    else {
                        Err(status)
                    }
                }
            }

            impl_get_data_for_command!(
                COMMAND,
                CmdReturn,
                EnabledLEFeaturesItr,
                error::Error
            );

            impl_command_data_future!(EnabledLEFeaturesItr, error::Error);

            #[derive(Clone,Copy)]
            struct Parameter;

            impl CommandParameter for Parameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {*self}
            }

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T> )
            -> impl Future<Output=Result<EnabledLEFeaturesItr, impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(Parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }
        pub mod read_supported_states {

            use crate::hci::*;
            use alloc::collections::BTreeSet;
            use core::mem::size_of_val;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::ReadSupportedStates);

            #[repr(packed)]
            pub(crate) struct CmdReturn {
                status: u8,
                states: [u8;8],
            }

            /// All possible states/roles a controller can be in
            #[derive(PartialEq,Eq,PartialOrd,Ord,Debug)]
            pub enum StatesAndRoles {
                ScannableAdvertisingState,
                ConnectableAdvertisingState,
                NonConnectableAdvertisingState,
                HighDutyCyleDirectedAdvertisingState,
                LowDutyCycleDirectedAdvertisingState,
                ActiveScanningState,
                PassiveScanningState,
                InitiatingState,
                ConnectionStateMasterRole,
                ConnectionStateSlaveRole
            }

            impl StatesAndRoles {

                /// Returns the total number of states and roles
                fn get_count() -> usize { 10 }

                /// Returns the total possible bit options
                ///
                /// See Bluetooth v5 vol 2 part E 7.8.27
                fn get_bit_count() -> usize { 41 }

                /// This function doesn't return all available states and roles of a device
                /// (since devices can set multiple of these bits indicating the available
                /// roles) so it doesn't return the special type name.
                fn get_states_for_bit_val( bit_val: usize) ->alloc::vec::Vec<Self> {
                    use self::StatesAndRoles::*;

                    match bit_val {
                        0  => alloc::vec![ NonConnectableAdvertisingState],
                        1  => alloc::vec![ ScannableAdvertisingState],
                        2  => alloc::vec![ ConnectableAdvertisingState],
                        3  => alloc::vec![ HighDutyCyleDirectedAdvertisingState],
                        4  => alloc::vec![ PassiveScanningState],
                        5  => alloc::vec![ ActiveScanningState],
                        6  => alloc::vec![ InitiatingState],
                        7  => alloc::vec![ ConnectionStateSlaveRole],
                        8  => alloc::vec![ NonConnectableAdvertisingState,
                                    PassiveScanningState],
                        9  => alloc::vec![ ScannableAdvertisingState,
                                    PassiveScanningState],
                        10 => alloc::vec![ ConnectableAdvertisingState,
                                    PassiveScanningState],
                        11 => alloc::vec![ HighDutyCyleDirectedAdvertisingState,
                                    PassiveScanningState],
                        12 => alloc::vec![ NonConnectableAdvertisingState,
                                    ActiveScanningState],
                        13 => alloc::vec![ ScannableAdvertisingState,
                                    ActiveScanningState],
                        14 => alloc::vec![ ConnectableAdvertisingState,
                                    ActiveScanningState],
                        15 => alloc::vec![ HighDutyCyleDirectedAdvertisingState,
                                    ActiveScanningState],
                        16 => alloc::vec![ NonConnectableAdvertisingState,
                                    InitiatingState],
                        17 => alloc::vec![ ScannableAdvertisingState,
                                    InitiatingState],
                        18 => alloc::vec![ NonConnectableAdvertisingState,
                                    ConnectionStateMasterRole],
                        19 => alloc::vec![ ScannableAdvertisingState,
                                    ConnectionStateMasterRole],
                        20 => alloc::vec![ NonConnectableAdvertisingState,
                                    ConnectionStateSlaveRole],
                        21 => alloc::vec![ ScannableAdvertisingState,
                                    ConnectionStateSlaveRole],
                        22 => alloc::vec![ PassiveScanningState,
                                    InitiatingState],
                        23 => alloc::vec![ ActiveScanningState,
                                    InitiatingState],
                        24 => alloc::vec![ PassiveScanningState,
                                    ConnectionStateMasterRole],
                        25 => alloc::vec![ ActiveScanningState,
                                    ConnectionStateMasterRole],
                        26 => alloc::vec![ PassiveScanningState,
                                    ConnectionStateSlaveRole],
                        27 => alloc::vec![ ActiveScanningState,
                                    ConnectionStateSlaveRole],
                        28 => alloc::vec![ InitiatingState,
                                    ConnectionStateMasterRole],
                        29 => alloc::vec![ LowDutyCycleDirectedAdvertisingState ],
                        30 => alloc::vec![ LowDutyCycleDirectedAdvertisingState,
                                    PassiveScanningState],
                        31 => alloc::vec![ LowDutyCycleDirectedAdvertisingState,
                                    ActiveScanningState],
                        32 => alloc::vec![ ConnectableAdvertisingState,
                                    InitiatingState],
                        33 => alloc::vec![ HighDutyCyleDirectedAdvertisingState,
                                    InitiatingState],
                        34 => alloc::vec![ LowDutyCycleDirectedAdvertisingState,
                                    InitiatingState],
                        35 => alloc::vec![ ConnectableAdvertisingState,
                                    ConnectionStateMasterRole],
                        36 => alloc::vec![ HighDutyCyleDirectedAdvertisingState,
                                    ConnectionStateMasterRole],
                        37 => alloc::vec![ LowDutyCycleDirectedAdvertisingState,
                                    ConnectionStateMasterRole],
                        38 => alloc::vec![ ConnectableAdvertisingState,
                                    ConnectionStateSlaveRole],
                        39 => alloc::vec![ HighDutyCyleDirectedAdvertisingState,
                                    ConnectionStateSlaveRole],
                        40 => alloc::vec![ LowDutyCycleDirectedAdvertisingState,
                                    ConnectionStateSlaveRole],
                        41 => alloc::vec![ InitiatingState,
                                    ConnectionStateSlaveRole],
                        _  => alloc::vec![],
                    }
                }

                fn collect_to_vec( bts: BTreeSet<StatesAndRoles> ) ->alloc::vec::Vec<Self> {
                    let mut retval =alloc::vec::Vec::<Self>::with_capacity(
                        StatesAndRoles::get_count()
                    );

                    for state_or_role in bts {
                        retval.push(state_or_role)
                    }

                    retval
                }

                /// This function will return all the supported states
                fn get_supported_states( rss: &CmdReturn) ->alloc::vec::Vec<Self> {

                    let mut set = BTreeSet::new();

                    let count = StatesAndRoles::get_bit_count();

                    for byte in 0..size_of_val(&rss.states) {
                        for bit in 0..8 {
                            if (byte * 8 + bit) < count {
                                if 0 != rss.states[byte] & ( 1 << bit ) {
                                    for state_or_role in StatesAndRoles::get_states_for_bit_val( bit ) {
                                        set.insert(state_or_role);
                                    }
                                }
                            }
                            else {
                                return StatesAndRoles::collect_to_vec(set);
                            }
                        }
                    }
                    StatesAndRoles::collect_to_vec(set)
                }

                fn try_from(packed: CmdReturn) -> Result<alloc::vec::Vec<Self>, error::Error> {
                    let status = error::Error::from(packed.status);

                    if let error::Error::NoError = status {
                        Ok(StatesAndRoles::get_supported_states(&packed))
                    }
                    else {
                        Err(status)
                    }
                }
            }

            impl_get_data_for_command!(
                COMMAND,
                CmdReturn,
                StatesAndRoles,
               alloc::vec::Vec<StatesAndRoles>,
                error::Error
            );

            impl_command_data_future!(StatesAndRoles,alloc::vec::Vec<StatesAndRoles>, error::Error);

            #[derive(Clone,Copy)]
            struct Parameter;

            impl CommandParameter for Parameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {*self}
            }

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T> )
            -> impl Future<Output=Result<alloc::vec::Vec<StatesAndRoles>, impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(Parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }

        pub mod read_white_list_size {

            use crate::hci::*;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::ReadWhiteListSize);

            #[repr(packed)]
            pub(crate) struct CmdReturn {
                status: u8,
                size: u8,
            }

            pub struct Return;

            impl Return {
                fn try_from( packed: CmdReturn) -> Result<usize, error::Error> {
                    let status = error::Error::from(packed.status);

                    if let error::Error::NoError = status {
                        Ok(packed.size as usize)
                    }
                    else {
                        Err(status)
                    }
                }
            }

            impl_get_data_for_command! (
                COMMAND,
                CmdReturn,
                Return,
                usize,
                error::Error
            );

            impl_command_data_future!(Return, usize, error::Error);

            #[derive(Clone,Copy)]
            struct Parameter;

            impl CommandParameter for Parameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {*self}
            }

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T> )
            -> impl Future<Output=Result<usize, impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(Parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }

        pub mod remove_device_from_white_list {

            const COMMAND: crate::hci::opcodes::HCICommand = crate::hci::opcodes::HCICommand::LEController(crate::hci::opcodes::LEController::RemoveDeviceFromWhiteList);

            add_remove_white_list_setup!(COMMAND);

        }

        pub mod set_event_mask {

            use crate::hci::*;
            use crate::hci::events::LEMeta;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::SetEventMask);

            impl LEMeta {

                fn bit_offset(&self) -> usize{
                    match *self {
                        LEMeta::ConnectionComplete => 0,
                        LEMeta::AdvertisingReport => 1,
                        LEMeta::ConnectionUpdateComplete => 2,
                        LEMeta::ReadRemoteFeaturesComplete => 3,
                        LEMeta::LongTermKeyRequest => 4,
                        LEMeta::RemoteConnectionParameterRequest => 5,
                        LEMeta::DataLengthChange => 6,
                        LEMeta::ReadLocalP256PublicKeyComplete => 7,
                        LEMeta::GenerateDHKeyComplete => 8,
                        LEMeta::EnhancedConnectionComplete => 9,
                        LEMeta::DirectedAdvertisingReport => 10,
                        LEMeta::PHYUpdateComplete => 11,
                        LEMeta::ExtendedAdvertisingReport => 12,
                        LEMeta::PeriodicAdvertisingSyncEstablished => 13,
                        LEMeta::PeriodicAdvertisingReport => 14,
                        LEMeta::PeriodicAdvertisingSyncLost => 15,
                        LEMeta::ScanTimeout => 16,
                        LEMeta::AdvertisingSetTerminated => 17,
                        LEMeta::ScanRequestReceived => 18,
                        LEMeta::ChannelSelectionAlgorithm => 19,
                    }
                }

                fn build_mask( events:alloc::vec::Vec<Self>) -> [u8;8] {
                    let mut mask = <[u8;8]>::default();

                    for event in events {
                        let bit = event.bit_offset();
                        let byte = bit/8;

                        mask[byte] |= 1 << (bit % 8);
                    }

                    mask
                }
            }

            impl_status_return!(COMMAND);

            #[repr(packed)]
            #[derive( Clone, Copy)]
            struct CmdParameter {
                _mask: [u8;8]
            }

            impl CommandParameter for CmdParameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {*self}
            }

            /// Set the enabled events on a device
            ///
            /// ```rust
            /// # use bo_tie_linux::hci::le::mandatory::set_event_mask::*;
            /// # let host_interface = bo_tie_linux::hci::crate::hci::test_util::get_adapter();
            ///
            /// let events = alloc::vec!(Events::LEConnectionComplete,Events::LEAdvertisingReport);
            ///
            /// // This will enable the LE Connection Complete Event and LE Advertising Report Event
            /// send(&host_interface, events);
            /// ```
            pub fn send<'a, T: 'static>( hi: &'a HostInterface<T>, enabled_events:alloc::vec::Vec<LEMeta>)
            -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {

                let command_pram = CmdParameter {
                    _mask: LEMeta::build_mask(enabled_events),
                };

                ReturnedFuture( hi.send_command(command_pram, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }

        pub mod test_end {

            use crate::hci::*;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::TestEnd);

            #[repr(packed)]
            pub(crate) struct CmdReturn {
                status: u8,
                number_of_packets: u16
            }

            pub struct Return;

            impl Return {
                fn try_from(packed: CmdReturn) -> Result<usize, error::Error> {
                    let status = error::Error::from(packed.status);

                    if let error::Error::NoError = status {
                        Ok(packed.number_of_packets as usize)
                    }
                    else {
                        Err(status)
                    }
                }
            }

            impl_get_data_for_command!(
                COMMAND,
                CmdReturn,
                Return,
                usize,
                error::Error
            );

            impl_command_data_future!(Return, usize, error::Error);

            #[derive(Clone,Copy)]
            struct Parameter;

            impl CommandParameter for Parameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {*self}
            }

            /// This will return a future with its type 'Output' being the number of packets
            /// received during what ever test was done
            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T> )
            -> impl Future<Output=Result<usize, impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(Parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }

        /// This is part of the Informational Parameters opcodesgroup
        // TODO when BR/EDR is enabled move this to a module for common features and import here
        pub mod ip_read_bd_addr {

            use crate::BluetoothDeviceAddress;
            use crate::hci::*;
            use core::fmt::{Display, Debug};

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::InformationParameters(opcodes::InformationParameters::ReadBD_ADDR);

            #[repr(packed)]
            pub(crate) struct CmdReturn {
                status: u8,
                address: BluetoothDeviceAddress,
            }

            struct Return;

            impl Return {
                fn try_from(packed: CmdReturn) -> Result<BluetoothDeviceAddress, error::Error> {
                    let status = error::Error::from(packed.status);

                    if let error::Error::NoError = status {
                        Ok(packed.address)
                    }
                    else {
                        Err(status)
                    }
                }
            }

            impl_get_data_for_command!(
                COMMAND,
                CmdReturn,
                Return,
                BluetoothDeviceAddress,
                error::Error
            );

            impl_command_data_future!(Return, BluetoothDeviceAddress, error::Error);

            #[derive(Clone,Copy)]
            struct Parameter;

            impl CommandParameter for Parameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {*self}
            }

            /// Returns the bluetooth device address for the device
            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T> ) -> impl Future<Output=Result<BluetoothDeviceAddress, impl Display + Debug>> + 'a where T: HostControllerInterface
            {
                use events::Events::CommandComplete;

                let cmd_rslt = hci.send_command(Parameter, CommandComplete, Duration::from_secs(1) );

                ReturnedFuture(cmd_rslt)
            }

        }

        /// This is part of the Informational Parameters opcodesgroup
        // TODO when BR/EDR is enabled move this to a module for common features and import here
        pub mod ip_read_local_supported_features {

            use crate::hci::*;
            use crate::hci::common::EnabledFeaturesIter;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::InformationParameters(opcodes::InformationParameters::ReadLocalSupportedFeatures);

            #[repr(packed)]
            pub(crate) struct CmdReturn {
                status: u8,
                features: [u8;8],
            }

            impl EnabledFeaturesIter {
                fn try_from(packed: CmdReturn) -> Result<Self, error::Error> {
                    let status = error::Error::from(packed.status);

                    if let error::Error::NoError = status {
                        Ok(EnabledFeaturesIter::from(packed.features))
                    }
                    else {
                        Err(status)
                    }
                }
            }

            impl_get_data_for_command! (
                COMMAND,
                CmdReturn,
                EnabledFeaturesIter,
                error::Error
            );

            impl_command_data_future!(EnabledFeaturesIter, error::Error);

            #[derive(Clone,Copy)]
            struct Parameter;

            impl CommandParameter for Parameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {*self}
            }

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T> )
            -> impl Future<Output=Result<EnabledFeaturesIter, impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(Parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }

        // This is part of the Information Parameters opcodesgroup
        // TODO when BR/EDR is enabled move this to a module for common features and import here
        pub mod ip_read_local_version_information {

            use crate::hci::*;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::InformationParameters(opcodes::InformationParameters::ReadLocalSupportedVersionInformation);

            #[repr(packed)]
            pub(crate) struct CmdReturn {
                status: u8,
                hci_version: u8,
                hci_revision: u16,
                lmp_pal_version: u8,
                manufacturer_name: u16,
                lmp_pal_subversion: u16,
            }

            #[derive(Debug)]
            pub struct VersionInformation {
                pub hci_version: u8,
                pub hci_revision: u16,
                pub lmp_pal_version: u8,
                pub manufacturer_name: u16,
                pub lmp_pal_subversion: u16,
            }

            impl VersionInformation {
                fn try_from(packed: CmdReturn) -> Result<Self, error::Error> {
                    let status = error::Error::from(packed.status);

                    if let error::Error::NoError = status {
                        Ok( Self {
                            hci_version: packed.hci_version,
                            hci_revision: packed.hci_revision,
                            lmp_pal_version: packed.lmp_pal_version,
                            manufacturer_name: packed.manufacturer_name,
                            lmp_pal_subversion: packed.lmp_pal_subversion,
                        })
                    }
                    else {
                        Err(status)
                    }
                }
            }

            impl_get_data_for_command!(
                COMMAND,
                CmdReturn,
                VersionInformation,
                error::Error
            );

            impl_command_data_future!(VersionInformation, error::Error);

            #[derive(Clone,Copy)]
            struct Parameter;

            impl CommandParameter for Parameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {*self}
            }

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T> )
            -> impl Future<Output=Result<VersionInformation, impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(Parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }
        // This is part of the Host Controller and Baseband opcodesgroup
        // TODO when BR/EDR is enabled move this to a module for common features and import here
        pub mod reset {

            use crate::hci::*;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::ControllerAndBaseband(opcodes::ControllerAndBaseband::Reset);

            impl_status_return!(COMMAND);

            #[derive(Clone,Copy)]
            struct Parameter;

            impl CommandParameter for Parameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter { *self }
            }

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T> ) -> impl Future<Output=Result<(), impl Display + Debug>> + 'a where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(Parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }

        // This is part of the Informational Parameters opcodesgroup
        // TODO when BR/EDR is enabled move this to a module for common features and import here
        pub mod ip_read_local_supported_commands {

            use crate::hci::*;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::InformationParameters(opcodes::InformationParameters::ReadLocalSupportedCommands);

            #[repr(packed)]
            pub(crate) struct CmdReturn {
                status: u8,
                supported_commands: [u8;64],
            }

            #[cfg_attr(test,derive(Debug))]
            #[derive(PartialEq)]
            pub enum SupportedCommands {
                Inquiry,
                InquiryCancel,
                PeriodicInquiryMode,
                ExitPeriodicInquiryMode,
                CreateConnection,
                Disconnect,
                /// Depreciated
                AddSCOConnection,
                CreateConnectionCancel,
                AcceptConnectionRequest,
                RejectConnectionRequest,
                LinkKeyRequestReply,
                LinkKeyRequestNegativeReply,
                PINCodeRequestReply,
                PINCodeRequestNegativeReply,
                ChangeConnectionPacketType,
                AuthenticationRequested,
                SetConnectionEncryption,
                ChangeConnectionLinkKey,
                MasterLinkKey,
                RemoteNameRequest,
                RemoteNameRequestCancel,
                ReadRemoteSupportedFeatures,
                ReadRemoteExtendedFeatures,
                ReadRemoteVersionInformation,
                ReadClockOffset,
                ReadLMPHandle,
                HoldMode,
                SniffMode,
                ExitSniffMode,
                QosSetup,
                RoleDiscovery,
                SwitchRole,
                ReadLinkPolicySettings,
                WriteLinkPolicySettings,
                ReadDefaultLinkPolicySettings,
                WriteDefaultLinkPolicySettings,
                FlowSpecification,
                SetEventMask,
                Reset,
                SetEVentFilter,
                Flush,
                ReadPINType,
                WritePINType,
                CreateNewUnitKey,
                ReadStoredLinkKey,
                WriteStoredLinkKey,
                DeleteStoredLinkKey,
                WriteLocalName,
                ReadLocalName,
                ReadConnectionAcceptedTimeout,
                WriteConnectionAcceptedTimeout,
                ReadPageTimeout,
                WritePageTimeout,
                ReadScanEnable,
                WriteScanEnable,
                ReadPageScanActivity,
                WritePageScanActivity,
                ReadInquiryScanActivity,
                WriteInquiryScanActivity,
                ReadAuthenticationEnable,
                WriteAuthenticationEnable,
                ///Depreciated
                ReadEncryptionMode,
                ///Depreciated
                WriteEncryptionMode,
                ReadClassOfDevice,
                WriteClassOfDevice,
                REadVoiceSetting,
                WriteVoiceSetting,
                ReadAutomaticFlushTimeout,
                WriteAutomaticFlushTimeout,
                ReadNumBroadcastRetransmission,
                WriteNumBroadcastRetransmissions,
                ReadHoldModeActivity,
                WriteHoldModeActiviy,
                ReadTransmitPowerLevel,
                ReadSynchronousFlowControlEnable,
                WriteSynchronousFlowControlEnable,
                SetConrollerToHostFlowControl,
                HostBufferSize,
                HostNumberOfCompletedPackets,
                ReadLinkSupervisionTimeout,
                WriteLinkSupervisionTimeout,
                ReadNumberOfSupportedIAC,
                ReadCurrentIACLAP,
                WriteCurrentIACLAP,
                /// Depreciated
                ReadPageScanModePeriod,
                /// Depreciated
                WritePageScanModePeriod,
                /// Depreciated
                ReadPageScanMode,
                /// Depreciated
                WritePageSanMode,
                SetAFHHostChannel,
                ReadInquiryScanType,
                WriteInquirySCanType,
                ReadInquiryMode,
                WriteInquiryMode,
                ReadPageScanType,
                WritePageScanType,
                ReadAFHChannelAssessmentMode,
                WriteAFHChannelAssessmentMode,
                ReadLocalVersionInformation,
                ReadLocalSupportedFeatures,
                ReadLocalExtendedFeatures,
                ReadBufferSize,
                /// Depreciated
                ReadCountryCode,
                ReadBDADDR,
                ReadFAiledContactCounter,
                ResetFailedContactCounter,
                ReadLinkQuality,
                ReadRSSI,
                ReadAFHChannelMap,
                ReadClock,
                ReadLoopbackMode,
                WriteLoopbackMode,
                EnableDeviceUnderTestMode,
                SetupSynchronousConnectionRequest,
                AcceptSynchronousConnectionRequest,
                RejectSynchronousConnectionRequest,
                ReadExtendedInquiryResponse,
                WriteExtendedInquiryResponse,
                RefreshEncryptionKey,
                SniffSubrating,
                ReadSimplePairingMode,
                WriteSimplePairingMode,
                ReadLocalOOBData,
                ReadInquiryResponseTransmitPowerLevel,
                WriteInquiryTransmitPowerLevel,
                ReadDefaultErroneousDataReporting,
                WriteDefaultErroneousDataReporting,
                IOCapabilityRequestReply,
                UserConfirmationRequestReply,
                UserConfirmationRequestNegativeReply,
                UserPasskeyRequestReply,
                UserPasskeyRequestNegativeReply,
                RemoteOOBDataRequestReply,
                WriteSimplePairingDebugMode,
                EnhancedFlush,
                RemoteOOBDataRequestNagativeReply,
                SendKeypressNotification,
                IOCapabilityRequestNegativeReply,
                ReadEncryptionKeySize,
                CreatePhysicalLink,
                AcceptPhysicalLink,
                DisconnectPhysicalLink,
                CreateLogicalLink,
                AcceptLogicalLink,
                DisconnectLogicalLink,
                LogicalLinkCancel,
                FlowSpecModify,
                ReadLogicalLinkAcceptTimeout,
                WriteLogicalLinkAcceptTimeout,
                SetEventMaskPage2,
                ReadLocationData,
                WRiteLocationData,
                ReadLocalAMPInfo,
                ReadLocalAMPASSOC,
                WriteRemoteAMPASSOC,
                READFlowControlMode,
                WriteFlowControlMode,
                ReadDataBlockSize,
                EnableAMPReceiverReports,
                AMPTestEnd,
                AmPTest,
                ReadEnhancedTransmitPowerLevel,
                ReadBestEffortFlushTimeout,
                WriteBestEffortFlushTimeout,
                ShortRangeMode,
                ReadLEHostSupport,
                WriteLEHostSupport,
                LESetEventMask,
                LEReadBufferSize,
                LEReadLocalSupportedFeatures,
                LESetRandomAddress,
                LESetAdvertisingParameters,
                LEReadAdvertisingChannelTXPower,
                LESetAdvertisingData,
                LESetScanResponseData,
                LESetAdvertisingEnable,
                LESetScanParameters,
                LESetScanEnable,
                LECreateConnection,
                LECreateConnectionCancel,
                LEReadWhiteListSize,
                LEClearWhiteList,
                LEAddDeviceToWhiteList,
                LERemoveDeviceFromWhiteList,
                LEConnectionUpdate,
                LESetHostChannelClassification,
                LEReadChannelMap,
                LEReadRemoteFeatures,
                LEEncrypt,
                LERand,
                LEStartEncryption,
                LELongTermKeyRequestReply,
                LELongTermKeyRequestNegativeReply,
                LEReadSupportedStates,
                LEReceiverTest,
                LETransmitterTest,
                LETestEnd,
                EnhancedSetupSynchronousConnection,
                EnhancedAcceptSynchronousConnection,
                ReadLocalSupportedCondecs,
                SetMWSChannelParameters,
                SetExternalFrameConfiguration,
                SetMWSSignaling,
                SetMWSTransportLayer,
                SetMWSScanFrequencyTable,
                GetMWSTransportLayerConfiguration,
                SetMWSPATTERNConfiguration,
                SetTriggeredClockCapture,
                TruncatedPage,
                TruncatedPageCancel,
                SetConnectionlessSlaveBroadcast,
                SetConnectionlessSlaveBroadcastReceive,
                StartSynchronizationTrain,
                ReceiveSynchronizationTrain,
                SetReservedLTADDR,
                DeleteReservedLTADDR,
                SetConnectionlessSlaveBroadcastData,
                ReadSynchronizationTrainParameters,
                WriteSynchronizationTrainParameters,
                RemoteOOBExtendedDataRequestReply,
                ReadSecureConnectionsHostSupport,
                WriteSecureConnectionsHostSupport,
                ReadAuthenticatedPayloadTimeout,
                WriteAuthenticatedPayloadTimeout,
                ReadLocalOOBExtendedData,
                WriteSecureConnectionsTestMode,
                ReadExtendedPageTimeout,
                WriteExtendedPageTimeout,
                ReadExtendedInquiryLength,
                WriteExtendedInquiryLengh,
                LERemoteConnectionParameterRequestReply,
                LERemoteConnectionParameterREquestNegativeReply,
                LESetDataLength,
                LEReadSuggestedDefaultDataLength,
                LEWriteSuggestedDefaultDataLength,
                LEReadLocalP256PublicKey,
                LEGenerateDHKey,
                LEAddDeviceToResolvingList,
                LERemoveDeviceFromResolvingList,
                LEClearResolvingList,
                LEReadResolvingListSize,
                LEReadPeerResolvableAddress,
                LEReadLocalResolvableAddress,
                LESetAddressResolutionEnable,
                LESetResolvablePrivateAddressTimeout,
                LEReadMaximumDataLength,
                LEReadPHYCommand,
                LESetDefaultPHYCommand,
                LESetPHYCommand,
                LEEnhancedReceiverTestCommand,
                LEEnhancedTransmitterTestCommand,
                LESetAdvertisingSetRandomAddressCommand,
                LESetExtendedAdvertisingParametersCommand,
                LESetExtendedAdvertisingDataCommand,
                LESetExtendedScanResponseDataCommand,
                LESetExtendedAdvertisingEnableCommand,
                LEReadMaximumAdvertisingDataLengthCommand,
                LEReadNumberOfSupportedAdvertisingSetCommand,
                LERemoveAdvertisingSetCommand,
                LEClearAdvertisingSetsCommand,
                LESetPeriodicAdvertisingParametersCommand,
                LESetPeriodicAdvertisingDataCommand,
                LESetPeriodicAdvertisingEnableCommand,
                LESetExtendedScanParametersCommand,
                LESetExtendedScanEnableCommand,
                LEExtendedCreateConnectionCommand,
                LEPeriodicAdvertisingCreateSyncCommand,
                LEPeriodicAdvertisingCreateSyncCancelCommand,
                LEPeriodicAdvertisingTerminateSyncCommand,
                LEAddDeviceToPeriodicAdvertiserListCommand,
                LERemoveDeviceFromPeriodicAdvertiserListCommand,
                LEClearPeriodicAdvertiserListCommand,
                LEReadPeriodicAdvertiserListSizeCommand,
                LEReadTransmitPowerCommand,
                LEReadRFPathCompensationCommand,
                LEWriteRFPathCompensationCommand,
                LESetPrivacyMode,
            }

            impl SupportedCommands {

                fn from_bit_pos( pos: (usize, usize) ) -> Option<SupportedCommands> {
                    use self::SupportedCommands::*;

                    match pos {
                        (0,0)  => Some(Inquiry),
                        (0,1)  => Some(InquiryCancel),
                        (0,2)  => Some(PeriodicInquiryMode),
                        (0,3)  => Some(ExitPeriodicInquiryMode),
                        (0,4)  => Some(CreateConnection),
                        (0,5)  => Some(Disconnect),
                        (0,6)  => Some(AddSCOConnection),
                        (0,7)  => Some(CreateConnectionCancel),
                        (1,0)  => Some(AcceptConnectionRequest),
                        (1,1)  => Some(RejectConnectionRequest),
                        (1,2)  => Some(LinkKeyRequestReply),
                        (1,3)  => Some(LinkKeyRequestNegativeReply),
                        (1,4)  => Some(PINCodeRequestReply),
                        (1,5)  => Some(PINCodeRequestNegativeReply),
                        (1,6)  => Some(ChangeConnectionPacketType),
                        (1,7)  => Some(AuthenticationRequested),
                        (2,0)  => Some(SetConnectionEncryption),
                        (2,1)  => Some(ChangeConnectionLinkKey),
                        (2,2)  => Some(MasterLinkKey),
                        (2,3)  => Some(RemoteNameRequest),
                        (2,4)  => Some(RemoteNameRequestCancel),
                        (2,5)  => Some(ReadRemoteSupportedFeatures),
                        (2,6)  => Some(ReadRemoteExtendedFeatures),
                        (2,7)  => Some(ReadRemoteVersionInformation),
                        (3,0)  => Some(ReadClockOffset),
                        (3,1)  => Some(ReadLMPHandle),
                        (4,1)  => Some(HoldMode),
                        (4,2)  => Some(SniffMode),
                        (4,3)  => Some(ExitSniffMode),
                        (4,6)  => Some(QosSetup),
                        (4,7)  => Some(RoleDiscovery),
                        (5,0)  => Some(SwitchRole),
                        (5,1)  => Some(ReadLinkPolicySettings),
                        (5,2)  => Some(WriteLinkPolicySettings),
                        (5,3)  => Some(ReadDefaultLinkPolicySettings),
                        (5,4)  => Some(WriteDefaultLinkPolicySettings),
                        (5,5)  => Some(FlowSpecification),
                        (5,6)  => Some(SetEventMask),
                        (5,7)  => Some(Reset),
                        (6,0)  => Some(SetEVentFilter),
                        (6,1)  => Some(Flush),
                        (6,2)  => Some(ReadPINType),
                        (6,3)  => Some(WritePINType),
                        (6,4)  => Some(CreateNewUnitKey),
                        (6,5)  => Some(ReadStoredLinkKey),
                        (6,6)  => Some(WriteStoredLinkKey),
                        (6,7)  => Some(DeleteStoredLinkKey),
                        (7,0)  => Some(WriteLocalName),
                        (7,1)  => Some(ReadLocalName),
                        (7,2)  => Some(ReadConnectionAcceptedTimeout),
                        (7,3)  => Some(WriteConnectionAcceptedTimeout),
                        (7,4)  => Some(ReadPageTimeout),
                        (7,5)  => Some(WritePageTimeout),
                        (7,6)  => Some(ReadScanEnable),
                        (7,7)  => Some(WriteScanEnable),
                        (8,0)  => Some(ReadPageScanActivity),
                        (8,1)  => Some(WritePageScanActivity),
                        (8,2)  => Some(ReadInquiryScanActivity),
                        (8,3)  => Some(WriteInquiryScanActivity),
                        (8,4)  => Some(ReadAuthenticationEnable),
                        (8,5)  => Some(WriteAuthenticationEnable),
                        (8,6)  => Some(ReadEncryptionMode),
                        (8,7)  => Some(WriteEncryptionMode),
                        (9,0)  => Some(ReadClassOfDevice),
                        (9,1)  => Some(WriteClassOfDevice),
                        (9,2)  => Some(REadVoiceSetting),
                        (9,3)  => Some(WriteVoiceSetting),
                        (9,4)  => Some(ReadAutomaticFlushTimeout),
                        (9,5)  => Some(WriteAutomaticFlushTimeout),
                        (9,6)  => Some(ReadNumBroadcastRetransmission),
                        (9,7)  => Some(WriteNumBroadcastRetransmissions),
                        (10,0) => Some(ReadHoldModeActivity),
                        (10,1) => Some(WriteHoldModeActiviy),
                        (10,2) => Some(ReadTransmitPowerLevel),
                        (10,3) => Some(ReadSynchronousFlowControlEnable),
                        (10,4) => Some(WriteSynchronousFlowControlEnable),
                        (10,5) => Some(SetConrollerToHostFlowControl),
                        (10,6) => Some(HostBufferSize),
                        (10,7) => Some(HostNumberOfCompletedPackets),
                        (11,0) => Some(ReadLinkSupervisionTimeout),
                        (11,1) => Some(WriteLinkSupervisionTimeout),
                        (11,2) => Some(ReadNumberOfSupportedIAC),
                        (11,3) => Some(ReadCurrentIACLAP),
                        (11,4) => Some(WriteCurrentIACLAP),
                        (11,5) => Some(ReadPageScanModePeriod),
                        (11,6) => Some(WritePageScanModePeriod),
                        (11,7) => Some(ReadPageScanMode),
                        (12,0) => Some(WritePageSanMode),
                        (12,1) => Some(SetAFHHostChannel),
                        (12,4) => Some(ReadInquiryScanType),
                        (12,5) => Some(WriteInquirySCanType),
                        (12,6) => Some(ReadInquiryMode),
                        (12,7) => Some(WriteInquiryMode),
                        (13,0) => Some(ReadPageScanType),
                        (13,1) => Some(WritePageScanType),
                        (13,2) => Some(ReadAFHChannelAssessmentMode),
                        (13,3) => Some(WriteAFHChannelAssessmentMode),
                        (14,3) => Some(ReadLocalVersionInformation),
                        (14,5) => Some(ReadLocalSupportedFeatures),
                        (14,6) => Some(ReadLocalExtendedFeatures),
                        (14,7) => Some(ReadBufferSize),
                        (15,0) => Some(ReadCountryCode),
                        (15,1) => Some(ReadBDADDR),
                        (15,2) => Some(ReadFAiledContactCounter),
                        (15,3) => Some(ResetFailedContactCounter),
                        (15,4) => Some(ReadLinkQuality),
                        (15,5) => Some(ReadRSSI),
                        (15,6) => Some(ReadAFHChannelMap),
                        (15,7) => Some(ReadClock),
                        (16,0) => Some(ReadLoopbackMode),
                        (16,1) => Some(WriteLoopbackMode),
                        (16,2) => Some(EnableDeviceUnderTestMode),
                        (16,3) => Some(SetupSynchronousConnectionRequest),
                        (16,4) => Some(AcceptSynchronousConnectionRequest),
                        (16,5) => Some(RejectSynchronousConnectionRequest),
                        (17,0) => Some(ReadExtendedInquiryResponse),
                        (17,1) => Some(WriteExtendedInquiryResponse),
                        (17,2) => Some(RefreshEncryptionKey),
                        (17,4) => Some(SniffSubrating),
                        (17,5) => Some(ReadSimplePairingMode),
                        (17,6) => Some(WriteSimplePairingMode),
                        (17,7) => Some(ReadLocalOOBData),
                        (18,0) => Some(ReadInquiryResponseTransmitPowerLevel),
                        (18,1) => Some(WriteInquiryTransmitPowerLevel),
                        (18,2) => Some(ReadDefaultErroneousDataReporting),
                        (18,3) => Some(WriteDefaultErroneousDataReporting),
                        (18,7) => Some(IOCapabilityRequestReply),
                        (19,0) => Some(UserConfirmationRequestReply),
                        (19,1) => Some(UserConfirmationRequestNegativeReply),
                        (19,2) => Some(UserPasskeyRequestReply),
                        (19,3) => Some(UserPasskeyRequestNegativeReply),
                        (19,4) => Some(RemoteOOBDataRequestReply),
                        (19,5) => Some(WriteSimplePairingDebugMode),
                        (19,6) => Some(EnhancedFlush),
                        (19,7) => Some(RemoteOOBDataRequestNagativeReply),
                        (20,2) => Some(SendKeypressNotification),
                        (20,3) => Some(IOCapabilityRequestNegativeReply),
                        (20,4) => Some(ReadEncryptionKeySize),
                        (21,0) => Some(CreatePhysicalLink),
                        (21,1) => Some(AcceptPhysicalLink),
                        (21,2) => Some(DisconnectPhysicalLink),
                        (21,3) => Some(CreateLogicalLink),
                        (21,4) => Some(AcceptLogicalLink),
                        (21,5) => Some(DisconnectLogicalLink),
                        (21,6) => Some(LogicalLinkCancel),
                        (21,7) => Some(FlowSpecModify),
                        (22,0) => Some(ReadLogicalLinkAcceptTimeout),
                        (22,1) => Some(WriteLogicalLinkAcceptTimeout),
                        (22,2) => Some(SetEventMaskPage2),
                        (22,3) => Some(ReadLocationData),
                        (22,4) => Some(WRiteLocationData),
                        (22,5) => Some(ReadLocalAMPInfo),
                        (22,6) => Some(ReadLocalAMPASSOC),
                        (22,7) => Some(WriteRemoteAMPASSOC),
                        (23,0) => Some(READFlowControlMode),
                        (23,1) => Some(WriteFlowControlMode),
                        (23,2) => Some(ReadDataBlockSize),
                        (23,5) => Some(EnableAMPReceiverReports),
                        (23,6) => Some(AMPTestEnd),
                        (23,7) => Some(AmPTest),
                        (24,0) => Some(ReadEnhancedTransmitPowerLevel),
                        (24,2) => Some(ReadBestEffortFlushTimeout),
                        (24,3) => Some(WriteBestEffortFlushTimeout),
                        (24,4) => Some(ShortRangeMode),
                        (24,5) => Some(ReadLEHostSupport),
                        (24,6) => Some(WriteLEHostSupport),
                        (25,0) => Some(LESetEventMask),
                        (25,1) => Some(LEReadBufferSize),
                        (25,2) => Some(LEReadLocalSupportedFeatures),
                        (25,4) => Some(LESetRandomAddress),
                        (25,5) => Some(LESetAdvertisingParameters),
                        (25,6) => Some(LEReadAdvertisingChannelTXPower),
                        (25,7) => Some(LESetAdvertisingData),
                        (26,0) => Some(LESetScanResponseData),
                        (26,1) => Some(LESetAdvertisingEnable),
                        (26,2) => Some(LESetScanParameters),
                        (26,3) => Some(LESetScanEnable),
                        (26,4) => Some(LECreateConnection),
                        (26,5) => Some(LECreateConnectionCancel),
                        (26,6) => Some(LEReadWhiteListSize),
                        (26,7) => Some(LEClearWhiteList),
                        (27,0) => Some(LEAddDeviceToWhiteList),
                        (27,1) => Some(LERemoveDeviceFromWhiteList),
                        (27,2) => Some(LEConnectionUpdate),
                        (27,3) => Some(LESetHostChannelClassification),
                        (27,4) => Some(LEReadChannelMap),
                        (27,5) => Some(LEReadRemoteFeatures),
                        (27,6) => Some(LEEncrypt),
                        (27,7) => Some(LERand),
                        (28,0) => Some(LEStartEncryption),
                        (28,1) => Some(LELongTermKeyRequestReply),
                        (28,2) => Some(LELongTermKeyRequestNegativeReply),
                        (28,3) => Some(LEReadSupportedStates),
                        (28,4) => Some(LEReceiverTest),
                        (28,5) => Some(LETransmitterTest),
                        (28,6) => Some(LETestEnd),
                        (29,3) => Some(EnhancedSetupSynchronousConnection),
                        (29,4) => Some(EnhancedAcceptSynchronousConnection),
                        (29,5) => Some(ReadLocalSupportedCondecs),
                        (29,6) => Some(SetMWSChannelParameters),
                        (29,7) => Some(SetExternalFrameConfiguration),
                        (30,0) => Some(SetMWSSignaling),
                        (30,1) => Some(SetMWSTransportLayer),
                        (30,2) => Some(SetMWSScanFrequencyTable),
                        (30,3) => Some(GetMWSTransportLayerConfiguration),
                        (30,4) => Some(SetMWSPATTERNConfiguration),
                        (30,5) => Some(SetTriggeredClockCapture),
                        (30,6) => Some(TruncatedPage),
                        (30,7) => Some(TruncatedPageCancel),
                        (31,0) => Some(SetConnectionlessSlaveBroadcast),
                        (31,1) => Some(SetConnectionlessSlaveBroadcastReceive),
                        (31,2) => Some(StartSynchronizationTrain),
                        (31,3) => Some(ReceiveSynchronizationTrain),
                        (31,4) => Some(SetReservedLTADDR),
                        (31,5) => Some(DeleteReservedLTADDR),
                        (31,6) => Some(SetConnectionlessSlaveBroadcastData),
                        (31,7) => Some(ReadSynchronizationTrainParameters),
                        (32,0) => Some(WriteSynchronizationTrainParameters),
                        (32,1) => Some(RemoteOOBExtendedDataRequestReply),
                        (32,2) => Some(ReadSecureConnectionsHostSupport),
                        (32,3) => Some(WriteSecureConnectionsHostSupport),
                        (32,4) => Some(ReadAuthenticatedPayloadTimeout),
                        (32,5) => Some(WriteAuthenticatedPayloadTimeout),
                        (32,6) => Some(ReadLocalOOBExtendedData),
                        (32,7) => Some(WriteSecureConnectionsTestMode),
                        (33,0) => Some(ReadExtendedPageTimeout),
                        (33,1) => Some(WriteExtendedPageTimeout),
                        (33,2) => Some(ReadExtendedInquiryLength),
                        (33,3) => Some(WriteExtendedInquiryLengh),
                        (33,4) => Some(LERemoteConnectionParameterRequestReply),
                        (33,5) => Some(LERemoteConnectionParameterREquestNegativeReply),
                        (33,6) => Some(LESetDataLength),
                        (33,7) => Some(LEReadSuggestedDefaultDataLength),
                        (34,0) => Some(LEWriteSuggestedDefaultDataLength),
                        (34,1) => Some(LEReadLocalP256PublicKey),
                        (34,2) => Some(LEGenerateDHKey),
                        (34,3) => Some(LEAddDeviceToResolvingList),
                        (34,4) => Some(LERemoveDeviceFromResolvingList),
                        (34,5) => Some(LEClearResolvingList),
                        (34,6) => Some(LEReadResolvingListSize),
                        (34,7) => Some(LEReadPeerResolvableAddress),
                        (35,0) => Some(LEReadLocalResolvableAddress),
                        (35,1) => Some(LESetAddressResolutionEnable),
                        (35,2) => Some(LESetResolvablePrivateAddressTimeout),
                        (35,3) => Some(LEReadMaximumDataLength),
                        (35,4) => Some(LEReadPHYCommand),
                        (35,5) => Some(LESetDefaultPHYCommand),
                        (35,6) => Some(LESetPHYCommand),
                        (35,7) => Some(LEEnhancedReceiverTestCommand),
                        (36,0) => Some(LEEnhancedTransmitterTestCommand),
                        (36,1) => Some(LESetAdvertisingSetRandomAddressCommand),
                        (36,2) => Some(LESetExtendedAdvertisingParametersCommand),
                        (36,3) => Some(LESetExtendedAdvertisingDataCommand),
                        (36,4) => Some(LESetExtendedScanResponseDataCommand),
                        (36,5) => Some(LESetExtendedAdvertisingEnableCommand),
                        (36,6) => Some(LEReadMaximumAdvertisingDataLengthCommand),
                        (36,7) => Some(LEReadNumberOfSupportedAdvertisingSetCommand),
                        (37,0) => Some(LERemoveAdvertisingSetCommand),
                        (37,1) => Some(LEClearAdvertisingSetsCommand),
                        (37,2) => Some(LESetPeriodicAdvertisingParametersCommand),
                        (37,3) => Some(LESetPeriodicAdvertisingDataCommand),
                        (37,4) => Some(LESetPeriodicAdvertisingEnableCommand),
                        (37,5) => Some(LESetExtendedScanParametersCommand),
                        (37,6) => Some(LESetExtendedScanEnableCommand),
                        (37,7) => Some(LEExtendedCreateConnectionCommand),
                        (38,0) => Some(LEPeriodicAdvertisingCreateSyncCommand),
                        (38,1) => Some(LEPeriodicAdvertisingCreateSyncCancelCommand),
                        (38,2) => Some(LEPeriodicAdvertisingTerminateSyncCommand),
                        (38,3) => Some(LEAddDeviceToPeriodicAdvertiserListCommand),
                        (38,4) => Some(LERemoveDeviceFromPeriodicAdvertiserListCommand),
                        (38,5) => Some(LEClearPeriodicAdvertiserListCommand),
                        (38,6) => Some(LEReadPeriodicAdvertiserListSizeCommand),
                        (38,7) => Some(LEReadTransmitPowerCommand),
                        (39,0) => Some(LEReadRFPathCompensationCommand),
                        (39,1) => Some(LEWriteRFPathCompensationCommand),
                        (39,2) => Some(LESetPrivacyMode),
                        _      => None
                    }
                }

                // TODO re-make this private
                pub(crate) fn try_from( packed: CmdReturn ) -> Result<alloc::vec::Vec<Self>, error::Error> {

                    let status = error::Error::from(packed.status);

                    if let error::Error::NoError = status {

                        let mut sup_commands =alloc::vec::Vec::new();

                        let raw = &packed.supported_commands;

                        for indx in 0..raw.len() {
                            for bit in 0..8 {
                                if 0 != raw[indx] & (1 << bit) {
                                    if let Some(command) = Self::from_bit_pos((indx,bit)) {
                                        sup_commands.push(command);
                                    }
                                }
                            }
                        }

                        Ok(sup_commands)
                    }
                    else {
                        Err(status)
                    }
                }
            }

            impl_get_data_for_command!(
                COMMAND,
                CmdReturn,
                SupportedCommands,
               alloc::vec::Vec<SupportedCommands>,
                error::Error
            );

            impl_command_data_future!(SupportedCommands,alloc::vec::Vec<SupportedCommands>, error::Error);

            #[derive(Clone,Copy)]
            struct Parameter;

            impl CommandParameter for Parameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {*self}
            }

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T> )
            -> impl Future<Output=Result<alloc::vec::Vec<SupportedCommands>, impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(Parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }


        }
    }

    pub mod transmitter {
        pub mod read_advertising_channel_tx_power {

            use crate::hci::*;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::ReadAdvertisingChannelTxPower);

            #[repr(packed)]
            pub(crate) struct CmdReturn {
                status: u8,
                tx_power_level: i8
            }

            /// The LE Read Advertising Channel Tx Power Command returns dBm, a unit of power
            /// provided to the radio antenna.
            #[derive(Debug)]
            pub struct TxPower(i8);

            impl TxPower {

                fn try_from(packed: CmdReturn) -> Result<Self, error::Error> {
                    let status = error::Error::from(packed.status);

                    if let error::Error::NoError = status {
                        Ok(TxPower(packed.tx_power_level))
                    }
                    else {
                        Err(status)
                    }
                }

                pub fn into_milli_watts(&self) -> f32 {
                    use core::f32;
                    10f32.powf( self.0 as f32 / 10f32 )
                }
            }

            impl_get_data_for_command!(
                COMMAND,
                CmdReturn,
                TxPower,
                error::Error
            );

            impl_command_data_future!(TxPower, error::Error);

            #[derive(Clone,Copy)]
            struct Parameter;

            impl CommandParameter for Parameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {*self}
            }

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T> )
            -> impl Future<Output=Result<TxPower, impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(Parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }

        pub mod transmitter_test{

            use crate::hci::*;
            use crate::hci::le::common::Frequency;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::TransmitterTest);

            #[repr(packed)]
            #[derive( Clone, Copy)]
            struct CmdParameter {
                _tx_channel: u8,
                _lenght_of_test_data: u8,
                _packet_payload: u8,
            }

            #[cfg_attr(test,derive(Debug))]
            pub enum TestPayload {
                PRBS9Sequence,
                Repeat11110000,
                Repeat10101010,
                PRBS15Sequence,
                Repeat11111111,
                Repeat00000000,
                Repeat00001111,
                Repeat01010101,
            }

            impl TestPayload {
                fn into_val(&self) -> u8 {
                    use self::TestPayload::*;
                    match *self {
                        PRBS9Sequence  => 0x00u8,
                        Repeat11110000 => 0x01u8,
                        Repeat10101010 => 0x02u8,
                        PRBS15Sequence => 0x03u8,
                        Repeat11111111 => 0x04u8,
                        Repeat00000000 => 0x05u8,
                        Repeat00001111 => 0x06u8,
                        Repeat01010101 => 0x07u8,
                    }
                }
            }

            impl_status_return!(COMMAND);

            impl CommandParameter for CmdParameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {*self}
            }

            pub fn send<'a, T: 'static>(
                hci: &'a HostInterface<T>,
                channel: Frequency,
                payload: TestPayload,
                payload_length: u8 )
            -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {

                let parameters = CmdParameter {
                    _tx_channel: channel.get_val(),
                    _lenght_of_test_data: payload_length,
                    _packet_payload: payload.into_val(),
                };

                ReturnedFuture( hci.send_command(parameters, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }

        pub mod set_advertising_data {

            use crate::hci::*;
            use crate::gap::advertise::{IntoRaw,DataTooLargeError};

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::SetAdvertisingData);

            type Payload = [u8;31];

            #[repr(packed)]
            #[doc(hidden)]
            pub struct CmdParameter {
                _length: u8,
                _data: [u8;31],
            }

            /// Advertising data
            ///
            /// The Adevertising data is made up of AD Structs. The maximum amount of bytes a
            /// regular advertising broadcast can send is 30 bytes (look at extended
            /// advertising for a larger payload). The total payload is 1 byte for the length,
            /// and 30 bytes for the AD structures. The data can consist of as many AD structs
            /// that can fit in it, but it must consist of at least one AD struct (unless
            /// early termination is desired).
            #[derive(Debug,Clone,Copy)]
            pub struct AdvertisingData {
                length: usize,
                payload: Payload,
            }

            impl AdvertisingData {

                /// Create an empty advertising data
                ///
                /// This is exactly the same as the function early_terminate, but makes more
                /// "readable" sense to use this in conjuntion with try_push.
                #[inline]
                pub fn new() -> Self {
                    Self::early_terminate()
                }

                /// Ealy termination of advertising
                ///
                /// This can also be use to build AdvertisingData object from an "empty" state,
                /// but it is recommended to use the try_from method.
                ///
                /// ```rust
                /// use bo_tie_linux::hci::le::transmitter::command::set_advertising_data::{ADStruct,AdvertisingData};
                ///
                /// // try to use the try_from method instead of doing it this way.
                /// let mut ad = AdvertisingData::early_terminate();
                ///
                /// ad.try_push( ADStruct {ad_type: 0x01u8, data: &[0x00u8]} ).unwrap();
                /// ```
                pub fn early_terminate() -> Self {
                    AdvertisingData{
                        length: 0,
                        payload: Payload::default(),
                    }
                }

                /// Add an ADStruct to the advertising data
                ///
                /// Returns self if the data was added to the advertising data
                ///
                /// # Error
                /// 'data' in its transmission form was too large for remaining free space in
                /// the advertising data.
                pub fn try_push<T>(&mut self, data: T )
                    -> Result<(), DataTooLargeError>
                    where T: IntoRaw
                {
                    let raw_data = data.into_raw();

                    if raw_data.len() + self.length <= self.payload.len() {
                        let old_len = self.length;

                        self.length += raw_data.len();

                        self.payload[old_len..self.length].copy_from_slice(&raw_data);

                        Ok(())
                    }
                    else {
                        Err(DataTooLargeError {
                            overflow: raw_data.len() + self.length - self.payload.len(),
                            remaining: self.payload.len() - self.length,
                        })
                    }
                }

                /// Get the remaining amount of space available for ADStructures
                ///
                /// Use this to get the remaining space that can be sent in an advertising
                /// packet.
                pub fn remaining_space(&self) -> usize {
                    self.payload.len() - self.length as usize
                }
            }

            impl CommandParameter for AdvertisingData {
                type Parameter = CmdParameter;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {
                    CmdParameter {
                        _length: self.length as u8,
                        _data: self.payload
                    }
                }
            }

            impl_status_return!(COMMAND);

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, adv_data: AdvertisingData )
            -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(adv_data, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }

        pub mod set_advertising_enable {

            use crate::hci::*;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::SetAdvertisingEnable);

            impl_status_return!(COMMAND);

            #[derive(Clone,Copy)]
            struct Parameter{
                enable: bool
            }

            impl CommandParameter for Parameter {
                type Parameter = u8;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {
                    if self.enable { 1u8 } else { 0u8 }
                }
            }

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, enable: bool ) -> impl Future<Output=Result<(), impl Display + Debug>> + 'a where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(Parameter{ enable: enable }, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }

        pub mod set_advertising_parameters {

            use crate::hci::*;
            use crate::hci::le::common::OwnAddressType;
            use core::default::Default;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::SetAdvertisingParameters);

            interval!( AdvertisingInterval, 0x0020, 0x4000, SpecDef, 0x0800, 625);

            /// Advertising Type
            ///
            /// Enumeration for the 'Advertising Type' advertising parameter.
            #[cfg_attr(test,derive(Debug))]
            pub enum AdvertisingType {
                ConnectableAndScannableUndirectedAdvertising,
                ConnectableHighDucyCycleDirectedAdvertising,
                ScannableUndirectedAdvertising,
                NonConnectableUndirectedAdvertising,
                ConnectableLowDutyCycleDirectedAdvertising,
            }

            impl AdvertisingType {

                fn into_val(&self) -> u8 {
                    match *self {
                        AdvertisingType::ConnectableAndScannableUndirectedAdvertising => 0x00,
                        AdvertisingType::ConnectableHighDucyCycleDirectedAdvertising => 0x01,
                        AdvertisingType::ScannableUndirectedAdvertising => 0x02,
                        AdvertisingType::NonConnectableUndirectedAdvertising => 0x03,
                        AdvertisingType::ConnectableLowDutyCycleDirectedAdvertising => 0x04,
                    }
                }
            }

            impl Default for AdvertisingType {
                fn default() -> Self {
                    AdvertisingType::ConnectableAndScannableUndirectedAdvertising
                }
            }

            /// Peer address type
            ///
            /// # Notes (from core 5.0 specification)
            /// - PublicAddress -> Public Device Address (default) or Public Identity Address
            /// - RandomAddress -> Random Device Address or Random (static) Identity Address
            #[cfg_attr(test,derive(Debug))]
            pub enum PeerAddressType {
                PublicAddress,
                RandomAddress,
            }

            impl PeerAddressType {
                fn into_val(&self) -> u8 {
                    match *self {
                        PeerAddressType::PublicAddress => 0x00,
                        PeerAddressType::RandomAddress => 0x01,
                    }
                }
            }

            impl Default for PeerAddressType {
                fn default() -> Self {
                    PeerAddressType::PublicAddress
                }
            }

            /// Advertising channels
            #[cfg_attr(test,derive(Debug))]
            pub enum AdvertisingChannel {
                Channel37,
                Channel38,
                Channel39,
            }

            impl AdvertisingChannel {
                fn into_val(&self) -> u8 {
                    match *self {
                        AdvertisingChannel::Channel37 => 0x01,
                        AdvertisingChannel::Channel38 => 0x02,
                        AdvertisingChannel::Channel39 => 0x04,
                    }
                }

                pub fn default_channels() -> &'static [AdvertisingChannel] {
                    &[
                        AdvertisingChannel::Channel37,
                        AdvertisingChannel::Channel38,
                        AdvertisingChannel::Channel39,
                    ]
                }
            }

            #[cfg_attr(test,derive(Debug))]
            pub enum AdvertisingFilterPolicy {
                AllDevices,
                AllConnectionRequestsWhitlistedDeviceScanRequests,
                AllScanRequestsWhitlistedDeviceConnectionRequests,
                WhitelistedDevices,
            }

            impl AdvertisingFilterPolicy {
                fn into_val(&self) -> u8 {
                    match *self {
                        AdvertisingFilterPolicy::AllDevices => 0x00,
                        AdvertisingFilterPolicy::AllConnectionRequestsWhitlistedDeviceScanRequests => 0x01,
                        AdvertisingFilterPolicy::AllScanRequestsWhitlistedDeviceConnectionRequests => 0x02,
                        AdvertisingFilterPolicy::WhitelistedDevices => 0x03,
                    }
                }
            }

            impl Default for AdvertisingFilterPolicy {
                fn default() -> Self {
                    AdvertisingFilterPolicy::AllDevices
                }
            }

            /// All the parameters required for advertising
            ///
            /// For the advertising_channel_map, provide a slice containing every channels
            /// desired to be advertised on.
            ///
            /// While most members are public, the only way to set the minimum and maximum
            /// advertising interval is through method calls.
            #[cfg_attr(test,derive(Debug))]
            pub struct AdvertisingParameters<'a> {
                pub minimum_advertising_interval: AdvertisingInterval,
                pub maximum_advertising_interval: AdvertisingInterval,
                pub advertising_type: AdvertisingType,
                pub own_address_type: OwnAddressType,
                pub peer_address_type: PeerAddressType,
                pub peer_address: crate::BluetoothDeviceAddress,
                pub advertising_channel_map: &'a[AdvertisingChannel],
                pub advertising_filter_policy: AdvertisingFilterPolicy,
            }

            impl<'a> Default for AdvertisingParameters<'a> {

                /// Create an AdvertisingParameters object with the default parameters (except
                /// for the peer_address member).
                ///
                /// The default parameter values are from the bluetooth core 5.0 specification,
                /// however there is no default value for the peer_address. This function sets
                /// the peer_address to zero, so it must be set after if a connection to a
                /// specific peer device is desired.
                fn default() -> Self {
                    AdvertisingParameters {
                        minimum_advertising_interval: AdvertisingInterval::default(),
                        maximum_advertising_interval: AdvertisingInterval::default(),
                        advertising_type: AdvertisingType::default(),
                        own_address_type: OwnAddressType::default(),
                        peer_address_type: PeerAddressType::default(),
                        peer_address: [0u8;6].into(),
                        advertising_channel_map: AdvertisingChannel::default_channels(),
                        advertising_filter_policy: AdvertisingFilterPolicy::default(),
                    }
                }
            }

            impl<'a> AdvertisingParameters<'a> {

                /// Create the default parameters except use the specified bluetooth device
                /// address for the peer_address member
                pub fn default_with_peer_address( addr: &'a crate::BluetoothDeviceAddress) ->
                        AdvertisingParameters
                {
                    let mut ap = AdvertisingParameters::default();

                    ap.peer_address = *addr;

                    ap
                }
            }

            #[repr(packed)]
            #[derive( Clone, Copy)]
            struct CmdParameter {
                _advertising_interval_min: u16,
                _advertising_interval_max: u16,
                _advertising_type: u8,
                _own_address_type: u8,
                _peer_address_type: u8,
                _peer_address: crate::BluetoothDeviceAddress,
                _advertising_channel_map: u8,
                _advertising_filter_policy: u8,
            }

            impl CommandParameter for CmdParameter{
                type Parameter = CmdParameter;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter { *self }
            }

            impl_status_return!(COMMAND);

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, params: AdvertisingParameters )
            -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {

                let parameter = CmdParameter {

                    _advertising_interval_min: params.minimum_advertising_interval.get_raw_val(),

                    _advertising_interval_max: params.maximum_advertising_interval.get_raw_val(),

                    _advertising_type: params.advertising_type.into_val(),

                    _own_address_type: params.own_address_type.into_val(),

                    _peer_address_type: params.peer_address_type.into_val(),

                    _peer_address: params.peer_address.into(),

                    _advertising_channel_map: params.advertising_channel_map.iter().fold(0u8, |v, x| v | x.into_val()),

                    _advertising_filter_policy: params.advertising_filter_policy.into_val(),
                };

                ReturnedFuture( hci.send_command(parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }
        }

        pub mod set_random_address {

            use crate::hci::*;


            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::SetAdvertisingParameters);

            impl_status_return!(COMMAND);

            struct Parameter {
                rand_address: crate::BluetoothDeviceAddress
            }

            impl CommandParameter for Parameter {
                type Parameter = crate::BluetoothDeviceAddress;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {
                    self.rand_address
                }
            }

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, rand_addr: crate::BluetoothDeviceAddress )
            -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(Parameter{ rand_address: rand_addr }, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }
    }

    pub mod receiver {
        pub mod receiver_test {

            use crate::hci::*;
            use crate::hci::le::common::Frequency;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::ReceiverTest);

            impl_status_return!(COMMAND);

            impl CommandParameter for Frequency
            {
                type Parameter = u8;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {
                    self.get_val()
                }
            }

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, frequency: Frequency )
            -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(frequency, events::Events::CommandComplete , Duration::from_secs(1) ) )
            }

        }

        pub mod set_scan_enable {

            use crate::hci::*;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::SetScanEnable);

            impl_status_return!(COMMAND);

            #[repr(packed)]
            struct CmdParameter {
                _enable: u8,
                _filter_duplicates: u8,
            }

            struct Parameter {
                enable: bool,
                filter_duplicates: bool,
            }

            impl CommandParameter for Parameter {
                type Parameter = CmdParameter;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {
                    CmdParameter {
                        _enable: if self.enable {1} else {0},
                        _filter_duplicates: if self.filter_duplicates {1} else {0},
                    }
                }
            }

            /// The command has the ability to enable/disable scanning and filter duplicate
            /// advertisement.
            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, enable: bool, filter_duplicates: bool)
            -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                let cmd_param = Parameter {
                    enable: enable,
                    filter_duplicates: filter_duplicates,
                };

                ReturnedFuture( hci.send_command(cmd_param, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }

        pub mod set_scan_parameters {

            use crate::hci::*;
            use crate::hci::le::common::OwnAddressType;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::SetScanParameters);

            interval!( ScanningInterval, 0x0004, 0x4000, SpecDef, 0x0010, 625);
            interval!( ScanningWindow, 0x0004, 0x4000, SpecDef, 0x0010, 625);

            pub enum LEScanType {
                /// Under passive scanning, the link layer will not respond to any advertising
                /// packets. This is usefull when listening to a device in the broadcast role.
                PassiveScanning,
                /// With Active scanning, the link layer will send packets to the advertisier. These
                /// packets can be for quering for more data.
                ActiveScanning,
            }

            impl LEScanType {
                fn into_val(&self) -> u8 {
                    match *self {
                        LEScanType::PassiveScanning => 0x00,
                        LEScanType::ActiveScanning  => 0x01,
                    }
                }
            }

            impl Default for LEScanType {
                fn default() -> Self {
                    LEScanType::PassiveScanning
                }
            }

            /// See the spec on this one (v5.0 | Vol 2, Part E, 7.8.10) to understand what
            /// the enumerations are representing.
            ///
            /// Value mapping
            /// 0x00 => AcceptAll
            /// 0x01 => WhiteListed
            /// 0x02 => AcceptAllExceptIdentityNotAddressed
            /// 0x03 => AcceptAllExceptIdentityNotInWhitelist
            pub enum ScanningFilterPolicy {
                AcceptAll,
                WhiteListed,
                AcceptAllExceptIdentityNotAddressed,
                AcceptAllExceptIdentityNotInWhitelist,
            }

            impl ScanningFilterPolicy {
                fn into_val(&self) -> u8 {
                    match *self {
                        ScanningFilterPolicy::AcceptAll => 0x00,
                        ScanningFilterPolicy::WhiteListed => 0x01,
                        ScanningFilterPolicy::AcceptAllExceptIdentityNotAddressed => 0x02,
                        ScanningFilterPolicy::AcceptAllExceptIdentityNotInWhitelist => 0x03,
                    }
                }
            }

            impl Default for ScanningFilterPolicy {
                fn default() -> Self {
                    ScanningFilterPolicy::AcceptAll
                }
            }

            pub struct ScanningParameters {
                pub scan_type: LEScanType,
                pub scan_interval: ScanningInterval,
                pub scan_window: ScanningWindow,
                pub own_address_type: OwnAddressType,
                pub scanning_filter_policy: ScanningFilterPolicy,
            }

            impl Default for ScanningParameters {
                fn default() -> Self {
                    ScanningParameters {
                        scan_type: LEScanType::default(),
                        scan_interval: ScanningInterval::default(),
                        scan_window: ScanningWindow::default(),
                        own_address_type: OwnAddressType::default(),
                        scanning_filter_policy: ScanningFilterPolicy::default(),
                    }
                }
            }

            impl_status_return!(COMMAND);

            #[repr(packed)]
            #[doc(hidden)]
            pub struct CmdParameter {
                _scan_type: u8,
                _scan_interval: u16,
                _scan_window: u16,
                _own_address_type: u8,
                _filter_policy: u8,
            }

            impl CommandParameter for ScanningParameters {
                type Parameter = CmdParameter;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {
                    CmdParameter {
                        _scan_type:        self.scan_type.into_val(),
                        _scan_interval:    self.scan_interval.get_raw_val(),
                        _scan_window:      self.scan_window.get_raw_val(),
                        _own_address_type: self.own_address_type.into_val(),
                        _filter_policy:    self.scanning_filter_policy.into_val(),
                    }
                }
            }

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, sp: ScanningParameters )
            -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(sp, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }
    }

    pub mod connection {

        pub struct ConnectionEventLength {
            minimum: u16,
            maximum: u16,
        }

        impl ConnectionEventLength {
            pub fn new(min: u16, max: u16) -> Self {
                Self {
                    minimum: min,
                    maximum: max
                }
            }
        }

        impl ::core::default::Default for ConnectionEventLength {
            fn default() -> Self {
                Self {
                    minimum: 0,
                    maximum: 0xFFFF,
                }
            }
        }

        interval!( #[derive(Clone, Copy)] ConnectionInterval, 0x0006, 0x0C80, ApiDef, 0x0006, 1250);

        /// ConnectionUpdateInterval contaings the minimum and maximum connection intervals for
        /// the le connection update
        pub struct ConnectionIntervalBounds {
            min: ConnectionInterval,
            max: ConnectionInterval,
        }

        impl ConnectionIntervalBounds {
            /// Create a ConnectionUpdateInterval
            ///
            /// # Errors
            /// An error is returned if the minimum is greater then the maximum
            pub fn try_from(min: ConnectionInterval, max: ConnectionInterval)
                -> Result<Self,&'static str>
            {
                if min.get_raw_val() <= max.get_raw_val() {
                    Ok( Self {
                        min: min,
                        max: max,
                    })
                }
                else {
                    Err("'min' is greater than 'max'")
                }
            }
        }

        // TODO when BR/EDR is enabled move this to a module for common features and import here
        pub mod disconnect {
            use crate::hci::*;
            use crate::hci::common::ConnectionHandle;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LinkControl(opcodes::LinkControl::Disconnect);

            /// These are the error codes that are given as reasons for disconnecting
            ///
            /// These enumerations are the acceptable error codes to be used as reasons for
            /// triggering the disconnect.
            pub enum DisconnectReason {
                AuthenticationFailure,
                RemoteUserTerminatedConnection,
                RemoteDeviceTerminatedConnectionDueToLowResources,
                RemoteDeviceTerminatedConnectionDueToPowerOff,
                UnsupportedRemoteFeature,
                PairingWithUnitKeyNotSupported,
                UnacceptableConnectionParameters,
            }

            impl DisconnectReason {

                // TODO implement when HCI error codes are added, and add parameter for the
                // error enumeraton name
                pub fn try_from_hci_error( error: error::Error ) -> Result<DisconnectReason, &'static str> {
                    match error {
                        error::Error::AuthenticationFailure => {
                            Ok(DisconnectReason::AuthenticationFailure)
                        }
                        error::Error::RemoteUserTerminatedConnection => {
                            Ok(DisconnectReason::RemoteUserTerminatedConnection)
                        }
                        error::Error::RemoteDeviceTerminatedConnectionDueToLowResources => {
                            Ok(DisconnectReason::RemoteDeviceTerminatedConnectionDueToLowResources)
                        }
                        error::Error::RemoteDeviceTerminatedConnectionDueToPowerOff => {
                            Ok(DisconnectReason::RemoteDeviceTerminatedConnectionDueToPowerOff)
                        }
                        error::Error::UnsupportedRemoteFeatureOrUnsupportedLMPFeature => {
                            Ok(DisconnectReason::UnsupportedRemoteFeature)
                        }
                        error::Error::PairingWithUnitKeyNotSupported => {
                            Ok(DisconnectReason::PairingWithUnitKeyNotSupported)
                        }
                        error::Error::UnacceptableConnectionParameters => {
                            Ok(DisconnectReason::UnacceptableConnectionParameters)
                        }
                        _ => {
                            Err("No Disconnect reason for error")
                        }
                    }
                }

                fn get_val(&self) -> u8 {
                    match *self {
                        DisconnectReason::AuthenticationFailure => 0x05,
                        DisconnectReason::RemoteUserTerminatedConnection => 0x13,
                        DisconnectReason::RemoteDeviceTerminatedConnectionDueToLowResources => 0x14,
                        DisconnectReason::RemoteDeviceTerminatedConnectionDueToPowerOff => 0x15,
                        DisconnectReason::UnsupportedRemoteFeature => 0x1A,
                        DisconnectReason::PairingWithUnitKeyNotSupported => 0x29,
                        DisconnectReason::UnacceptableConnectionParameters => 0x3B,
                    }
                }
            }

            #[repr(packed)]
            #[doc(hidden)]
            pub struct CmdParameter {
                _handle: u16,
                _reason: u8,
            }

            pub struct DisconnectParameters {
                pub connection_handle: ConnectionHandle,
                pub disconnect_reason: DisconnectReason,
            }

            impl CommandParameter for DisconnectParameters {
                type Parameter = CmdParameter;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {
                    CmdParameter {
                        _handle: self.connection_handle.get_raw_handle(),
                        _reason: self.disconnect_reason.get_val(),
                    }
                }
            }

            impl_command_status_future!();

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, dp: DisconnectParameters )
            -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(dp, events::Events::CommandStatus, Duration::from_secs(1) ) )
            }

        }

        pub mod connection_update {
            use crate::hci::*;
            use crate::hci::common::{
                ConnectionHandle,
                SupervisionTimeout,
            };
            use super::{ ConnectionEventLength, ConnectionIntervalBounds };

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::ConnectionUpdate);

            #[repr(packed)]
            #[doc(hidden)]
            pub struct CmdParameter {
                _handle: u16,
                _conn_interval_min: u16,
                _conn_interval_max: u16,
                _conn_latency: u16,
                _supervision_timeout: u16,
                _minimum_ce_length: u16,
                _maximum_ce_length: u16,
            }

            pub struct ConnectionUpdate {
                pub handle: ConnectionHandle,
                pub interval: ConnectionIntervalBounds,
                pub latency: u16,
                pub supervision_timeout: SupervisionTimeout,
                pub connection_event_len: ConnectionEventLength,
            }


            impl CommandParameter for ConnectionUpdate {
                type Parameter = CmdParameter;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {
                    CmdParameter {
                        _handle:              self.handle.get_raw_handle(),
                        _conn_interval_min:   self.interval.min.get_raw_val(),
                        _conn_interval_max:   self.interval.max.get_raw_val(),
                        _conn_latency:        self.latency,
                        _supervision_timeout: self.supervision_timeout.get_timeout(),
                        _minimum_ce_length:   self.connection_event_len.minimum,
                        _maximum_ce_length:   self.connection_event_len.maximum,
                    }
                }
            }

            impl_returned_future!(
                crate::hci::events::LEConnectionUpdateCompleteData,
                events::EventsData::LEMeta,
                events::LEMetaData::ConnectionUpdateComplete(data),
                &'static str, // useless type that has both Display + Debug
                {
                    core::task::Poll::Ready(Ok(data))
                }
            );

            /// The event expected to be returned is the LEMeta event carrying a Connection Update
            /// Complete lE event
            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, cu: ConnectionUpdate, timeout: Duration)
            -> impl Future<Output=Result<crate::hci::events::LEConnectionUpdateCompleteData, impl Display + Debug>> + 'a where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command( cu, events::Events::LEMeta( events::LEMeta::ConnectionUpdateComplete ), timeout ) )
            }

        }

        pub mod create_connection_cancel {

            use crate::hci::*;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::CreateConnectionCancel);

            impl_status_return!(COMMAND);

            #[derive(Clone,Copy)]
            struct Parameter;

            impl CommandParameter for Parameter {
                type Parameter = Self;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter { *self }
            }

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>)
            -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command( Parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }

        pub mod create_connection {

            use super::{ConnectionEventLength, ConnectionIntervalBounds};
            use crate::hci::*;
            use crate::hci::common::{
                ConnectionLatency,
                LEAddressType,
                SupervisionTimeout,
            };
            use crate::hci::le::common::OwnAddressType;
            use core::time::Duration;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::CreateConnection);

            interval!(ScanningInterval, 0x0004, 0x4000, SpecDef, 0x0010, 625);
            interval!(ScanningWindow, 0x0004, 0x4000, SpecDef, 0x0010, 625);

            pub enum InitiatorFilterPolicy {
                DoNotUseWhiteList,
                UseWhiteList,
            }

            impl InitiatorFilterPolicy {
                fn into_raw(&self) -> u8 {
                    match *self {
                        InitiatorFilterPolicy::DoNotUseWhiteList => 0x00,
                        InitiatorFilterPolicy::UseWhiteList => 0x01,
                    }
                }
            }

            pub struct ConnectionParameters {
                scan_interval : ScanningInterval,
                scan_window : ScanningWindow,
                initiator_filter_policy: InitiatorFilterPolicy,
                peer_address_type: LEAddressType,
                peer_address: crate::BluetoothDeviceAddress,
                own_address_type: OwnAddressType,
                connection_interval: ConnectionIntervalBounds,
                connection_latency: ConnectionLatency,
                supervision_timeout: SupervisionTimeout,
                connection_event_len: ConnectionEventLength,
            }

            #[repr(packed)]
            #[doc(hidden)]
            pub struct CmdParameter {
                _scan_interval: u16,
                _scan_window: u16,
                _initiator_filter_policy: u8,
                _peer_address_type: u8,
                _peer_address: crate::BluetoothDeviceAddress,
                _own_address_type: u8,
                _conn_interval_min: u16,
                _conn_interval_max: u16,
                _conn_latency: u16,
                _supervision_timeout: u16,
                _minimum_ce_length: u16,
                _maximum_ce_length: u16,
            }

            impl CommandParameter for ConnectionParameters {
                type Parameter = CmdParameter;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {
                    CmdParameter {
                        _scan_interval:           self.scan_interval.get_raw_val(),
                        _scan_window:             self.scan_window.get_raw_val(),
                        _initiator_filter_policy: self.initiator_filter_policy.into_raw(),
                        _peer_address_type:       self.peer_address_type.into_raw(),
                        _peer_address:            self.peer_address,
                        _own_address_type:        self.own_address_type.into_val(),
                        _conn_interval_min:       self.connection_interval.min.get_raw_val(),
                        _conn_interval_max:       self.connection_interval.max.get_raw_val(),
                        _conn_latency:            self.connection_latency.get_latency(),
                        _supervision_timeout:     self.supervision_timeout.get_timeout(),
                        _minimum_ce_length:       self.connection_event_len.minimum,
                        _maximum_ce_length:       self.connection_event_len.maximum,
                    }
                }
            }

            impl ConnectionParameters {

                /// Command Parameters for connecting without the white list
                pub fn new_without_whitelist(
                    scan_interval : ScanningInterval,
                    scan_window : ScanningWindow,
                    peer_address_type: LEAddressType,
                    peer_address: crate::BluetoothDeviceAddress,
                    own_address_type: OwnAddressType,
                    connection_interval: ConnectionIntervalBounds,
                    connection_latency: ConnectionLatency,
                    supervision_timeout: SupervisionTimeout,
                    connection_event_len: ConnectionEventLength,
                ) -> Self {
                    Self {
                        scan_interval : scan_interval,
                        scan_window : scan_window,
                        initiator_filter_policy: InitiatorFilterPolicy::DoNotUseWhiteList,
                        peer_address_type : peer_address_type,
                        peer_address : peer_address,
                        own_address_type : own_address_type,
                        connection_interval : connection_interval,
                        connection_latency : connection_latency,
                        supervision_timeout : supervision_timeout,
                        connection_event_len : connection_event_len,
                    }
                }

                /// Command parameters for connecting with the white list
                pub fn new_with_whitelist(
                    scan_interval : ScanningInterval,
                    scan_window : ScanningWindow,
                    own_address_type: OwnAddressType,
                    connection_interval: ConnectionIntervalBounds,
                    connection_latency: ConnectionLatency,
                    supervision_timeout: SupervisionTimeout,
                    connection_event_len: ConnectionEventLength,
                ) -> Self {
                    Self {
                        scan_interval : scan_interval,
                        scan_window : scan_window,
                        initiator_filter_policy: InitiatorFilterPolicy::UseWhiteList,
                        peer_address_type : LEAddressType::PublicDeviceAddress, // This is not used (see spec)
                        peer_address : [0u8;6], // This is not used (see spec)
                        own_address_type : own_address_type,
                        connection_interval : connection_interval,
                        connection_latency : connection_latency,
                        supervision_timeout : supervision_timeout,
                        connection_event_len : connection_event_len,
                    }
                }

            }

            impl_command_status_future!();

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, cp: ConnectionParameters )
            -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(cp, events::Events::CommandStatus , Duration::from_secs(1) ) )
            }

        }
        pub mod read_channel_map {

            use crate::hci::*;
            use crate::hci::common::ConnectionHandle;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::ReadChannelMap);

            #[repr(packed)]
            pub(crate) struct CmdReturn {
                status: u8,
                connection_handle: u16,
                channel_map: [u8;5]
            }

            pub struct ChannelMapInfo {
                pub handle: ConnectionHandle,
                /// This is the list of channels (from 0 through 36)
                pub channel_map: ::alloc::boxed::Box<[usize]>,
            }

            impl ChannelMapInfo {
                fn try_from(packed: CmdReturn) -> Result<Self, error::Error> {
                    let status = error::Error::from(packed.status);

                    if let error::Error::NoError = status {

                        // 37 is the number of channels (as of bluetooth 5.0)
                        let channel_count = 37;

                        let mut count = 0;

                        let mut mapped_channels =alloc::vec::Vec::with_capacity(channel_count);

                        'outer: for byte in packed.channel_map.iter() {
                            for bit in 0..8 {
                                if count < channel_count {
                                    if 0 != (byte & (1 << bit)) {
                                        mapped_channels.push(count);
                                        count += 1;
                                    }
                                }
                                else {
                                    break 'outer;
                                }
                            }
                        }

                        Ok( Self {
                            handle: ConnectionHandle::try_from(packed.connection_handle).unwrap(),
                            channel_map: mapped_channels.into_boxed_slice(),
                        })
                    }
                    else {
                        Err(status)
                    }
                }
            }

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

            impl_get_data_for_command!(
                COMMAND,
                CmdReturn,
                ChannelMapInfo,
                error::Error
            );

            impl_command_data_future!(ChannelMapInfo, error::Error);

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, handle: ConnectionHandle )
            -> impl Future<Output=Result<ChannelMapInfo, impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {

                let parameter = CmdParameter {
                    _connection_handle: handle.get_raw_handle()
                };

                ReturnedFuture( hci.send_command(parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }

        }

        pub mod read_remote_features {

            use crate::hci::*;
            use crate::hci::common::ConnectionHandle;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::ReadRemoteFeatures);

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

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, handle: ConnectionHandle )
            -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {

                let parameter = CmdParameter {
                    _connection_handle: handle.get_raw_handle(),
                };

                ReturnedFuture( hci.send_command(parameter, events::Events::CommandStatus, Duration::from_secs(1) ) )
            }

        }

        pub mod set_host_channel_classification {
            use crate::hci::*;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::LEController(opcodes::LEController::SetHostChannelClassification);

            #[repr(packed)]
            #[doc(hidden)]
            pub struct CmdParemeter {
                _channel_map: [u8;5]
            }

            const CHANNEL_MAP_MAX: usize = 37;

            pub struct ChannelMap {
                channels: [bool;CHANNEL_MAP_MAX]
            }

            impl ChannelMap {
                pub const MAX: usize = 37;

                /// try to create a Channel Map
                ///
                /// This will form a channel map so long as every value in slice referenced by
                /// channels is less then CHANNEL_MAP_MAX
                ///
                /// # Error
                /// A value in the parameter was found to be larger then CHANNEL_MAP_MAX
                pub fn try_from<'a>(channels: &'a[usize]) -> Result<Self, usize> {

                    let mut channel_flags = [false;CHANNEL_MAP_MAX];

                    for val in channels {
                        if *val < CHANNEL_MAP_MAX {
                            channel_flags[*val] = true;
                        }
                        else {
                            return Err(*val);
                        }
                    }

                    Ok( Self {
                        channels: channel_flags
                    })
                }
            }

            impl CommandParameter for ChannelMap {
                type Parameter = CmdParemeter;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {

                    let mut raw = [0u8;5];

                    for val in 0..CHANNEL_MAP_MAX {
                        if self.channels[val] {
                            raw[val / 8] |= 1 << (val % 8)
                        }
                    }

                    CmdParemeter {
                        _channel_map : raw
                    }
                }
            }

            impl_status_return!(COMMAND);

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, map: ChannelMap )
            -> impl Future<Output=Result<(), impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command( map, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }
        }

        // TODO when BR/EDR is enabled move this to a module for common features and import here
        pub mod read_transmit_power_level {
            use crate::hci::*;
            use crate::hci::common::ConnectionHandle;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::ControllerAndBaseband(opcodes::ControllerAndBaseband::ReadTransmitPowerLevel);

            #[repr(packed)]
            #[doc(hidden)]
            pub struct CmdParameter {
                _connection_handle: u16,
                _level_type: u8,
            }

            #[repr(packed)]
            struct CmdReturn {
                status: u8,
                connection_handle: u16,
                power_level: i8,
            }

            /// Transmit power range (from minimum to maximum levels)
            pub struct TransmitPowerLevel {
                pub connection_handle: ConnectionHandle,
                pub power_level: i8,
            }

            impl TransmitPowerLevel {

                fn try_from(packed: CmdReturn) -> Result<Self, error::Error> {
                    let status = error::Error::from(packed.status);

                    if let error::Error::NoError = status {
                        Ok(Self {
                            // If this panics here the controller returned a bad connection handle
                            connection_handle: ConnectionHandle::try_from(packed.connection_handle).unwrap(),
                            power_level: packed.power_level,
                        })
                    }
                    else {
                        Err(status)
                    }
                }
            }

            impl_get_data_for_command!(
                COMMAND,
                CmdReturn,
                TransmitPowerLevel,
                error::Error
            );

            impl_command_data_future!(TransmitPowerLevel, error::Error);

            pub enum TransmitPowerLevelType {
                CurrentPowerLevel,
                MaximumPowerLevel,
            }

            pub struct Parameter {
                pub connection_handle: ConnectionHandle,
                pub level_type: TransmitPowerLevelType,
            }

            impl CommandParameter for Parameter {
                type Parameter = CmdParameter;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter {
                    CmdParameter {
                        _connection_handle: self.connection_handle.get_raw_handle(),
                        _level_type: match self.level_type {
                            TransmitPowerLevelType::CurrentPowerLevel => 0,
                            TransmitPowerLevelType::MaximumPowerLevel => 1,
                        }
                    }
                }
            }

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, parameter: Parameter )
            -> impl Future<Output=Result<TransmitPowerLevel, impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                ReturnedFuture( hci.send_command(parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }
        }

        // TODO when BR/EDR is enabled move this to a module for common features and import here
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

        // TODO when BR/EDR is enabled move this to a module for common features and import here
        pub mod read_rssi {

            use crate::hci::*;
            use crate::hci::common::ConnectionHandle;

            const COMMAND: opcodes::HCICommand = opcodes::HCICommand::StatusParameters(opcodes::StatusParameters::ReadRSSI);

            #[repr(packed)]
            pub(crate) struct CmdReturn {
                status: u8,
                handle: u16,
                rssi: i8
            }

            struct Parameter {
                handle: u16
            }

            impl CommandParameter for Parameter {
                type Parameter = u16;
                const COMMAND: opcodes::HCICommand = COMMAND;
                fn get_parameter(&self) -> Self::Parameter { self.handle }
            }

            pub struct RSSIInfo {
                pub handle: ConnectionHandle,
                pub rssi: i8
            }

            impl RSSIInfo {
                fn try_from(packed: CmdReturn) -> Result<Self, error::Error > {
                    let status = error::Error::from(packed.status);

                    if let error::Error::NoError = status {
                        Ok( Self {
                            handle: ConnectionHandle::try_from(packed.handle).unwrap(),
                            rssi: packed.rssi
                        })
                    }
                    else {
                        Err(status)
                    }
                }
            }

            impl_get_data_for_command!(
                COMMAND,
                CmdReturn,
                RSSIInfo,
                error::Error
            );

            impl_command_data_future!(RSSIInfo, error::Error);

            pub fn send<'a, T: 'static>( hci: &'a HostInterface<T>, handle: ConnectionHandle )
            -> impl Future<Output=Result<RSSIInfo, impl Display + Debug>> + 'a
            where T: HostControllerInterface
            {
                let parameter = Parameter {
                    handle: handle.get_raw_handle()
                };

                ReturnedFuture( hci.send_command(parameter, events::Events::CommandComplete, Duration::from_secs(1) ) )
            }
        }
    }
//
//     pub mod br_edr {
//         // TODO does this module make sense?
//         pub mod support {
//             pub fn read_le_host() { unimplemented!() }
//             pub fn write_le_host() { unimplemented!() }
//         }
//         pub mod command {
//             pub fn read_buffer_size() { unimplemented!() }
//         }
//     }
//
//     pub mod scannable {
//         pub mod command {
//             pub fn set_scan_response_data() { unimplemented!() }
//         }
//     }
//
//     pub mod encryption {
//         pub mod event {
//             pub fn encryption_change() { unimplemented!() }
//             pub fn encryption_key_refresh_complete() { unimplemented!() }
//             pub fn long_term_key_request() { unimplemented!() }
//         }
//         pub mod command {
//             pub fn encrypt() { unimplemented!() }
//             pub fn long_term_key_request_reply() { unimplemented!() }
//             pub fn long_term_key_request_negative_reply() { unimplemented!() }
//             pub fn rand() {unimplemented!()}
//             pub fn start_encryption() { unimplemented!() }
//         }
//     }
//
//     pub mod connection_parameters_request_procedure {
//         pub mod event {
//             pub fn remote_connection_paramter_request() { unimplemented!() }
//         }
//         pub mod command {
//             pub fn remote_connection_parameter_request_reply() { unimplemented!() }
//             pub fn remote_connection_parameter_request_negative_reply() { unimplemented!() }
//         }
//     }
//
//     pub mod ping {
//         pub mod event {
//             pub fn authenticated_payload_timeout_expired() { unimplemented!() }
//         }
//         pub mod command {
//             pub fn write_authenticated_payload_timeout() { unimplemented!() }
//             pub fn read_authenticated_payload_timeout() { unimplemented!() }
//             pub fn set_event_mask_page_2() { unimplemented!() }
//         }
//     }
//
//     pub mod data_packet_length_extension {
//         pub mod event {
//             pub fn data_length_change() { unimplemented!() }
//         }
//         pub mod command {
//             pub fn set_data_length() { unimplemented!() }
//             pub fn read_suggested_default_data_length() { unimplemented!() }
//             pub fn write_suggested_default_data_length() { unimplemented!() }
//         }
//     }
//
//     pub mod privacy {
//         pub mod event {
//             pub fn directed_advertising_report() { unimplemented!() }
//         }
//         pub mod command {
//             pub fn set_resolvable_private_address_timeout() { unimplemented!() }
//             pub fn set_address_resolution_enable() { unimplemented!() }
//             pub fn add_device_to_resolving_list() { unimplemented!() }
//             pub fn clear_resolving_list() { unimplemented!() }
//             pub fn set_privacy_mode() { unimplemented!() }
//             pub fn read_peer_resolvable_address() { unimplemented!() }
//             pub fn read_local_resolvable_address() { unimplemented!() }
//         }
//     }
//
//     pub mod phy_2m_or_coded {
//         pub mod event {
//             pub fn phy_update_complete() { unimplemented!() }
//         }
//         pub mod command {
//             pub fn read_phy() { unimplemented!() }
//             pub fn set_default_phy() { unimplemented!() }
//             pub fn set_phy() { unimplemented!() }
//             pub fn enhanced_transmitter_test() { unimplemented!() }
//             pub fn enhanced_receiver_test() { unimplemented!() }
//         }
//     }
//
//     pub mod extended_advertising {
//         pub mod event {
//             pub fn scan_request_received() { unimplemented!() }
//             pub fn advertising_set_terminated() { unimplemented!() }
//             pub fn scan_timeout() { unimplemented!() }
//             pub fn extended_advertising_report() { unimplemented!() }
//         }
//         pub mod legacy_event {
//             /// Superseded by extended_advertising_report
//             pub fn advertising_report() { unimplemented!() }
//             /// Superseded by exted_advertising_report
//             pub fn direted_advertising_report() { unimplemented!() }
//         }
//         pub mod command {
//             pub fn set_advertising_set_random_address() { unimplemented!() }
//             pub fn set_extended_advertising_parameters() { unimplemented!() }
//             pub fn set_extended_advertising_data() { unimplemented!() }
//             pub fn set_extended_scan_response_data() { unimplemented!() }
//             pub fn set_extended_advertising_enable() { unimplemented!() }
//             pub fn read_maximum_advertising_data_length() { unimplemented!() }
//             pub fn read_number_of_supported_advertising_sets() { unimplemented!() }
//             pub fn remove_advertising_set() { unimplemented!() }
//             pub fn clear_advertisisng_sets() { unimplemented!() }
//             pub fn set_extended_scan_parameters() { unimplemented!() }
//             pub fn set_extended_scan_enable() { unimplemented!() }
//             pub fn extended_create_connection() { unimplemented!() }
//         }
//         pub mod legacy_command {
//             /// Superseded by set_extended_advertising_parameters
//             pub fn set_advertising_parameters() { unimplemented!() }
//             /// No longer used
//             pub fn read_advertising_channel_tx_power() { unimplemented!() }
//             /// Superseded by set_extended_advertising_data
//             pub fn set_advertising_data() { unimplemented!() }
//             /// Superseded by set_extended_advertising_enable
//             pub fn set_scan_parameters() { unimplemented!() }
//             /// Superseded by set_extended_scan_enable
//             pub fn set_scan_enable() { unimplemented!() }
//             /// Superseded by extended_create_connection
//             pub fn extended_create_connection() { unimplemented!() }
//         }
//     }
//
//     pub mod periodic_advertising {
//         pub mod event {
//             pub fn periodic_advertising_report() { unimplemented!() }
//             pub fn periodic_advertising_sync_established() { unimplemented!() }
//             pub fn periodic_advertising_sync_lost() { unimplemented!() }
//         }
//         pub mod command {
//             pub fn set_periodic_advertising_parameters() { unimplemented!() }
//             pub fn set_periodic_advertising_data() { unimplemented!() }
//             pub fn set_periodic_advertising_enable() { unimplemented!() }
//             pub fn periodic_advertising_create_sync() { unimplemented!() }
//             pub fn periodic_advertising_create_sync_cancel() { unimplemented!() }
//             pub fn periodic_advertising_terminate_sync() { unimplemented!() }
//             pub fn add_device_to_periodic_advertising_list() { unimplemented!() }
//             pub fn remove_device_from_periodic_advertiser_list() { unimplemented!() }
//             pub fn clear_periodic_advertiser_list() { unimplemented!() }
//             pub fn read_periodic_advertiser_list_size() { unimplemented!() }
//         }
//     }
//
//     pub mod advertising_of_tx_power {
//         pub mod command {
//             pub fn read_rf_path_compensation() { unimplemented!() }
//             pub fn write_rf_path_compensation() { unimplemented!() }
//         }
//     }
//
//     pub mod channel_selection_algorithm_2 {
//         pub mod event {
//             pub fn chennel_selection_algorithm() { unimplemented!() }
//         }
//     }
//
//     pub mod other {
//         pub mod event {
//             pub fn data_buffer_overflow() { unimplemented!() }
//             pub fn hardware_error() { unimplemented!() }
//             pub fn read_local_p256_public_key_complete() { unimplemented!() }
//             pub fn generate_dh_key_complete() { unimplemented!() }
//         }
//         pub mod command {
//             pub fn host_buffer_size() { unimplemented!() }
//             pub fn host_number_of_completed_packets() { unimplemented!() }
//             pub fn le_read_transmit_power() { unimplemented!() }
//             pub fn le_read_p256_public_key() { unimplemented!() }
//             pub fn generate_dh_key() { unimplemented!() }
//         }
//     }
}
