use hci::events;
use std::boxed::Box;
use std::collections::BTreeMap;
use std::default;
use std::error;
use std::fmt;
use std::future;
use std::ops::Drop;
use std::option::Option;
use std::ptr;
use std::sync::{Arc,Mutex};
use std::task;
use std::thread;
use std::time::Duration;
use std::os::unix::io::RawFd;

mod bluez {
    use std::os::raw::c_void;

    // Linux Bluetooth socket constants
    pub const SOL_HCI: u32 = 0;
    pub const HCI_FILTER: u32 = 2;
    //pub const HCI_COMMAND_PKT: u32 = 1;
    //pub const HCI_ACLDATA_PKT: u32 = 2;
    //pub const HCI_SCODATA_PKT: u32 = 3;
    pub const HCI_EVENT_PKT: u32 = 4;
    //pub const HCI_VENDOR_PKT: u32 = 255;

    // HCI filter constants from the bluez library
    pub const HCI_FLT_TYPE_BITS: usize = 31;
    // const HCI_FLT_EVENT_BITS: u32 = 63;

    #[link(name = "bluetooth")]
    extern "C" {
        pub fn hci_open_dev(dev_id: i32) -> i32;
        pub fn hci_get_route(bt_dev_addr: *mut ::BluetoothDeviceAddress) -> i32;
        pub fn hci_send_cmd(dev: i32, ogf: u16, ocf: u16, parameter_len: u8, parameter: *mut c_void) -> i32;
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct hci_filter {
        pub type_mask: u32,
        pub event_mask: [u32; 2usize],
        pub opcode: u16,
    }
}

#[derive(Debug,PartialEq,Eq,Clone)]
pub struct FileDescriptor(RawFd);

impl Drop for FileDescriptor {
    fn drop(&mut self) {
        use nix::unistd::close;

        close(self.0).unwrap();
    }
}

#[derive(Debug,PartialEq,Eq,Clone)]
pub struct ArcFileDesc(Arc<FileDescriptor>);

impl From<RawFd> for ArcFileDesc {
    fn from(rfd: RawFd) -> Self {
        ArcFileDesc(Arc::new(FileDescriptor(rfd)))
    }
}

impl ArcFileDesc {
    fn raw_fd(&self) -> RawFd {
        (*self.0).0
    }
}

macro_rules! lock {
    ($( $mutex:ident ).*) => {
        $($mutex).*.lock().map_err(|e| Error::MPSCError(e.to_string()))?
    }
}

mod event;

/// For Epoll, a value is assigned to signify what file descriptor had an event occur.
/// * 0 -> BluetoothController,
/// * 1 -> TaskExit,
/// * else -> Timeout
enum EPollResult {
    BluetoothController,
    TaskExit,
    Timeout(u64),
}

impl From<u64> for EPollResult {
    fn from(val: u64) -> Self {
        match val {
            0 => EPollResult::BluetoothController,
            1 => EPollResult::TaskExit,
            _ => EPollResult::Timeout(val),
        }
    }
}

impl From<EPollResult> for u64 {
    fn from(epr: EPollResult) -> Self {
        match epr {
            EPollResult::BluetoothController => 0,
            EPollResult::TaskExit => 1,
            EPollResult::Timeout(val) => val,
        }
    }
}

fn make_timeout_id(timeout_fd: RawFd) -> u64 {
    // The (ch)easy way to make unique id's for the timeouts
    timeout_fd as u64 + 2
}

#[derive(Clone, PartialEq, Debug)]
pub enum Error {
    EventNotSentFromController(String),
    IOError(::nix::Error),
    MPSCError(String),
    /// The first item in the tuple is the expected events, the second item is the received event
    IncorrectEvent(Vec<events::Events>, events::Events),
    Timeout,
    Other(String),
}

impl fmt::Display for Error  {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::EventNotSentFromController(ref reason) =>
                write!(f, "Event not sent from controller {}", reason),

            Error::IOError(ref errno) => write!(f, "IO error: {}", errno ),

            Error::MPSCError(ref msg) => write!(f, "{}", msg ),

