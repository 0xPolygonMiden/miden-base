use super::{Digest, Felt, NoteError, NOTE_TREE_DEPTH};
use crypto::merkle::{MerklePath, NodeIndex};

/// Represents the origin of a note.  This includes:
/// block_num  - the block number the note was created in.
/// sub_hash   - the sub hash of the block the note was created in.
/// note_root  - the note root of the block the note was created in.
/// note_index - the index of the note in the note Merkle tree of the block the note was created
///              in.
/// note_path  - the Merkle path to the note in the note Merkle tree of the block the note was
///              created in.
#[derive(Debug)]
pub struct NoteOrigin {
    block_num: Felt,
    sub_hash: Digest,
    note_root: Digest,
    node_index: NodeIndex,
    note_path: MerklePath,
}

impl NoteOrigin {
    /// Creates a new note origin.
    pub fn new(
        block_num: Felt,
        sub_hash: Digest,
        note_root: Digest,
        index: u64,
        note_path: MerklePath,
    ) -> Result<Self, NoteError> {
        Ok(Self {
            block_num,
            sub_hash,
            note_root,
            node_index: NodeIndex::new(NOTE_TREE_DEPTH, index)
                .map_err(|e| NoteError::invalid_origin_index(e.to_string()))?,
            note_path,
        })
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the block number the note was created in.
    pub fn block_num(&self) -> Felt {
        self.block_num
    }

    /// Returns the sub hash of the block header the note was created in.
    pub fn sub_hash(&self) -> Digest {
        self.sub_hash
    }

    /// Returns the note root of the block header the note was created in.
    pub fn note_root(&self) -> Digest {
        self.note_root
    }

    /// Returns the node index of the note in the note Merkle tree.
    pub fn node_index(&self) -> NodeIndex {
        self.node_index
    }

    /// Returns the Merkle path to the note in the note Merkle tree of the block the note was
    /// created in.
    pub fn note_path(&self) -> &MerklePath {
        &self.note_path
    }
}
