use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

use crate::{
    block::BlockInclusionProof,
    note::{NoteId, NoteInclusionProof},
};

// TODO: Document.
#[derive(Clone, Default, Debug)]
pub struct NoteInclusionProofs {
    block_proofs: Vec<BlockInclusionProof>,
    note_proofs: BTreeMap<NoteId, NoteInclusionProof>,
}

impl NoteInclusionProofs {
    pub fn block_proofs(&self) -> &[BlockInclusionProof] {
        &self.block_proofs
    }

    pub fn note_proofs(&self) -> impl Iterator<Item = (&NoteId, &NoteInclusionProof)> {
        self.note_proofs.iter()
    }

    pub fn contains_note(&self, note: &NoteId) -> bool {
        self.note_proofs.contains_key(note)
    }

    pub fn note_ids(&self) -> BTreeSet<NoteId> {
        self.note_proofs.keys().copied().collect()
    }
}
