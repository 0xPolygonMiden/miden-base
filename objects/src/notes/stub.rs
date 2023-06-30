use super::{Digest, Felt, Hasher, NoteError, NoteMetadata, NoteVault, Word, WORD_SIZE};
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
pub struct NoteStub {
    hash: Digest,
    recipient: Digest,
    vault: NoteVault,
    metadata: NoteMetadata,
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
            hash,
            recipient,
            vault,
            metadata,
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
        &self.metadata
    }

    /// Returns the hash of this note stub.
    pub fn hash(&self) -> &Digest {
        &self.hash
    }
}

impl TryFrom<&[Felt]> for NoteStub {
    type Error = NoteError;

    fn try_from(elements: &[Felt]) -> Result<Self, Self::Error> {
        if elements.len() < CREATED_NOTE_CORE_DATA_SIZE * WORD_SIZE {
            return Err(NoteError::InvalidStubDataLen(elements.len()));
        }

        let hash: Digest = TryInto::<Word>::try_into(
            &elements[CREATED_NOTE_HASH_OFFSET as usize * WORD_SIZE
                ..(CREATED_NOTE_HASH_OFFSET as usize + 1) * WORD_SIZE],
        )
        .expect("word is correct size")
        .into();
        let metadata: NoteMetadata = TryInto::<Word>::try_into(
            &elements[CREATED_NOTE_METADATA_OFFSET as usize * WORD_SIZE
                ..(CREATED_NOTE_METADATA_OFFSET as usize + 1) * WORD_SIZE],
        )
        .expect("word is correct size")
        .try_into()?;
        let recipient = TryInto::<Word>::try_into(
            &elements[CREATED_NOTE_RECIPIENT_OFFSET as usize * WORD_SIZE
                ..(CREATED_NOTE_RECIPIENT_OFFSET as usize + 1) * WORD_SIZE],
        )
        .expect("word is correct size")
        .into();
        let vault_hash: Digest = TryInto::<Word>::try_into(
            &elements[CREATED_NOTE_VAULT_HASH_OFFSET as usize * WORD_SIZE
                ..(CREATED_NOTE_VAULT_HASH_OFFSET as usize + 1) * WORD_SIZE],
        )
        .expect("word is correct size")
        .into();

        if elements.len()
            < (CREATED_NOTE_ASSETS_OFFSET as usize + metadata.num_assets().as_int() as usize)
                * WORD_SIZE
        {
            return Err(NoteError::InvalidStubDataLen(elements.len()));
        }

        let vault: NoteVault = elements[CREATED_NOTE_ASSETS_OFFSET as usize * WORD_SIZE
            ..(CREATED_NOTE_ASSETS_OFFSET as usize + metadata.num_assets().as_int() as usize)
                * WORD_SIZE]
            .try_into()?;
        if vault.hash() != vault_hash {
            return Err(NoteError::InconsistentStubVaultHash(vault_hash, vault.hash()));
        }

        let stub = Self::new(recipient, vault, metadata)?;
        if stub.hash != hash {
            return Err(NoteError::InconsistentStubHash(stub.hash, hash));
        }

        Ok(stub)
    }
}
