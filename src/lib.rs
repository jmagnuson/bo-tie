#![feature(test)]
#![feature(futures_api)]
#![feature(pin)]
#![feature(arbitrary_self_types)]
#![feature(async_await)]
#![feature(await_macro)]
#![feature(alloc)]

#[cfg_attr(not(any(test,unix)),no_std)]

extern crate core;
extern crate alloc;

#[cfg(all(unix, not(target_os = "android")))] extern crate nix;

#[cfg(target_os = "android")] extern crate jni;

#[cfg(test)] extern crate test;
#[cfg(test)] #[cfg_attr(test,macro_use)] extern crate lazy_static;

pub type BluetoothDeviceAddress = [u8; 6];

#[cfg(not(target_os = "android"))] pub mod hci;
pub mod gap;