            Error::IncorrectEvent( ref expected_events, ref received_event) => {
                let ret_0 = write!(f, "Received incorrect event '{:?}' from the controller",
                    received_event);

                if expected_events.len() != 0 {
                    write!(f, ", expected event(s): ");

                    for event in expected_events[..(expected_events.len() - 1)].iter() {
                        write!(f, " {:?},", event);
                    }

                    write!(f, " {:?}", expected_events[expected_events.len() - 1])
                }
                else {
                    ret_0
                }
            },

            Error::Timeout => write!(f, "Timeout Occured"),

            Error::Other( ref msg) => write!(f, "{}", msg),
        }
    }
}

impl error::Error for Error  {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::EventNotSentFromController(_) => None,
            Error::IOError(ref errno) => errno.cause().clone(),
            Error::MPSCError(_) => None,
            Error::IncorrectEvent(_,_) => None,
            Error::Timeout => None,
            Error::Other(_) => None,
        }
    }
}

impl From<::nix::Error> for Error  {
    fn from( e: ::nix::Error ) -> Self {
        Error::IOError(e)
    }
}

impl From<::nix::errno::Errno> for Error {
    fn from( e: ::nix::errno::Errno ) -> Self {
        Error::IOError(::nix::Error::Sys(e))
    }
}


impl From<Error> for super::CommandError<Error, Error>  {
    fn from(e: Error) -> Self {
        match e {
            Error::EventNotSentFromController(_) |
            Error::IncorrectEvent(_,_) |
            Error::Timeout |
            Error::Other(_) |
            Error::MPSCError(_) => super::CommandError::Specific(e),
            Error::IOError(_) => super::CommandError::SystemError(e),
        }
    }
}

/// Controller Message type
///
/// The way to differentiate between messages over the HCI
enum CtrlMsgType {
    Command,
    Event,
    ACLData,
    SyncData,
}

impl CtrlMsgType {
    fn from(raw: u8) -> Result<Self, ()> {
        match raw {
            0x01 => Ok(CtrlMsgType::Command),
            0x02 => Ok(CtrlMsgType::ACLData),
            0x03 => Ok(CtrlMsgType::SyncData),
            0x04 => Ok(CtrlMsgType::Event),
            _ => Err(())
        }
    }
}

impl From<::nix::Error> for super::CommandError<::nix::errno::Errno,::nix::Error> {
    fn from( err: ::nix::Error ) -> Self {
        match err {
            ::nix::Error::Sys(errno) => super::CommandError::SystemError(errno),
            _ => super::CommandError::Specific(err),
        }
    }
}

fn remove_timer_from_epoll( epoll_fd: ArcFileDesc, timer_fd: ArcFileDesc) -> Result<(), Error> {
    use nix::sys::epoll;

    epoll::epoll_ctl(
        epoll_fd.raw_fd(),
        epoll::EpollOp::EpollCtlDel,
        timer_fd.raw_fd(),
        None
    )
    .map_err(|e| Error::from(e))?;

    Ok(())
}

struct Timeout {
    epoll_fd: ArcFileDesc,
    timer_fd: ArcFileDesc,
    callback: Box<OnTimeout>,
}

impl Timeout {

    fn remove_timer(&self) -> Result<(), Error>{
        remove_timer_from_epoll(self.epoll_fd.clone(), self.timer_fd.clone())
    }

    /// Triggers the callback and removes the timer
    fn trigger(self) -> Result<(), Error> {
        self.remove_timer()?;

        self.callback.on_timeout()
    }
}

struct StopTimeout {
    epoll_fd: ArcFileDesc,
    timer_fd: ArcFileDesc,
    id: u64,
    timeout_manager: Arc<Mutex<TimeoutManager>>,
}

impl StopTimeout {
    fn stop(self) -> Result<(), Error> {
        lock!(self.timeout_manager)
        .remove(self.id)
        .or(Err(Error::Other("Timeout ID doesn't exist".to_string())))?;

        remove_timer_from_epoll(self.epoll_fd, self.timer_fd)
    }
}

trait OnTimeout: Send {
    fn on_timeout(&self) -> Result<(), Error>;
}

pub struct TimeoutBuilder {
    epoll_fd: ArcFileDesc,
    timer_fd: ArcFileDesc,
    callback: Option<Box<OnTimeout>>,
    timeout_manager: Arc<Mutex<TimeoutManager>>,
    time: Duration,
    id: u64,
}

