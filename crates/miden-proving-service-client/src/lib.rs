#![no_std]
// We allow unused imports here in order because this `macro_use` only makes sense for code
// generated by prost under certain circumstances (when `tx-prover` is enabled and the environment
// is not wasm)
#![allow(unused_imports)]
#[macro_use]
extern crate alloc;
use alloc::string::{String, ToString};

#[cfg(feature = "std")]
extern crate std;

use thiserror::Error;

#[cfg(feature = "tx-prover")]
pub mod tx_prover;

/// Protobuf definition for the Miden proving service
pub const TX_PROVER_PROTO: &str = include_str!("../proto/tx_prover.proto");

/// ERRORS
/// ===============================================================================================

#[derive(Debug, Error)]
pub enum RemoteProverError {
    /// Indicates that the provided gRPC server endpoint is invalid.
    #[error("invalid uri {0}")]
    InvalidEndpoint(String),
    #[error("failed to connect to prover {0}")]
    /// Indicates that the connection to the server failed.
    ConnectionFailed(String),
}

impl From<RemoteProverError> for String {
    fn from(err: RemoteProverError) -> Self {
        err.to_string()
    }
}
