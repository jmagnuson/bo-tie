#![feature(futures_api)]
#![feature(pin)]
#![feature(arbitrary_self_types)]
#![feature(async_await)]
#![feature(await_macro)]
#![feature(alloc)]

// So this library can be used with no_std targets
#[cfg_attr(not(any(
    test,
    unix,
)), no_std)]
extern crate core;
extern crate alloc;

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
#[cfg(not(
    target_os = "android",
))]
pub mod hci;

// The rest is not target or os specific

pub mod gap;

pub type BluetoothDeviceAddress = [u8; 6];
