use alloc::collections::BTreeMap;

use crate::{
    account::AccountId,
    block::{AccountWitness, BlockHeader, NullifierWitness},
    note::{NoteId, NoteInclusionProof, Nullifier},
    transaction::ChainMmr,
};

// BLOCK INPUTS
// ================================================================================================

/// The required inputs for building a [`ProposedBlock`](crate::block::ProposedBlock).
#[derive(Clone, Debug)]
pub struct BlockInputs {
    /// The previous block header that the block should reference.
    prev_block_header: BlockHeader,

    /// The chain state at the previous block with authentication paths for:
    /// - each block referenced by a batch in the block,
    /// - each block referenced by a note inclusion proof for an unauthenticated note.
    chain_mmr: ChainMmr,

    /// The state commitments of the accounts in the block and their authentication paths.
    account_witnesses: BTreeMap<AccountId, AccountWitness>,

    /// The nullifiers of the notes consumed in the block and their authentication paths.
    nullifiers: BTreeMap<Nullifier, NullifierWitness>,

    /// Note inclusion proofs for all unauthenticated notes in the block that are not erased (i.e.
    /// created and consumed within the block).
    unauthenticated_note_proofs: BTreeMap<NoteId, NoteInclusionProof>,
}

impl BlockInputs {
    /// Creates new [`BlockInputs`] from the provided parts.
    pub fn new(
        prev_block_header: BlockHeader,
        chain_mmr: ChainMmr,
        account_witnesses: BTreeMap<AccountId, AccountWitness>,
        nullifiers: BTreeMap<Nullifier, NullifierWitness>,
        unauthenticated_note_proofs: BTreeMap<NoteId, NoteInclusionProof>,
    ) -> Self {
        Self {
            prev_block_header,
            chain_mmr,
            account_witnesses,
            nullifiers,
            unauthenticated_note_proofs,
        }
    }

    /// Returns a reference to the previous block header.
    pub fn prev_block_header(&self) -> &BlockHeader {
        &self.prev_block_header
    }

    /// Returns a reference to the [`ChainMmr`].
    pub fn chain_mmr(&self) -> &ChainMmr {
        &self.chain_mmr
    }

    /// Returns a reference to the account witnesses.
    pub fn account_witnesses(&self) -> &BTreeMap<AccountId, AccountWitness> {
        &self.account_witnesses
    }

    /// Returns a reference to the nullifier witnesses.
    pub fn nullifiers(&self) -> &BTreeMap<Nullifier, NullifierWitness> {
        &self.nullifiers
    }

    /// Returns a reference to the note inclusion proofs.
    pub fn unauthenticated_note_proofs(&self) -> &BTreeMap<NoteId, NoteInclusionProof> {
        &self.unauthenticated_note_proofs
    }

    /// Consumes self and returns the underlying parts.
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
            self.account_witnesses,
            self.nullifiers,
            self.unauthenticated_note_proofs,
        )
    }
}
