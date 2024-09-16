#[rustfmt::skip]
pub mod generated;

pub use generated::api::{ProveTransactionRequest, ProveTransactionResponse};
#[cfg(feature = "async")]
pub mod remote_tx_prover;
