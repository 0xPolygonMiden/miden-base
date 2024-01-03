use miden_crypto::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use miden_verifier::ExecutionProof;
use vm_processor::DeserializationError;

use super::{AccountId, Digest, InputNotes, NoteEnvelope, Nullifier, OutputNotes, TransactionId};

// PROVEN TRANSACTION
// ================================================================================================

/// The result of executing and proving a transaction.
///
/// This struct contains all the data required to verify that a transaction was executed correctly.
/// Specifically:
/// - account_id: ID of the account that the transaction was executed against.
/// - initial_account_hash: the hash of the account before the transaction was executed.
/// - final_account_hash: the hash of the account after the transaction was executed.
/// - input_notes: a list of nullifier for all notes consumed by the transaction.
/// - output_notes: a list of (note_id, metadata) tuples for all notes created by the
///   transaction.
/// - tx_script_root: the script root of the transaction, if one was used.
/// - block_ref: the block hash of the last known block at the time the transaction was executed.
/// - proof: a STARK proof that attests to the correct execution of the transaction.
#[derive(Clone, Debug)]
pub struct ProvenTransaction {
    id: TransactionId,
    account_id: AccountId,
    initial_account_hash: Digest,
    final_account_hash: Digest,
    input_notes: InputNotes<Nullifier>,
    output_notes: OutputNotes<NoteEnvelope>,
    tx_script_root: Option<Digest>,
    block_ref: Digest,
    proof: ExecutionProof,
}

impl ProvenTransaction {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new [ProvenTransaction] instantiated from the provided parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        account_id: AccountId,
        initial_account_hash: Digest,
        final_account_hash: Digest,
        input_notes: InputNotes<Nullifier>,
        output_notes: OutputNotes<NoteEnvelope>,
        tx_script_root: Option<Digest>,
        block_ref: Digest,
        proof: ExecutionProof,
    ) -> Self {
        Self {
            id: TransactionId::new(
                initial_account_hash,
                final_account_hash,
                input_notes.commitment(),
                output_notes.commitment(),
            ),
            account_id,
            initial_account_hash,
            final_account_hash,
            input_notes,
            output_notes,
            tx_script_root,
            block_ref,
            proof,
        }
    }

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

    /// Returns a reference to the notes consumed by the transaction.
    pub fn input_notes(&self) -> &InputNotes<Nullifier> {
        &self.input_notes
    }

    /// Returns a reference to the notes produced by the transaction.
    pub fn output_notes(&self) -> &OutputNotes<NoteEnvelope> {
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
}

// SERIALIZATION
// ================================================================================================

impl Serializable for ProvenTransaction {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.account_id.write_into(target);
        self.initial_account_hash.write_into(target);
        self.final_account_hash.write_into(target);
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

        let input_notes = InputNotes::<Nullifier>::read_from(source)?;
        let output_notes = OutputNotes::<NoteEnvelope>::read_from(source)?;

        let tx_script_root = Deserializable::read_from(source)?;

        let block_ref = Digest::read_from(source)?;
        let proof = ExecutionProof::read_from(source)?;

        Ok(Self {
            id: TransactionId::new(
                initial_account_hash,
                final_account_hash,
                input_notes.commitment(),
                output_notes.commitment(),
            ),
            account_id,
            initial_account_hash,
            final_account_hash,
            input_notes,
            output_notes,
            tx_script_root,
            block_ref,
            proof,
        })
    }
}
