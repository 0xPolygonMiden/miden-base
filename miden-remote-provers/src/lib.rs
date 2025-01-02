extern crate alloc;

use alloc::string::String;

#[cfg(feature = "tx-prover")]
pub mod generated;

#[cfg(feature = "tx-prover")]
mod prover;
#[cfg(feature = "tx-prover")]
pub use prover::RemoteTransactionProver;

/// Contains the protobuf definitions
pub const PROTO_MESSAGES: &str = include_str!("../proto/api.proto");

/// ERRORS
/// ===============================================================================================

#[derive(Debug)]
pub enum RemoteProverError {
    /// Indicates that the provided gRPC server endpoint is invalid.
    InvalidEndpoint(String),

    /// Indicates that the connection to the server failed.
    ConnectionFailed(String),
}

impl std::fmt::Display for RemoteProverError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RemoteProverError::InvalidEndpoint(endpoint) => {
                write!(f, "Invalid endpoint: {}", endpoint)
            },
            RemoteProverError::ConnectionFailed(endpoint) => {
                write!(f, "Failed to connect to remote prover at: {}", endpoint)
            },
        }
    }
}

impl core::error::Error for RemoteProverError {}
