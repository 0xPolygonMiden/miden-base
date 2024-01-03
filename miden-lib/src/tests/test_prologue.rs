use assembly::ast::ProgramAst;
use miden_objects::transaction::{PreparedTransaction, TransactionScript};
use mock::{
    constants::{generate_account_seed, AccountSeedType},
    consumed_note_data_ptr,
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    prepare_transaction, run_tx,
};

use super::{
    build_module_path, build_tx_inputs, AdviceProvider, ContextId, DefaultHost, Felt, Process,
    ProcessState, Word, TX_KERNEL_DIR, ZERO,
};
use crate::transaction::{
    memory::{
        ACCT_CODE_ROOT_PTR, ACCT_DB_ROOT_PTR, ACCT_ID_AND_NONCE_PTR, ACCT_ID_PTR,
        ACCT_STORAGE_ROOT_PTR, ACCT_STORAGE_SLOT_TYPE_DATA_OFFSET, ACCT_VAULT_ROOT_PTR,
        BATCH_ROOT_PTR, BLK_HASH_PTR, BLOCK_METADATA_PTR, BLOCK_NUMBER_IDX,
        CHAIN_MMR_NUM_LEAVES_PTR, CHAIN_MMR_PEAKS_PTR, CHAIN_ROOT_PTR,
        CONSUMED_NOTE_SECTION_OFFSET, INIT_ACCT_HASH_PTR, INIT_NONCE_PTR, NOTE_ROOT_PTR,
        NULLIFIER_COM_PTR, NULLIFIER_DB_ROOT_PTR, PREV_BLOCK_HASH_PTR, PROOF_HASH_PTR,
        PROTOCOL_VERSION_IDX, TIMESTAMP_IDX, TX_SCRIPT_ROOT_PTR,
    },
    TransactionKernel,
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

    let mock_tx_script_code = ProgramAst::parse(
        "
        begin
            push.1.2.3.4 dropw
        end
    ",
    )
    .unwrap();
    let (tx_script, _) =
        TransactionScript::new(mock_tx_script_code, vec![], &mut TransactionKernel::assembler())
            .unwrap();

    let assembly_file = build_module_path(TX_KERNEL_DIR, PROLOGUE_FILE);
    let transaction = prepare_transaction(
        account,
        None,
        block_header,
        chain,
        notes,
        Some(tx_script),
        code,
        "",
        Some(assembly_file),
    );
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider).unwrap();

    global_input_memory_assertions(&process, &transaction);
    block_data_memory_assertions(&process, &transaction);
    chain_mmr_memory_assertions(&process, &transaction);
    account_data_memory_assertions(&process, &transaction);
    consumed_notes_memory_assertions(&process, &transaction);
}

fn global_input_memory_assertions<A: AdviceProvider>(
    process: &Process<DefaultHost<A>>,
    inputs: &PreparedTransaction,
) {
    // The block hash should be stored at the BLK_HASH_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), BLK_HASH_PTR).unwrap(),
        inputs.block_header().hash().as_elements()
    );

    // The account ID should be stored at the ACCT_ID_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), ACCT_ID_PTR).unwrap()[0],
        inputs.account().id().into()
    );

    // The account commitment should be stored at the ACCT_HASH_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), INIT_ACCT_HASH_PTR).unwrap(),
        inputs.account().hash().as_elements()
    );

    // The nullifier commitment should be stored at the NULLIFIER_COM_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), NULLIFIER_COM_PTR).unwrap(),
        inputs.input_notes().commitment().as_elements()
    );

    // The initial nonce should be stored at the INIT_NONCE_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), INIT_NONCE_PTR).unwrap()[0],
        inputs.account().nonce()
    );

    // The transaction script root should be stored at the TX_SCRIPT_ROOT_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), TX_SCRIPT_ROOT_PTR).unwrap(),
        **inputs.tx_script().as_ref().unwrap().hash()
    );
}

