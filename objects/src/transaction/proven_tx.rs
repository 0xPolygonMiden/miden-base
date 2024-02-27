use miden_verifier::ExecutionProof;

use super::{AccountId, Digest, InputNotes, NoteEnvelope, Nullifier, OutputNotes, TransactionId};
use crate::{
    accounts::{Account, AccountDelta},
    notes::{Note, NoteId},
    utils::{
        collections::*,
        format,
        serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    },
    TransactionInputError, TransactionOutputError,
};

// PROVEN TRANSACTION
// ================================================================================================

#[derive(Clone, Debug)]
pub enum AccountDetails {
    /// The whole state is needed for new accounts
    Full(Account),

    /// For existing accounts, only the delta is needed.
    Delta(AccountDelta),
}

/// Result of executing and proving a transaction. Contains all the data required to verify that a
/// transaction was executed correctly.
#[derive(Clone, Debug)]
pub struct ProvenTransaction {
    /// A unique identifier for the transaction, see [TransactionId] for additional details.
    id: TransactionId,

    /// ID of the account that the transaction was executed against.
    account_id: AccountId,

    /// The hash of the account before the transaction was executed.
    initial_account_hash: Digest,

    /// The hash of the account after the transaction was executed.
    final_account_hash: Digest,

    /// Optional account state changes used for on-chain accounts, This data is used to update an
    /// on-chain account's state after a local transaction execution.
    account_details: Option<AccountDetails>,

    /// A list of nullifiers for all notes consumed by the transaction.
    input_notes: InputNotes<Nullifier>,

    /// The id and  metadata of all notes created by the transaction.
    output_notes: OutputNotes<NoteEnvelope>,

    /// Optionally the output note's data, used to share the note with the network.
    output_note_details: BTreeMap<NoteId, Note>,

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

    /// Returns the account details.
    pub fn account_details(&self) -> Option<&AccountDetails> {
        self.account_details.as_ref()
    }

    /// Returns a reference to the notes consumed by the transaction.
    pub fn input_notes(&self) -> &InputNotes<Nullifier> {
        &self.input_notes
    }

    /// Returns a reference to the notes produced by the transaction.
    pub fn output_notes(&self) -> &OutputNotes<NoteEnvelope> {
        &self.output_notes
    }

    /// Returns the [NoteId] details, if present.
    pub fn get_output_note_details(&self, note_id: &NoteId) -> Option<&Note> {
        self.output_note_details.get(note_id)
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
}

// PROVEN TRANSACTION BUILDER
// ================================================================================================

/// Builder for a proven transaction.
#[derive(Clone, Debug)]
pub struct ProvenTransactionBuilder {
    /// ID of the account that the transaction was executed against.
    account_id: Option<AccountId>,

    /// The hash of the account before the transaction was executed.
    initial_account_hash: Option<Digest>,

    /// The hash of the account after the transaction was executed.
    final_account_hash: Option<Digest>,

    /// State changes to the account due to the transaction.
    account_details: Option<AccountDetails>,

    /// List of [Nullifier]s of all consumed notes by the transaction.
    input_notes: Vec<Nullifier>,

    /// List of [NoteEnvelope]s of all notes created by the transaction.
    output_notes: Vec<NoteEnvelope>,

    /// State of the output notes.
    output_note_details: BTreeMap<NoteId, Note>,

    /// The script root of the transaction, if one was used.
    tx_script_root: Option<Digest>,

    /// Block [Digest] of the transaction's reference block.
    block_ref: Option<Digest>,

    /// A STARK proof that attests to the correct execution of the transaction.
    proof: Option<ExecutionProof>,
}

#[derive(Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum ProvenTransactionBuilderError {
    #[error("A ProvenTransaction can not be created without an account id.")]
    MissingAccountId,
    #[error("A ProvenTransaction can not be created without the account's initial hash.")]
    MissingInitialHash,
    #[error("A ProvenTransaction can not be created without the account's final hash.")]
    MissingFinalHash,
    #[error("A ProvenTransaction can not be created without a reference block.")]
    MissingBlockReference,
    #[error("A ProvenTransaction can not be created without the execution proof.")]
    MissingExecutionProof,

