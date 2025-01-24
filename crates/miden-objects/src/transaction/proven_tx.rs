use alloc::{string::ToString, vec::Vec};

use miden_verifier::ExecutionProof;

use super::{InputNote, ToInputNoteCommitments};
use crate::{
    account::{delta::AccountUpdateDetails, AccountUpdate},
    block::BlockNumber,
    note::NoteHeader,
    transaction::{
        AccountId, Digest, InputNotes, Nullifier, OutputNote, OutputNotes, TransactionId,
    },
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

    /// The update to the account from this transaction.
    account_update: AccountUpdate,

    /// Committed details of all notes consumed by the transaction.
    input_notes: InputNotes<InputNoteCommitment>,

    /// Notes created by the transaction. For private notes, this will contain only note headers,
    /// while for public notes this will also contain full note details.
    output_notes: OutputNotes,

    /// The block hash of the last known block at the time the transaction was executed.
    block_ref: Digest,

    /// The block number by which the transaction will expire, as defined by the executed scripts.
    expiration_block_num: BlockNumber,

    /// A STARK proof that attests to the correct execution of the transaction.
    proof: ExecutionProof,
}

impl ProvenTransaction {
    /// Returns unique identifier of this transaction.
    pub fn id(&self) -> TransactionId {
        self.id
    }

    /// Returns ID of the account against which this transaction was executed.
    pub fn account_id(&self) -> AccountId {
        self.account_update.account_id()
    }

    /// Returns the account update details.
    pub fn account_update(&self) -> &AccountUpdate {
        &self.account_update
    }

    /// Returns a reference to the notes consumed by the transaction.
    pub fn input_notes(&self) -> &InputNotes<InputNoteCommitment> {
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

    /// Returns an iterator of the headers of unauthenticated input notes in this transaction.
    pub fn get_unauthenticated_notes(&self) -> impl Iterator<Item = &NoteHeader> {
        self.input_notes.iter().filter_map(|note| note.header())
    }

    /// Returns the block number at which the transaction will expire.
    pub fn expiration_block_num(&self) -> BlockNumber {
        self.expiration_block_num
    }

    /// Returns an iterator over the nullifiers of all input notes in this transaction.
    ///
    /// This includes both authenticated and unauthenticated notes.
    pub fn get_nullifiers(&self) -> impl Iterator<Item = Nullifier> + '_ {
        self.input_notes.iter().map(InputNoteCommitment::nullifier)
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    fn validate(self) -> Result<Self, ProvenTransactionError> {
        if self.account_id().is_public() {
            self.account_update.validate()?;

            let is_new_account =
                self.account_update.initial_state_commitment() == Digest::default();
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
                        return Err(ProvenTransactionError::AccountIdMismatch {
                            tx_account_id: self.account_id(),
                            details_account_id: account.id(),
                        });
                    }
                    if account.hash() != self.account_update.final_state_commitment() {
                        return Err(ProvenTransactionError::AccountFinalHashMismatch {
                            tx_final_hash: self.account_update.final_state_commitment(),
                            details_hash: account.hash(),
                        });
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

impl Serializable for ProvenTransaction {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.account_update.write_into(target);
        self.input_notes.write_into(target);
        self.output_notes.write_into(target);
        self.block_ref.write_into(target);
        self.expiration_block_num.write_into(target);
        self.proof.write_into(target);
    }
}

impl Deserializable for ProvenTransaction {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let account_update = AccountUpdate::read_from(source)?;

        let input_notes = <InputNotes<InputNoteCommitment>>::read_from(source)?;
        let output_notes = OutputNotes::read_from(source)?;

        let block_ref = Digest::read_from(source)?;
        let expiration_block_num = BlockNumber::read_from(source)?;
        let proof = ExecutionProof::read_from(source)?;

        let id = TransactionId::new(
            account_update.initial_state_commitment(),
            account_update.final_state_commitment(),
            input_notes.commitment(),
            output_notes.commitment(),
        );

        let proven_transaction = Self {
            id,
            account_update,
            input_notes,
            output_notes,
            block_ref,
            expiration_block_num,
            proof,
        };

        proven_transaction
            .validate()
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))
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

    /// List of [InputNoteCommitment]s of all consumed notes by the transaction.
    input_notes: Vec<InputNoteCommitment>,

    /// List of [OutputNote]s of all notes created by the transaction.
    output_notes: Vec<OutputNote>,

    /// Block [Digest] of the transaction's reference block.
    block_ref: Digest,

    /// The block number by which the transaction will expire, as defined by the executed scripts.
    expiration_block_num: BlockNumber,

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
        expiration_block_num: BlockNumber,
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
            expiration_block_num,
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
    pub fn add_input_notes<I, T>(mut self, notes: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<InputNoteCommitment>,
    {
        self.input_notes.extend(notes.into_iter().map(|note| note.into()));
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
    /// Or if the account details, i.e. account ID and final hash, don't match the transaction.
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
        let account_update = AccountUpdate::new(
            self.account_id,
            self.initial_account_hash,
            self.final_account_hash,
            self.account_update_details,
            vec![id],
        );

        let proven_transaction = ProvenTransaction {
            id,
            account_update,
            input_notes,
            output_notes,
            block_ref: self.block_ref,
            expiration_block_num: self.expiration_block_num,
            proof: self.proof,
        };

        proven_transaction.validate()
    }
}

// INPUT NOTE COMMITMENT
// ================================================================================================

/// The commitment to an input note.
///
/// For notes authenticated by the transaction kernel, the commitment consists only of the note's
/// nullifier. For notes whose authentication is delayed to batch/block kernels, the commitment
/// also includes full note header (i.e., note ID and metadata).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputNoteCommitment {
    nullifier: Nullifier,
    header: Option<NoteHeader>,
}

impl InputNoteCommitment {
    /// Returns the nullifier of the input note committed to by this commitment.
    pub fn nullifier(&self) -> Nullifier {
        self.nullifier
    }

