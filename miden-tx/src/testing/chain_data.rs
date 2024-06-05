use alloc::vec::Vec;

use miden_objects::{
    crypto::merkle::{LeafIndex, Mmr, PartialMmr},
    notes::{Note, NoteInclusionProof},
    transaction::{ChainMmr, InputNote},
    BlockHeader, NOTE_TREE_DEPTH,
};
use vm_processor::crypto::SimpleSmt;

pub fn mock_chain_data(consumed_notes: Vec<Note>) -> (ChainMmr, Vec<InputNote>) {
    let mut note_trees = Vec::new();

    // TODO: Consider how to better represent note authentication data.
    // we use the index for both the block number and the leaf index in the note tree
    for (index, note) in consumed_notes.iter().enumerate() {
        let smt_entries = vec![(index as u64, note.authentication_hash().into())];
        let smt = SimpleSmt::<NOTE_TREE_DEPTH>::with_leaves(smt_entries).unwrap();
        note_trees.push(smt);
    }

    let mut note_tree_iter = note_trees.iter();

    // create a dummy chain of block headers
    let block_chain = vec![
        BlockHeader::mock(0, None, note_tree_iter.next().map(|x| x.root()), &[]),
        BlockHeader::mock(1, None, note_tree_iter.next().map(|x| x.root()), &[]),
        BlockHeader::mock(2, None, note_tree_iter.next().map(|x| x.root()), &[]),
        BlockHeader::mock(3, None, note_tree_iter.next().map(|x| x.root()), &[]),
    ];

    // instantiate and populate MMR
    let mut mmr = Mmr::default();
    for block_header in block_chain.iter() {
        mmr.add(block_header.hash())
    }
    let chain_mmr = mmr_to_chain_mmr(&mmr, &block_chain);

    // set origin for consumed notes using chain and block data
    let recorded_notes = consumed_notes
        .into_iter()
        .enumerate()
        .map(|(index, note)| {
            let block_header = &block_chain[index];
            let auth_index = LeafIndex::new(index as u64).unwrap();

            InputNote::new(
                note,
                NoteInclusionProof::new(
                    block_header.block_num(),
                    block_header.sub_hash(),
                    block_header.note_root(),
                    index as u64,
                    note_trees[index].open(&auth_index).path,
                )
                .unwrap(),
            )
        })
        .collect::<Vec<_>>();

    (chain_mmr, recorded_notes)
}

// HELPER FUNCTIONS
// ================================================================================================

/// Converts the MMR into partial MMR by copying all leaves from MMR to partial MMR.
fn mmr_to_chain_mmr(mmr: &Mmr, blocks: &[BlockHeader]) -> ChainMmr {
    let num_leaves = mmr.forest();
    let mut partial_mmr = PartialMmr::from_peaks(mmr.peaks(mmr.forest()).unwrap());

    for i in 0..num_leaves {
        let node = mmr.get(i).unwrap();
        let path = mmr.open(i, mmr.forest()).unwrap().merkle_path;
        partial_mmr.track(i, node, &path).unwrap();
    }

    ChainMmr::new(partial_mmr, blocks.to_vec()).unwrap()
}
