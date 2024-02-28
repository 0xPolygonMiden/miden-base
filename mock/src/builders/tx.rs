use miden_objects::{
    accounts::AccountId,
    notes::{NoteEnvelope, Nullifier},
    transaction::{InputNotes, OutputNotes, ProvenTransaction, ToEnvelope, ToNullifier},
    vm::ExecutionProof,
    Digest,
};
use miden_prover::{HashFunction, StarkProof};

/// Builder for an `ProvenTransaction`, the builder allows for a fluent API to construct an account.
/// Each transaction needs a unique builder.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ProvenTransactionBuilder {
    account_id: AccountId,
    note_envelopes: Vec<NoteEnvelope>,
    nullifiers: Vec<Nullifier>,
    initial_account_hash: Digest,
    final_account_hash: Digest,
}

impl ProvenTransactionBuilder {
    pub fn new(
        account_id: AccountId,
        initial_account_hash: Digest,
        final_account_hash: Digest,
    ) -> Self {
        Self {
            account_id,
            initial_account_hash,
            final_account_hash,
            note_envelopes: Vec::new(),
            nullifiers: Vec::new(),
        }
    }

    pub fn add_note_envelope<I: ToEnvelope>(mut self, note_envelope: I) -> Self {
        self.note_envelopes.push(note_envelope.to_envelope());
        self
    }
    pub fn add_note_envelopes<I: IntoIterator<Item = NoteEnvelope>>(
        mut self,
        note_envelopes: I,
    ) -> Self {
        for note_envelope in note_envelopes.into_iter() {
            self.note_envelopes.push(note_envelope.to_envelope());
        }
        self
    }

    pub fn add_nullifier<I: ToNullifier>(mut self, nullifier: I) -> Self {
        self.nullifiers.push(nullifier.nullifier());
        self
    }

    pub fn build(self) -> ProvenTransaction {
        ProvenTransaction::new(
            self.account_id,
            self.initial_account_hash,
            self.final_account_hash,
            InputNotes::new(self.nullifiers).unwrap(),
            OutputNotes::new(self.note_envelopes).unwrap(),
            None,
            Digest::default(),
            ExecutionProof::new(StarkProof::new_dummy(), HashFunction::Blake3_192),
        )
    }
}
