#![feature(async_await)]
#![feature(await_macro)]
#![feature(futures_api)]
#![feature(gen_future)]

extern crate bo_tie;

use bo_tie::hci;
use bo_tie::hci::events::EventsData;
use std::task;
use std::thread;
use std::time::Duration;

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

fn is_desired_device<T> (expcted: T, to_compare: T ) -> bool where T: PartialEq + ::std::fmt::Display {
    if expcted == to_compare {
        println!("Found the device");
        true
    }
    else {
        println!(r#"Found a device with local name "{}", expected "{}""#, to_compare, expcted);
        false
    }
}

async fn remove_from_white_list(hi: &hci::HostInterface, address: bo_tie::BluetoothDeviceAddress) {
    use bo_tie::hci::le::mandatory::remove_device_from_white_list::send;
    use bo_tie::hci::le::common::AddressType::RandomDeviceAddress;

    await!(send(&hi, RandomDeviceAddress, address).unwrap()).unwrap();
}

async fn scan_for_local_name<'a>( hi: &'a hci::HostInterface, name: &'a str )
    -> Option<Box<::bo_tie::hci::events::LEAdvertisingReportData>>
{
    use bo_tie::gap::advertise::{local_name, TryFromRaw};
    use bo_tie::hci::events::{EventsData, LEMeta, LEMetaData};
    use bo_tie::hci::le::mandatory::set_event_mask;
    use bo_tie::hci::le::receiver::{ set_scan_parameters, set_scan_enable };

    let mut scan_prms = set_scan_parameters::ScanningParameters::default();

    scan_prms.scan_type = set_scan_parameters::LEScanType::PassiveScanning;
    scan_prms.scanning_filter_policy = set_scan_parameters::ScanningFilterPolicy::AcceptAll;

    let le_event = LEMeta::AdvertisingReport;

    await!(set_scan_enable::send(&hi, false, false).unwrap()).unwrap();

    await!(set_event_mask::send(&hi, vec![le_event]).unwrap()).unwrap();

    await!(set_scan_parameters::send(&hi, scan_prms).unwrap()).unwrap();

    await!(set_scan_enable::send(&hi, true, true).unwrap()).unwrap();

    // This will stop 15 seconds after the last advertising packet is received
    while let Ok(event) = await!(hi.wait_for_event(le_event.into(), Duration::from_secs(5)).unwrap())
    {
        if let EventsData::LEMeta(LEMetaData::AdvertisingReport(reports)) = event {
            for report in reports.iter() {
                for data_rsl in report.data_iter() {
                    if let Ok(local_name) = local_name::LocalName::try_from_raw(data_rsl.unwrap()) {
                        if is_desired_device( name, local_name.as_ref()) {
                            await!(set_scan_enable::send(&hi, false, false).unwrap()).unwrap();
                            return Some(Box::new(report.clone()));
                        }
                    }
                }
            }
        }
    }

    await!(set_scan_enable::send(&hi, false, false).unwrap()).unwrap();
    println!("Couldn't find the device");

    None
}

async fn connect( hi: &hci::HostInterface, address: bo_tie::BluetoothDeviceAddress)
    -> Result<EventsData, impl std::fmt::Debug>
{
    use bo_tie::hci::common;
    use bo_tie::hci::events::LEMeta;
    use bo_tie::hci::le::common::OwnAddressType;
    use bo_tie::hci::le::connection;
    use bo_tie::hci::le::connection::create_connection;
    use bo_tie::hci::le::mandatory::set_event_mask;
    use std::time::Duration;

    let connect_event = LEMeta::ConnectionComplete;

    let min_connection_interval = Duration::from_millis(10);
    let max_connection_interval = Duration::from_millis(40);
    let slave_latency = 0;
    let supervision_timeout = Duration::from_secs(5);

    let parameters = create_connection::ConnectionParameters::new_without_whitelist(
        create_connection::ScanningInterval::default(),
        create_connection::ScanningWindow::default(),
        common::LEAddressType::RandomDeviceAddress,
        address,
        OwnAddressType::default(),
        connection::ConnectionIntervalBounds::try_from(
            connection::ConnectionInterval::try_from_duration(min_connection_interval).unwrap(),
            connection::ConnectionInterval::try_from_duration(max_connection_interval).unwrap(),
        ).unwrap(),
        common::ConnectionLatency::try_from(slave_latency).unwrap(),
        common::SupervisionTimeout::try_from_duration(supervision_timeout).unwrap(),
        connection::ConnectionEventLength::new(0x0, 0x1000),
    );

    // enable the LEConnectionComplete event
    await!(set_event_mask::send(&hi, vec![connect_event]).unwrap()).unwrap();

    // create the connection
    await!(create_connection::send(&hi, parameters).unwrap()).unwrap();

    // wait for the LEConnectionUpdate event
    await!(hi.wait_for_event(connect_event.into(), Duration::from_secs(25)).unwrap())
}

async fn cancel_connect(hi: &hci::HostInterface ) {
    use bo_tie::hci::le::connection::create_connection_cancel;

    await!(create_connection_cancel::send(&hi).unwrap()).unwrap();
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
    use std::io::stdin;
    use std::io::BufRead;
    use bo_tie::hci::events::{LEMetaData, EventsData};

    let host_interface = hci::HostInterface::default();

    let mut name = String::new();

    println!("Enter local name of device to connect to");

    let stdin = stdin();

    stdin.lock().read_line(&mut name).expect("Couldn't read input from terminal");

    name = name.trim().to_string();

    // wait!(scan_for_local_name(&host_interface, &name));

    if let Some(adv_report) = wait!(scan_for_local_name(&host_interface, &name)) {
        let address = adv_report.address;

        // Since the example uses `new_whithout_whitelist` (in function `connect`) it needs to be
        // removed from the whitelist in case it is already in the whitelist
        wait!(remove_from_white_list(&host_interface, address));

        match wait!(connect(&host_interface, address)) {
            Ok(EventsData::LEMeta(LEMetaData::ConnectionComplete(data))) =>  {

                println!("Connected! ... waiting 5 seconds then disconnecting");

                ::std::thread::sleep(::std::time::Duration::from_secs(5));

                wait!(disconnect(&host_interface, data.connection_handle));

                // its weird to remove the device from the white list here, but because this is an example
                // and you're probably connecting with some test device you don't probably want this
                // device in your whitelist.
                wait!(remove_from_white_list(&host_interface, address));
            }
            Err(e) => {
                wait!(cancel_connect(&host_interface));
                println!("Couldn't connect: {:?}", e);
            }
            Ok(_) => println!("Any other event should never be returned"),
        }
    }
}