fn block_data_memory_assertions<A: AdviceProvider>(
    process: &Process<DefaultHost<A>>,
    inputs: &PreparedTransaction,
) {
    // The block hash should be stored at the BLK_HASH_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), BLK_HASH_PTR).unwrap(),
        inputs.block_header().hash().as_elements()
    );

    // The previous block hash should be stored at the PREV_BLK_HASH_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), PREV_BLOCK_HASH_PTR).unwrap(),
        inputs.block_header().prev_hash().as_elements()
    );

    // The chain root should be stored at the CHAIN_ROOT_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), CHAIN_ROOT_PTR).unwrap(),
        inputs.block_header().chain_root().as_elements()
    );

    // The account db root should be stored at the ACCT_DB_ROOT_PRT
    assert_eq!(
        process.get_mem_value(ContextId::root(), ACCT_DB_ROOT_PTR).unwrap(),
        inputs.block_header().account_root().as_elements()
    );

    // The nullifier db root should be stored at the NULLIFIER_DB_ROOT_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), NULLIFIER_DB_ROOT_PTR).unwrap(),
        inputs.block_header().nullifier_root().as_elements()
    );

    // The batch root should be stored at the BATCH_ROOT_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), BATCH_ROOT_PTR).unwrap(),
        inputs.block_header().batch_root().as_elements()
    );

    // The note root should be stored at the NOTE_ROOT_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), NOTE_ROOT_PTR).unwrap(),
        inputs.block_header().note_root().as_elements()
    );

    // The proof hash should be stored at the PROOF_HASH_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), PROOF_HASH_PTR).unwrap(),
        inputs.block_header().proof_hash().as_elements()
    );

    // The block number should be stored at BLOCK_METADATA_PTR[BLOCK_NUMBER_IDX]
    assert_eq!(
        process.get_mem_value(ContextId::root(), BLOCK_METADATA_PTR).unwrap()[BLOCK_NUMBER_IDX],
        inputs.block_header().block_num().into()
    );

    // The protocol version should be stored at BLOCK_METADATA_PTR[PROTOCOL_VERSION_IDX]
    assert_eq!(
        process.get_mem_value(ContextId::root(), BLOCK_METADATA_PTR).unwrap()[PROTOCOL_VERSION_IDX],
        inputs.block_header().version()
    );

    // The timestamp should be stored at BLOCK_METADATA_PTR[TIMESTAMP_IDX]
    assert_eq!(
        process.get_mem_value(ContextId::root(), BLOCK_METADATA_PTR).unwrap()[TIMESTAMP_IDX],
        inputs.block_header().timestamp()
    );
}

fn chain_mmr_memory_assertions<A: AdviceProvider>(
    process: &Process<DefaultHost<A>>,
    inputs: &PreparedTransaction,
) {
    // The number of leaves should be stored at the CHAIN_MMR_NUM_LEAVES_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), CHAIN_MMR_NUM_LEAVES_PTR).unwrap()[0],
        Felt::new(inputs.tx_inputs().block_chain().chain_length() as u64)
    );

    for (i, peak) in inputs.tx_inputs().block_chain().peaks().peaks().iter().enumerate() {
        // The peaks should be stored at the CHAIN_MMR_PEAKS_PTR
        let i: u32 = i.try_into().expect(
            "Number of peaks is log2(number_of_leaves), this value won't be larger than 2**32",
        );
        assert_eq!(
            process.get_mem_value(ContextId::root(), CHAIN_MMR_PEAKS_PTR + i).unwrap(),
            Word::from(peak)
        );
    }
}

