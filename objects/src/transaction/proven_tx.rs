use alloc::{string::ToString, vec::Vec};

use miden_verifier::ExecutionProof;

use super::{
    AccountId, Digest, InputNotes, Nullifier, OutputNote, OutputNotes, TransactionId,
    TxAccountUpdate,
};
use crate::{
    accounts::delta::AccountUpdateDetails,
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

    /// Account update data.
    account_update: TxAccountUpdate,

    /// A list of nullifiers for all notes consumed by the transaction.
    input_notes: InputNotes<Nullifier>,

    /// The id and  metadata of all notes created by the transaction.
    output_notes: OutputNotes,

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
        self.account_update.account_id()
    }

    /// Returns the account update details.
    pub fn account_update(&self) -> &TxAccountUpdate {
        &self.account_update
    }

    /// Returns a reference to the notes consumed by the transaction.
    pub fn input_notes(&self) -> &InputNotes<Nullifier> {
        &self.input_notes
    }

    /// Returns a reference to the notes produced by the transaction.
    pub fn output_notes(&self) -> &OutputNotes {
        &self.output_notes
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
        if self.account_id().is_on_chain() {
            let is_new_account = self.account_update.init_state_hash() == Digest::default();
            match self.account_update.details() {
                AccountUpdateDetails::Private => {
                    return Err(ProvenTransactionError::OnChainAccountMissingDetails(
                        self.account_id(),
                    ))
                },
                AccountUpdateDetails::New(ref account) => {
                    if !is_new_account {
                        return Err(
                            ProvenTransactionError::ExistingOnChainAccountRequiresDeltaDetails(
                                self.account_id(),
                            ),
                        );
                    }
                    if account.id() != self.account_id() {
                        return Err(ProvenTransactionError::AccountIdMismatch(
                            self.account_id(),
                            account.id(),
                        ));
                    }
                    if account.hash() != self.account_update.final_state_hash() {
                        return Err(ProvenTransactionError::AccountFinalHashMismatch(
                            self.account_update.final_state_hash(),
                            account.hash(),
                        ));
                    }
                },
                AccountUpdateDetails::Delta(_) => {
                    if is_new_account {
                        return Err(ProvenTransactionError::NewOnChainAccountRequiresFullDetails(
                            self.account_id(),
                        ));
                    }
                },
            }
        } else if !self.account_update.is_private() {
            return Err(ProvenTransactionError::OffChainAccountWithDetails(self.account_id()));
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
    account_update_details: AccountUpdateDetails,

    /// List of [Nullifier]s of all consumed notes by the transaction.
    input_notes: Vec<Nullifier>,

    /// List of [NoteEnvelope]s of all notes created by the transaction.
    output_notes: Vec<OutputNote>,

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
            account_update_details: AccountUpdateDetails::Private,
            input_notes: Vec::new(),
            output_notes: Vec::new(),
            block_ref,
            proof,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Sets the account's update details.
    pub fn account_update_details(mut self, details: AccountUpdateDetails) -> Self {
        self.account_update_details = details;
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

    /// Builds the [ProvenTransaction].
    ///
    /// # Errors
    ///
    /// An error will be returned if an on-chain account is used without provided on-chain detail.
    /// Or if the account details, i.e. account id and final hash, don't match the transaction.
    pub fn build(self) -> Result<ProvenTransaction, ProvenTransactionError> {
        let input_notes =
            InputNotes::new(self.input_notes).map_err(ProvenTransactionError::InputNotesError)?;
        let output_notes = OutputNotes::new(self.output_notes)
            .map_err(ProvenTransactionError::OutputNotesError)?;
        let id = TransactionId::new(
            self.initial_account_hash,
            self.final_account_hash,
            input_notes.commitment(),
            output_notes.commitment(),
        );
        let account_update = TxAccountUpdate::new(
            self.account_id,
            self.initial_account_hash,
            self.final_account_hash,
            self.account_update_details,
        );

        let proven_transaction = ProvenTransaction {
            id,
            account_update,
            input_notes,
            output_notes,
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
        self.account_update.write_into(target);
        self.input_notes.write_into(target);
        self.output_notes.write_into(target);
        self.block_ref.write_into(target);
        self.proof.write_into(target);
    }
}

impl Deserializable for ProvenTransaction {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let account_update = TxAccountUpdate::read_from(source)?;

        let input_notes = InputNotes::<Nullifier>::read_from(source)?;
        let output_notes = OutputNotes::read_from(source)?;

        let block_ref = Digest::read_from(source)?;
        let proof = ExecutionProof::read_from(source)?;

        let id = TransactionId::new(
            account_update.init_state_hash(),
            account_update.final_state_hash(),
            input_notes.commitment(),
            output_notes.commitment(),
        );

        let proven_transaction = Self {
            id,
            account_update,
            input_notes,
            output_notes,
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
