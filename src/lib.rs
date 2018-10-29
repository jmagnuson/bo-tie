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
#[cfg(unix)] extern crate nix;
#[cfg(test)] extern crate test;
#[cfg(test)] #[macro_use] extern crate lazy_static;

pub type BluetoothDeviceAddress = [u8; 6];

pub mod hci;
pub mod gap;
