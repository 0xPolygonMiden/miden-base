use common::{build_module_path, TX_KERNEL_DIR};
use miden_lib::transaction::{
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
use miden_objects::{
    assembly::ProgramAst,
    transaction::{PreparedTransaction, TransactionScript},
    Digest, Felt, Word, ZERO,
};
use mock::{
    constants::{generate_account_seed, AccountSeedType},
    consumed_note_data_ptr,
    mock::{
        account::MockAccountType,
        host::MockHost,
        notes::AssetPreservationStatus,
        transaction::{mock_inputs, mock_inputs_with_account_seed},
    },
    prepare_transaction, run_tx, run_tx_with_inputs,
};
use vm_processor::{AdviceInputs, ContextId, Process, ProcessState};

mod common;

const PROLOGUE_FILE: &str = "prologue.masm";

// TESTS
// ================================================================================================

#[test]
fn test_transaction_prologue() {
    let tx_inputs =
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
    let transaction = prepare_transaction(tx_inputs, Some(tx_script), code, Some(assembly_file));
    let process = run_tx(&transaction).unwrap();

    global_input_memory_assertions(&process, &transaction);
    block_data_memory_assertions(&process, &transaction);
    chain_mmr_memory_assertions(&process, &transaction);
    account_data_memory_assertions(&process, &transaction);
    consumed_notes_memory_assertions(&process, &transaction);
}

fn global_input_memory_assertions(process: &Process<MockHost>, inputs: &PreparedTransaction) {
    // The block hash should be stored at the BLK_HASH_PTR
    assert_eq!(
        read_root_mem_value(process, BLK_HASH_PTR),
        inputs.block_header().hash().as_elements()
    );

    // The account ID should be stored at the ACCT_ID_PTR
    assert_eq!(read_root_mem_value(process, ACCT_ID_PTR)[0], inputs.account().id().into());

    // The account commitment should be stored at the ACCT_HASH_PTR
    assert_eq!(
        read_root_mem_value(process, INIT_ACCT_HASH_PTR),
        inputs.account().hash().as_elements()
    );

    // The nullifier commitment should be stored at the NULLIFIER_COM_PTR
    assert_eq!(
        read_root_mem_value(process, NULLIFIER_COM_PTR),
        inputs.input_notes().commitment().as_elements()
    );

    // The initial nonce should be stored at the INIT_NONCE_PTR
    assert_eq!(read_root_mem_value(process, INIT_NONCE_PTR)[0], inputs.account().nonce());

    // The transaction script root should be stored at the TX_SCRIPT_ROOT_PTR
    assert_eq!(
        read_root_mem_value(process, TX_SCRIPT_ROOT_PTR),
        **inputs.tx_script().as_ref().unwrap().hash()
    );
}

fn block_data_memory_assertions(process: &Process<MockHost>, inputs: &PreparedTransaction) {
    // The block hash should be stored at the BLK_HASH_PTR
    assert_eq!(
        read_root_mem_value(process, BLK_HASH_PTR),
        inputs.block_header().hash().as_elements()
    );

    // The previous block hash should be stored at the PREV_BLK_HASH_PTR
    assert_eq!(
        read_root_mem_value(process, PREV_BLOCK_HASH_PTR),
        inputs.block_header().prev_hash().as_elements()
    );

    // The chain root should be stored at the CHAIN_ROOT_PTR
    assert_eq!(
        read_root_mem_value(process, CHAIN_ROOT_PTR),
        inputs.block_header().chain_root().as_elements()
    );

    // The account db root should be stored at the ACCT_DB_ROOT_PRT
    assert_eq!(
        read_root_mem_value(process, ACCT_DB_ROOT_PTR),
        inputs.block_header().account_root().as_elements()
    );

    // The nullifier db root should be stored at the NULLIFIER_DB_ROOT_PTR
    assert_eq!(
        read_root_mem_value(process, NULLIFIER_DB_ROOT_PTR),
        inputs.block_header().nullifier_root().as_elements()
    );

    // The batch root should be stored at the BATCH_ROOT_PTR
    assert_eq!(
        read_root_mem_value(process, BATCH_ROOT_PTR),
        inputs.block_header().batch_root().as_elements()
    );

    // The note root should be stored at the NOTE_ROOT_PTR
    assert_eq!(
        read_root_mem_value(process, NOTE_ROOT_PTR),
        inputs.block_header().note_root().as_elements()
    );

    // The proof hash should be stored at the PROOF_HASH_PTR
    assert_eq!(
        read_root_mem_value(process, PROOF_HASH_PTR),
        inputs.block_header().proof_hash().as_elements()
    );

    // The block number should be stored at BLOCK_METADATA_PTR[BLOCK_NUMBER_IDX]
    assert_eq!(
        read_root_mem_value(process, BLOCK_METADATA_PTR)[BLOCK_NUMBER_IDX],
        inputs.block_header().block_num().into()
    );

    // The protocol version should be stored at BLOCK_METADATA_PTR[PROTOCOL_VERSION_IDX]
    assert_eq!(
        read_root_mem_value(process, BLOCK_METADATA_PTR)[PROTOCOL_VERSION_IDX],
        inputs.block_header().version()
    );

    // The timestamp should be stored at BLOCK_METADATA_PTR[TIMESTAMP_IDX]
    assert_eq!(
        read_root_mem_value(process, BLOCK_METADATA_PTR)[TIMESTAMP_IDX],
        inputs.block_header().timestamp()
    );
}

fn chain_mmr_memory_assertions(process: &Process<MockHost>, prepared_tx: &PreparedTransaction) {
    // update the chain MMR to point to the block against which this transaction is being executed
    let mut chain_mmr = prepared_tx.tx_inputs().block_chain().clone();
    chain_mmr.add_block(*prepared_tx.tx_inputs().block_header(), true);

    // The number of leaves should be stored at the CHAIN_MMR_NUM_LEAVES_PTR
    assert_eq!(
        read_root_mem_value(process, CHAIN_MMR_NUM_LEAVES_PTR)[0],
        Felt::new(chain_mmr.chain_length() as u64)
    );

    for (i, peak) in chain_mmr.peaks().peaks().iter().enumerate() {
        // The peaks should be stored at the CHAIN_MMR_PEAKS_PTR
        let i: u32 = i.try_into().expect(
            "Number of peaks is log2(number_of_leaves), this value won't be larger than 2**32",
        );
        assert_eq!(read_root_mem_value(process, CHAIN_MMR_PEAKS_PTR + i), Word::from(peak));
    }
}

fn account_data_memory_assertions(process: &Process<MockHost>, inputs: &PreparedTransaction) {
    // The account id should be stored at ACCT_ID_AND_NONCE_PTR[0]
    assert_eq!(
        read_root_mem_value(process, ACCT_ID_AND_NONCE_PTR),
        [inputs.account().id().into(), ZERO, ZERO, inputs.account().nonce()]
    );

    // The account vault root commitment should be stored at ACCT_VAULT_ROOT_PTR
    assert_eq!(
        read_root_mem_value(process, ACCT_VAULT_ROOT_PTR),
        inputs.account().vault().commitment().as_elements()
    );

    // The account storage root commitment should be stored at ACCT_STORAGE_ROOT_PTR
    assert_eq!(
        read_root_mem_value(process, ACCT_STORAGE_ROOT_PTR),
        Word::from(inputs.account().storage().root())
    );

    // The account code commitment should be stored at (ACCOUNT_DATA_OFFSET + 4)
    assert_eq!(
        read_root_mem_value(process, ACCT_CODE_ROOT_PTR),
        inputs.account().code().root().as_elements()
    );

    // The account types data should be stored in
    // (ACCT_STORAGE_SLOT_TYPE_DATA_OFFSET..ACCT_STORAGE_SLOT_TYPE_DATA_OFFSET + 64)
    for (types, types_ptr) in inputs
        .account()
        .storage()
        .layout()
        .chunks(4)
        .zip(ACCT_STORAGE_SLOT_TYPE_DATA_OFFSET..)
    {
        assert_eq!(
            read_root_mem_value(process, types_ptr),
            Word::try_from(types.iter().map(Felt::from).collect::<Vec<_>>()).unwrap()
        );
    }
}

fn consumed_notes_memory_assertions(process: &Process<MockHost>, inputs: &PreparedTransaction) {
    // The number of consumed notes should be stored at the CONSUMED_NOTES_OFFSET
    assert_eq!(
        read_root_mem_value(process, CONSUMED_NOTE_SECTION_OFFSET)[0],
        Felt::new(inputs.input_notes().num_notes() as u64)
    );

    for (note, note_idx) in inputs.input_notes().iter().zip(0_u32..) {
        let note = note.note();

        // The note nullifier should be computer and stored at (CONSUMED_NOTES_OFFSET + 1 + note_idx)
        assert_eq!(
            read_root_mem_value(process, CONSUMED_NOTE_SECTION_OFFSET + 1 + note_idx),
            note.nullifier().as_elements()
        );

        // The ID hash should be computed and stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024)
        assert_eq!(
            read_root_mem_value(process, consumed_note_data_ptr(note_idx)),
            note.id().as_elements()
        );

        // The note serial num should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 1)
        assert_eq!(
            read_root_mem_value(process, consumed_note_data_ptr(note_idx) + 1),
            note.serial_num()
        );

        // The note script hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 2)
        assert_eq!(
            read_root_mem_value(process, consumed_note_data_ptr(note_idx) + 2),
            note.script().hash().as_elements()
        );

        // The note input hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 3)
        assert_eq!(
            read_root_mem_value(process, consumed_note_data_ptr(note_idx) + 3),
            note.inputs().hash().as_elements()
        );

        // The note asset hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 4)
        assert_eq!(
            read_root_mem_value(process, consumed_note_data_ptr(note_idx) + 4),
            note.assets().commitment().as_elements()
        );

        // The note metadata should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 5)
        assert_eq!(
            read_root_mem_value(process, consumed_note_data_ptr(note_idx) + 5),
            Word::from(note.metadata())
        );

        // The number of assets should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 6)
        assert_eq!(
            read_root_mem_value(process, consumed_note_data_ptr(note_idx) + 6),
            [Felt::from(note.assets().num_assets() as u32), ZERO, ZERO, ZERO]
        );

        // The assets should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 7..)
        for (asset, asset_idx) in note.assets().iter().cloned().zip(0u32..) {
            let word: Word = asset.into();
            assert_eq!(
                read_root_mem_value(process, consumed_note_data_ptr(note_idx) + 7 + asset_idx),
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
    let tx_inputs = mock_inputs_with_account_seed(
        MockAccountType::StandardNew,
        AssetPreservationStatus::Preserved,
        Some(account_seed),
    );
    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let transaction = prepare_transaction(tx_inputs, None, code, None);
    let _process = run_tx(&transaction).unwrap();
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_valid_fungible_faucet_reserved_slot() {
    let (acct_id, account_seed) =
        generate_account_seed(AccountSeedType::FungibleFaucetValidInitialBalance);
    let tx_inputs = mock_inputs_with_account_seed(
        MockAccountType::FungibleFaucet {
            acct_id: acct_id.into(),
            nonce: ZERO,
            empty_reserved_slot: true,
        },
        AssetPreservationStatus::Preserved,
        Some(account_seed),
    );
    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let transaction = prepare_transaction(tx_inputs, None, code, None);
    let process = run_tx(&transaction);

    assert!(process.is_ok());
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_invalid_fungible_faucet_reserved_slot() {
    let (acct_id, account_seed) =
        generate_account_seed(AccountSeedType::FungibleFaucetInvalidInitialBalance);
    let tx_inputs = mock_inputs_with_account_seed(
        MockAccountType::FungibleFaucet {
            acct_id: acct_id.into(),
            nonce: ZERO,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
        Some(account_seed),
    );
    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let transaction = prepare_transaction(tx_inputs, None, code, None);
    let process = run_tx(&transaction);

    assert!(process.is_err());
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_valid_non_fungible_faucet_reserved_slot() {
    let (acct_id, account_seed) =
        generate_account_seed(AccountSeedType::NonFungibleFaucetValidReservedSlot);
    let tx_inputs = mock_inputs_with_account_seed(
        MockAccountType::NonFungibleFaucet {
            acct_id: acct_id.into(),
            nonce: ZERO,
            empty_reserved_slot: true,
        },
        AssetPreservationStatus::Preserved,
        Some(account_seed),
    );
    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let transaction = prepare_transaction(tx_inputs, None, code, None);
    let process = run_tx(&transaction);

    assert!(process.is_ok())
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_invalid_non_fungible_faucet_reserved_slot() {
    let (acct_id, account_seed) =
        generate_account_seed(AccountSeedType::NonFungibleFaucetInvalidReservedSlot);
    let tx_inputs = mock_inputs_with_account_seed(
        MockAccountType::NonFungibleFaucet {
            acct_id: acct_id.into(),
            nonce: ZERO,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
        Some(account_seed),
    );
    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let transaction = prepare_transaction(tx_inputs, None, code, None);
    let process = run_tx(&transaction);
    assert!(process.is_err());
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_invalid_seed() {
    let (_acct_id, account_seed) =
        generate_account_seed(AccountSeedType::RegularAccountUpdatableCodeOnChain);
    let tx_inputs = mock_inputs_with_account_seed(
        MockAccountType::StandardNew,
        AssetPreservationStatus::Preserved,
        Some(account_seed),
    );
    let account_seed_key = [tx_inputs.account().id().into(), ZERO, ZERO, ZERO];

    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let transaction = prepare_transaction(tx_inputs, None, code, None);
    //let (program, stack_inputs, mut advice_provider) = build_tx_inputs(&transaction);

    // lets override the seed with an invalid seed to ensure the kernel fails
    let adv_inputs = AdviceInputs::default()
        .with_map([(Digest::from(account_seed_key).as_bytes(), vec![ZERO; 4])]);

    let process = run_tx_with_inputs(&transaction, adv_inputs);
    assert!(process.is_err());
}

#[test]
fn test_get_blk_version() {
    let tx_inputs =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let code = "
    use.miden::kernels::tx::memory
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
        exec.memory::get_blk_version
    end
    ";

    let transaction = prepare_transaction(tx_inputs.clone(), None, code, None);
    let process = run_tx(&transaction).unwrap();

    assert_eq!(process.stack.get(0), tx_inputs.block_header().version());
}

#[test]
fn test_get_blk_timestamp() {
    let tx_inputs =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let code = "
    use.miden::kernels::tx::memory
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
        exec.memory::get_blk_timestamp
    end
    ";

    let transaction = prepare_transaction(tx_inputs.clone(), None, code, None);
    let process = run_tx(&transaction).unwrap();

    assert_eq!(process.stack.get(0), tx_inputs.block_header().timestamp());
}

// HELPER FUNCTIONS
// ================================================================================================

fn read_root_mem_value(process: &Process<MockHost>, addr: u32) -> Word {
    process.get_mem_value(ContextId::root(), addr).unwrap()
}
