use super::{AccountId, ConsumedNoteInfo, Digest, NoteEnvelope, Vec};

use miden_verifier::ExecutionProof;

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
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ProvenTransaction {
    account_id: AccountId,
    initial_account_hash: Digest,
    final_account_hash: Digest,
    consumed_notes: Vec<ConsumedNoteInfo>,
    created_notes: Vec<NoteEnvelope>,
    tx_script_root: Option<Digest>,
    block_ref: Digest,
    #[cfg_attr(feature = "serde", serde(with = "serialization"))]
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

#[cfg(feature = "serde")]
mod serialization {
    pub fn serialize<S>(proof: &super::ExecutionProof, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = proof.to_bytes();
        serializer.serialize_bytes(&bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<super::ExecutionProof, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = <Vec<u8> as serde::Deserialize>::deserialize(deserializer)?;

        super::ExecutionProof::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}
