use super::{build_module_path, AdviceProvider, Felt, MemAdviceProvider, Process, Word, ZERO};
use crate::memory::{
    ACCT_CODE_ROOT_PTR, ACCT_DB_ROOT_PTR, ACCT_ID_AND_NONCE_PTR, ACCT_ID_PTR,
    ACCT_STORAGE_ROOT_PTR, ACCT_VAULT_ROOT_PTR, BATCH_ROOT_PTR, BLK_HASH_PTR, BLOCK_METADATA_PTR,
    BLOCK_NUMBER_IDX, CHAIN_MMR_NUM_LEAVES_PTR, CHAIN_MMR_PEAKS_PTR, CHAIN_ROOT_PTR,
    CONSUMED_NOTE_SECTION_OFFSET, INIT_ACCT_HASH_PTR, NOTE_ROOT_PTR, NULLIFIER_COM_PTR,
    NULLIFIER_DB_ROOT_PTR, PREV_BLOCK_HASH_PTR, PROOF_HASH_PTR, PROTOCOL_VERSION_IDX,
    TIMESTAMP_IDX,
};
use miden_objects::transaction::PreparedTransaction;
use mock::{
    constants::ACCOUNT_SEED_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
    consumed_note_data_ptr,
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    prepare_transaction, run_tx, TX_KERNEL_DIR,
};

const PROLOGUE_FILE: &str = "prologue.masm";

#[test]
fn test_transaction_prologue() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let code = "
        begin
            exec.prepare_transaction
        end
        ";

    let assembly_file = build_module_path(TX_KERNEL_DIR, PROLOGUE_FILE);
    let transaction = prepare_transaction(
        account,
        None,
        block_header,
        chain,
        notes,
        &code,
        "",
        Some(assembly_file),
    );
    let process = run_tx(
        transaction.tx_program().clone(),
        transaction.stack_inputs(),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();

    public_input_memory_assertions(&process, &transaction);
    block_data_memory_assertions(&process, &transaction);
    chain_mmr_memory_assertions(&process, &transaction);
    account_data_memory_assertions(&process, &transaction);
    consumed_notes_memory_assertions(&process, &transaction);
}

fn public_input_memory_assertions<A: AdviceProvider>(
    process: &Process<A>,
    inputs: &PreparedTransaction,
) {
    // The block hash should be stored at the BLK_HASH_PTR
    assert_eq!(
        process.get_memory_value(0, BLK_HASH_PTR).unwrap(),
        inputs.block_header().hash().as_elements()
    );

    // The account ID should be stored at the ACCT_ID_PTR
    assert_eq!(
        process.get_memory_value(0, ACCT_ID_PTR).unwrap()[0],
        inputs.account().id().into()
    );

    // The account commitment should be stored at the ACCT_HASH_PTR
    assert_eq!(
        process.get_memory_value(0, INIT_ACCT_HASH_PTR).unwrap(),
        inputs.account().hash().as_elements()
    );

    // The nullifier commitment should be stored at the NULLIFIER_COM_PTR
    assert_eq!(
        process.get_memory_value(0, NULLIFIER_COM_PTR).unwrap(),
        inputs.consumed_notes_commitment().as_elements()
    );
}

