use std::collections::BTreeMap;

use crate::{
    account::AccountId,
    block::{AccountWitness, BlockHeader, NullifierWitness},
    note::{NoteId, NoteInclusionProof, Nullifier},
    transaction::ChainMmr,
};

// BLOCK INPUTS
// ================================================================================================

/// Information needed from the store to build a block
#[derive(Clone, Debug)]
pub struct BlockInputs {
    /// Previous block header
    prev_block_header: BlockHeader,

    /// The chain state at the previous block with authentication paths for:
    /// - each block referenced by a batch in the block,
    /// - each block referenced by a note inclusion proof for an unauthenticated note.
    chain_mmr: ChainMmr,

    /// The hashes of the requested accounts and their authentication paths
    accounts: BTreeMap<AccountId, AccountWitness>,

    /// The requested nullifiers and their authentication paths.
    nullifiers: BTreeMap<Nullifier, NullifierWitness>,

    /// List of unauthenticated notes found in the store
    unauthenticated_note_proofs: BTreeMap<NoteId, NoteInclusionProof>,
}

impl BlockInputs {
    pub fn new(
        prev_block_header: BlockHeader,
        chain_mmr: ChainMmr,
        accounts: BTreeMap<AccountId, AccountWitness>,
        nullifiers: BTreeMap<Nullifier, NullifierWitness>,
        unauthenticated_note_proofs: BTreeMap<NoteId, NoteInclusionProof>,
    ) -> Self {
        Self {
            prev_block_header,
            chain_mmr,
            accounts,
            nullifiers,
            unauthenticated_note_proofs,
        }
    }

    pub fn prev_block_header(&self) -> &BlockHeader {
        &self.prev_block_header
    }

    pub fn chain_mmr(&self) -> &ChainMmr {
        &self.chain_mmr
    }

    pub fn accounts(&self) -> &BTreeMap<AccountId, AccountWitness> {
        &self.accounts
    }

    pub fn accounts_mut(&mut self) -> &mut BTreeMap<AccountId, AccountWitness> {
        &mut self.accounts
    }

    pub fn nullifiers(&self) -> &BTreeMap<Nullifier, NullifierWitness> {
        &self.nullifiers
    }

    pub fn unauthenticated_note_proofs(&self) -> &BTreeMap<NoteId, NoteInclusionProof> {
        &self.unauthenticated_note_proofs
    }

    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        BlockHeader,
        ChainMmr,
        BTreeMap<AccountId, AccountWitness>,
        BTreeMap<Nullifier, NullifierWitness>,
        BTreeMap<NoteId, NoteInclusionProof>,
    ) {
        (
            self.prev_block_header,
            self.chain_mmr,
            self.accounts,
            self.nullifiers,
            self.unauthenticated_note_proofs,
        )
    }
}
