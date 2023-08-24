use super::{
    Digest, Hasher, Note, NoteEnvelope, NoteError, NoteMetadata, NoteVault, Word, WORD_SIZE,
};
use crypto::StarkField;
use miden_lib::memory::{
    CREATED_NOTE_ASSETS_OFFSET, CREATED_NOTE_CORE_DATA_SIZE, CREATED_NOTE_HASH_OFFSET,
    CREATED_NOTE_METADATA_OFFSET, CREATED_NOTE_RECIPIENT_OFFSET, CREATED_NOTE_VAULT_HASH_OFFSET,
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

impl TryFrom<&[Word]> for NoteStub {
    type Error = NoteError;

    fn try_from(elements: &[Word]) -> Result<Self, Self::Error> {
        if elements.len() < CREATED_NOTE_CORE_DATA_SIZE {
            return Err(NoteError::InvalidStubDataLen(elements.len()));
        }

        let hash: Digest = elements[CREATED_NOTE_HASH_OFFSET as usize].into();
        let metadata: NoteMetadata = elements[CREATED_NOTE_METADATA_OFFSET as usize].try_into()?;
        let recipient = elements[CREATED_NOTE_RECIPIENT_OFFSET as usize].into();
        let vault_hash: Digest = elements[CREATED_NOTE_VAULT_HASH_OFFSET as usize].into();

        if elements.len()
            < (CREATED_NOTE_ASSETS_OFFSET as usize + metadata.num_assets().as_int() as usize)
                * WORD_SIZE
        {
            return Err(NoteError::InvalidStubDataLen(elements.len()));
        }

        let vault: NoteVault = elements[CREATED_NOTE_ASSETS_OFFSET as usize
            ..(CREATED_NOTE_ASSETS_OFFSET as usize + metadata.num_assets().as_int() as usize)]
            .try_into()?;
        if vault.hash() != vault_hash {
            return Err(NoteError::InconsistentStubVaultHash(vault_hash, vault.hash()));
        }

        let stub = Self::new(recipient, vault, metadata)?;
        if stub.hash() != hash {
            return Err(NoteError::InconsistentStubHash(stub.hash(), hash));
        }

        Ok(stub)
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
        Self::new(recipient, note.vault.clone(), note.metadata)
            .expect("Note vault and metadate weren't consistent")
    }
}