fn account_data_memory_assertions<A: AdviceProvider>(
    process: &Process<DefaultHost<A>>,
    inputs: &PreparedTransaction,
) {
    // The account id should be stored at ACCT_ID_AND_NONCE_PTR[0]
    assert_eq!(
        process.get_mem_value(ContextId::root(), ACCT_ID_AND_NONCE_PTR).unwrap(),
        [inputs.account().id().into(), ZERO, ZERO, inputs.account().nonce()]
    );

    // The account vault root commitment should be stored at ACCT_VAULT_ROOT_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), ACCT_VAULT_ROOT_PTR).unwrap(),
        inputs.account().vault().commitment().as_elements()
    );

    // The account storage root commitment should be stored at ACCT_STORAGE_ROOT_PTR
    assert_eq!(
        process.get_mem_value(ContextId::root(), ACCT_STORAGE_ROOT_PTR).unwrap(),
        Word::from(inputs.account().storage().root())
    );

    // The account code commitment should be stored at (ACCOUNT_DATA_OFFSET + 4)
    assert_eq!(
        process.get_mem_value(ContextId::root(), ACCT_CODE_ROOT_PTR).unwrap(),
        inputs.account().code().root().as_elements()
    );

    // The account types data should be stored in
    // (ACCT_STORAGE_SLOT_TYPE_DATA_OFFSET..ACCT_STORAGE_SLOT_TYPE_DATA_OFFSET + 64)
    for (types, types_ptr) in inputs
        .account()
        .storage()
        .slot_types()
        .chunks(4)
        .zip(ACCT_STORAGE_SLOT_TYPE_DATA_OFFSET..)
    {
        assert_eq!(
            process.get_mem_value(ContextId::root(), types_ptr).unwrap(),
            Word::try_from(types.iter().map(Felt::from).collect::<Vec<_>>()).unwrap()
        );
    }
}