    /// Returns the header of the input committed to by this commitment.
    ///
    /// Note headers are present only for notes whose presence in the change has not yet been
    /// authenticated.
    pub fn header(&self) -> Option<&NoteHeader> {
        self.header.as_ref()
    }

    /// Returns true if this commitment is for a note whose presence in the chain has been
    /// authenticated.
    ///
    /// Authenticated notes are represented solely by their nullifiers and are missing the note
    /// header.
    pub fn is_authenticated(&self) -> bool {
        self.header.is_none()
    }
}

impl From<InputNote> for InputNoteCommitment {
    fn from(note: InputNote) -> Self {
        Self::from(&note)
    }
}

impl From<&InputNote> for InputNoteCommitment {
    fn from(note: &InputNote) -> Self {
        match note {
            InputNote::Authenticated { note, .. } => Self {
                nullifier: note.nullifier(),
                header: None,
            },
            InputNote::Unauthenticated { note } => Self {
                nullifier: note.nullifier(),
                header: Some(*note.header()),
            },
        }
    }
}

impl From<Nullifier> for InputNoteCommitment {
    fn from(nullifier: Nullifier) -> Self {
        Self { nullifier, header: None }
    }
}

impl ToInputNoteCommitments for InputNoteCommitment {
    fn nullifier(&self) -> Nullifier {
        self.nullifier
    }

    fn note_hash(&self) -> Option<Digest> {
        self.header.map(|header| header.hash())
    }
}

// SERIALIZATION
// ------------------------------------------------------------------------------------------------

impl Serializable for InputNoteCommitment {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.nullifier.write_into(target);
        self.header.write_into(target);
    }
}

impl Deserializable for InputNoteCommitment {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let nullifier = Nullifier::read_from(source)?;
        let header = <Option<NoteHeader>>::read_from(source)?;

        Ok(Self { nullifier, header })
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::ProvenTransaction;

    fn check_if_sync<T: Sync>() {}
    fn check_if_send<T: Send>() {}

    /// [ProvenTransaction] being Sync is part of its public API and changing it is backwards
    /// incompatible.
    #[test]
    fn test_proven_transaction_is_sync() {
        check_if_sync::<ProvenTransaction>();
    }

    /// [ProvenTransaction] being Send is part of its public API and changing it is backwards
    /// incompatible.
    #[test]
    fn test_proven_transaction_is_send() {
        check_if_send::<ProvenTransaction>();
    }
}
