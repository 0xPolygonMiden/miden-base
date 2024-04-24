use alloc::vec::Vec;

use super::{Digest, Felt, Hasher, ZERO};

mod header;
pub use header::BlockHeader;
mod note_tree;
pub use note_tree::{BlockNoteIndex, BlockNoteTree};

use crate::{
    notes::Nullifier,
    transaction::{AccountUpdateData, OutputNote},
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
};

pub type NoteBatch = Vec<OutputNote>;

/// A block in the block chain.
#[derive(Debug, Clone)]
pub struct Block {
    /// Block header.
    header: BlockHeader,

    /// Account updates for the block.
    updated_accounts: Vec<AccountUpdateData>,

    /// Note batches created in transactions in the block.
    created_notes: Vec<NoteBatch>,

    /// Nullifiers produced in transactions in the block.
    created_nullifiers: Vec<Nullifier>,
    //
    // TODO: add zk proof
}

impl Block {
    /// Creates a new block.
    pub const fn new(
        header: BlockHeader,
        updated_accounts: Vec<AccountUpdateData>,
        created_notes: Vec<NoteBatch>,
        created_nullifiers: Vec<Nullifier>,
    ) -> Self {
        Self {
            header,
            updated_accounts,
            created_notes,
            created_nullifiers,
        }
    }

    /// Returns the block header.
    pub fn header(&self) -> BlockHeader {
        self.header
    }

    /// Returns the account updates.
    pub fn updated_accounts(&self) -> &[AccountUpdateData] {
        &self.updated_accounts
    }

    /// Returns the note batches.
    pub fn created_notes(&self) -> &[NoteBatch] {
        &self.created_notes
    }

    /// Returns the nullifiers.
    pub fn created_nullifiers(&self) -> &[Nullifier] {
        &self.created_nullifiers
    }

    /// Returns an iterator over created notes in the block.
    pub fn notes(&self) -> impl Iterator<Item = (BlockNoteIndex, &OutputNote)> {
        self.created_notes.iter().enumerate().flat_map(|(batch_idx, notes)| {
            notes.iter().enumerate().map(move |(note_idx_in_batch, note)| {
                (BlockNoteIndex::new(batch_idx, note_idx_in_batch), note)
            })
        })
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for Block {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.header.write_into(target);
        self.updated_accounts.write_into(target);
        self.created_notes.write_into(target);
        self.created_nullifiers.write_into(target);
    }
}

impl Deserializable for Block {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        Ok(Self {
            header: BlockHeader::read_from(source)?,
            updated_accounts: <Vec<AccountUpdateData>>::read_from(source)?,
            created_notes: <Vec<NoteBatch>>::read_from(source)?,
            created_nullifiers: <Vec<Nullifier>>::read_from(source)?,
        })
    }
}
