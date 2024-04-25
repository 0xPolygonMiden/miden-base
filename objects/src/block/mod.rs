use alloc::vec::Vec;

use super::{Digest, Felt, Hasher, ZERO};

mod header;
pub use header::BlockHeader;
mod note_tree;
pub use note_tree::{BlockNoteIndex, BlockNoteTree};

use crate::{
    notes::Nullifier,
    transaction::{AccountUpdate, OutputNote},
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
};

pub type NoteBatch = Vec<OutputNote>;

// BLOCK
// ================================================================================================

/// A block in the Miden chain.
///
/// A block contains information resulting from executing a set of transactions against the chain
/// state defined by the previous block. It consists of 3 main components:
/// - A set of change descriptors for all accounts updated in this block. For private accounts,
///   the block contains only the new account state hashes; for public accounts, the block also
///   contains a set of state deltas which can be applied to the previous account state to get the
///   new account state.
/// - A set of new notes created in this block. For private notes, the block contains only note IDs
///   and note metadata; for public notes, full note details are recorded.
/// - A set of new nullifiers created for all notes that were consumed in the block.
///
/// In addition to the above components, a block also contains a block header which contains
/// commitments to the new state of the chain as well as a ZK proof attesting that a set of valid
/// transactions was executed to transition the chain into the state described by this block (the
/// ZK proof part is not yet implemented).
#[derive(Debug, Clone)]
pub struct Block {
    /// Block header.
    header: BlockHeader,

    /// Account updates for the block.
    updated_accounts: Vec<AccountUpdate>,

    /// Note batches created in transactions in the block.
    created_notes: Vec<NoteBatch>,

    /// Nullifiers produced in transactions in the block.
    created_nullifiers: Vec<Nullifier>,
    //
    // TODO: add zk proof
}

impl Block {
    /// Returns a new [Block] instantiated from the provided components.
    ///
    /// Note: consistency of the provided components is not validated.
    pub const fn new(
        header: BlockHeader,
        updated_accounts: Vec<AccountUpdate>,
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

    /// Returns a set of account update descriptions for all accounts updated in this block.
    pub fn updated_accounts(&self) -> &[AccountUpdate] {
        &self.updated_accounts
    }

    /// Returns a set of note batches containing all notes created in this block.
    pub fn created_notes(&self) -> &[NoteBatch] {
        &self.created_notes
    }

    /// Returns an iterator over all notes created in this block.
    ///
    /// Each note is accompanies with a corresponding index specifying where the note is located
    /// in the blocks note tree.
    pub fn notes(&self) -> impl Iterator<Item = (BlockNoteIndex, &OutputNote)> {
        self.created_notes.iter().enumerate().flat_map(|(batch_idx, notes)| {
            notes.iter().enumerate().map(move |(note_idx_in_batch, note)| {
                (BlockNoteIndex::new(batch_idx, note_idx_in_batch), note)
            })
        })
    }

    /// Returns a set of nullifiers for all notes consumed in the block.
    pub fn created_nullifiers(&self) -> &[Nullifier] {
        &self.created_nullifiers
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
            updated_accounts: <Vec<AccountUpdate>>::read_from(source)?,
            created_notes: <Vec<NoteBatch>>::read_from(source)?,
            created_nullifiers: <Vec<Nullifier>>::read_from(source)?,
        })
    }
}