impl TimeoutBuilder {

    fn new( epoll_fd: ArcFileDesc, time: Duration, tm: Arc<Mutex<TimeoutManager>>) -> Result<TimeoutBuilder, Error>
    {
        use nix::libc;
        use nix::errno::Errno;
        use nix::sys::epoll;

        let timer_fd = unsafe{ libc::timerfd_create(libc::CLOCK_MONOTONIC, libc::TFD_CLOEXEC) };

        if timer_fd < 0 { return Err(Error::from(Errno::last())); }

        let timer_id = make_timeout_id(timer_fd);

        epoll::epoll_ctl(
            epoll_fd.raw_fd(),
            epoll::EpollOp::EpollCtlAdd,
            timer_fd,
            &mut epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, timer_id)
        )
        .map_err(|e| Error::from(e))?;

        Ok(TimeoutBuilder {
            epoll_fd: epoll_fd,
            timer_fd: ArcFileDesc::from(timer_fd),
            callback: None,
            timeout_manager: tm,
            time: time,
            id: timer_id,
        })
    }

    /// Must be called to set the function that is called when a timeout occurs.
    fn set_timeout_callback(&mut self, callback: Box<OnTimeout>) {
        self.callback = Some(callback);
    }

    /// set_timeout_callback must be called before this is called to set the callback method
    /// because a callback is needed to construct a "dummy" timeout object
    fn make_stop_timer(&self) -> Result<StopTimeout, Error> {
        Ok(StopTimeout {
            epoll_fd: self.epoll_fd.clone(),
            timer_fd: self.timer_fd.clone(),
            id: self.id.clone(),
            timeout_manager: self.timeout_manager.clone(),
        })
    }

    /// set_timeout_callback must be called to set the timeout callback or this will just return
    /// an error
    fn enable_timer(mut self) -> Result<(), Error>
    {
        use nix::errno::Errno;
        use nix::libc;
        use std::ptr::null_mut;

        let timeout = Timeout {
            epoll_fd: self.epoll_fd.clone(),
            timer_fd: self.timer_fd.clone(),
            callback: self.callback.take().ok_or(Error::Other("timeout callback not set".into()))?,
        };

        let timeout_spec = libc::itimerspec {
            it_interval: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            it_value: libc::timespec {
                tv_sec: self.time.as_secs() as libc::time_t,
                tv_nsec: self.time.subsec_nanos() as libc::c_long,
            }
        };

        lock!(self.timeout_manager).add(self.id, timeout)?;

        if 0 > unsafe{ libc::timerfd_settime(
            self.timer_fd.raw_fd(),
            0,
            &timeout_spec as *const libc::itimerspec,
            null_mut()) }
        {
            lock!(self.timeout_manager).remove(self.id)?;
            return Err(Error::from(Errno::last()));
        }

        Ok(())
    }
}

struct TimeoutManager {
    timeouts: BTreeMap<u64,Timeout>
}

impl TimeoutManager {
    fn new() -> Self {
        TimeoutManager {
            timeouts: BTreeMap::new()
        }
    }

    fn add(&mut self, timeout_id: u64, timeout: Timeout ) -> Result<(), Error> {
        match self.timeouts.insert(timeout_id, timeout) {
            None => Ok(()),
            Some(v) => {
                self.timeouts.insert(timeout_id, v);
                Err(Error::Other("Timeout ID already exists".to_string()))
            }
        }
    }

    fn remove(&mut self, timeout_id: u64) -> Result<Timeout, Error> {
        self.timeouts.remove(&timeout_id).ok_or(Error::Other("Timeout ID doesn't exist".to_string()))
    }
}

struct AdapterThread {
    adapter_fd: ArcFileDesc,
    exit_fd: ArcFileDesc,
    epoll_fd: ArcFileDesc,
    event_processor: event::EventProcessor,
    timeout_manager: Arc<Mutex<TimeoutManager>>
}

impl AdapterThread {

    /// Spawn self
    fn spawn(self) -> thread::JoinHandle<()> {
        thread::spawn( move || {
            self.task();
        })
    }

