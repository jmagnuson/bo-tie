use super::bluez;
use crate::hci::events;
use std::cell::Cell;
use std::cmp::{PartialEq,PartialOrd,Ordering};
use std::collections::{BTreeSet};
use std::convert::From;
use std::future;
use std::pin::Pin;
use std::sync::{mpsc, Arc, Mutex};
use std::task;
use std::vec::Vec;
use super::{Error, WakerToken};

pub type EventResponse = Result<events::EventsData, super::Error>;

#[derive(Clone,Eq,Debug)]
struct EventFlag {
    event: events::Events,
    adapter_fd: Option<super::ArcFileDesc>,
    count: Cell<usize>,
}

impl EventFlag {

    /// Create a new EventFlag
    fn new(event: events::Events, adapter_fd: super::ArcFileDesc) -> Self {

        use nix::errno::Errno;
        use nix::libc::{ c_void, c_int, getsockopt, setsockopt, socklen_t };

        let mut filter = bluez::hci_filter::default();

        let eval = event.get_val() as usize;

        unsafe { Errno::result(getsockopt(
            adapter_fd.raw_fd(),
            bluez::SOL_HCI as i32,
            bluez::HCI_FILTER as i32,
            &mut filter as *mut bluez::hci_filter as *mut c_void,
            &mut ::std::mem::size_of::<bluez::hci_filter>() as *mut usize as *mut socklen_t
        ))}.unwrap();

        filter.type_mask |= 1 << bluez::HCI_EVENT_PKT;

        // Copied from bluez hci_lib.h hci_set_bit
        filter.event_mask[ eval >> 5] |= 1 << (eval & bluez::HCI_FLT_TYPE_BITS);

        Errno::result( unsafe { setsockopt(
            adapter_fd.raw_fd(),
            bluez::SOL_HCI as c_int,
            bluez::HCI_FILTER as c_int,
            &mut filter as *mut bluez::hci_filter as *mut c_void,
            ::std::mem::size_of::<bluez::hci_filter>() as socklen_t
        )}).unwrap();

        EventFlag {
            event: event,
            adapter_fd: Some(adapter_fd),
            count: Cell::new(1),
        }
    }
}

impl PartialEq for EventFlag {
    fn eq(&self, other: &Self) -> bool {
        self.event == other.event
    }
}

impl PartialOrd for EventFlag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.event.partial_cmp(&other.event)
    }
}

impl Ord for EventFlag {
    fn cmp(&self, other: &EventFlag) -> Ordering {
        self.event.cmp(&other.event)
    }
}

impl Drop for EventFlag {
    fn drop(&mut self) {
        use nix::errno::Errno;
        use nix::libc::{ c_void, c_int, getsockopt, setsockopt, socklen_t };

        if let Some(ref fd) = self.adapter_fd {
            let fd_c = fd.clone();
            let eval = self.event.get_val() as usize;

            let mut filter = bluez::hci_filter::default();

            // Remove self.event from the linux filter list
            Errno::result( unsafe { getsockopt(
                fd_c.raw_fd(),
                bluez::SOL_HCI as c_int,
                bluez::HCI_FILTER as c_int,
                &mut filter as *mut bluez::hci_filter as *mut c_void,
                &mut (::std::mem::size_of::<bluez::hci_filter>() as socklen_t) as *mut socklen_t,
            )}).unwrap();

            filter.type_mask |= 1 << bluez::HCI_EVENT_PKT;

            // Copied from bluez hci_lib.h hci_clear_bit
            filter.event_mask[eval >> 5] &= !(1 << (eval & 31));

            Errno::result( unsafe { setsockopt(
                fd_c.raw_fd(),
                bluez::SOL_HCI as c_int,
                bluez::HCI_FILTER as c_int,
                &mut filter as *mut bluez::hci_filter as *mut c_void,
                ::std::mem::size_of::<bluez::hci_filter>() as socklen_t
            )}).unwrap();
        }
    }
}

impl From<events::Events> for EventFlag {

