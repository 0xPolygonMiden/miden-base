pub mod generated;

use crate::RemoteProverError;

#[cfg(feature = "tx-prover")]
pub mod tx_prover;

#[cfg(feature = "batch-prover")]
pub mod batch_prover;

#[cfg(feature = "block-prover")]
pub mod block_prover;