fn consumed_notes_memory_assertions<A: AdviceProvider>(
    process: &Process<DefaultHost<A>>,
    inputs: &PreparedTransaction,
) {
    // The number of consumed notes should be stored at the CONSUMED_NOTES_OFFSET
    assert_eq!(
        process.get_mem_value(ContextId::root(), CONSUMED_NOTE_SECTION_OFFSET).unwrap()[0],
        Felt::new(inputs.input_notes().num_notes() as u64)
    );

    for (note, note_idx) in inputs.input_notes().iter().zip(0_u32..) {
        // The note nullifier should be computer and stored at (CONSUMED_NOTES_OFFSET + 1 + note_idx)
        assert_eq!(
            process
                .get_mem_value(ContextId::root(), CONSUMED_NOTE_SECTION_OFFSET + 1 + note_idx)
                .unwrap(),
            note.note().nullifier().as_elements()
        );

        // The ID hash should be computed and stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024)
        assert_eq!(
            process
                .get_mem_value(ContextId::root(), consumed_note_data_ptr(note_idx))
                .unwrap(),
            note.id().as_elements()
        );

        // The note serial num should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 1)
        assert_eq!(
            process
                .get_mem_value(ContextId::root(), consumed_note_data_ptr(note_idx) + 1)
                .unwrap(),
            note.note().serial_num()
        );

        // The note script hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 2)
        assert_eq!(
            process
                .get_mem_value(ContextId::root(), consumed_note_data_ptr(note_idx) + 2)
                .unwrap(),
            note.note().script().hash().as_elements()
        );

        // The note input hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 3)
        assert_eq!(
            process
                .get_mem_value(ContextId::root(), consumed_note_data_ptr(note_idx) + 3)
                .unwrap(),
            note.note().inputs().hash().as_elements()
        );

        // The note vault hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 4)
        assert_eq!(
            process
                .get_mem_value(ContextId::root(), consumed_note_data_ptr(note_idx) + 4)
                .unwrap(),
            note.note().vault().hash().as_elements()
        );

        // The number of assets should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 5)
        assert_eq!(
            process
                .get_mem_value(ContextId::root(), consumed_note_data_ptr(note_idx) + 5)
                .unwrap(),
            Word::from(note.note().metadata())
        );

        // The assets should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 6..)
        for (asset, asset_idx) in note.note().vault().iter().cloned().zip(0u32..) {
            let word: Word = asset.into();
            assert_eq!(
                process
                    .get_mem_value(
                        ContextId::root(),
                        consumed_note_data_ptr(note_idx) + 6 + asset_idx
                    )
                    .unwrap(),
                word
            );
        }
    }
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account() {
    let (_acct_id, account_seed) =
        generate_account_seed(AccountSeedType::RegularAccountUpdatableCodeOnChain);
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardNew, AssetPreservationStatus::Preserved);
    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let transaction = prepare_transaction(
        account,
        Some(account_seed),
        block_header,
        chain,
        notes,
        None,
        code,
        "",
        None,
    );
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let _process = run_tx(program, stack_inputs, advice_provider).unwrap();
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_valid_fungible_faucet_reserved_slot() {
    let (acct_id, account_seed) =
        generate_account_seed(AccountSeedType::FungibleFaucetValidInitialBalance);
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::FungibleFaucet {
            acct_id: acct_id.into(),
            nonce: ZERO,
            empty_reserved_slot: true,
        },
        AssetPreservationStatus::Preserved,
    );
    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let transaction = prepare_transaction(
        account,
        Some(account_seed),
        block_header,
        chain,
        notes,
        None,
        code,
        "",
        None,
    );
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_ok());
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_invalid_fungible_faucet_reserved_slot() {
    let (acct_id, account_seed) =
        generate_account_seed(AccountSeedType::FungibleFaucetInvalidInitialBalance);
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::FungibleFaucet {
            acct_id: acct_id.into(),
            nonce: ZERO,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );
    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let transaction = prepare_transaction(
        account,
        Some(account_seed),
        block_header,
        chain,
        notes,
        None,
        code,
        "",
        None,
    );
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_valid_non_fungible_faucet_reserved_slot() {
    let (acct_id, account_seed) =
        generate_account_seed(AccountSeedType::NonFungibleFaucetValidReservedSlot);
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::NonFungibleFaucet {
            acct_id: acct_id.into(),
            nonce: ZERO,
            empty_reserved_slot: true,
        },
        AssetPreservationStatus::Preserved,
    );
    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let transaction = prepare_transaction(
        account,
        Some(account_seed),
        block_header,
        chain,
        notes,
        None,
        code,
        "",
        None,
    );
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_ok())
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_invalid_non_fungible_faucet_reserved_slot() {
    let (acct_id, account_seed) =
        generate_account_seed(AccountSeedType::NonFungibleFaucetInvalidReservedSlot);
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::NonFungibleFaucet {
            acct_id: acct_id.into(),
            nonce: ZERO,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );
    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let transaction = prepare_transaction(
        account,
        Some(account_seed),
        block_header,
        chain,
        notes,
        None,
        code,
        "",
        None,
    );
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);

    let process = run_tx(program, stack_inputs, advice_provider);
    assert!(process.is_err());
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_invalid_seed() {
    let (_acct_id, account_seed) =
        generate_account_seed(AccountSeedType::RegularAccountUpdatableCodeOnChain);
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardNew, AssetPreservationStatus::Preserved);
    let account_seed_key = [account.id().into(), ZERO, ZERO, ZERO];

    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let transaction = prepare_transaction(
        account,
        Some(account_seed),
        block_header,
        chain,
        notes,
        None,
        code,
        "",
        None,
    );
    let (program, stack_inputs, mut advice_provider) = build_tx_inputs(&transaction);

    // lets override the seed with an invalid seed to ensure the kernel fails
    advice_provider
        .insert_into_map(account_seed_key, vec![ZERO, ZERO, ZERO, ZERO])
        .unwrap();

    let process = run_tx(program, stack_inputs, &mut advice_provider);
    assert!(process.is_err());
}

#[test]
fn test_get_blk_version() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let code = "
    use.miden::kernels::tx::memory
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
        exec.memory::get_blk_version
    end
    ";

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider).unwrap();

    assert_eq!(process.stack.get(0), block_header.version());
}

#[test]
fn test_get_blk_timestamp() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let code = "
    use.miden::kernels::tx::memory
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
        exec.memory::get_blk_timestamp
    end
    ";

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider).unwrap();

    assert_eq!(process.stack.get(0), block_header.timestamp());
}
