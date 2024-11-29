extern crate alloc;

use alloc::string::String;

pub mod generated;

#[cfg(feature = "async")]
mod prover;
#[cfg(feature = "async")]
pub use prover::RemoteTransactionProver;

/// Contains the protobuf definitions
pub const PROTO_MESSAGES: &str = include_str!("../proto/api.proto");

/// Name of the configuration file
pub const PROVER_SERVICE_CONFIG_FILE_NAME: &str = "miden-tx-prover.toml";

/// ERRORS
/// ===============================================================================================

#[derive(Debug)]
pub enum RemoteTransactionProverError {
    /// Indicates that the provided gRPC server endpoint is invalid.
    InvalidEndpoint(String),

    /// Indicates that the connection to the server failed.
    ConnectionFailed(String),
}

impl std::fmt::Display for RemoteTransactionProverError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RemoteTransactionProverError::InvalidEndpoint(endpoint) => {
                write!(f, "Invalid endpoint: {}", endpoint)
            },
            RemoteTransactionProverError::ConnectionFailed(endpoint) => {
                write!(f, "Failed to connect to transaction prover at: {}", endpoint)
            },
        }
    }
}

impl core::error::Error for RemoteTransactionProverError {}
