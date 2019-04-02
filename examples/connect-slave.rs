//! Connection state in the slave role example
//!
//! # WARNING
//! There is no security implemented in this example, but no data is exposed either. Be careful
//! when extending/using this example for your purposes.
//!
//! This example shows the basic way to form a connection with this device in the slave role. A
//! random device address will be prompted for (just make up one e.g. 11:22:33:44:55:66 is fine)
//! when run. Afterwards the example will advertise for one minute waiting for a connection to be
//! made. If another device initiates & completes a connection with this one then a "Connection
//! Complete" message is printed. After 5 more seconds then disconnect is called on the connection.
//! If no connection is made after 1 minuite of advertising, then a timeout error message is output
//! and the example exits.
//!
//! # Note
//! Super User privaleges may be required to interact with your bluetooth peripheral. To do will
//! probably require the full path to cargo. The cargo binary is usually locacted in your home
//! directory at `.cargo/bin/cargo`.

#![feature(async_await)]
#![feature(await_macro)]
#![feature(futures_api)]
#![feature(gen_future)]

use bo_tie::gap::advertise;
use bo_tie::hci;
use bo_tie::hci::events;
use bo_tie::hci::le::transmitter::{
    set_advertising_data,
    set_advertising_parameters,
    set_advertising_enable,
    set_random_address,
};
use std::task;
use std::time::Duration;
use std::thread;

unsafe fn waker_clone(data: *const ()) -> task::RawWaker {
    task::RawWaker::new( data, &RAW_WAKER_V_TABLE)
}

unsafe fn waker_wake(data: *const ()) {
    (*(data as *const thread::Thread)).unpark();
}

unsafe fn waker_drop(_: *const ()) { }

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
                    &this_thread_handle as *const thread::Thread as *const (),
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

fn get_address() -> Result<bo_tie::BluetoothDeviceAddress, String> {
    use std::io::stdin;
    use std::io::BufRead;

    println!("Input the bluetooth address in the format of XX:XX:XX:XX:XX:XX (most significant \
        byte -> least significant) to set the random address:");

    let mut buffer = String::new();

    let stdin = stdin();

    stdin.lock().read_line(&mut buffer).expect("Couldn't read input from terminal");

    let adr_vec = buffer.rsplit(":")
        .map(|v| {
            u8::from_str_radix(v.trim(), 16).expect("Couldn't convert bluetooth address")
        })
        .collect::<Vec<u8>>();

    if adr_vec.len() == 6 {
        let mut address = bo_tie::BluetoothDeviceAddress::default();

        address.copy_from_slice(adr_vec.as_slice());

        Ok(address)
    }
    else {
        Err(format!("{} is not an address in the form of XX:XX:XX:XX:XX:XX", buffer))
    }
}

/// This sets up the advertising and waits for the connection complete event
async fn advertise_setup<'a>(
    hi: &'a hci::HostInterface,
    local_name: &'a str,
    rand_address: bo_tie::BluetoothDeviceAddress ) -> Result<hci::common::ConnectionHandle, ()>
{
    let adv_name = advertise::local_name::LocalName::new(local_name, false);

    let mut adv_data = set_advertising_data::AdvertisingData::new();

    adv_data.try_push(adv_name).unwrap();

    await!(set_advertising_enable::send(&hi, false).unwrap()).unwrap();

    await!(set_advertising_data::send(&hi, adv_data).unwrap()).unwrap();

    let mut adv_prams = set_advertising_parameters::AdvertisingParameters::default();

    adv_prams.own_address_type = bo_tie::hci::le::common::OwnAddressType::RandomDeviceAddress;

    await!(set_random_address::send(&hi, rand_address).unwrap()).unwrap();

    await!(set_advertising_parameters::send(&hi, adv_prams).unwrap()).unwrap();

    await!(set_advertising_enable::send(&hi, true).unwrap()).unwrap();

    let evt_rsl = await!(hi.wait_for_event(events::LEMeta::ConnectionComplete.into(), Duration::from_secs(60)).unwrap());

    await!(set_advertising_enable::send(&hi, false).unwrap()).unwrap();

    match evt_rsl {
        Ok(event) => {
            use bo_tie::hci::events::{EventsData,LEMetaData};

            println!("Connection Made!");

            if let EventsData::LEMeta(LEMetaData::ConnectionComplete(le_conn_comp_event)) = event {
                Ok(le_conn_comp_event.connection_handle)
            }
            else {
                println!("Received the incorrect event {:?}", event);
                Err(())
            }
        }
        Err(e) => {
            println!("Timeout Occured: {:?}", e);

            Err(())
        }
    }
}

async fn disconnect(hi: &hci::HostInterface, connection_handle: hci::common::ConnectionHandle ) {
    use bo_tie::hci::le::connection::disconnect;

    let prams = disconnect::DisconnectParameters {
        connection_handle: connection_handle,
        disconnect_reason: disconnect::DisconnectReason::RemoteUserTerminatedConnection,
    };

    await!(disconnect::send(&hi, prams).unwrap()).unwrap();
}

fn main() {

    let address = get_address().unwrap();

    let interface = hci::HostInterface::default();

    if let Ok(handle) = wait!(advertise_setup(&interface, "Connection Test", address)) {

        println!("Disconnecting in 5 seconds");
        thread::sleep(Duration::from_secs(5));

        wait!(disconnect(&interface, handle));
    }
}
