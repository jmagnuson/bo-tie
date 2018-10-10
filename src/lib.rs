#![feature(test)]
#![feature(chunks_exact)]
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

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
mod bluez {
    use core::convert::From;

    #[link(name = "bluetooth", kind = "dylib")]
    extern "C" {}
    include!("../build/generated_bindings/bluetooth.rs");
    include!("../build/bindings_glue.rs");

    impl From<[u8; 6]> for bdaddr_t {
        fn from(arr: [u8; 6]) -> Self {
            bdaddr_t { b: arr }
        }
    }

    macro_rules! ptype_pram {
        ( $t:expr ) => {
            if $t == HCI_VENDOR_PKT {
                0
            } else {
                $t & HCI_FLT_TYPE_BITS
            }
        };
    }

    macro_rules! event_pram {
        ( $e:expr ) => {
            $e & HCI_FLT_EVENT_BITS
        };
    }

    /// Bluez inline methods (from hci_lib.h) for manipulating the bluetooth socket bits
    ///
    /// The function signatures should closly match the inline methods in the header files. I don't
    /// know what the parameter names mean or represent so they are just copied.
    ///
    /// # Note
    /// hci_filter_clear isn't here because its mostly pointless in rust
    impl hci_filter {
        #[inline]
        pub fn hci_filter_set_ptype(&mut self, t: u32) {
            self.type_mask |= 1 << (ptype_pram!(t) & 31)
        }

        #[inline]
        pub fn hci_filter_clear_ptype(&mut self, t: u32) {
            self.type_mask &= 1 << (ptype_pram!(t) & 31)
        }

        #[inline]
        pub fn hci_filter_test_ptype(&mut self, t: u32) -> u32 {
            self.type_mask & 1 << (ptype_pram!(t) & 31)
        }

        #[inline]
        pub fn hci_filter_set_event(&mut self, e: u32) {
            self.event_mask[e as usize >> 5] |= 1 << (event_pram!(e) & 31)
        }

        #[inline]
        pub fn hci_filter_clear_event(&mut self, e: u32) {
            self.event_mask[e as usize >> 5] &= 1 << (event_pram!(e) & 31)
        }

        #[inline]
        pub fn hci_filter_test_event(&mut self, e: u32) -> u32 {
            self.event_mask[e as usize >> 5] & 1 << (event_pram!(e) & 31)
        }
    }
}

/// This is an "unpacked" version of the bdaddr_t address type
type BluetoothDeviceAddress = [u8; 6];

pub mod hci;
pub mod gap;
