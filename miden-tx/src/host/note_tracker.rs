use alloc::vec::Vec;

use miden_objects::{assets::Asset, notes::NoteMetadata, Digest, Felt};

// OUTPUT NOTE BUILDER
// ================================================================================================
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OutputNoteData {
    metadata: NoteMetadata,
    note_ptr: Felt,
    recipient: Digest,
    assets: Vec<Asset>,
}

impl OutputNoteData {
    pub fn new(
        metadata: NoteMetadata,
        note_ptr: Felt,
        recipient: Digest,
        assets: Vec<Asset>,
    ) -> Self {
        Self { metadata, note_ptr, recipient, assets }
    }

    pub fn metadata(&self) -> NoteMetadata {
        self.metadata
    }

    pub fn note_ptr(&self) -> Felt {
        self.note_ptr
    }

    pub fn recipient(&self) -> Digest {
        self.recipient
    }

    // Returns a mutable reference to the assets
    pub fn assets_mut(&mut self) -> &mut Vec<Asset> {
        &mut self.assets
    }
}