    /// Ignores the Unix errors EAGAIN and EINTR
    fn ignore_eagain_and_eintr<F,R>( mut func: F ) -> Result<R, Error>
        where F: FnMut() -> Result<R, Error>
    {
        use nix;
        use nix::errno::Errno;

        loop {

            let result = func();

            if let Err(ref err) = &result {
                if let Error::IOError(nix_err) = err {
                    if let nix::Error::Sys(err_val) = nix_err {
                        if *err_val == Errno::EAGAIN || *err_val == Errno::EINTR {
                            continue;
                        }
                    }
                }
            }
            break result
        }
    }

    /// Task for processing HCI messages from the controller
    ///
    /// This functions takes that data from the controller and splits it up into different
    /// processors based on the HCI message type. Only Events, ACL data, and Syncronous data messages
    /// have processors since they are the only messages from the controller. This task forever
    /// polls the device id of the adapter to wait for
    ///
    /// This task can only exit by closing the device or timeout file descriptors.
    fn task(mut self) {
        use nix::sys::epoll;
        use nix::unistd::read;

        'task: loop {

            let epoll_events = &mut [epoll::EpollEvent::empty();256];

            let event_count = match Self::ignore_eagain_and_eintr( || {

                epoll::epoll_wait(self.epoll_fd.raw_fd(), epoll_events, -1).map_err(|e| {
                    Error::from(e)
                })

            }) {
                Ok(size) => size,
                Err(e) => {
                    self.event_processor.send_error(e);
                    continue;
                }
            };

            for epoll_event in epoll_events[..event_count].iter() {
                match EPollResult::from(epoll_event.data()) {
                    EPollResult:: BluetoothController => {

                        // size per the bluetooth spec for the HCI Event Packet
                        // (in v5 | vol 2, Part E 5.4.4 )
                        let mut buffer = [0u8; 256];

                        // received the data
                        let len = match Self::ignore_eagain_and_eintr( || {
                            read( self.adapter_fd.raw_fd(), &mut buffer).map_err( |e| { Error::from(e) })
                        }) {
                            Ok(val) => val,
                            Err(e)  => {
                                self.event_processor.send_error(e);
                                continue;
                            },
                        };

                        // The first byte is the indicator of the mssage type, next byte is the length of the
                        // message, the rest is the hci message
                        //
                        // Any other values are ignored (including the special 0xFF value)
                        if let Ok(msg) = CtrlMsgType::from(buffer[0])
                        {
                            match msg {
                                CtrlMsgType::Command => {
                                    panic!("Received a command message, the HCI adapter task should \
                                        only receive ACL, Syncronous, or Event Data from a controller")
                                },
                                CtrlMsgType::Event => { self.event_processor.process(&buffer[..len]) },
                                CtrlMsgType::ACLData => {unimplemented!()},
                                CtrlMsgType::SyncData => {unimplemented!()},
                            }
                        }
                    },

                    EPollResult::TaskExit => {
                        // Clear the block for the main task
                        read( self.exit_fd.raw_fd(), &mut [0u8;8]).unwrap();
                        break 'task;
                    },

                    EPollResult::Timeout(id) => {
                        let timeout = self.timeout_manager.lock().expect("Missing Timeout").remove(id).unwrap();

                        timeout.trigger().unwrap();
                    },
                }
            }
        }
    }
}

/// Bluetooth Host Controller Interface Adapter
///
/// Interfacing with the Bluetooth radio is done through an interface labeled as an adapter. This
/// is the structure used to handle the Host Controller Interface (HCI) as specified in the
/// Bluetooth specification.
///
/// Each Bluetooth adapter (if there is any) is assigned an identifier (just a number) by your
/// system.
pub struct HCIAdapter {
    adapter_fd: ArcFileDesc,
    exit_fd: ArcFileDesc,
    epoll_fd: ArcFileDesc,
    task_handle: Option<thread::JoinHandle<()>>,
    event_expecter: event::EventExpecter,
    timeout_manager: Arc<Mutex<TimeoutManager>>,
}

impl From<i32> for HCIAdapter {

