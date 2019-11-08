//! Bluetooth Security Manager
//!
//! The Security Manager contains the algorithms and protocols for the pairing processes between two
//! connected devices.
//!
//! # The Algorithms
//! The names of each algorithm match the names as stated in the Security Manager section of the
//! Host Volume of the Bluetooth Specification (V 5.0 | Vol 3, Part H, Section 2.2). Unfortunately
//! these names are shortened, making them obtuse to understand going by their name.
//!
//! The security function *e* is built using the functions [`ah`], [`c1`], and [`s1`].
//!
//! The security function AES-CMAC is built using the functions ['f4'], ['f5'], ['f6'], and ['g2']

pub mod toolbox;

