pub mod advertise;

use alloc::boxed::Box;

/// The minimum number of data bytes in an attribute protocol based packet for bluetooth le
pub const MIN_ATT_MTU_LE: u16 = 23;

/// The minimum number of data bytes in an attribute protocol based packet for bluetooth BR/EDR
pub const MIN_ATT_MTU_BR_EDR: u16 = 48;

pub trait ConnectionChannel {
    const DEFAULT_ATT_MTU: u16;
    fn send(&self, data: &[u8]);
    fn receive(&self, waker: core::task::Waker) -> Option<Box<[u8]>>;
}
