#[cfg(not(target_family = "wasm"))]
mod internal {
    use crate::{
        mock::mock_block_header,
        notes::{Note, NoteInclusionProof, NOTE_LEAF_DEPTH, NOTE_TREE_DEPTH},
        ChainMmr, Felt, Vec,
    };
    use crypto::merkle::SimpleSmt;
    use vm_core::{crypto::merkle::NodeIndex, FieldElement};

    pub fn mock_chain_data(consumed_notes: &mut [Note]) -> ChainMmr {
        let mut note_trees = Vec::new();

        // TODO: Consider how to better represent note authentication data.
        // we use the index for both the block number and the leaf index in the note tree
        for (index, note) in consumed_notes.iter().enumerate() {
            let tree_index = 2 * index;
            let smt_entries = vec![
                (tree_index as u64, note.hash().into()),
                ((tree_index + 1) as u64, note.metadata().into()),
            ];
            let smt = SimpleSmt::with_leaves(NOTE_LEAF_DEPTH, smt_entries).unwrap();
            note_trees.push(smt);
        }

        let mut note_tree_iter = note_trees.iter();

        // create a dummy chain of block headers
        let block_chain = vec![
            mock_block_header(Felt::ZERO, None, note_tree_iter.next().map(|x| x.root()), &[]),
            mock_block_header(Felt::ONE, None, note_tree_iter.next().map(|x| x.root()), &[]),
            mock_block_header(Felt::new(2), None, note_tree_iter.next().map(|x| x.root()), &[]),
            mock_block_header(Felt::new(3), None, note_tree_iter.next().map(|x| x.root()), &[]),
        ];

        // instantiate and populate MMR
        let mut chain_mmr = ChainMmr::default();
        for block_header in block_chain.iter() {
            chain_mmr.mmr_mut().add(block_header.hash())
        }

        // set origin for consumed notes using chain and block data
        for (index, note) in consumed_notes.iter_mut().enumerate() {
            let block_header = &block_chain[index];
            let auth_index = NodeIndex::new(NOTE_TREE_DEPTH, index as u64).unwrap();
            note.set_proof(
                NoteInclusionProof::new(
                    block_header.block_num(),
                    block_header.sub_hash(),
                    block_header.note_root(),
                    index as u64,
                    note_trees[index].get_path(auth_index).unwrap(),
                )
                .unwrap(),
            );
        }

        chain_mmr
    }
}

#[cfg(target_family = "wasm")]
mod internal {}

pub use internal::*;
