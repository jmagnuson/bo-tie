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

fn get_arg_options() -> getopts::Options {
    let mut opts = getopts::Options::new();
    opts.parsing_style(getopts::ParsingStyle::FloatingFrees);
    opts.long_only(false);
    opts.optflag("h", "help", "Print this help menu" );
    opts.opt("s",
            "service-uuid",
            "Space-separated 128 bit service uuids to advertise with. The UUIDs must be in the \
            format of XX:XX:XX:XX:XX:XX (From most significant to least significant byte)",
            "UUIDs",
            getopts::HasArg::Yes,
            getopts::Occur::Multi);
    opts
}

struct ParsedArgs {
    advertising_data: set_advertising_data::AdvertisingData
}

fn parse_args(mut args: std::env::Args ) -> Option<ParsedArgs> {
    let options = get_arg_options();

    let program_name = args.next().unwrap();

    let matches = match options.parse( &args.collect::<Vec<_>>() ) {
        Ok(all_match) => all_match,
        Err(no_match) => panic!(no_match.to_string())
    };

    if matches.opt_present("h") {
        print!("{}", options.usage(&format!("Usage: {} [options]", program_name)));
        None
    } else {
        let mut advertising_data = set_advertising_data::AdvertisingData::new();

        // Add service UUIDs to the advertising data
        let services_128 = matches.opt_strs("s")
            .into_iter()
            .fold( bo_tie::gap::advertise::service_class_uuid::new_128(true), |mut services, str_uuid|
            {
                use std::convert::TryFrom;

                let uuid = bo_tie::UUID::try_from(str_uuid.as_str()).expect("Invalid UUID");

                services.insert(uuid.into());

                services
            }
        );

        if ! services_128.as_ref().is_empty() {
            advertising_data.try_push(services_128).expect("Couldn't add services");
        }

        Some(
            ParsedArgs {
                advertising_data: advertising_data
            }
        )
    }
}

fn main() {

    let adv_flag = Arc::new(RwLock::new(true));

    let interface = hci::HostInterface::default();

    let adv_name = advertise::local_name::LocalName::new("Adv Test", false);

    let mut adv_data = match parse_args(std::env::args()) {
        Some(parse_args) => parse_args.advertising_data,
        None => set_advertising_data::AdvertisingData::new(),
    };

    adv_data.try_push(adv_name).unwrap();

    handle_sig(adv_flag.clone());

    wait!(advertise_setup(&interface, adv_data, adv_flag.clone()));

    println!("Waiting for 'ctrl-C' to stop advertising");

    while *adv_flag.read().unwrap() {}

    wait!(advertise_teardown(&interface));
}
