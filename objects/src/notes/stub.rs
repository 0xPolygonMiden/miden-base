use crate::{
    notes::{Note, NoteEnvelope, NoteMetadata, NoteVault},
    NoteError,
};
use crypto::{
    hash::rpo::{Rpo256 as Hasher, RpoDigest as Digest},
    StarkField,
};

// NOTE STUB
// ================================================================================================

/// An object that represents the stub of a note. When a note is produced in a transaction it can
/// be the case that only the recipient, vault and metadata are known. In this case, the note
/// stub can be used to represent the note.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteStub {
    envelope: NoteEnvelope,
    recipient: Digest,
    vault: NoteVault,
}

impl NoteStub {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates a new [NoteStub].
    pub fn new(
        recipient: Digest,
        vault: NoteVault,
        metadata: NoteMetadata,
    ) -> Result<Self, NoteError> {
        if vault.num_assets() as u64 != metadata.num_assets().as_int() {
            return Err(NoteError::InconsistentStubNumAssets(
                vault.num_assets() as u64,
                metadata.num_assets().as_int(),
            ));
        }
        let hash = Hasher::merge(&[recipient, vault.hash()]);
        Ok(Self {
            envelope: NoteEnvelope::new(hash, metadata),
            recipient,
            vault,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------
    /// Returns the recipient of the note.
    pub fn recipient(&self) -> &Digest {
        &self.recipient
    }

    /// Returns a reference to the asset vault of this note.
    pub fn vault(&self) -> &NoteVault {
        &self.vault
    }

    /// Returns the metadata associated with this note.
    pub fn metadata(&self) -> &NoteMetadata {
        self.envelope.metadata()
    }

    /// Returns the hash of this note stub.
    pub fn hash(&self) -> Digest {
        self.envelope.note_hash()
    }
}

impl From<NoteStub> for NoteEnvelope {
    fn from(note_stub: NoteStub) -> Self {
        note_stub.envelope
    }
}

impl From<&NoteStub> for NoteEnvelope {
    fn from(note_stub: &NoteStub) -> Self {
        note_stub.envelope
    }
}

impl From<Note> for NoteStub {
    fn from(note: Note) -> Self {
        (&note).into()
    }
}

impl From<&Note> for NoteStub {
    fn from(note: &Note) -> Self {
        let recipient = note.recipient();
        Self::new(recipient, note.vault().clone(), *note.metadata())
            .expect("Note vault and metadate weren't consistent")
    }
}
