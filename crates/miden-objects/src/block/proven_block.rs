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
/// A block is built from batches of transactions, i.e. multiple
/// [`ProvenBatch`](crate::batch::ProvenBatch)es, and each batch contains multiple
/// [`ProvenTransaction`](crate::transaction::ProvenTransaction)s.
///
/// It consists of the following components:
/// - A [`BlockHeader`] committing to the current state of the chain and against which account, note
///   or nullifier inclusion or absence can be proven. See its documentation for details on what it
///   commits to. Eventually, it will also contain a ZK proof of the validity of the block.
/// - A list of account updates for all accounts updated in this block. For private accounts, the
///   update contains only the new account state commitments while for public accounts, the update
///   also includes the delta which can be applied to the previous account state to get the new
///   account state.
/// - A list of new notes created in this block. For private notes, the block contains only note IDs
///   and note metadata while for public notes the full note details are included.
/// - A list of new nullifiers created for all notes that were consumed in the block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenBlock {
    /// The header of the block, committing to the current state of the chain.
    header: BlockHeader,

    /// Account updates for the block.
    updated_accounts: Vec<BlockAccountUpdate>,

    /// Note batches created by the transactions in this block.
    output_note_batches: Vec<OutputNoteBatch>,

    /// Nullifiers created by the transactions in this block through the consumption of notes.
    created_nullifiers: Vec<Nullifier>,
}

impl ProvenBlock {
    /// Returns a new [`ProvenBlock`] instantiated from the provided components.
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

    /// Returns the slice of [`BlockAccountUpdate`]s for all accounts updated in this block.
    pub fn updated_accounts(&self) -> &[BlockAccountUpdate] {
        &self.updated_accounts
    }

    /// Returns the slice of [`OutputNoteBatch`]es for all output notes created in this block.
    pub fn output_note_batches(&self) -> &[OutputNoteBatch] {
        &self.output_note_batches
    }

    /// Returns an iterator over all [`OutputNote`]s created in this block.
    ///
    /// Each note is accompanied by a corresponding index specifying where the note is located
    /// in the block's [`BlockNoteTree`].
    pub fn output_notes(&self) -> impl Iterator<Item = (BlockNoteIndex, &OutputNote)> {
        self.output_note_batches.iter().enumerate().flat_map(|(batch_idx, notes)| {
            notes.iter().map(move |(note_idx_in_batch, note)| {
                (
                    // SAFETY: The proven block contains at most the max allowed number of batches
                    // and each batch is guaranteed to contain at most the
                    // max allowed number of output notes.
                    BlockNoteIndex::new(batch_idx, *note_idx_in_batch)
                        .expect("max batches in block and max notes in batches should be enforced"),
                    note,
                )
            })
        })
    }

    /// Returns the [`BlockNoteTree`] containing all [`OutputNote`]s created in this block.
    pub fn build_output_note_tree(&self) -> BlockNoteTree {
        let entries = self
            .output_notes()
            .map(|(note_index, note)| (note_index, note.id(), *note.metadata()));

        // SAFETY: We only construct proven blocks that:
        // - do not contain duplicates
        // - contain at most the max allowed number of batches and each batch is guaranteed to
        //   contain at most the max allowed number of output notes.
        BlockNoteTree::with_entries(entries)
            .expect("the output notes of the block should not contain duplicates and contain at most the allowed maximum")
    }

    /// Returns a reference to the slice of nullifiers for all notes consumed in the block.
    pub fn created_nullifiers(&self) -> &[Nullifier] {
        &self.created_nullifiers
    }

    /// Returns an iterator over all transactions which affected accounts in the block with
    /// their corresponding account IDs.
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