    #[error("Note detail {0} for an unknown output note.")]
    InvalidNoteDetail(NoteId, Note),

    #[error("Provent transaction account_id {0} and account_details.id must match {1}.")]
    AccountIdMismatch(AccountId, AccountId),

    #[error("Provent transaction account_final_hash {0} and account_details.hash must match {1}.")]
    AccountFinalHashMismatch(Digest, Digest),
    #[error("Invalid input notes: {0}")]
    InputNotesError(#[from] TransactionInputError),
    #[error("Invalid output notes: {0}")]
    OutputNotesError(#[from] TransactionOutputError),
    #[error("Note details for unknown note ids: {0:?}")]
    NoteDetailsForUnknownNotes(Vec<NoteId>),
}

impl ProvenTransactionBuilder {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a [ProvenTransactionBuilder] used to build a [ProvenTransaction].
    pub fn new() -> Self {
        Self {
            account_id: None,
            initial_account_hash: None,
            final_account_hash: None,
            account_details: None,
            input_notes: Vec::new(),
            output_notes: Vec::new(),
            output_note_details: BTreeMap::new(),
            tx_script_root: None,
            block_ref: None,
            proof: None,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Sets the [Account] for which the was executed.
    pub fn account_id(mut self, account_id: AccountId) -> Self {
        self.account_id = Some(account_id);
        self
    }

    /// Sets the account's initial state hash.
    pub fn initial_account_hash(mut self, initial_account_hash: Digest) -> Self {
        self.initial_account_hash = Some(initial_account_hash);
        self
    }

    /// Sets the account's final account state hash.
    pub fn final_account_hash(mut self, final_account_hash: Digest) -> Self {
        self.final_account_hash = Some(final_account_hash);
        self
    }

    /// Sets the account's details.
    pub fn account_details(mut self, account_details: AccountDetails) -> Self {
        self.account_details = Some(account_details);
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
        T: IntoIterator<Item = NoteEnvelope>,
    {
        self.output_notes.extend(notes);
        self
    }

    /// Add produce notes details.
    pub fn add_output_note_details<T>(mut self, notes: T) -> Self
    where
        T: IntoIterator<Item = (NoteId, Note)>,
    {
        self.output_note_details.extend(notes);
        self
    }

    /// Set transaction's script root.
    pub fn tx_script_root(mut self, tx_script_root: Digest) -> Self {
        self.tx_script_root = Some(tx_script_root);
        self
    }

    /// Set the transaction's proof.
    pub fn proof(mut self, proof: ExecutionProof) -> Self {
        self.proof = Some(proof);
        self
    }

    /// Set reference block the transaction was executed against.
    pub fn block_ref(mut self, block_ref: Digest) -> Self {
        self.block_ref = Some(block_ref);
        self
    }

    pub fn build(mut self) -> Result<ProvenTransaction, ProvenTransactionBuilderError> {
        // Required fields
        let account_id =
            self.account_id.take().ok_or(ProvenTransactionBuilderError::MissingAccountId)?;
        let initial_account_hash = self
            .initial_account_hash
            .take()
            .ok_or(ProvenTransactionBuilderError::MissingInitialHash)?;
        let final_account_hash = self
            .final_account_hash
            .take()
            .ok_or(ProvenTransactionBuilderError::MissingFinalHash)?;
        let block_ref = self
            .block_ref
            .take()
            .ok_or(ProvenTransactionBuilderError::MissingBlockReference)?;
        let proof =
            self.proof.take().ok_or(ProvenTransactionBuilderError::MissingExecutionProof)?;

        let output_note_details = self.output_note_details;
        let known_output_ids = BTreeSet::from_iter(self.output_notes.iter().map(|n| n.note_id()));
        let unknown_ids: Vec<_> = output_note_details
            .keys()
            .filter(|k| !known_output_ids.contains(k))
            .cloned()
            .collect();
        if !unknown_ids.is_empty() {
            return Err(ProvenTransactionBuilderError::NoteDetailsForUnknownNotes(unknown_ids));
        }

        let account_details = self.account_details.take();
        let input_notes = InputNotes::new(self.input_notes)?;
        let output_notes = OutputNotes::new(self.output_notes)?;
        let tx_script_root = self.tx_script_root;

        if let Some(ref details) = account_details {
            match details {
                AccountDetails::Full(account) => {
                    if account.id() != account_id {
                        return Err(ProvenTransactionBuilderError::AccountIdMismatch(
                            account_id,
                            account.id(),
                        ));
                    }
                    if account.hash() != final_account_hash {
                        return Err(ProvenTransactionBuilderError::AccountFinalHashMismatch(
                            final_account_hash,
                            account.hash(),
                        ));
                    }
                },
                AccountDetails::Delta(_) => todo!(),
            }
        }

        let id = TransactionId::new(
            initial_account_hash,
            final_account_hash,
            input_notes.commitment(),
            output_notes.commitment(),
        );

        Ok(ProvenTransaction {
            id,
            account_id,
            initial_account_hash,
            final_account_hash,
            account_details,
            input_notes,
            output_notes,
            output_note_details,
            tx_script_root,
            block_ref,
            proof,
        })
    }
}

impl Default for ProvenTransactionBuilder {
    fn default() -> Self {
        ProvenTransactionBuilder::new()
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountDetails {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            AccountDetails::Full(account) => {
                0_u8.write_into(target);
                account.write_into(target);
            },
            AccountDetails::Delta(delta) => {
                1_u8.write_into(target);
                delta.write_into(target);
            },
        }
    }
}

impl Deserializable for AccountDetails {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        match u8::read_from(source)? {
            0_u8 => Ok(Self::Full(Account::read_from(source)?)),
            1_u8 => Ok(Self::Delta(AccountDelta::read_from(source)?)),
            v => Err(DeserializationError::InvalidValue(format!(
                "Unknown variant {v} for AccountDetails"
            ))),
        }
    }
}

impl Serializable for ProvenTransaction {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.account_id.write_into(target);
        self.initial_account_hash.write_into(target);
        self.final_account_hash.write_into(target);
        self.account_details.write_into(target);
        self.input_notes.write_into(target);
        self.output_notes.write_into(target);

        target.write_usize(self.output_note_details.len());
<<<<<<< HEAD
        target.write_many(self.output_note_details.iter());
=======
        let elements: Vec<_> =
            self.output_note_details.iter().map(|(k, v)| (*k, v.clone())).collect();
        target.write_many(&elements);
>>>>>>> 350922a (objects: add account/note details to ProvenTransaction #403)

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
        let account_details = <Option<AccountDetails>>::read_from(source)?;

        let input_notes = InputNotes::<Nullifier>::read_from(source)?;
        let output_notes = OutputNotes::<NoteEnvelope>::read_from(source)?;

        let output_notes_details_len = usize::read_from(source)?;
        let details = source.read_many(output_notes_details_len)?;
        let output_note_details = BTreeMap::from_iter(details);

        let tx_script_root = Deserializable::read_from(source)?;

        let block_ref = Digest::read_from(source)?;
        let proof = ExecutionProof::read_from(source)?;

        let id = TransactionId::new(
            initial_account_hash,
            final_account_hash,
            input_notes.commitment(),
            output_notes.commitment(),
        );

        Ok(Self {
            id,
            account_id,
            initial_account_hash,
            final_account_hash,
            account_details,
            input_notes,
            output_notes,
            output_note_details,
            tx_script_root,
            block_ref,
            proof,
        })
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
