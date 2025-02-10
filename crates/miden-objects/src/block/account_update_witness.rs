use alloc::vec::Vec;

use miden_crypto::merkle::MerklePath;
use vm_processor::Digest;

use crate::{account::delta::AccountUpdateDetails, transaction::TransactionId};

/// TODO
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountUpdateWitness {
    initial_state_commitment: Digest,
    final_state_commitment: Digest,
    initial_state_proof: MerklePath,
    /// A set of changes which can be applied to the previous account state (i.e., the state as of
    /// the last block) to get the new account state. For private accounts, this is set to
    /// [AccountUpdateDetails::Private].
    details: AccountUpdateDetails,
    transactions: Vec<TransactionId>,
}

impl AccountUpdateWitness {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Constructs a new, partial [`AccountUpdateWitness`] from the provided parts.
    pub fn new(
        initial_state_commitment: Digest,
        final_state_commitment: Digest,
        initial_state_proof: MerklePath,
        details: AccountUpdateDetails,
        transactions: Vec<TransactionId>,
    ) -> Self {
        Self {
            initial_state_commitment,
            final_state_commitment,
            initial_state_proof,
            details,
            transactions,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the initial state commitment of the account.
    pub fn initial_state_commitment(&self) -> Digest {
        self.initial_state_commitment
    }

    /// Returns the final state commitment of the account.
    pub fn final_state_commitment(&self) -> Digest {
        self.final_state_commitment
    }

    /// Returns a reference to the initial state proof of the account.
    pub fn initial_state_proof(&self) -> &MerklePath {
        &self.initial_state_proof
    }

    /// Returns a reference to the underlying [`AccountUpdateDetails`] of this update.
    pub fn details(&self) -> &AccountUpdateDetails {
        &self.details
    }

    /// Returns the transactions that affected the account.
    pub fn transactions(&self) -> &[TransactionId] {
        &self.transactions
    }

    // STATE MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Returns a mutable reference to the initial state proof of the account.
    pub fn initial_state_proof_mut(&mut self) -> &mut MerklePath {
        &mut self.initial_state_proof
    }

    /// Consumes self and returns its parts.
    pub fn into_parts(self) -> (Digest, Digest, MerklePath, Vec<TransactionId>) {
        (
            self.initial_state_commitment,
            self.final_state_commitment,
            self.initial_state_proof,
            self.transactions,
        )
    }
}
