extern crate alloc;

pub(crate) mod generated;

#[cfg(feature = "async")]
mod prover;
#[cfg(feature = "async")]
pub use prover::RemoteTransactionProver;

/// Contains the protobuf definitions
pub const PROTO_MESSAGES: &str = include_str!("../proto/api.proto");
