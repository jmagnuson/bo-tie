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
#![feature(pin)]

#[cfg(unix)] extern crate simple_signal;
extern crate bo_tie;

#[cfg(not(target_os = "android"))]
mod example {
    use bo_tie::hci;
    use bo_tie::gap::advertise;
    use bo_tie::hci::le::transmitter::{
        set_advertising_data,
        set_advertising_parameters,
        set_advertising_enable,
    };
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc,RwLock};
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
                let pin_fut = unsafe { Pin::new_unchecked(&mut gen_fut) };

                match pin_fut.poll(&waker) {
                    task::Poll::Ready(val) => break val,
                    task::Poll::Pending => main_waker.sleep(),
                }
            }
        }}
    }

    async fn advertise_setup (
        hi: &hci::HostInterface,
        data: set_advertising_data::AdvertisingData,
        flag: Arc<RwLock<bool>> )
    {

        await!(set_advertising_enable::send(&hi, false).unwrap()).unwrap();

        await!(set_advertising_data::send(&hi, data).unwrap()).unwrap();

        let mut adv_prams = set_advertising_parameters::AdvertisingParameters::default();

        adv_prams.advertising_type = set_advertising_parameters::AdvertisingType::NonConnectableUndirectedAdvertising;

        await!(set_advertising_parameters::send(&hi, adv_prams).unwrap()).unwrap();

        await!(set_advertising_enable::send(&hi, *flag.read().unwrap() ).unwrap()).unwrap();

        println!("Advertising");
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

    pub fn run() {
        let adv_flag = Arc::new(RwLock::new(true));

        let interface = hci::HostInterface::default();

        let adv_name = advertise::local_name::LocalName::new("Advertiser Test", false);

        let mut adv_data = set_advertising_data::AdvertisingData::new();

        adv_data.try_push(adv_name).unwrap();

        handle_sig(adv_flag.clone());

        wait!(advertise_setup(&interface, adv_data, adv_flag.clone()));

        while *adv_flag.read().unwrap() {}

        wait!(advertise_teardown(&interface));
    }
}

#[cfg(not(target_os = "android"))]
fn main() {
    example::run();
}

#[cfg(target_os = "android")]
fn main() {
    panic!("bo-tie doesn't support hci for android")
}
