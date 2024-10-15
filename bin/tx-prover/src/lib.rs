extern crate alloc;
use alloc::string::String;

pub(crate) mod generated;

#[cfg(any(feature = "async"))]
mod prover;
#[cfg(any(feature = "async"))]
pub use prover::RemoteTransactionProver;

/// Contains the protobuf definitions
pub const PROTO_MESSAGES: &str = include_str!("../proto/api.proto");

/// ERRORS
/// ===============================================================================================

#[derive(Debug)]
pub enum RemoteTransactionProverError {
    /// Indicates that the provided gRPC server endpoint is invalid.
    InvalidEndpoint(String),

    /// Indicates that the connection to the server failed.
    ConnectionFailed(String),
}
