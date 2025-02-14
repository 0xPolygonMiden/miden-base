use alloc::vec::Vec;

use crate::{
    account::AccountId,
    block::{BlockAccountUpdate, BlockHeader, BlockNoteIndex, BlockNoteTree, OutputNoteBatch},
    note::Nullifier,
    transaction::{OutputNote, TransactionId},
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest,
};

// PROVEN BLOCK
// ================================================================================================

/// A block in the Miden chain.
///
/// A block contains information resulting from executing a set of transactions against the chain
/// state defined by the previous block. It consists of 3 main components:
/// - A set of change descriptors for all accounts updated in this block. For private accounts, the
///   block contains only the new account state hashes; for public accounts, the block also contains
///   a set of state deltas which can be applied to the previous account state to get the new
///   account state.
/// - A set of new notes created in this block. For private notes, the block contains only note IDs
///   and note metadata; for public notes, full note details are recorded.
/// - A set of new nullifiers created for all notes that were consumed in the block.
///
/// In addition to the above components, a block also contains a block header which contains
/// commitments to the new state of the chain as well as a ZK proof attesting that a set of valid
/// transactions was executed to transition the chain into the state described by this block (the
/// ZK proof part is not yet implemented).
#[derive(Debug, Clone)]
pub struct ProvenBlock {
    /// Block header.
    header: BlockHeader,

    /// Account updates for the block.
    updated_accounts: Vec<BlockAccountUpdate>,

    /// Note batches created by the transactions in this block.
    output_note_batches: Vec<OutputNoteBatch>,

    /// Nullifiers created by the transactions in this block through the consumption of notes.
    created_nullifiers: Vec<Nullifier>,
}

impl ProvenBlock {
    /// Returns a new [Block] instantiated from the provided components.
    ///
    /// # Warning
    ///
    /// This constructor does not do any validation, so passing incorrect values may lead to later
    /// panics.
    pub fn new_unchecked(
        header: BlockHeader,
        updated_accounts: Vec<BlockAccountUpdate>,
        output_note_batches: Vec<OutputNoteBatch>,
        created_nullifiers: Vec<Nullifier>,
    ) -> Self {
        Self {
            header,
            updated_accounts,
            output_note_batches,
            created_nullifiers,
        }
    }

    /// Returns a commitment to this block.
    pub fn hash(&self) -> Digest {
        self.header.hash()
    }

    /// Returns the header of this block.
    pub fn header(&self) -> BlockHeader {
        self.header
    }

    /// Returns a set of account update descriptions for all accounts updated in this block.
    pub fn updated_accounts(&self) -> &[BlockAccountUpdate] {
        &self.updated_accounts
    }

    /// Returns a set of note batches containing all notes created in this block.
    pub fn output_note_batches(&self) -> &[OutputNoteBatch] {
        &self.output_note_batches
    }

    /// Returns an iterator over all notes created in this block.
    ///
    /// Each note is accompanied by a corresponding index specifying where the note is located
    /// in the block's note tree.
    pub fn notes(&self) -> impl Iterator<Item = (BlockNoteIndex, &OutputNote)> {
        self.output_note_batches.iter().enumerate().flat_map(|(batch_idx, notes)| {
            notes.iter().enumerate().map(move |(note_idx_in_batch, note)| {
                (
                    // SAFETY: Batch and note index are assumed to be valid by construction of the
                    // block.
                    BlockNoteIndex::new(batch_idx, note_idx_in_batch),
                    note,
                )
            })
        })
    }

    /// Returns a note tree containing all notes created in this block.
    pub fn build_note_tree(&self) -> BlockNoteTree {
        let entries =
            self.notes().map(|(note_index, note)| (note_index, note.id(), *note.metadata()));

        BlockNoteTree::with_entries(entries)
            .expect("Something went wrong: block is invalid, but passed or skipped validation")
    }

    /// Returns a reference to the slice of nullifiers for all notes consumed in the block.
    pub fn created_nullifiers(&self) -> &[Nullifier] {
        &self.created_nullifiers
    }

    /// Returns an iterator over all transactions which affected accounts in the block with
    /// corresponding account IDs.
    pub fn transactions(&self) -> impl Iterator<Item = (TransactionId, AccountId)> + '_ {
        self.updated_accounts.iter().flat_map(|update| {
            update
                .transactions()
                .iter()
                .map(|transaction_id| (*transaction_id, update.account_id()))
        })
    }
}

impl Serializable for ProvenBlock {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.header.write_into(target);
        self.updated_accounts.write_into(target);
        self.output_note_batches.write_into(target);
        self.created_nullifiers.write_into(target);
    }
}

impl Deserializable for ProvenBlock {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let block = Self {
            header: BlockHeader::read_from(source)?,
            updated_accounts: <Vec<BlockAccountUpdate>>::read_from(source)?,
            output_note_batches: <Vec<OutputNoteBatch>>::read_from(source)?,
            created_nullifiers: <Vec<Nullifier>>::read_from(source)?,
        };

        Ok(block)
    }
}
