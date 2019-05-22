//! Connection state in the slave role example
//!
//! This example shows the basic way to form a connection with this device in the slave role. The
//! only real imporant part to look at are the async funtions.
//!
//! To fully execute this example you'll need another bluetooth enabled device that can run in the
//! master role. If you have an android phone, you can use the 'nRF Connect' app to connect with
//! this example
//!
//! # WARNING
//! There is no security implemented in this example, but no data is exposed either. Be careful
//! when extending/using this example for your purposes.
//!
//! # Note
//! Super User privaleges may be required to interact with your bluetooth peripheral. To do will
//! probably require the full path to cargo. The cargo binary is usually locacted in your home
//! directory at `.cargo/bin/cargo`.

#![feature(async_await)]
#![feature(await_macro)]
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
use std::sync::{Arc, atomic::{AtomicU16, Ordering}};
use std::time::Duration;

/// 0xFFFF is a reserved value as of the Bluetooth Spec. v5, so it isn't a valid value sent
/// from the controller to the user.
const INVALID_CONNECTION_HANDLE: u16 = 0xFFFF;

fn get_address() -> Result<Option<bo_tie::BluetoothDeviceAddress>, String> {
    use std::io::stdin;
    use std::io::BufRead;

    println!("Input the bluetooth address in the format of XX:XX:XX:XX:XX:XX (most significant \
        byte -> least significant) to set the random address or hit enter to use the default");

    let mut buffer = String::new();

    let stdin = stdin();

    stdin.lock().read_line(&mut buffer).expect("Couldn't read input from terminal");

    if (buffer.len() == 1 && buffer.ends_with('\n')) ||
       (buffer.len() == 2 && buffer.ends_with("\r\n"))
    {
        Ok( None )
    } else {

        let adr_vec = buffer.rsplit(":")
            .map(|v| {
                u8::from_str_radix(v.trim(), 16).expect("Couldn't convert bluetooth address")
            })
            .collect::<Vec<u8>>();

        if adr_vec.len() == 6 {
            let mut address = bo_tie::BluetoothDeviceAddress::default();

            address.copy_from_slice(adr_vec.as_slice());

            Ok( Some(address) )
        }
        else {
            Err(format!("{} is not an address in the form of XX:XX:XX:XX:XX:XX", buffer))
        }
    }
}

/// This sets up the advertising and waits for the connection complete event
async fn advertise_setup<'a>(
    hi: &'a hci::HostInterface<bo_tie_linux::HCIAdapter>,
    local_name: &'a str,
    rand_address: Option<bo_tie::BluetoothDeviceAddress> )
{
    let adv_name = advertise::local_name::LocalName::new(local_name, false);

    let mut adv_flags = advertise::flags::Flags::new();

    // This is the flag specification for a LE-only, limited discoverable advertising
    adv_flags.get_core(advertise::flags::CoreFlags::LELimitedDiscoverableMode).enable();
    adv_flags.get_core(advertise::flags::CoreFlags::LEGeneralDiscoverableMode).disable();
    adv_flags.get_core(advertise::flags::CoreFlags::BREDRNotSupported).enable();
    adv_flags.get_core(advertise::flags::CoreFlags::ControllerSupportsSimultaniousLEAndBREDR).disable();
    adv_flags.get_core(advertise::flags::CoreFlags::HostSupportsSimultaniousLEAndBREDR).disable();

    // TODO add the Tx power, an example service UUID, and the slave connection interval range to
    //      the advertising data.

    let mut adv_data = set_advertising_data::AdvertisingData::new();

    adv_data.try_push(adv_name).unwrap();
    adv_data.try_push(adv_flags).unwrap();

    await!(set_advertising_enable::send(&hi, false)).unwrap();

    await!(set_advertising_data::send(&hi, adv_data)).unwrap();

    let mut adv_prams = set_advertising_parameters::AdvertisingParameters::default();

    adv_prams.own_address_type = bo_tie::hci::le::common::OwnAddressType::RandomDeviceAddress;

    if let Some(address) = rand_address {
        await!(set_random_address::send(&hi, address)).unwrap()
    }

    await!(set_advertising_parameters::send(&hi, adv_prams)).unwrap();

    await!(set_advertising_enable::send(&hi, true)).unwrap();
}

// For simplicity, I've left the race condition in here. There could be a case where the connection
// is made and the ConnectionComplete event isn't propicated & processed
async fn wait_for_connection<'a>(hi: &'a hci::HostInterface<bo_tie_linux::HCIAdapter>)
-> Result<hci::common::ConnectionHandle, impl std::fmt::Display>
{
    println!("Waiting for a connection (timeout is 60 seconds)");

    let evt_rsl = await!(hi.wait_for_event(events::LEMeta::ConnectionComplete.into(), Duration::from_secs(60)));

    await!(set_advertising_enable::send(&hi, false)).unwrap();

    match evt_rsl {
        Ok(event) => {
            use bo_tie::hci::events::{EventsData,LEMetaData};

            println!("Connection Made!");

            if let EventsData::LEMeta(LEMetaData::ConnectionComplete(le_conn_comp_event)) = event {
                Ok(le_conn_comp_event.connection_handle)
            }
            else {
                Err(format!("Received the incorrect event {:?}", event))
            }
        }
        Err(e) => {
            Err(format!("Timeout Occured: {:?}", e))
        }
    }
}

async fn disconnect(
    hi: &hci::HostInterface<bo_tie_linux::HCIAdapter>,
    connection_handle: hci::common::ConnectionHandle )
{
    use bo_tie::hci::le::connection::disconnect;

    let prams = disconnect::DisconnectParameters {
        connection_handle: connection_handle,
        disconnect_reason: disconnect::DisconnectReason::RemoteUserTerminatedConnection,
    };

    await!(disconnect::send(&hi, prams)).expect("Failed to disconnect");
}

fn handle_sig(
    hi: Arc<hci::HostInterface<bo_tie_linux::HCIAdapter>>,
    raw_handle: Arc<AtomicU16> )
{
    simple_signal::set_handler(&[simple_signal::Signal::Int, simple_signal::Signal::Term],
        move |_| {
            // Cancel advertising if advertising (there is no consequence if not advertising)
            futures::executor::block_on(set_advertising_enable::send(&hi, false)).unwrap();

            // todo fix the race condition where a connection is made but the handle hasn't been
            // stored here yet
            let handle_val = raw_handle.load(Ordering::SeqCst);

            if handle_val != INVALID_CONNECTION_HANDLE {

                let handle = bo_tie::hci::common::ConnectionHandle::try_from(handle_val).expect("Incorrect Handle");

                futures::executor::block_on(disconnect(&hi, handle));
            }
        }
    );
}

fn main() {
    use futures::executor;

    simple_logging::log_to_stderr(log::LevelFilter::Info);

    let address = get_address().unwrap();

    let raw_connection_handle = Arc::new(AtomicU16::new(INVALID_CONNECTION_HANDLE));

    let interface = Arc::new(hci::HostInterface::default());

    handle_sig(interface.clone(), raw_connection_handle.clone());

    // Its fine for the setup to be blocked on b/c its fast to the user
    executor::block_on(advertise_setup(&interface, "Connection Test", address));

    // Waiting for some bluetooth device to connect is slow, so the waiting for the future is done
    // on a different thread.
    match executor::block_on(wait_for_connection(&interface)) {
        Ok(handle) => {
            raw_connection_handle.store(handle.get_raw_handle(), Ordering::SeqCst);

            println!("Device Connected! (use ctrl-c to disconnect and exit)");

            loop { std::thread::park(); }
        },
        Err(err) => println!("Error: {}", err),
    };
}
