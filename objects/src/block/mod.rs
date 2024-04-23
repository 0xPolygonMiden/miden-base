use alloc::vec::Vec;

use super::{Digest, Felt, Hasher, ZERO};

mod header;
pub use header::BlockHeader;
mod note_tree;
pub use note_tree::BlockNoteTree;

use crate::{
    notes::Nullifier,
    transaction::{AccountUpdateData, OutputNote},
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
    produced_nullifiers: Vec<Nullifier>,
    // TODO:
    // - full states for created public notes
    // - zk proof
}

impl Block {
    /// Creates a new block.
    pub const fn new(
        header: BlockHeader,
        updated_accounts: Vec<AccountUpdateData>,
        created_notes: Vec<NoteBatch>,
        produced_nullifiers: Vec<Nullifier>,
    ) -> Self {
        Self {
            header,
            updated_accounts,
            created_notes,
            produced_nullifiers,
        }
    }

    /// Returns the block header.
    pub fn header(&self) -> BlockHeader {
        self.header
    }

    /// Returns the account updates.
    pub fn updated_accounts(&self) -> &Vec<AccountUpdateData> {
        &self.updated_accounts
    }

    /// Returns the note batches.
    pub fn created_notes(&self) -> &Vec<NoteBatch> {
        &self.created_notes
    }

    /// Returns the nullifiers.
    pub fn produced_nullifiers(&self) -> &Vec<Nullifier> {
        &self.produced_nullifiers
    }
}