fn block_data_memory_assertions<A: AdviceProvider>(
    process: &Process<A>,
    inputs: &PreparedTransaction,
) {
    // The block hash should be stored at the BLK_HASH_PTR
    assert_eq!(
        process.get_memory_value(0, BLK_HASH_PTR).unwrap(),
        inputs.block_header().hash().as_elements()
    );

    // The previous block hash should be stored at the PREV_BLK_HASH_PTR
    assert_eq!(
        process.get_memory_value(0, PREV_BLOCK_HASH_PTR).unwrap(),
        inputs.block_header().prev_hash().as_elements()
    );

    // The chain root should be stored at the CHAIN_ROOT_PTR
    assert_eq!(
        process.get_memory_value(0, CHAIN_ROOT_PTR).unwrap(),
        inputs.block_header().chain_root().as_elements()
    );

    // The account db root should be stored at the ACCT_DB_ROOT_PRT
    assert_eq!(
        process.get_memory_value(0, ACCT_DB_ROOT_PTR).unwrap(),
        inputs.block_header().account_root().as_elements()
    );

    // The nullifier db root should be stored at the NULLIFIER_DB_ROOT_PTR
    assert_eq!(
        process.get_memory_value(0, NULLIFIER_DB_ROOT_PTR).unwrap(),
        inputs.block_header().nullifier_root().as_elements()
    );

    // The batch root should be stored at the BATCH_ROOT_PTR
    assert_eq!(
        process.get_memory_value(0, BATCH_ROOT_PTR).unwrap(),
        inputs.block_header().batch_root().as_elements()
    );

    // The note root should be stored at the NOTE_ROOT_PTR
    assert_eq!(
        process.get_memory_value(0, NOTE_ROOT_PTR).unwrap(),
        inputs.block_header().note_root().as_elements()
    );

    // The proof hash should be stored at the PROOF_HASH_PTR
    assert_eq!(
        process.get_memory_value(0, PROOF_HASH_PTR).unwrap(),
        inputs.block_header().proof_hash().as_elements()
    );

    // The block number should be stored at BLOCK_METADATA_PTR[BLOCK_NUMBER_IDX]
    assert_eq!(
        process.get_memory_value(0, BLOCK_METADATA_PTR).unwrap()[BLOCK_NUMBER_IDX],
        inputs.block_header().block_num()
    );

    // The protocol version should be stored at BLOCK_METADATA_PTR[PROTOCOL_VERSION_IDX]
    assert_eq!(
        process.get_memory_value(0, BLOCK_METADATA_PTR).unwrap()[PROTOCOL_VERSION_IDX],
        inputs.block_header().version()
    );

    // The timestamp should be stored at BLOCK_METADATA_PTR[TIMESTAMP_IDX]
    assert_eq!(
        process.get_memory_value(0, BLOCK_METADATA_PTR).unwrap()[TIMESTAMP_IDX],
        inputs.block_header().timestamp()
    );
}

fn chain_mmr_memory_assertions<A: AdviceProvider>(
    process: &Process<A>,
    inputs: &PreparedTransaction,
) {
    // The number of leaves should be stored at the CHAIN_MMR_NUM_LEAVES_PTR
    assert_eq!(
        process.get_memory_value(0, CHAIN_MMR_NUM_LEAVES_PTR).unwrap()[0],
        Felt::new(inputs.block_chain().mmr().forest() as u64)
    );

    for (i, peak) in inputs.block_chain().mmr().accumulator().peaks.iter().enumerate() {
        // The peaks should be stored at the CHAIN_MMR_PEAKS_PTR
        let i: u32 = i.try_into().expect(
            "Number of peaks is log2(number_of_leaves), this value won't be larger than 2**32",
        );
        assert_eq!(process.get_memory_value(0, CHAIN_MMR_PEAKS_PTR + i).unwrap(), Word::from(peak));
    }
}

fn account_data_memory_assertions<A: AdviceProvider>(
    process: &Process<A>,
    inputs: &PreparedTransaction,
) {
    // The account id should be stored at ACCT_ID_AND_NONCE_PTR[0]
    assert_eq!(
        process.get_memory_value(0, ACCT_ID_AND_NONCE_PTR).unwrap(),
        [inputs.account().id().into(), ZERO, ZERO, inputs.account().nonce()]
    );

    // The account vault root commitment should be stored at ACCT_VAULT_ROOT_PTR
    assert_eq!(
        process.get_memory_value(0, ACCT_VAULT_ROOT_PTR).unwrap(),
        inputs.account().vault().commitment().as_elements()
    );

    // The account storage root commitment should be stored at ACCT_STORAGE_ROOT_PTR
    assert_eq!(
        process.get_memory_value(0, ACCT_STORAGE_ROOT_PTR).unwrap(),
        Word::from(inputs.account().storage().root())
    );

    // The account code commitment should be stored at (ACCOUNT_DATA_OFFSET + 4)
    assert_eq!(
        process.get_memory_value(0, ACCT_CODE_ROOT_PTR).unwrap(),
        inputs.account().code().root().as_elements()
    );
}

