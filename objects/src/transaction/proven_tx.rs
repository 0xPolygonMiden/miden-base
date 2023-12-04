use super::{AccountId, ConsumedNoteInfo, Digest, NoteEnvelope, Vec};

use miden_crypto::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use miden_verifier::ExecutionProof;
use vm_processor::DeserializationError;

/// Resultant object of executing and proving a transaction. It contains the minimal
/// amount of data needed to verify that the transaction was executed correctly.
/// Contains:
/// - account_id: the account that the transaction was executed against.
/// - initial_account_hash: the hash of the account before the transaction was executed.
/// - final_account_hash: the hash of the account after the transaction was executed.
/// - consumed_notes: a list of consumed notes.
/// - created_notes: a list of created notes.
/// - tx_script_root: the script root of the transaction.
/// - block_ref: the block hash of the last known block at the time the transaction was executed.
/// - proof: the proof of the transaction.
#[derive(Clone, Debug)]
pub struct ProvenTransaction {
    account_id: AccountId,
    initial_account_hash: Digest,
    final_account_hash: Digest,
    consumed_notes: Vec<ConsumedNoteInfo>,
    created_notes: Vec<NoteEnvelope>,
    tx_script_root: Option<Digest>,
    block_ref: Digest,
    proof: ExecutionProof,
}

impl ProvenTransaction {
    #[allow(clippy::too_many_arguments)]
    /// Creates a new ProvenTransaction object.
    pub fn new(
        account_id: AccountId,
        initial_account_hash: Digest,
        final_account_hash: Digest,
        consumed_notes: Vec<ConsumedNoteInfo>,
        created_notes: Vec<NoteEnvelope>,
        tx_script_root: Option<Digest>,
        block_ref: Digest,
        proof: ExecutionProof,
    ) -> Self {
        Self {
            account_id,
            initial_account_hash,
            final_account_hash,
            consumed_notes,
            created_notes,
            tx_script_root,
            block_ref,
            proof,
        }
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------
    /// Returns the account ID.
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns the initial account hash.
    pub fn initial_account_hash(&self) -> Digest {
        self.initial_account_hash
    }

    /// Returns the final account hash.
    pub fn final_account_hash(&self) -> Digest {
        self.final_account_hash
    }

    /// Returns the consumed notes.
    pub fn consumed_notes(&self) -> &[ConsumedNoteInfo] {
        &self.consumed_notes
    }

    /// Returns the created notes.
    pub fn created_notes(&self) -> &[NoteEnvelope] {
        &self.created_notes
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
        target.write_u64(self.consumed_notes.len() as u64);
        self.consumed_notes.write_into(target);
        target.write_u64(self.created_notes.len() as u64);
        self.created_notes.write_into(target);
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

        let count = source.read_u64()?;
        let consumed_notes = ConsumedNoteInfo::read_batch_from(source, count as usize)?;

        let count = source.read_u64()?;
        let created_notes = NoteEnvelope::read_batch_from(source, count as usize)?;

        let tx_script_root = Deserializable::read_from(source)?;

        let block_ref = Digest::read_from(source)?;
        let proof = ExecutionProof::read_from(source)?;

        Ok(Self {
            account_id,
            initial_account_hash,
            final_account_hash,
            consumed_notes,
            created_notes,
            tx_script_root,
            block_ref,
            proof,
        })
    }
}
