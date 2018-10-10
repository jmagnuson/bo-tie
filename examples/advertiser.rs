#![feature(async_await)]
#![feature(await_macro)]
#![feature(futures_api)]
#![feature(pin)]

#[cfg(unix)] extern crate simple_signal;
extern crate bo_tie;

use bo_tie::hci;
use bo_tie::gap::advertise;
use bo_tie::hci::le::transmitter::{
    set_advertising_data,
    set_advertising_parameters,
    set_advertising_enable,
};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task;
use std::thread;

// All this does is just wake & sleep the current thread
struct MainWaker {
    thread: thread::Thread
}

impl MainWaker {
    fn new() -> Self {
        MainWaker {
            thread: thread::current()
        }
    }

    fn sleep(&self) {
        thread::park();
    }
}

impl task::Wake for MainWaker {
    fn wake(arc_self: &Arc<Self>) {
        arc_self.thread.unpark();
    }
}

macro_rules! wait {
    ($gen_fut:expr) => {{
        let main_waker = Arc::new(MainWaker::new());
        let waker = ::std::task::local_waker_from_nonlocal(main_waker.clone());
        let mut gen_fut = $gen_fut;

        loop {
            let pin_fut = unsafe { ::std::pin::Pin::new_unchecked(&mut gen_fut) };

            match pin_fut.poll(&waker) {
                task::Poll::Ready(val) => break val,
                task::Poll::Pending => main_waker.sleep(),
            }
        }
    }}
}

async fn advertise_setup(hi: &hci::HostInterface, data: set_advertising_data::AdvertisingData) {

    await!(set_advertising_enable::send(&hi, false).unwrap()).unwrap();

    await!(set_advertising_data::send(&hi, data).unwrap()).unwrap();

    let mut adv_prams = set_advertising_parameters::AdvertisingParameters::default();

    adv_prams.advertising_type = set_advertising_parameters::AdvertisingType::NonConnectableUndirectedAdvertising;

    await!(set_advertising_parameters::send(&hi, adv_prams).unwrap()).unwrap();

    await!(set_advertising_enable::send(&hi, true).unwrap()).unwrap();
}

async fn advertise_teardown(hi: &hci::HostInterface) {
    await!(set_advertising_enable::send(&hi, false).unwrap()).unwrap();
}

#[cfg(unix)]
fn handle_sig() -> Arc<AtomicBool> {
    use simple_signal;

    let running = Arc::new(AtomicBool::new(true));

    let ret = running.clone();

    simple_signal::set_handler(&[simple_signal::Signal::Int, simple_signal::Signal::Term],
        move |_| { running.store(false, Ordering::Relaxed) }
    );

    ret
}

fn main() {

    let interface = hci::HostInterface::default();

    let adv_name = advertise::local_name::LocalName::new("Advertiser Test", false);

    let mut adv_data = set_advertising_data::AdvertisingData::new();

    adv_data.try_push(adv_name).unwrap();

    wait!(advertise_setup(&interface, adv_data));

    let run_flag = handle_sig();

    while run_flag.load(Ordering::Relaxed) {}

    wait!(advertise_teardown(&interface));
}