fn consumed_notes_memory_assertions<A: AdviceProvider>(
    process: &Process<A>,
    inputs: &PreparedTransaction,
) {
    // The number of consumed notes should be stored at the CONSUMED_NOTES_OFFSET
    assert_eq!(
        process.get_memory_value(0, CONSUMED_NOTE_SECTION_OFFSET).unwrap()[0],
        Felt::new(inputs.consumed_notes().notes().len() as u64)
    );

    for (note, note_idx) in inputs.consumed_notes().notes().iter().zip(0u32..) {
        // The note nullifier should be computer and stored at (CONSUMED_NOTES_OFFSET + 1 + note_idx)
        assert_eq!(
            process
                .get_memory_value(0, CONSUMED_NOTE_SECTION_OFFSET + 1 + note_idx)
                .unwrap(),
            note.nullifier().as_elements()
        );

        // The note hash should be computed and stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024)
        assert_eq!(
            process.get_memory_value(0, consumed_note_data_ptr(note_idx)).unwrap(),
            note.hash().as_elements()
        );

        // The note serial num should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 1)
        assert_eq!(
            process.get_memory_value(0, consumed_note_data_ptr(note_idx) + 1).unwrap(),
            note.serial_num()
        );

        // The note script hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 2)
        assert_eq!(
            process.get_memory_value(0, consumed_note_data_ptr(note_idx) + 2).unwrap(),
            note.script().hash().as_elements()
        );

        // The note input hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 3)
        assert_eq!(
            process.get_memory_value(0, consumed_note_data_ptr(note_idx) + 3).unwrap(),
            note.inputs().hash().as_elements()
        );

        // The note vault hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 4)
        assert_eq!(
            process.get_memory_value(0, consumed_note_data_ptr(note_idx) + 4).unwrap(),
            note.vault().hash().as_elements()
        );

        // The number of assets should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 5)
        assert_eq!(
            process.get_memory_value(0, consumed_note_data_ptr(note_idx) + 5).unwrap(),
            Word::from(note.metadata())
        );

        // The assets should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 6..)
        for (asset, asset_idx) in note.vault().iter().cloned().zip(0u32..) {
            let word: Word = asset.into();
            assert_eq!(
                process
                    .get_memory_value(0, consumed_note_data_ptr(note_idx) + 6 + asset_idx)
                    .unwrap(),
                word
            );
        }
    }
}

#[test]
pub fn test_prologue_create_account() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardNew, AssetPreservationStatus::Preserved);
    let code = "
    use.miden::sat::internal::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let account_seed: Word = ACCOUNT_SEED_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN
        .iter()
        .map(|x| Felt::new(*x))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let transaction = prepare_transaction(
        account,
        Some(account_seed),
        block_header,
        chain,
        notes,
        code,
        "",
        None,
    );

    let _process = run_tx(
        transaction.tx_program().clone(),
        transaction.stack_inputs(),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();
}

#[test]
pub fn test_prologue_create_account_invalid_seed() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardNew, AssetPreservationStatus::Preserved);
    let account_seed_key = [account.id().into(), ZERO, ZERO, ZERO];

    let code = "
    use.miden::sat::internal::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    // we must provide a valid seed to `prepare_transaction` otherwise it will error
    let account_seed: Word = ACCOUNT_SEED_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN
        .iter()
        .map(|x| Felt::new(*x))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let transaction = prepare_transaction(
        account,
        Some(account_seed),
        block_header,
        chain,
        notes,
        code,
        "",
        None,
    );

    // lets override the seed with an invalid seed to ensure the kernel fails
    let mut advice_provider = MemAdviceProvider::from(transaction.advice_provider_inputs());
    advice_provider
        .insert_into_map(account_seed_key, vec![ZERO, ZERO, ZERO, ZERO])
        .unwrap();

    let process =
        run_tx(transaction.tx_program().clone(), transaction.stack_inputs(), advice_provider);
    assert!(process.is_err());
}

#[test]
fn test_get_blk_version() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let code = "
    use.miden::sat::internal::layout
    use.miden::sat::internal::prologue

    begin
        exec.prologue::prepare_transaction
        exec.layout::get_blk_version
    end
    ";

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, code, "", None);

    let process = run_tx(
        transaction.tx_program().clone(),
        transaction.stack_inputs(),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();

    assert_eq!(process.stack.get(0), block_header.version());
}

#[test]
fn test_get_blk_timestamp() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let code = "
    use.miden::sat::internal::layout
    use.miden::sat::internal::prologue

    begin
        exec.prologue::prepare_transaction
        exec.layout::get_blk_timestamp
    end
    ";

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, code, "", None);

    let process = run_tx(
        transaction.tx_program().clone(),
        transaction.stack_inputs(),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();

    assert_eq!(process.stack.get(0), block_header.timestamp());
}
