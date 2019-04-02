//! Advertising example
//!
//! This examples sets up the bluetooth device to advertise. The only data sent in each advertising
//! message is just the local name "Advertiser Test". The application will continue to run until
//! the example is sent a signal (e.g. by pressing ctrl-c on a unix system).
//!
//! # Note
//! Super User privaleges may be required to interact with your bluetooth peripheral. To do will
//! probably require the full path to cargo. The cargo binary is usually locacted in your home
//! directory at `.cargo/bin/cargo`.

#![feature(async_await)]
#![feature(await_macro)]
#![feature(futures_api)]
#![feature(gen_future)]

use bo_tie::hci;
use bo_tie::gap::advertise;
use bo_tie::hci::le::transmitter::{
    set_advertising_data,
    set_advertising_parameters,
    set_advertising_enable,
};
use std::sync::{Arc,RwLock};
use std::task;
use std::thread;

unsafe fn waker_clone(data: *const ()) -> task::RawWaker {
    let arc_thread = Arc::from_raw(data);
    let raw_waker = task::RawWaker::new( Arc::into_raw(arc_thread.clone()), &RAW_WAKER_V_TABLE);
    Arc::into_raw(arc_thread);
    raw_waker
}

unsafe fn waker_wake(data: *const ()) {
    let arc_thread = Arc::from_raw(data as *const thread::Thread);
    arc_thread.unpark();
    Arc::into_raw(arc_thread);
}

unsafe fn waker_drop(data: *const ()) {
    Arc::from_raw(data as *const thread::Thread);
}

static RAW_WAKER_V_TABLE: task::RawWakerVTable = task::RawWakerVTable {
    clone: waker_clone,
    wake: waker_wake,
    drop: waker_drop,
};

macro_rules! wait {
    ($gen_fut:expr) => {{
        use std::future::Future;

        let this_thread_handle = thread::current();

        let waker = unsafe {
            std::task::Waker::new_unchecked(
                std::task::RawWaker::new(
                    Arc::into_raw( Arc::new(this_thread_handle) ) as *const (),
                    &RAW_WAKER_V_TABLE
                )
            )
        };

        let mut future = $gen_fut;

        loop {
            match unsafe { std::pin::Pin::new_unchecked(&mut future ).poll(&waker) }  {
                task::Poll::Ready(val) => break val,
                task::Poll::Pending => thread::park(),
            }
        }
    }}
}

async fn advertise_setup (
    hi: &hci::HostInterface,
    data: set_advertising_data::AdvertisingData,
    flag: Arc<RwLock<bool>> )
{

    println!("Advertsinging Setup:");

    await!(set_advertising_enable::send(&hi, false).unwrap()).unwrap();

    println!("{:5>}", "Advertising Disabled");

    await!(set_advertising_data::send(&hi, data).unwrap()).unwrap();

    println!("{:5>}", "Set Advertising Data");

    let mut adv_prams = set_advertising_parameters::AdvertisingParameters::default();

    adv_prams.advertising_type = set_advertising_parameters::AdvertisingType::NonConnectableUndirectedAdvertising;

    await!(set_advertising_parameters::send(&hi, adv_prams).unwrap()).unwrap();

    println!("{:5>}", "Set Advertising Parameters");

    await!(set_advertising_enable::send(&hi, *flag.read().unwrap() ).unwrap()).unwrap();

    println!("{:5>}", "Advertising Enabled");
}

async fn advertise_teardown(hi: &hci::HostInterface) {
    await!(set_advertising_enable::send(&hi, false).unwrap()).unwrap();
}

#[cfg(unix)]
fn handle_sig( flag: Arc<RwLock<bool>> ) {
    use simple_signal;

    simple_signal::set_handler(&[simple_signal::Signal::Int, simple_signal::Signal::Term],
        move |_| { *flag.write().unwrap() = false }
    );
}

fn main() {

    let adv_flag = Arc::new(RwLock::new(true));

    let interface = hci::HostInterface::default();

    let adv_name = advertise::local_name::LocalName::new("Advertiser Test", false);

    let mut adv_data = set_advertising_data::AdvertisingData::new();

    adv_data.try_push(adv_name).unwrap();

    handle_sig(adv_flag.clone());

    wait!(advertise_setup(&interface, adv_data, adv_flag.clone()));

    println!("Waiting for 'ctrl-C' to stop advertising");

    while *adv_flag.read().unwrap() {}

    wait!(advertise_teardown(&interface));
}
