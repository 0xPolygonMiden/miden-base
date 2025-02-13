use miden_crypto::merkle::MerkleError;
use miden_objects::account::AccountId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProvenBlockError {
    #[error("nullifier witness has a different root than the current nullifier tree root")]
    NullifierWitnessRootMismatch { source: MerkleError },
    #[error("account witness for account {account_id} has a different root than the current account tree root")]
    AccountWitnessRootMismatch {
        account_id: AccountId,
        source: MerkleError,
    },
}