    /// Create a HCIAdapter with the given bluetooth adapter id if an adapter exists
    ///
    /// Call "default" if the device id is unknown or any adapter is acceptable
    ///
    /// # Panics
    /// There is no Bluetooth Adapter with the given device id
    fn from( adapter_id: i32 ) -> Self {

        use nix::sys::eventfd::{EfdFlags, eventfd};
        use nix::sys::epoll::{
            epoll_create1,
            epoll_ctl,
            EpollCreateFlags,
            EpollOp,
            EpollEvent,
            EpollFlags,
        };

        let device_fd = unsafe { bluez::hci_open_dev(adapter_id) };

        if device_fd < 0 {
            panic!("No Bluetooth Adapter with device id {} exists", adapter_id);
        }

        let exit_evt_fd = eventfd(0, EfdFlags::EFD_CLOEXEC).expect("eventfd failed");

        let epoll_fd = epoll_create1(EpollCreateFlags::EPOLL_CLOEXEC).expect("epoll_create1 failed");

        epoll_ctl(
            epoll_fd,
            EpollOp::EpollCtlAdd,
            device_fd,
            &mut EpollEvent::new(EpollFlags::EPOLLIN, EPollResult::BluetoothController.into())
        ).expect("epoll_ctl failed");

        epoll_ctl(
            epoll_fd,
            EpollOp::EpollCtlAdd,
            exit_evt_fd,
            &mut EpollEvent::new(EpollFlags::EPOLLIN, EPollResult::TaskExit.into())
        ).expect("epoll_ctl failed");

        let arc_adapter_fd = ArcFileDesc::from(device_fd);
        let arc_exit_fd = ArcFileDesc::from(exit_evt_fd);
        let arc_epoll_fd = ArcFileDesc::from(epoll_fd);

        let (event_expecter, event_processor) = event::EventSetup::setup(arc_adapter_fd.clone());

        let to_manager = Arc::new(Mutex::new(TimeoutManager::new()));

        let thread_handle = AdapterThread {
            adapter_fd: arc_adapter_fd.clone(),
            exit_fd: arc_exit_fd.clone(),
            epoll_fd: arc_epoll_fd.clone(),
            event_processor: event_processor,
            timeout_manager: to_manager.clone(),
        }
        .spawn();

        HCIAdapter {
            adapter_fd: arc_adapter_fd,
            exit_fd: arc_exit_fd,
            epoll_fd: arc_epoll_fd,
            task_handle: Some(thread_handle),
            event_expecter: event_expecter,
            timeout_manager: to_manager,
        }
    }
}

/// Create a HCIAdapter object with the first bluetooth adapter returned by the system
///
/// # Panics
/// * No bluetooth adapter exists on the system
/// * The system couldn't allocate another file descriptor for the device
impl default::Default for HCIAdapter {

    fn default() -> Self {

        let adapter_id = unsafe { bluez::hci_get_route(ptr::null_mut()) };

        if adapter_id < 0 {
            panic!("No bluetooth adapter on this system");
        }

        HCIAdapter::from(adapter_id)
    }
}

impl Drop for HCIAdapter {

    fn drop(&mut self) {
        use nix::unistd::write;

        // Send the exit signal.
        // The value sent doesn't really matter (just that it is 8 bytes, not 0, and not !0 )
        match write( self.exit_fd.raw_fd(), &[1u8;8]) {
            Ok(_) => self.task_handle.take().unwrap().join().expect("Couldn't join with thread"),
            Err(e) => panic!("Couldn't exit adapter gracefully! err:{:?}", e),
        };
    }
}

impl HCIAdapter {