    /// This creates a dummy EventFlag.
    ///
    /// No magic happens when drop is called on objects created with this method. The only use for
    /// this is for searching in EventFlags::flags for a specific event flag.
    fn from(event: events::Events) -> Self {
        Self {
            event: event,
            adapter_fd: None,
            count: Cell::new(0),
        }
    }
}

#[derive(Debug, Clone)]
struct EventFlags {
    adapter_fd: super::ArcFileDesc,
    flags: BTreeSet<EventFlag>,
}

impl EventFlags {

    fn new(adapter_fd: super::ArcFileDesc) -> Self {
        Self {
            adapter_fd: adapter_fd.clone(),
            flags: BTreeSet::new(),
        }
    }

    fn insert_event_flag(&mut self, event: events::Events) {
        // this looks weird because you can't borrow mutably after borrowing unmutably so the
        // unmutable borrow needs to be dropped before the mutable borrow.
        if if let Some(event_flag) = self.flags.get(&event.into()) {
                event_flag.count.set(event_flag.count.get() + 1);
                false
            }
            else {
                true
            }
        {
            assert!(self.flags.insert(EventFlag::new(event, self.adapter_fd.clone())));
        }
    }

    /// This method assumes that the event is in the BTreeSet 'self.flags' and will panic if an
    /// EventFlag object for the event is not in the set.
    fn remove_event_flag(&mut self, event: events::Events) {

        if 0 == {
            if let Some(event_flag) = self.flags.get(&event.into()) {

                event_flag.count.set( event_flag.count.get() - 1);

                event_flag.count.get()
            }
            else {
                panic!("Couldn't get flag {:?}, from {:?}",
                    EventFlag::from(event),
                    self.flags
                );
            }
        } {
            self.flags.remove(&event.into());
        }
    }
}

#[derive(Clone,Debug)]
struct ExpectedEvent {
    event: events::Events,
    waker_token: Arc<Mutex<WakerToken>>,
    event_response: mpsc::Sender<EventResponse>,
    stop_timer: Option<super::StopTimeout>,
}

#[derive(Clone,Debug)]
pub struct EventExpecter {
    sender: mpsc::Sender<ExpectedEvent>,
    flags: Arc<Mutex<EventFlags>>,
}

#[derive(Clone,Debug)]
struct TimeoutCallback {
    tx: mpsc::Sender<EventResponse>,
    flags: Arc<Mutex<EventFlags>>,
    event: events::Events,
    waker_token: Arc<Mutex<WakerToken>>
}

impl super::OnTimeout for TimeoutCallback {
    fn on_timeout(&self) -> Result<(), super::Error> {
        self.tx.send(Err(super::Error::Timeout))
            .expect("Couldn't send timeout error, receiver disconnected");

        self.flags.lock()
            .or(Err(super::Error::MPSCError("Couldn't unlock flags, HCI processor task is likely nonexistent".into())))?
            .remove_event_flag(self.event);

        lock!(self.waker_token).trigger();

        Ok(())
    }
}

impl EventExpecter {

    pub fn expect_event(&self, event: events::Events, timeout: &mut super::TimeoutBuilder)
        -> Result<impl future::Future<Output=hci_future_output!()>, super::Error>
    {
        let waker_token = Arc::new(Mutex::new(WakerToken::default()));

        let (tx, rx) = mpsc::channel();

        self.sender.send(ExpectedEvent {
            event: event,
            waker_token: waker_token.clone(),
            event_response: tx.clone(),
            stop_timer: Some(timeout.make_stop_timer()?),
        })
        .or(Err(super::Error::MPSCError("Receiver is disconnected for expected event, HCI procesor \
            task is likely nonexistent".into())))?;

        lock!(self.flags).insert_event_flag(event);

        timeout.set_timeout_callback( Box::new(TimeoutCallback {
            tx: tx.clone(),
            flags: self.flags.clone(),
            event: event.clone(),
            waker_token: waker_token.clone(),
        }));

        struct EventFuture {
            waker_token:     Arc<Mutex<WakerToken>>,
            events_response: mpsc::Receiver<EventResponse>,
        }

        impl future::Future for EventFuture {
            type Output = EventResponse;

            fn poll (self: Pin<&mut Self>, cx: &mut task::Context) ->
                task::Poll<Self::Output>
            {
                use self::task::Poll;

                let mut waker = match self.waker_token.lock() {
                    Ok(waker) => waker,
                    Err(e) => return task::Poll::Ready(Err(Error::Other(e.to_string()))),
                };

                if waker.triggered() {
                    match self.events_response.try_recv() {
                        Ok(result) => Poll::Ready(result),
                        Err(err) => Poll::Ready(Err(Error::MPSCError(err.to_string()))),
                    }
                }
                else {

                    waker.set_waker(cx.waker().clone());

                    Poll::Pending
                }
            }
        }

        Ok(EventFuture {
            waker_token: waker_token.clone(),
            events_response: rx,
        })
    }
}

