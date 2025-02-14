pub mod generated;

use alloc::{
    boxed::Box,
    string::{String, ToString},
};

use generated::api_client::ApiClient;
use miden_objects::transaction::{ProvenTransaction, TransactionWitness};
use miden_tx::{utils::sync::RwLock, TransactionProver, TransactionProverError};

use crate::RemoteProverError;

#[cfg(feature = "tx-prover")]
pub mod tx_prover;

#[cfg(feature = "batch-prover")]
pub mod batch_prover;
