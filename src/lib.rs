#![feature(futures_api)]
#![feature(pin)]
#![feature(arbitrary_self_types)]
#![feature(async_await)]
#![feature(await_macro)]
#![feature(alloc)]
#![feature(test)]
#![feature(try_from)]
#![feature(unsize)]

// These crates are used all the time
extern crate alloc;
extern crate serde;
extern crate bincode as serializer;

// test related
#[cfg(test)]
extern crate test;
#[cfg(any(test, target_os = "android"))]
#[macro_use]
extern crate lazy_static;

// So this library can be used with no_std targets
#[cfg_attr(not(any(
    test,
    unix,
)), no_std)]
extern crate core;

// Nix crate for just unix targets (except for android)
#[cfg(all(
    unix,
    not(target_os = "android"))
)]
extern crate nix;

// Android target related
#[cfg(target_os = "android")]
extern crate jni;
#[cfg(target_os = "android")]
pub mod android;

// Host Controller interface
#[cfg(not(target_os = "android"))]
pub mod hci;

// The rest is not target or os specific

pub mod gap;

pub type BluetoothDeviceAddress = [u8; 6];

pub fn bluetooth_address_from_string( addr: &str ) -> Result<BluetoothDeviceAddress, &'static str> {
    let mut address = BluetoothDeviceAddress::default();

    if let None = {
        let mut addr_itr = address.iter_mut();

        for val in addr.split(':').rev() {
            if let Some(byte) = addr_itr.next() {
                *byte = u8::from_str_radix(&val, 16).or(Err("Address contains invalid characters"))?;
            } else {
                return Err("Address contain too few bytes, all six are required");
            }
        }

        addr_itr.next()
    } {
        Ok(address)
    } else {
        Err("Address contains too many bytes, there are only six bytes in a bluetooth address")
    }
}

pub fn bluetooth_address_into_string( addr: BluetoothDeviceAddress ) -> String {
    format!("{}:{}:{}:{}:{}:{}", addr[5], addr[4], addr[3], addr[2], addr[1], addr[0])
}

/// Universially Unique Identifier
///
/// A UUID in bluetooth is used to idientify a Service and is part of many different protocols
/// with bluetooth.
///
/// This structure always handles UUIDs in their 128 bit value form.
#[derive(Clone,Copy,Debug,PartialEq,Eq,PartialOrd,Ord,Hash,Default)]
pub struct UUID {
    base_uuid: u128,
}

impl UUID {
    /// See V 5.0 Vol 3 part B sec 2.5.1 for where this value comes from.
    /// This can also be found as the Bluetooth Base UUID in the assigned numbers document.
    const BLUETOOTH_BASE_UUID: u128 = 0x0000000000001000800000805F9B34FB;
}

impl From<u128> for UUID {
    fn from(v: u128) -> UUID {
        UUID {
            base_uuid: v
        }
    }
}

impl From<u32> for UUID {
    fn from(v: u32) -> UUID {
        UUID {
            /// See V 5.0 Vol 3 part B sec 2.5.1 for this equation
            base_uuid: ((v as u128) << 96) + Self::BLUETOOTH_BASE_UUID,
        }
    }
}

impl From<u16> for UUID {
    fn from(v: u16) -> UUID {
        UUID {
            /// See V 5.0 Vol 3 part B sec 2.5.1 for this equation
            base_uuid: ((v as u128) << 96) + Self::BLUETOOTH_BASE_UUID,
        }
    }
}

impl From<UUID> for u128 {
    fn from(uuid: UUID) -> u128 {
        uuid.base_uuid
    }
}

impl std::convert::TryFrom<UUID> for u32 {
    type Error = ();

    /// Try to convert a UUID into its 32 bit shortened form. This doesn't check that the value is
    /// pre-allocated (assigned number).
    fn try_from(uuid: UUID) -> Result<u32, ()> {
        match !(((!0u32) as u128) << 96) | uuid.base_uuid {
            0 => Ok((uuid.base_uuid >> 96) as u32),
            _ => Err(())
        }
    }
}

impl std::convert::TryFrom<UUID> for u16 {
    type Error = ();

    /// Try to convert a UUID into its 32 bit shortened form. This doesn't check that the value is
    /// pre-allocated (assigned number).
    fn try_from(uuid: UUID) -> Result<u16, ()> {
        match !(((!0u16) as u128) << 96) | uuid.base_uuid {
            0 => Ok((uuid.base_uuid >> 96) as u16),
            _ => Err(())
        }
    }
}
