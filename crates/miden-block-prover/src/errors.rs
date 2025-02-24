use miden_crypto::merkle::MerkleError;
use miden_objects::{account::AccountId, Digest, NullifierTreeError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProvenBlockError {
    #[error("nullifier witness has a different root than the current nullifier tree root")]
    NullifierWitnessRootMismatch(#[source] NullifierTreeError),

    #[error("account witness for account {account_id} has a different root than the current account tree root")]
    AccountWitnessRootMismatch {
        account_id: AccountId,
        source: MerkleError,
    },

    #[error("account tree root of the previous block header is {prev_block_account_root} but the root of the partial tree computed from account witnesses is {stale_account_root}, indicating that the witnesses are stale")]
    StaleAccountTreeRoot {
        prev_block_account_root: Digest,
        stale_account_root: Digest,
    },

    #[error("nullifier tree root of the previous block header is {prev_block_nullifier_root} but the root of the partial tree computed from nullifier witnesses is {stale_nullifier_root}, indicating that the witnesses are stale")]
    StaleNullifierTreeRoot {
        prev_block_nullifier_root: Digest,
        stale_nullifier_root: Digest,
    },
}
