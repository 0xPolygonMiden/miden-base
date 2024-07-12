use alloc::{collections::BTreeMap, vec::Vec};

use miden_lib::transaction::{
    memory::{
        MemoryOffset, ACCT_CODE_ROOT_PTR, ACCT_DB_ROOT_PTR, ACCT_ID_AND_NONCE_PTR, ACCT_ID_PTR,
        ACCT_STORAGE_ROOT_PTR, ACCT_STORAGE_SLOT_TYPE_DATA_OFFSET, ACCT_VAULT_ROOT_PTR,
        BLK_HASH_PTR, BLOCK_METADATA_PTR, BLOCK_NUMBER_IDX, CHAIN_MMR_NUM_LEAVES_PTR,
        CHAIN_MMR_PEAKS_PTR, CHAIN_ROOT_PTR, INIT_ACCT_HASH_PTR, INIT_NONCE_PTR,
        INPUT_NOTES_COMMITMENT_PTR, INPUT_NOTE_ARGS_OFFSET, INPUT_NOTE_ASSETS_HASH_OFFSET,
        INPUT_NOTE_ASSETS_OFFSET, INPUT_NOTE_ID_OFFSET, INPUT_NOTE_INPUTS_HASH_OFFSET,
        INPUT_NOTE_METADATA_OFFSET, INPUT_NOTE_NUM_ASSETS_OFFSET, INPUT_NOTE_SCRIPT_ROOT_OFFSET,
        INPUT_NOTE_SECTION_OFFSET, INPUT_NOTE_SERIAL_NUM_OFFSET, NOTE_ROOT_PTR,
        NULLIFIER_DB_ROOT_PTR, PREV_BLOCK_HASH_PTR, PROOF_HASH_PTR, PROTOCOL_VERSION_IDX,
        TIMESTAMP_IDX, TX_HASH_PTR, TX_SCRIPT_ROOT_PTR,
    },
    TransactionKernel,
};
use miden_objects::{
    accounts::account_id::testing::ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
    assembly::ProgramAst,
    testing::{
        constants::FUNGIBLE_FAUCET_INITIAL_BALANCE,
        storage::{generate_account_seed, AccountSeedType},
    },
    transaction::{TransactionArgs, TransactionScript},
    Digest, FieldElement,
};
use vm_processor::{AdviceInputs, ONE};

use super::{Felt, Process, Word, ZERO};
use crate::{
    testing::{
        utils::input_note_data_ptr, MockHost, TransactionContext, TransactionContextBuilder,
    },
    tests::kernel_tests::read_root_mem_value,
};

#[test]
fn test_transaction_prologue() {
    let mut tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .with_mock_notes_preserved()
    .build();

    let code = "
        use.miden::kernels::tx::prologue

        begin
            exec.prologue::prepare_transaction
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
    let (tx_script, _) = TransactionScript::new(
        mock_tx_script_code,
        vec![],
        &TransactionKernel::assembler().with_debug_mode(true),
    )
    .unwrap();

    let note_args = [
        [Felt::new(91), Felt::new(91), Felt::new(91), Felt::new(91)],
        [Felt::new(92), Felt::new(92), Felt::new(92), Felt::new(92)],
    ];

    let note_args_map = BTreeMap::from([
        (tx_context.input_notes().get_note(0).note().id(), note_args[0]),
        (tx_context.input_notes().get_note(1).note().id(), note_args[1]),
    ]);

    let tx_args = TransactionArgs::new(
        Some(tx_script),
        Some(note_args_map),
        tx_context.tx_args().advice_inputs().clone().map,
    );

    tx_context.set_tx_args(tx_args);
    let process = tx_context.execute_code(code).unwrap();

    global_input_memory_assertions(&process, &tx_context);
    block_data_memory_assertions(&process, &tx_context);
    chain_mmr_memory_assertions(&process, &tx_context);
    account_data_memory_assertions(&process, &tx_context);
    input_notes_memory_assertions(&process, &tx_context, &note_args);
}

