use super::bluez;
use bo_tie::hci::{
    events,
    EventMatcher,
};
use std::cell::Cell;
use std::cmp::{PartialEq,PartialOrd,Ordering};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::From;
use std::fmt;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use super::{WakerToken};

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

struct TimeoutCallback {
    todo: Box<dyn Fn() + Send + Sync>
}

impl fmt::Debug for TimeoutCallback {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "TimeoutCallback")
    }
}

impl super::OnTimeout for TimeoutCallback {
    fn on_timeout(&self) {
        (self.todo)()
    }
}

#[derive(Clone)]
struct DynEventMatcher {
    matcher: Pin<Arc<dyn EventMatcher>>,
}

impl fmt::Debug for DynEventMatcher {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "DynEventMatcher")
    }
}

impl Eq for DynEventMatcher {}

impl Ord for DynEventMatcher {
    fn cmp(&self, other: &Self) -> Ordering {
        (&*self.matcher as *const EventMatcher).cmp(&(&*other.matcher as *const EventMatcher))
    }
}

impl std::cmp::PartialEq for DynEventMatcher {
    fn eq(&self, other: &DynEventMatcher) -> bool {
        (&*self.matcher as *const EventMatcher) == (&*other.matcher as *const EventMatcher)
    }
}

impl std::cmp::PartialOrd for DynEventMatcher {
    fn partial_cmp(&self, other: &DynEventMatcher) -> Option<std::cmp::Ordering> {
        (&*self.matcher as *const EventMatcher).partial_cmp(&(&*other.matcher as *const EventMatcher))
    }
}

#[derive(Debug)]
struct ExpEventInfo {
    /// The data returned when the coresponding event is received
    ///
    /// Will also contain the error is any error occurs
    data: Option<Result<events::EventsData, crate::Error>>,
    /// Waker token used for waking the thread when an event comes
    waker_token: WakerToken,
    /// Timer used to shortcircuit
    stop_timer: Option<super::StopTimeout>,
}

/// Expected event manager
///
/// Flags are used by the Linux Os for determining what events are to be propigated from the
/// Bluetooth Controller to the user. This will map those flags to useable events with the
/// bo-tie library.
///
/// This also has limited capabilities for matching multiple events but it requires a matcher
/// that implements the `hci::EventMatcher` trait from the bo-tie crate.
#[derive(Debug)]
pub struct EventExpecter {
    expected: BTreeMap<events::Events, BTreeMap<DynEventMatcher, ExpEventInfo>>,
    flags: EventFlags,
}

impl EventExpecter {

    fn get_stop_timer(
        self: &mut std::sync::MutexGuard<Self>,
        mut timeout_builder: super::TimeoutBuilder,
        callback: Box<dyn Fn() + Send + Sync>)
    -> Result<super::StopTimeout, crate::Error>
    {
        timeout_builder.set_timeout_callback( Box::new(TimeoutCallback { todo: callback }) );

        let stop_timer = match timeout_builder.make_stop_timer() {
            Ok(val) => val,
            Err(e) => return Err(e),
        };

        if let Err(e) = timeout_builder.enable_timer() {
            return Err(e)
        }

        Ok(stop_timer)
    }

    fn remove_expected_event(
        self: &mut std::sync::MutexGuard<Self>,
        event: events::Events,
        pattern: &DynEventMatcher)
    -> Option<ExpEventInfo>
    {
        if let Some(map) = self.expected.get_mut(&event) {

            let retval = map.remove(&pattern);

            if map.len() == 0 {
                self.expected.remove(&event);
            }

            retval
        } else {
            None
        }
    }

    pub fn expect_event<P>(
        mutex: Arc<Mutex<Self>>,
        event: events::Events,
        waker: core::task::Waker,
        matcher: Pin<Arc<P>>,
        timeout_builder: Option<super::TimeoutBuilder>)
        -> Option<Result<events::EventsData, crate::Error>>
        where P: bo_tie::hci::EventMatcher + 'static
    {
        let pat_key = DynEventMatcher { matcher };

        let mut gaurd = mutex.lock().expect("Couldn't acquire lock");

        match gaurd.expected.get(&event).and_then(|map| map.get(&pat_key) )
        {
            None => {
                let waker_token = WakerToken::from(waker);

                let mutex_clone = mutex.clone();
                let event_clone = event.clone();
                let pat_key_clone = pat_key.clone();

                let callback = move || {
                    let mut gaurd = mutex_clone.lock().expect("Couldn't unlock");

                    gaurd.flags.remove_event_flag(event);

                    if let Some(info) = gaurd.expected.get_mut(&event_clone)
                        .and_then(|map| map.get_mut(&pat_key_clone) )
                    {
                        info.data = Some(Err(crate::Error::Timeout));

                        info.stop_timer = None;

                        info.waker_token.trigger();
                    }
                };

                let stop_timer = match timeout_builder {
                    Some(tb) => {
                        match gaurd.get_stop_timer( tb, Box::new( callback ) ) {
                            Ok(ti) => Some(ti),
                            Err(e) => return Some(Err(e))
                        }
                    },
                    None => None,
                };

                let val = ExpEventInfo {
                    data: None,
                    waker_token: waker_token,
                    stop_timer: stop_timer,
                };

                gaurd.expected.entry(event).or_insert(BTreeMap::new()).insert(pat_key, val);

                gaurd.flags.insert_event_flag(event);

                None
            }
            Some(ref val) => {
                if val.waker_token.triggered() {

                    let expected = gaurd.remove_expected_event(event, &pat_key).unwrap();

                    if let Some(stop_timer) = expected.stop_timer {
                         if let Err(err) = stop_timer.stop() {
                             return Some(Err(err));
                         }
                    }

                    expected.data

                } else {
                    None
                }
            }
        }
    }
}

pub struct EventProcessor {
    expected_events: Arc<Mutex<EventExpecter>>,
}

impl EventProcessor {

    /// Processor for events from a bluetooth controller
    pub fn process(&mut self, raw_data: &[u8]) {

        match events::EventsData::from_packet(raw_data) {
            Ok(event_data) => {
                let received_event = event_data.get_enum_name();

                if let Some(ref mut patterns_map) = self.expected_events.lock()
                    .expect("Couldn't acquire mutex")
                    .expected.get_mut(&received_event)
                {
                    for (dyn_matcher, ref mut exp_event_info) in patterns_map.iter_mut() {

                        if dyn_matcher.matcher.match_event(&event_data) {
                            exp_event_info.waker_token.trigger()
                        }
                    }

                    // Any events not matched are ignored
                }
            },
            Err(e) => log::error!("HCI Event Error: {}", e),
        }
    }
}

pub struct EventSetup;

impl EventSetup {

    pub fn setup(adapter_fd: super::ArcFileDesc) -> (Arc<Mutex<EventExpecter>>, EventProcessor) {

        let expecter = Arc::new(Mutex::new(EventExpecter {
            expected: BTreeMap::new(),
            flags: EventFlags::new(adapter_fd.clone()),
        }));

        let processor = EventProcessor {
            expected_events: expecter.clone(),
        };

        (expecter, processor)
    }
}
