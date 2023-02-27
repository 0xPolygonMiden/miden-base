pub use imports::*;

#[cfg(not(feature = "std"))]
mod imports {
    pub use alloc::{
        string::{String, ToString},
        vec::Vec,
    };
}

#[cfg(feature = "std")]
mod imports {
    pub use std::{
        string::{String, ToString},
        vec::Vec,
    };
}