fn global_input_memory_assertions(process: &Process<MockHost>, inputs: &TransactionContext) {
    assert_eq!(
        read_root_mem_value(process, BLK_HASH_PTR),
        inputs.tx_inputs().block_header().hash().as_elements(),
        "The block hash should be stored at the BLK_HASH_PTR"
    );

    assert_eq!(
        read_root_mem_value(process, ACCT_ID_PTR)[0],
        inputs.account().id().into(),
        "The account ID should be stored at the ACCT_ID_PTR"
    );

    assert_eq!(
        read_root_mem_value(process, INIT_ACCT_HASH_PTR),
        inputs.account().hash().as_elements(),
        "The account commitment should be stored at the ACCT_HASH_PTR"
    );

    assert_eq!(
        read_root_mem_value(process, INPUT_NOTES_COMMITMENT_PTR),
        inputs.input_notes().commitment().as_elements(),
        "The nullifier commitment should be stored at the INPUT_NOTES_COMMITMENT_PTR"
    );

    assert_eq!(
        read_root_mem_value(process, INIT_NONCE_PTR)[0],
        inputs.account().nonce(),
        "The initial nonce should be stored at the INIT_NONCE_PTR"
    );

    assert_eq!(
        read_root_mem_value(process, TX_SCRIPT_ROOT_PTR),
        **inputs.tx_args().tx_script().as_ref().unwrap().hash(),
        "The transaction script root should be stored at the TX_SCRIPT_ROOT_PTR"
    );
}

fn block_data_memory_assertions(process: &Process<MockHost>, inputs: &TransactionContext) {
    assert_eq!(
        read_root_mem_value(process, BLK_HASH_PTR),
        inputs.tx_inputs().block_header().hash().as_elements(),
        "The block hash should be stored at the BLK_HASH_PTR"
    );

    assert_eq!(
        read_root_mem_value(process, PREV_BLOCK_HASH_PTR),
        inputs.tx_inputs().block_header().prev_hash().as_elements(),
        "The previous block hash should be stored at the PREV_BLK_HASH_PTR"
    );

    assert_eq!(
        read_root_mem_value(process, CHAIN_ROOT_PTR),
        inputs.tx_inputs().block_header().chain_root().as_elements(),
        "The chain root should be stored at the CHAIN_ROOT_PTR"
    );

    assert_eq!(
        read_root_mem_value(process, ACCT_DB_ROOT_PTR),
        inputs.tx_inputs().block_header().account_root().as_elements(),
        "The account db root should be stored at the ACCT_DB_ROOT_PRT"
    );

    assert_eq!(
        read_root_mem_value(process, NULLIFIER_DB_ROOT_PTR),
        inputs.tx_inputs().block_header().nullifier_root().as_elements(),
        "The nullifier db root should be stored at the NULLIFIER_DB_ROOT_PTR"
    );

    assert_eq!(
        read_root_mem_value(process, TX_HASH_PTR),
        inputs.tx_inputs().block_header().tx_hash().as_elements(),
        "The TX hash should be stored at the TX_HASH_PTR"
    );

    assert_eq!(
        read_root_mem_value(process, NOTE_ROOT_PTR),
        inputs.tx_inputs().block_header().note_root().as_elements(),
        "The note root should be stored at the NOTE_ROOT_PTR"
    );

    assert_eq!(
        read_root_mem_value(process, PROOF_HASH_PTR),
        inputs.tx_inputs().block_header().proof_hash().as_elements(),
        "The proof hash should be stored at the PROOF_HASH_PTR"
    );

    assert_eq!(
        read_root_mem_value(process, BLOCK_METADATA_PTR)[BLOCK_NUMBER_IDX],
        inputs.tx_inputs().block_header().block_num().into(),
        "The block number should be stored at BLOCK_METADATA_PTR[BLOCK_NUMBER_IDX]"
    );

    assert_eq!(
        read_root_mem_value(process, BLOCK_METADATA_PTR)[PROTOCOL_VERSION_IDX],
        inputs.tx_inputs().block_header().version().into(),
        "The protocol version should be stored at BLOCK_METADATA_PTR[PROTOCOL_VERSION_IDX]"
    );

    assert_eq!(
        read_root_mem_value(process, BLOCK_METADATA_PTR)[TIMESTAMP_IDX],
        inputs.tx_inputs().block_header().timestamp().into(),
        "The timestamp should be stored at BLOCK_METADATA_PTR[TIMESTAMP_IDX]"
    );
}

fn chain_mmr_memory_assertions(process: &Process<MockHost>, prepared_tx: &TransactionContext) {
    // update the chain MMR to point to the block against which this transaction is being executed
    let mut chain_mmr = prepared_tx.tx_inputs().block_chain().clone();
    chain_mmr.add_block(*prepared_tx.tx_inputs().block_header(), true);

    assert_eq!(
        read_root_mem_value(process, CHAIN_MMR_NUM_LEAVES_PTR)[0],
        Felt::new(chain_mmr.chain_length() as u64),
        "The number of leaves should be stored at the CHAIN_MMR_NUM_LEAVES_PTR"
    );

    for (i, peak) in chain_mmr.peaks().peaks().iter().enumerate() {
        // The peaks should be stored at the CHAIN_MMR_PEAKS_PTR
        let i: u32 = i.try_into().expect(
            "Number of peaks is log2(number_of_leaves), this value won't be larger than 2**32",
        );
        assert_eq!(read_root_mem_value(process, CHAIN_MMR_PEAKS_PTR + i), Word::from(peak));
    }
}