    /// Send a command to the controller
    ///
    /// Any events that are filterable on the controller side (and by default are not enabled) must
    /// be enabled by the HCI Set Event Mask/Filter Commands
    ///
    /// Much of this command should mirror what is done in hci_send_req (from hci.c in bluez).
    /// Unfortunately, some of what bluez did is magic to me.
    pub(super) fn send_command<CmdData>(&self, input: CmdData, expected_event: events::Events, timeout: Duration)
        -> Result<impl future::Future<Output=hci_future_output!()>, Error>
        where CmdData: ::hci::CommandParameters
    {
        use nix::errno::Errno;
        use std::mem::size_of;

        let mut timeout_builder = TimeoutBuilder::new(
            self.epoll_fd.clone(),
            timeout,
            self.timeout_manager.clone()
        )?;

        // set the future for the event
        let future = self.event_expecter.expect_event(expected_event, &mut timeout_builder)?;

        let oc_pair = CmdData::COMMAND.as_opcode_pair();

        timeout_builder.enable_timer()?;

        // send the command
        if let Err(err) = Errno::result( unsafe { bluez::hci_send_cmd(
            self.adapter_fd.raw_fd(),
            oc_pair.ogf,
            oc_pair.ocf,
            size_of::<CmdData::Parameter>() as u8,
            &mut input.get_parameter() as *mut CmdData::Parameter as *mut ::std::os::raw::c_void
        )}){
            Err(Error::from(err))
        } else {
            Ok(future)
        }
    }

    pub(super) fn wait_for_event(&self, event: events::Events, timeout: Duration) ->
        Result<impl future::Future<Output=hci_future_output!()>, Error>
    {
        let mut timeout_builder = TimeoutBuilder::new(
            self.epoll_fd.clone(),
            timeout,
            self.timeout_manager.clone()
        )?;

        let future = self.event_expecter.expect_event(event, &mut timeout_builder)?;

        timeout_builder.enable_timer()?;

        Ok(future)
    }
}

/// A token to keep track of what waker (from futres), if any, should be triggered when an event
/// is received from the bluetooth controller
///
/// An instance must be wrapped in Arc-Mutex/RwLock to be multi-thread safe (per the usual)
struct WakerToken {
    waker: Option<task::Waker>,
    waker_triggered: bool,
}

impl WakerToken {

    /// Trigger the waker if there is a waker.
    ///
    /// A trigger flag is set by this method to indicate to the method set_waker that it needs
    /// to immediately call the wake method of its waker paramter.
    fn trigger(&mut self) {

        self.waker_triggered = true;

        if let Some(ref waker) = self.waker {
            waker.wake()
        }
    }

    /// Determine if the trigger method was called or an error occured
    fn triggered(&self) -> bool {
        self.waker_triggered
    }

    /// Set the waker
    fn set_waker(&mut self, waker: task::Waker) {
        self.waker = Some(waker);
    }
}

impl Default for WakerToken {

    /// Create a default waker token. No Waker object is supplied so instead the function
    /// triggered must be checked to see if trigger was called.
    fn default() -> Self {
        WakerToken {
            waker: None,
            waker_triggered: false,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::pin::Pin;
    use std::task;
    use hci::test_util::block_for_result;
    use std::future::Future;

    #[test]
    fn exit_test() {
        let _adapter = HCIAdapter::default();
    }

    struct TimeoutFuture {
        waker_token: Arc<Mutex<WakerToken>>
    }

    impl Future for TimeoutFuture {
        type Output = ();

        fn poll(self: Pin<&mut Self>, lw: &task::LocalWaker) -> task::Poll<Self::Output> {
            let mut waker = self.waker_token.lock().unwrap();

            waker.set_waker(lw.clone().into_waker());

            if waker.triggered() {
                task::Poll::Ready(())
            } else {
                task::Poll::Pending
            }
        }
    }

    struct TimeoutHappened {
        waker_token: Arc<Mutex<WakerToken>>
    }

    impl OnTimeout for TimeoutHappened {
        fn on_timeout(&self) -> Result<(), Error> {
            lock!(self.waker_token).trigger();
            Ok(())
        }
    }

    #[test]
    fn timeout_test() {
        let adapter = HCIAdapter::default();
        let timeout = Duration::from_secs(1);
        let waker_token = Arc::new(Mutex::new(WakerToken::default()));

        let future = TimeoutFuture {
            waker_token: waker_token.clone()
        };

        let mut timeout_builder = TimeoutBuilder::new(
            adapter.epoll_fd.clone(),
            timeout,
            adapter.timeout_manager.clone()
        ).unwrap();

        timeout_builder.set_timeout_callback(Box::new( TimeoutHappened {
            waker_token: waker_token.clone()
        }));

        timeout_builder.enable_timer().unwrap();

        assert_eq!( block_for_result(future), () );
    }
}
