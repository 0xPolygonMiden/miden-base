extern crate alloc;

use alloc::string::String;

use thiserror::Error;

#[cfg(feature = "tx-prover")]
pub mod generated;

#[cfg(feature = "tx-prover")]
mod prover;
#[cfg(feature = "tx-prover")]
pub use prover::RemoteTransactionProver;

/// Contains the protobuf definitions
pub const PROTO_MESSAGES: &str = include_str!("../../proto/api.proto");

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
