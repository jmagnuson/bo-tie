//! Connection state in the slave role example
//!
//! This example shows the basic way to form a connection with this device in the slave role. The
//! only real important part to look at are the async functions.
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
//! Super User privileges may be required to interact with your bluetooth peripheral. To do will
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

use bo_tie:: {
    att,
    gap::advertise,
    gatt,
    hci,
    hci::events,
    hci::le::transmitter::{
        set_advertising_data,
        set_advertising_parameters,
        set_advertising_enable,
    },
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

    set_advertising_enable::send(&hi, false).await.unwrap();

    set_advertising_data::send(&hi, adv_data).await.unwrap();

    let mut adv_prams = set_advertising_parameters::AdvertisingParameters::default();

    adv_prams.own_address_type = bo_tie::hci::le::common::OwnAddressType::RandomDeviceAddress;

    set_advertising_parameters::send(&hi, adv_prams).await.unwrap();

    set_advertising_enable::send(&hi, true).await.unwrap();
}

// For simplicity, I've left the race condition in here. There could be a case where the connection
// is made and the ConnectionComplete event isn't propicated & processed
async fn wait_for_connection(hi: &hci::HostInterface<bo_tie_linux::HCIAdapter>)
-> Result<hci::events::LEConnectionCompleteData, impl std::fmt::Display>
{
    println!("Waiting for a connection (timeout is 60 seconds)");

    let evt_rsl = hi.wait_for_event(events::LEMeta::ConnectionComplete.into(), Duration::from_secs(60)).await;

    match evt_rsl {
        Ok(event) => {
            use bo_tie::hci::events::{EventsData,LEMetaData};

            if let EventsData::LEMeta(LEMetaData::ConnectionComplete(event_data)) = event {

                Ok(event_data)
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

    disconnect::send(&hi, prams).await.expect("Failed to disconnect");
}

/// Initialize the Attribute Server
///
/// The attribute server is organized via the gatt protocol. This example is about connecting
/// to a client and not about featuring the attribue server, so only the minimalistic gatt server
/// is present.
fn gatt_server_init<C>(channel: &C, local_name: &str) -> gatt::Server<C>
where C: bo_tie::l2cap::ConnectionChannel
{
    let att_mtu = 256;

    let gsb = gatt::GapServiceBuilder::new(local_name, None);

    let mut server = gatt::ServerBuilder::new_with_gap(gsb).make_server(channel, att_mtu);

    server.as_mut().give_permission_to_client(att::AttributePermissions::Read);

    server
}

fn server_loop<C>(
    hi: &hci::HostInterface<bo_tie_linux::HCIAdapter>
    connection_channel: C,
    mut att_server: gatt::Server<C>,
    mut slave_security_manager: SlaveSecurityManager<'_,C>
)
where C: bo_tie::l2cap::ConnectionChannel
{
    use bo_tie::l2cap::ChannelIdentifier;
    use bo_tie::l2cap::LeUserChannelIdentifier;
    use bo_tie::hci::le::encryption::start_encryption;
    use bo_tie::hci::cb::set_event_mask;
    use bo_tie::hci::events::Events;
    use core::time::Duration;

    loop {
        futures::executor::block_on(async {
            connection_channel.future_receiver()
            .await
            .iter()
            .for_each(|acl_data| {
                match acl_data.get_cahnnel_id() {
                    ChannelIdentifier::Le(LeUserChannelIdentifier::AttributeProtocol) =>
                        match att_server.process_acl_data(acl_data) {
                            Ok(_) => (),
                            Err(e) => println!("Cannot process acl data for ATT, '{}'", e),
                        }
                    ChannelIdentifier::Le(LeUserChannelIdentifier::SecurityManagerProtocol) =>
                        match slave_security_manager.process_command(acl_data) {
                            Ok(false) => (),
                            Err(e) => println!("Cannot process acl data for SM, '{:?}'", e),
                            Ok(true) => {

                                let enabled_events = &[
                                    set_event_mask::EventMask::EncryptionChange,
                                    set_event_mask::EventMask::EncryptionKeyRefreshComplete,
                                ];

                                set_event_mask::send(hi, enabled_events).await.unwrap();

                                let e_change_fut = hi.wait_for_event(
                                    Events::EncryptionChange,
                                    Duration::from_secs(1)
                                );

                                let e_key_refresh_fut = hi.wait_for_event(
                                    Events::EncryptionKeyRefreshComplete,
                                    Duration::from_secs(1)
                                );

                                match futures::future::select(e_change_fut, e_key_refresh_fut).await {
                                    futures::future::Either::Left(r) => r.unwrap(),
                                    futures::future::Either::Right(r) => r.unwrap(),
                                }

                                // keys can now be exchanged
                            }
                        }
                }
            })
        })
    }
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

    let local_name = "Connection Test";

    TermLogger::init( LevelFilter::Trace, Config::default(), TerminalMode::Mixed ).unwrap();

    let raw_connection_handle = Arc::new(AtomicU16::new(INVALID_CONNECTION_HANDLE));

    let interface = Arc::new(hci::HostInterface::default());

    handle_sig(interface.clone(), raw_connection_handle.clone());

    executor::block_on(advertise_setup(&interface, local_name));

    // Waiting for some bluetooth device to connect is slow, so the waiting for the future is done
    // on a different thread.
    match executor::block_on(wait_for_connection(&interface)) {
        Ok(event_data) => {
            
            raw_connection_handle.store(event_data.connection_handle.get_raw_handle(), Ordering::SeqCst);

            let interface_clone = interface.clone();

            let master_address = event_data.peer_address.clone();

            let master_address_type = event_data.peer_address_type.clone();

            let this_address = executor::block_on(
                bo_tie::hci::le::mandatory::ip_read_bd_addr::send(&interface)
            ).unwrap();

            std::thread::spawn( move || {

                let connection_channel = interface_clone.new_le_acl_connection_channel(&event_data);

                let server = gatt_server_init(&connection_channel, local_name);

                let sm = bo_tie::sm::SecurityManager::new(Vec::new());

                let slave_sm = sm.new_slave_security_manager_builder(
                    &interface_clone,
                    server.as_ref().as_ref(),
                    &master_address,
                    master_address_type == bo_tie::hci::events::LEConnectionAddressType::RandomDeviceAddress,
                    &this_address,
                    false
                )
                .set_min_and_max_encryption_key_size(16,16).unwrap()
                .create_security_manager();

                server_loop(connection_channel, server, slave_sm);
            });

            executor::block_on(set_advertising_enable::send(&interface, false)).unwrap();

            println!("Device Connected! (use ctrl-c to disconnect and exit)");

            executor::block_on(interface.wait_for_event(events::Events::DisconnectionComplete, None)).ok();
        },
        Err(err) => println!("Error: {}", err),
    };
}
