use alloc::{string::ToString, vec::Vec};

use miden_verifier::ExecutionProof;

use super::{AccountId, Digest, InputNotes, Nullifier, OutputNote, OutputNotes, TransactionId};
use crate::{
    accounts::{Account, AccountDelta},
    utils::serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    ProvenTransactionError,
};

// PROVEN TRANSACTION
// ================================================================================================

/// Result of executing and proving a transaction. Contains all the data required to verify that a
/// transaction was executed correctly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenTransaction {
    /// A unique identifier for the transaction, see [TransactionId] for additional details.
    id: TransactionId,

    /// ID of the account that the transaction was executed against.
    account_id: AccountId,

    /// The hash of the account before the transaction was executed.
    ///
    /// Set to `Digest::default()` for new accounts.
    initial_account_hash: Digest,

    /// The hash of the account after the transaction was executed.
    final_account_hash: Digest,

    /// Optional account state changes used for on-chain accounts, This data is used to update an
    /// on-chain account's state after a local transaction execution.
    account_delta: Option<AccountDelta>,

    /// A list of nullifiers for all notes consumed by the transaction.
    input_notes: InputNotes<Nullifier>,

    /// The id and  metadata of all notes created by the transaction.
    output_notes: OutputNotes,

    /// The script root of the transaction, if one was used.
    tx_script_root: Option<Digest>,

    /// The block hash of the last known block at the time the transaction was executed.
    block_ref: Digest,

    /// A STARK proof that attests to the correct execution of the transaction.
    proof: ExecutionProof,
}

impl ProvenTransaction {
    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns unique identifier of this transaction.
    pub fn id(&self) -> TransactionId {
        self.id
    }

    /// Returns ID of the account against which this transaction was executed.
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns the initial account state hash.
    pub fn initial_account_hash(&self) -> Digest {
        self.initial_account_hash
    }

    /// Returns the final account state hash.
    pub fn final_account_hash(&self) -> Digest {
        self.final_account_hash
    }

    /// Returns the delta for on-chain accounts changes.
    pub fn account_delta(&self) -> Option<&AccountDelta> {
        self.account_delta.as_ref()
    }

    /// Returns a reference to the notes consumed by the transaction.
    pub fn input_notes(&self) -> &InputNotes<Nullifier> {
        &self.input_notes
    }

    /// Returns a reference to the notes produced by the transaction.
    pub fn output_notes(&self) -> &OutputNotes {
        &self.output_notes
    }

    /// Returns the script root of the transaction.
    pub fn tx_script_root(&self) -> Option<Digest> {
        self.tx_script_root
    }

    /// Returns the proof of the transaction.
    pub fn proof(&self) -> &ExecutionProof {
        &self.proof
    }

    /// Returns the block reference the transaction was executed against.
    pub fn block_ref(&self) -> Digest {
        self.block_ref
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    fn validate(self) -> Result<Self, ProvenTransactionError> {
        if self.account_id.is_on_chain() {
            match self.account_delta {
                None => {
                    return Err(ProvenTransactionError::OnChainAccountMissingDelta(self.account_id))
                },
                Some(ref delta) => {
                    let is_new_account = self.initial_account_hash == Digest::default();
                    if is_new_account {
                        if delta.nonce().is_none() {
                            return Err(ProvenTransactionError::NewOnChainAccountMissingNonce(
                                self.account_id,
                            ));
                        }

                        if delta.code().is_none() {
                            return Err(ProvenTransactionError::NewOnChainAccountMissingCode(
                                self.account_id,
                            ));
                        }

                        let final_account = Account::from_delta(self.account_id, delta)
                            .map_err(ProvenTransactionError::AccountError)?;
                        if final_account.hash() != self.final_account_hash {
                            return Err(ProvenTransactionError::AccountFinalHashMismatch(
                                self.final_account_hash,
                                final_account.hash(),
                            ));
                        }
                    }
                },
            }
        } else if self.account_delta.is_some() {
            return Err(ProvenTransactionError::OffChainAccountWithDelta(self.account_id));
        }

        Ok(self)
    }
}

// PROVEN TRANSACTION BUILDER
// ================================================================================================

/// Builder for a proven transaction.
#[derive(Clone, Debug)]
pub struct ProvenTransactionBuilder {
    /// ID of the account that the transaction was executed against.
    account_id: AccountId,

    /// The hash of the account before the transaction was executed.
    initial_account_hash: Digest,

    /// The hash of the account after the transaction was executed.
    final_account_hash: Digest,

    /// State changes to the account due to the transaction.
    account_delta: Option<AccountDelta>,

