use super::{Digest, Felt, NoteError, ToString, NOTE_TREE_DEPTH};
use crate::crypto::merkle::{MerklePath, NodeIndex};

/// Contains information about the origin of a note.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteOrigin {
    pub block_num: Felt,
    pub node_index: NodeIndex,
}

/// Contains the data required to prove inclusion of a note in the canonical chain.
///
/// block_num  - the block number the note was created in.
/// sub_hash   - the sub hash of the block the note was created in.
/// note_root  - the note root of the block the note was created in.
/// note_index - the index of the note in the note Merkle tree of the block the note was created
///              in.
/// note_path  - the Merkle path to the note in the note Merkle tree of the block the note was
///              created in.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteInclusionProof {
    origin: NoteOrigin,
    sub_hash: Digest,
    note_root: Digest,
    note_path: MerklePath,
}

impl NoteInclusionProof {
    /// Creates a new note origin.
    pub fn new(
        block_num: Felt,
        sub_hash: Digest,
        note_root: Digest,
        index: u64,
        note_path: MerklePath,
    ) -> Result<Self, NoteError> {
        let node_index = NodeIndex::new(NOTE_TREE_DEPTH, index)
            .map_err(|e| NoteError::invalid_origin_index(e.to_string()))?;
        Ok(Self {
            origin: NoteOrigin {
                block_num,
                node_index,
            },
            sub_hash,
            note_root,
            note_path,
        })
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the sub hash of the block header the note was created in.
    pub fn sub_hash(&self) -> Digest {
        self.sub_hash
    }

    /// Returns the note root of the block header the note was created in.
    pub fn note_root(&self) -> Digest {
        self.note_root
    }

    /// Returns the origin of the note.
    pub fn origin(&self) -> &NoteOrigin {
        &self.origin
    }

    /// Returns the Merkle path to the note in the note Merkle tree of the block the note was
    /// created in.
    pub fn note_path(&self) -> &MerklePath {
        &self.note_path
    }
}
