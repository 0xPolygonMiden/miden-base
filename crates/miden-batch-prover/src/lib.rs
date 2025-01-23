#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(any(feature = "testing", test))]
pub mod testing;
