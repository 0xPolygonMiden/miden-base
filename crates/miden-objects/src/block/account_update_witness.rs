use alloc::vec::Vec;

use crate::{
    account::delta::AccountUpdateDetails,
    crypto::merkle::MerklePath,
    transaction::TransactionId,
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest,
};

/// This type encapsulates essentially three components:
/// - The witness is a merkle path of the initial state commitment of the account before the block
///   in which the witness is included, that is, in the account tree at the state of the previous
///   block header.
/// - The account update details represent the delta between the state of the account before the
///   block and the state after this block.
/// - Additionally contains a list of transaction IDs that contributed to this update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountUpdateWitness {
    /// The state commitment before the update.
    initial_state_commitment: Digest,
    /// The state commitment after the update.
    final_state_commitment: Digest,
    /// The merkle path for the account tree proving that the initial state commitment is the
    /// current state.
    initial_state_proof: MerklePath,
    /// A set of changes which can be applied to the previous account state (i.e., the state as of
    /// the last block, equivalent to `initial_state_commitment`) to get the new account state. For
    /// private accounts, this is set to [`AccountUpdateDetails::Private`].
    details: AccountUpdateDetails,
    /// All transaction IDs that contributed to this account update.
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

    /// Returns a reference to the underlying [`AccountUpdateDetails`] of this update, representing
    /// the state transition of the account from the previous block to the block this witness is
    /// for.
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
    pub fn into_parts(
        self,
    ) -> (Digest, Digest, MerklePath, AccountUpdateDetails, Vec<TransactionId>) {
        (
            self.initial_state_commitment,
            self.final_state_commitment,
            self.initial_state_proof,
            self.details,
            self.transactions,
        )
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountUpdateWitness {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write(self.initial_state_commitment);
        target.write(self.final_state_commitment);
        target.write(&self.initial_state_proof);
        target.write(&self.details);
        target.write(&self.transactions);
    }
}

impl Deserializable for AccountUpdateWitness {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let initial_state_commitment = source.read()?;
        let final_state_commitment = source.read()?;
        let initial_state_proof = source.read()?;
        let details = source.read()?;
        let transactions = source.read()?;
        Ok(AccountUpdateWitness {
            initial_state_commitment,
            final_state_commitment,
            initial_state_proof,
            details,
            transactions,
        })
    }
}
