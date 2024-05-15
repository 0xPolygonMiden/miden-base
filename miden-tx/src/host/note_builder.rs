use miden_objects::{
    assets::Asset,
    notes::{Note, NoteAssets, NoteHeader, NoteId},
};

use super::{Digest, NoteMetadata, NoteRecipient, OutputNote, TransactionKernelError};

// OUTPUT NOTE BUILDER
// ================================================================================================

/// Builder of an output note, provided primarily to enable adding assets to a note incrementally.
pub struct OutputNoteBuilder {
    metadata: NoteMetadata,
    assets: NoteAssets,
    recipient_digest: Digest,
    recipient: Option<NoteRecipient>,
}

impl OutputNoteBuilder {
    /// Returns a new [OutputNoteBuilder] instantiated from the provided metadata and recipient
    /// digest.
    ///
    /// # Errors
    /// Returns an error if the note type specified by the metadata is not [NoteType::OffChain] as
    /// for public and encrypted note additional note details must be available.
    pub fn new(
        metadata: NoteMetadata,
        recipient_digest: Digest,
    ) -> Result<Self, TransactionKernelError> {
        if !metadata.is_offchain() {
            return Err(TransactionKernelError::MissingNoteDetails(metadata, recipient_digest));
        }

        Ok(Self {
            metadata,
            recipient_digest,
            assets: NoteAssets::default(),
            recipient: None,
        })
    }

    /// Returns a new [OutputNoteBuilder] instantiated from the provided metadata and recipient.
    pub fn with_recipient(metadata: NoteMetadata, recipient: NoteRecipient) -> Self {
        Self {
            metadata,
            recipient_digest: recipient.digest(),
            recipient: Some(recipient),
            assets: NoteAssets::default(),
        }
    }

    /// Adds the specified asset to the note.
    ///
    /// # Errors
    /// Returns an error if adding the asset to the note fails. This can happen for the following
    /// reasons:
    /// - The same non-fungible asset is already added to the note.
    /// - A fungible asset issued by the same faucet is already added to the note and adding both
    ///   assets together results in an invalid asset.
    /// - Adding the asset to the note will push the list beyond the [NoteAssets::MAX_NUM_ASSETS]
    ///   limit.
    pub fn add_asset(&mut self, asset: Asset) -> Result<(), TransactionKernelError> {
        self.assets
            .add_asset(asset)
            .map_err(TransactionKernelError::FailedToAddAssetToNote)?;
        Ok(())
    }

    /// Converts this builder to an [OutputNote].
    pub fn build(self) -> OutputNote {
        match self.recipient {
            Some(recipient) => {
                let note = Note::new(self.assets, self.metadata, recipient);
                OutputNote::Full(note)
            },
            None => {
                let note_id = NoteId::new(self.recipient_digest, self.assets.commitment());
                let header = NoteHeader::new(note_id, self.metadata);
                OutputNote::Header(header)
            },
        }
    }
}
