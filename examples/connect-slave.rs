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
//! # Important Notes
//! Super User privaleges may be required to interact with your bluetooth peripheral. To do will
//! probably require the full path to cargo. The cargo binary is usually locacted in your home
//! directory at `.cargo/bin/cargo`.
//!
//! This example assumes there isn't any bonding/caching between the device that is to be connected
//! with this example. This will cause the the example to get stuck and eventually time out waiting
//! to connect to the device. If this occurs, using a different random address should work (or
//! power cycle the bluetooth controller to get a newly generated default random address). If
//! there are still problems, delete the cache, whitelist, and any other memory associted with the
//! bluetooth on the device to connect with, but please note this will git rid of all information
//! associated with the bluetooth and other devices will need to be reconnected.

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
};
use std::sync::{Arc, atomic::{AtomicU16, Ordering}};
use std::time::Duration;

/// 0xFFFF is a reserved value as of the Bluetooth Spec. v5, so it isn't a valid value sent
/// from the controller to the user.
const INVALID_CONNECTION_HANDLE: u16 = 0xFFFF;

/// This sets up the advertising and waits for the connection complete event
async fn advertise_setup<'a>(
    hi: &'a hci::HostInterface<bo_tie_linux::HCIAdapter>,
    local_name: &'a str )
{
    let adv_name = advertise::local_name::LocalName::new(local_name, false);

    let mut adv_flags = advertise::flags::Flags::new();

    // This is the flag specification for a LE-only, limited discoverable advertising
    adv_flags.get_core(advertise::flags::CoreFlags::LELimitedDiscoverableMode).enable();
    adv_flags.get_core(advertise::flags::CoreFlags::LEGeneralDiscoverableMode).disable();
    adv_flags.get_core(advertise::flags::CoreFlags::BREDRNotSupported).enable();
    adv_flags.get_core(advertise::flags::CoreFlags::ControllerSupportsSimultaniousLEAndBREDR).disable();
    adv_flags.get_core(advertise::flags::CoreFlags::HostSupportsSimultaniousLEAndBREDR).disable();

    let mut adv_data = set_advertising_data::AdvertisingData::new();

    adv_data.try_push(adv_flags).unwrap();
    adv_data.try_push(adv_name).unwrap();

    await!(set_advertising_enable::send(&hi, false)).unwrap();

    await!(set_advertising_data::send(&hi, adv_data)).unwrap();

    let mut adv_prams = set_advertising_parameters::AdvertisingParameters::default();

    adv_prams.own_address_type = bo_tie::hci::le::common::OwnAddressType::RandomDeviceAddress;

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

                println!("Bluetooth connection terminated")
            }

            println!("Exiting example");

            std::process::exit(0);
        }
    );
}

fn main() {
    use futures::executor;
    use simplelog::{TermLogger, LevelFilter, Config, TerminalMode};

    TermLogger::init( LevelFilter::Debug, Config::default(), TerminalMode::Mixed ).unwrap();

    let raw_connection_handle = Arc::new(AtomicU16::new(INVALID_CONNECTION_HANDLE));

    let interface = Arc::new(hci::HostInterface::default());

    handle_sig(interface.clone(), raw_connection_handle.clone());

    executor::block_on(advertise_setup(&interface, "Connection Test"));

    // Waiting for some bluetooth device to connect is slow, so the waiting for the future is done
    // on a different thread.
    match executor::block_on(wait_for_connection(&interface)) {
        Ok(handle) => {
            raw_connection_handle.store(handle.get_raw_handle(), Ordering::SeqCst);

            println!("Device Connected! (use ctrl-c to disconnect and exit)");

            executor::block_on(interface.wait_for_event(events::Events::DisconnectionComplete, None)).ok();
        },
        Err(err) => println!("Error: {}", err),
    };
}
