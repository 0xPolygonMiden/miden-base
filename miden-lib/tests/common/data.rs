use super::{
    Account, AccountId, Asset, BlockHeader, Digest, ExecutedTransaction, Felt, FieldElement,
    FungibleAsset, MerkleStore, Mmr, NodeIndex, Note, NoteOrigin, TransactionInputs, Word,
    NOTE_LEAF_DEPTH, NOTE_TREE_DEPTH,
};
use crypto::merkle::SimpleSmt;
use test_utils::rand;

// MOCK DATA
// ================================================================================================
pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u64 = 0b0010011011u64 << 54;
pub const ACCOUNT_ID_SENDER: u64 = 0b0110111011u64 << 54;

pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1010011100 << 54;

pub const NONCE: Felt = Felt::ZERO;

pub fn mock_block_header(
    block_num: Felt,
    chain_root: Option<Digest>,
    note_root: Option<Digest>,
) -> BlockHeader {
    let prev_hash: Digest = rand::rand_array().into();
    let chain_root: Digest = chain_root.unwrap_or(rand::rand_array().into());
    let state_root: Digest = rand::rand_array().into();
    let note_root: Digest = note_root.unwrap_or(rand::rand_array().into());
    let batch_root: Digest = rand::rand_array().into();
    let proof_hash: Digest = rand::rand_array().into();

    BlockHeader::new(
        prev_hash, block_num, chain_root, state_root, note_root, batch_root, proof_hash,
    )
}

pub fn mock_chain_data(merkle_store: &mut MerkleStore, consumed_notes: &mut [Note]) -> Mmr {
    let mut peaks = Vec::new();

    // we use the index for both the block number and the leaf index
    for (index, note) in consumed_notes.iter().enumerate() {
        let tree_index = 2 * index;
        let smt_entries = vec![
            (tree_index as u64, note.hash().into()),
            ((tree_index + 1) as u64, note.metadata().into()),
        ];
        let smt = SimpleSmt::new(NOTE_LEAF_DEPTH).unwrap().with_leaves(smt_entries).unwrap();
        merkle_store.extend(smt.inner_nodes());
        peaks.push(smt.root());
    }

    // create a dummy chain of block headers
    let block_chain = vec![
        mock_block_header(Felt::ZERO, None, Some(peaks[0].into())),
        mock_block_header(Felt::ONE, None, Some(peaks[1].into())),
        mock_block_header(Felt::new(2), None, None),
        mock_block_header(Felt::new(3), None, None),
    ];

    // convert block hashes into words
    let block_hashes: Vec<Word> = block_chain.iter().map(|h| Word::from(h.hash())).collect();

    // instantiate and populate MMR
    let mut mmr = Mmr::new();
    for hash in block_hashes.iter() {
        mmr.add(*hash)
    }

    // set origin for consumed notes using chain and block data
    for (index, note) in consumed_notes.iter_mut().enumerate() {
        let block_header = &block_chain[index];
        let auth_index = NodeIndex::new(NOTE_TREE_DEPTH, index as u64).unwrap();
        note.set_origin(
            NoteOrigin::new(
                block_header.block_num(),
                block_header.sub_hash(),
                block_header.note_root(),
                index as u64,
                merkle_store.get_path(*block_header.note_root(), auth_index).unwrap().path,
            )
            .unwrap(),
        );
    }

    // add MMR to the store
    merkle_store.extend(mmr.inner_nodes());
    mmr
}

pub fn mock_inputs() -> (MerkleStore, TransactionInputs) {
    // Create an account
    let account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let account = Account::new(account_id, &[], "proc.test_proc push.1 end", Felt::ZERO).unwrap();

    // Create a Merkle store
    let mut merkle_store = MerkleStore::new();

    // Consumed notes
    let mut consumed_notes = mock_consumed_notes();

    // Chain data
    let chain_mmr: Mmr = mock_chain_data(&mut merkle_store, &mut consumed_notes);

    // Block header
    let block_header: BlockHeader =
        mock_block_header(Felt::new(4), Some(chain_mmr.accumulator().hash_peaks().into()), None);

    // Transaction inputs
    (
        merkle_store,
        TransactionInputs::new(account, block_header, chain_mmr, consumed_notes, None),
    )
}

pub fn mock_executed_tx() -> (MerkleStore, ExecutedTransaction) {
    // AccountId
    let account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();

    // Initial Account
    let initial_account =
        Account::new(account_id, &[], "proc.test_proc push.1 end", Felt::ZERO).unwrap();

    // Finial Account (nonce incremented by 1)
    let final_account =
        Account::new(account_id, &[], "proc.test_proc push.1 end", Felt::ONE).unwrap();

    // Consumed notes
    let mut consumed_notes = mock_consumed_notes();

    // Created notes
    let created_notes = mock_created_notes();

    // Create a Merkle store
    let mut merkle_store = MerkleStore::new();

    // Chain data
    let chain_mmr: Mmr = mock_chain_data(&mut merkle_store, &mut consumed_notes);

    // Block header
    let block_header: BlockHeader =
        mock_block_header(Felt::new(4), Some(chain_mmr.accumulator().hash_peaks().into()), None);

    // Executed Transaction
    (
        merkle_store,
        ExecutedTransaction::new(
            initial_account,
            final_account,
            consumed_notes,
            created_notes,
            None,
            block_header,
            chain_mmr,
        ),
    )
}

fn mock_consumed_notes() -> Vec<Note> {
    // Note Assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 10).unwrap();
    let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 20).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();
    let fungible_asset_2: Asset = FungibleAsset::new(faucet_id_2, 200).unwrap().into();
    let fungible_asset_3: Asset = FungibleAsset::new(faucet_id_3, 300).unwrap().into();

    // Sender account
    let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // Consumed Notes
    const SERIAL_NUM_1: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let note_1 = Note::new(
        "begin push.1 end",
        &[Felt::new(1)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_1,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    const SERIAL_NUM_2: Word = [Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)];
    let note_2 = Note::new(
        "begin push.1 end",
        &[Felt::new(2)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_2,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    vec![note_1, note_2]
}

fn mock_created_notes() -> Vec<Note> {
    // Note assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 10).unwrap();
    let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 20).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();
    let fungible_asset_2: Asset = FungibleAsset::new(faucet_id_2, 100).unwrap().into();
    let fungible_asset_3: Asset = FungibleAsset::new(faucet_id_3, 100).unwrap().into();

    // sender account
    let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // Created Notes
    const SERIAL_NUM_1: Word = [Felt::new(9), Felt::new(10), Felt::new(11), Felt::new(12)];
    let note_1 = Note::new(
        "begin push.1 end",
        &[Felt::new(1)],
        &[fungible_asset_1, fungible_asset_2],
        SERIAL_NUM_1,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    const SERIAL_NUM_2: Word = [Felt::new(13), Felt::new(14), Felt::new(15), Felt::new(16)];
    let note_2 = Note::new(
        "begin push.1 end",
        &[Felt::new(2)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_2,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    const SERIAL_NUM_3: Word = [Felt::new(17), Felt::new(18), Felt::new(19), Felt::new(20)];
    let note_3 = Note::new(
        "begin push.1 end",
        &[Felt::new(2)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_3,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    vec![note_1, note_2, note_3]
}