    /// List of [Nullifier]s of all consumed notes by the transaction.
    input_notes: Vec<Nullifier>,

    /// List of [NoteEnvelope]s of all notes created by the transaction.
    output_notes: Vec<OutputNote>,

    /// The script root of the transaction, if one was used.
    tx_script_root: Option<Digest>,

    /// Block [Digest] of the transaction's reference block.
    block_ref: Digest,

    /// A STARK proof that attests to the correct execution of the transaction.
    proof: ExecutionProof,
}

impl ProvenTransactionBuilder {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a [ProvenTransactionBuilder] used to build a [ProvenTransaction].
    pub fn new(
        account_id: AccountId,
        initial_account_hash: Digest,
        final_account_hash: Digest,
        block_ref: Digest,
        proof: ExecutionProof,
    ) -> Self {
        Self {
            account_id,
            initial_account_hash,
            final_account_hash,
            account_delta: None,
            input_notes: Vec::new(),
            output_notes: Vec::new(),
            tx_script_root: None,
            block_ref,
            proof,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Sets the account's delta.
    pub fn account_delta(mut self, account_delta: AccountDelta) -> Self {
        self.account_delta = Some(account_delta);
        self
    }

    /// Add notes consumed by the transaction.
    pub fn add_input_notes<T>(mut self, notes: T) -> Self
    where
        T: IntoIterator<Item = Nullifier>,
    {
        self.input_notes.extend(notes);
        self
    }

    /// Add notes produced by the transaction.
    pub fn add_output_notes<T>(mut self, notes: T) -> Self
    where
        T: IntoIterator<Item = OutputNote>,
    {
        self.output_notes.extend(notes);
        self
    }

    /// Set transaction's script root.
    pub fn tx_script_root(mut self, tx_script_root: Digest) -> Self {
        self.tx_script_root = Some(tx_script_root);
        self
    }

    /// Builds the [ProvenTransaction].
    ///
    /// # Errors
    ///
    /// An error will be returned if an on-chain account is used without provided account delta.
    pub fn build(mut self) -> Result<ProvenTransaction, ProvenTransactionError> {
        let account_delta = self.account_delta.take();
        let input_notes =
            InputNotes::new(self.input_notes).map_err(ProvenTransactionError::InputNotesError)?;
        let output_notes = OutputNotes::new(self.output_notes)
            .map_err(ProvenTransactionError::OutputNotesError)?;
        let tx_script_root = self.tx_script_root;

        let id = TransactionId::new(
            self.initial_account_hash,
            self.final_account_hash,
            input_notes.commitment(),
            output_notes.commitment(),
        );

        let proven_transaction = ProvenTransaction {
            id,
            account_id: self.account_id,
            initial_account_hash: self.initial_account_hash,
            final_account_hash: self.final_account_hash,
            account_delta,
            input_notes,
            output_notes,
            tx_script_root,
            block_ref: self.block_ref,
            proof: self.proof,
        };

        proven_transaction.validate()
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for ProvenTransaction {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.account_id.write_into(target);
        self.initial_account_hash.write_into(target);
        self.final_account_hash.write_into(target);
        self.account_delta.write_into(target);
        self.input_notes.write_into(target);
        self.output_notes.write_into(target);
        self.tx_script_root.write_into(target);
        self.block_ref.write_into(target);
        self.proof.write_into(target);
    }
}

impl Deserializable for ProvenTransaction {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let account_id = AccountId::read_from(source)?;
        let initial_account_hash = Digest::read_from(source)?;
        let final_account_hash = Digest::read_from(source)?;
        let account_delta = <Option<AccountDelta>>::read_from(source)?;

        let input_notes = InputNotes::<Nullifier>::read_from(source)?;
        let output_notes = OutputNotes::read_from(source)?;

        let tx_script_root = Deserializable::read_from(source)?;

        let block_ref = Digest::read_from(source)?;
        let proof = ExecutionProof::read_from(source)?;

        let id = TransactionId::new(
            initial_account_hash,
            final_account_hash,
            input_notes.commitment(),
            output_notes.commitment(),
        );

        let proven_transaction = Self {
            id,
            account_id,
            initial_account_hash,
            final_account_hash,
            account_delta,
            input_notes,
            output_notes,
            tx_script_root,
            block_ref,
            proof,
        };

        proven_transaction
            .validate()
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::ProvenTransaction;

    fn check_if_sync<T: Sync>() {}
    fn check_if_send<T: Send>() {}

    #[test]
    fn proven_transaction_is_sync() {
        check_if_sync::<ProvenTransaction>();
    }

    #[test]
    fn proven_transaction_is_send() {
        check_if_send::<ProvenTransaction>();
    }
}