fn account_data_memory_assertions(process: &Process<MockHost>, inputs: &TransactionContext) {
    assert_eq!(
        read_root_mem_value(process, ACCT_ID_AND_NONCE_PTR),
        [inputs.account().id().into(), ZERO, ZERO, inputs.account().nonce()],
        "The account id should be stored at ACCT_ID_AND_NONCE_PTR[0]"
    );

    assert_eq!(
        read_root_mem_value(process, ACCT_VAULT_ROOT_PTR),
        inputs.account().vault().commitment().as_elements(),
        "The account vault root commitment should be stored at ACCT_VAULT_ROOT_PTR"
    );

    assert_eq!(
        read_root_mem_value(process, ACCT_STORAGE_ROOT_PTR),
        Word::from(inputs.account().storage().root()),
        "The account storage root commitment should be stored at ACCT_STORAGE_ROOT_PTR"
    );

    assert_eq!(
        read_root_mem_value(process, ACCT_CODE_ROOT_PTR),
        inputs.account().code().root().as_elements(),
        "account code commitment should be stored at (ACCOUNT_DATA_OFFSET + 4)"
    );

    for (types, types_ptr) in inputs
        .account()
        .storage()
        .layout()
        .chunks(4)
        .zip(ACCT_STORAGE_SLOT_TYPE_DATA_OFFSET..)
    {
        assert_eq!(
            read_root_mem_value(process, types_ptr),
            Word::try_from(types.iter().map(Felt::from).collect::<Vec<_>>()).unwrap(),
            "The account types data should be stored in (ACCT_STORAGE_SLOT_TYPE_DATA_OFFSET..ACCT_STORAGE_SLOT_TYPE_DATA_OFFSET + 64)"
        );
    }
}