pub struct EventProcessor {
    expected_events: mpsc::Receiver<ExpectedEvent>,
    flags: Arc<Mutex<EventFlags>>,
    passed_events: Vec<ExpectedEvent>,
}

impl EventProcessor {

    /// Send an error to all pending events
    ///
    /// This will cause all pending futures for ExpectedEvent to go to the ready state with the
    /// specified future.
    ///
    /// If an error occurs it's impossible to determine what event (if any) caused the error, so
    /// all pending futures immediately go to the ready state with the
    ///
    /// # Panics
    /// A panic occurs if the mpsc receiver is disconnected
    pub fn send_error(&mut self, err: super::Error) {

        let send_err = | exp_evnt: ExpectedEvent | {
            exp_evnt.event_response.send(Err((&err).clone()))
                .expect(r#"Couldn't send error "{:/}", receiver disconnected"#);
            exp_evnt.waker_token.lock().expect(r#"Couldn't unlock waker token for "{:?}""#)
                .trigger()
        };

        self.expected_events.iter().for_each(send_err);

        self.passed_events.drain(..).for_each(send_err);
    }

    fn send_event_data(&self, event_data: events::EventsData, mut expected_event: ExpectedEvent) {

        expected_event.stop_timer.take().unwrap().stop().unwrap();

        self.flags.lock()
        .expect("Couldn't acqure lock for EventProcessor::flags")
        .remove_event_flag(expected_event.event);

        expected_event.event_response.send(Ok(event_data)).expect("Receiver Disconnected");

        expected_event.waker_token.lock()
        .expect("Couldn't acquire lock for Expected::Event")
        .trigger();
    }

    /// Processor for events from a bluetooth controller
    pub fn process(&mut self, raw_data: &[u8]) {
        let event_data = events::EventsData::from_packet(raw_data);

        let received_event = event_data.get_enum_name();

        // First search passed_events if it was this event was passed up earlier
        if let Some(pos) = self.passed_events.iter().position(|ref passed| passed.event == received_event) {
            let removed = self.passed_events.remove(pos);
            self.send_event_data(event_data, removed);
            return;
        }

        for expected_event in self.expected_events.try_iter() {
            if received_event == expected_event.event {
                self.send_event_data(event_data, expected_event);
                return;
            }
            else {
                self.passed_events.push(expected_event);
            }
        }

        // If nothing matches then the event data is ignored

        // TODO decide whether or not to process the CommandCompelete event here for determining how many HCI command packets to process
    }
}

pub struct EventSetup;

impl EventSetup {

    pub fn setup(adapter_fd: super::ArcFileDesc) -> (EventExpecter, EventProcessor) {

        let (tx, rx) = mpsc::channel();

        let event_flags = Arc::new(Mutex::new(EventFlags::new(adapter_fd.clone())));

        let expecter = EventExpecter {
            sender: tx,
            flags: event_flags.clone(),
        };

        let processor = EventProcessor {
            expected_events: rx,
            flags: event_flags,
            passed_events: Vec::new(),
        };

        (expecter, processor)
    }
}