fn input_notes_memory_assertions(
    process: &Process<MockHost>,
    inputs: &TransactionContext,
    note_args: &[[Felt; 4]],
) {
    assert_eq!(
        read_root_mem_value(process, INPUT_NOTE_SECTION_OFFSET),
        [Felt::new(inputs.input_notes().num_notes() as u64), ZERO, ZERO, ZERO],
        "number of input notes should be stored at the INPUT_NOTES_OFFSET"
    );

    for (input_note, note_idx) in inputs.input_notes().iter().zip(0_u32..) {
        let note = input_note.note();

        assert_eq!(
            read_root_mem_value(process, INPUT_NOTE_SECTION_OFFSET + 1 + note_idx),
            note.nullifier().as_elements(),
            "note nullifier should be computer and stored at the correct offset"
        );

        assert_eq!(
            read_note_element(process, note_idx, INPUT_NOTE_ID_OFFSET),
            note.id().as_elements(),
            "ID hash should be computed and stored at the correct offset"
        );

        assert_eq!(
            read_note_element(process, note_idx, INPUT_NOTE_SERIAL_NUM_OFFSET),
            note.serial_num(),
            "note serial num should be stored at the correct offset"
        );

        assert_eq!(
            read_note_element(process, note_idx, INPUT_NOTE_SCRIPT_ROOT_OFFSET),
            note.script().hash().as_elements(),
            "note script hash should be stored at the correct offset"
        );

        assert_eq!(
            read_note_element(process, note_idx, INPUT_NOTE_INPUTS_HASH_OFFSET),
            note.inputs().commitment().as_elements(),
            "note input hash should be stored at the correct offset"
        );

        assert_eq!(
            read_note_element(process, note_idx, INPUT_NOTE_ASSETS_HASH_OFFSET),
            note.assets().commitment().as_elements(),
            "note asset hash should be stored at the correct offset"
        );

        assert_eq!(
            read_note_element(process, note_idx, INPUT_NOTE_METADATA_OFFSET),
            Word::from(note.metadata()),
            "note metadata should be stored at the correct offset"
        );

        assert_eq!(
            read_note_element(process, note_idx, INPUT_NOTE_ARGS_OFFSET),
            Word::from(note_args[note_idx as usize]),
            "note args should be stored at the correct offset"
        );

        assert_eq!(
            read_note_element(process, note_idx, INPUT_NOTE_NUM_ASSETS_OFFSET),
            [Felt::from(note.assets().num_assets() as u32), ZERO, ZERO, ZERO],
            "number of assets should be stored at the correct offset"
        );

        for (asset, asset_idx) in note.assets().iter().cloned().zip(0_u32..) {
            let word: Word = asset.into();
            assert_eq!(
                read_note_element(process, note_idx, INPUT_NOTE_ASSETS_OFFSET + asset_idx),
                word,
                "assets should be stored at (INPUT_NOTES_OFFSET + (note_index + 1) * 1024 + 7..)"
            );
        }
    }
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account() {
    let (acct_id, account_seed) = generate_account_seed(
        AccountSeedType::RegularAccountUpdatableCodeOnChain,
        &TransactionKernel::assembler().with_debug_mode(true),
    );
    let tx_context = TransactionContextBuilder::with_standard_account(acct_id.into(), ZERO)
        .account_seed(account_seed)
        .build();

    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    tx_context.execute_code(code).unwrap();
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_valid_fungible_faucet_reserved_slot() {
    let (acct_id, account_seed) = generate_account_seed(
        AccountSeedType::FungibleFaucetValidInitialBalance,
        &TransactionKernel::assembler().with_debug_mode(true),
    );

    let tx_context =
        TransactionContextBuilder::with_fungible_faucet(acct_id.into(), Felt::ZERO, ZERO)
            .account_seed(account_seed)
            .build();

    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let process = tx_context.execute_code(code);
    assert!(process.is_ok());
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_invalid_fungible_faucet_reserved_slot() {
    let (acct_id, account_seed) = generate_account_seed(
        AccountSeedType::FungibleFaucetInvalidInitialBalance,
        &TransactionKernel::assembler().with_debug_mode(true),
    );

    let tx_context = TransactionContextBuilder::with_fungible_faucet(
        acct_id.into(),
        Felt::ZERO,
        Felt::new(FUNGIBLE_FAUCET_INITIAL_BALANCE),
    )
    .account_seed(account_seed)
    .build();

    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let process = tx_context.execute_code(code);
    assert!(process.is_err());
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_valid_non_fungible_faucet_reserved_slot() {
    let (acct_id, account_seed) = generate_account_seed(
        AccountSeedType::NonFungibleFaucetValidReservedSlot,
        &TransactionKernel::assembler().with_debug_mode(true),
    );

    let tx_context =
        TransactionContextBuilder::with_non_fungible_faucet(acct_id.into(), Felt::ZERO, true)
            .account_seed(account_seed)
            .build();

    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let process = tx_context.execute_code(code);

    assert!(process.is_ok())
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_invalid_non_fungible_faucet_reserved_slot() {
    let (acct_id, account_seed) = generate_account_seed(
        AccountSeedType::NonFungibleFaucetInvalidReservedSlot,
        &TransactionKernel::assembler().with_debug_mode(true),
    );

    let tx_context =
        TransactionContextBuilder::with_non_fungible_faucet(acct_id.into(), Felt::ZERO, false)
            .account_seed(account_seed)
            .build();

    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    let process = tx_context.execute_code(code);

    assert!(process.is_err());
}

#[cfg_attr(not(feature = "testing"), ignore)]
#[test]
pub fn test_prologue_create_account_invalid_seed() {
    let (acct_id, account_seed) = generate_account_seed(
        AccountSeedType::RegularAccountUpdatableCodeOnChain,
        &TransactionKernel::assembler().with_debug_mode(true),
    );

    let code = "
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
    end
    ";

    // override the seed with an invalid seed to ensure the kernel fails
    let account_seed_key = [acct_id.into(), ZERO, ZERO, ZERO];
    let adv_inputs =
        AdviceInputs::default().with_map([(Digest::from(account_seed_key), vec![ZERO; 4])]);

    let tx_context = TransactionContextBuilder::with_standard_account(acct_id.into(), ZERO)
        .account_seed(account_seed)
        .advice_inputs(adv_inputs)
        .build();

    let process = tx_context.execute_code(code);
    assert!(process.is_err());
}

#[test]
fn test_get_blk_version() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .build();
    let code = "
    use.miden::kernels::tx::memory
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
        exec.memory::get_blk_version
    end
    ";

    let process = tx_context.execute_code(code).unwrap();

    assert_eq!(process.stack.get(0), tx_context.tx_inputs().block_header().version().into());
}

#[test]
fn test_get_blk_timestamp() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .build();
    let code = "
    use.miden::kernels::tx::memory
    use.miden::kernels::tx::prologue

    begin
        exec.prologue::prepare_transaction
        exec.memory::get_blk_timestamp
    end
    ";

    let process = tx_context.execute_code(code).unwrap();

    assert_eq!(process.stack.get(0), tx_context.tx_inputs().block_header().timestamp().into());
}

// HELPER FUNCTIONS
// ================================================================================================

fn read_note_element(process: &Process<MockHost>, note_idx: u32, offset: MemoryOffset) -> Word {
    read_root_mem_value(process, input_note_data_ptr(note_idx) + offset)
}
