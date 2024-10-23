use alloc::vec::Vec;
use std::string::String;

use miden_lib::transaction::{
    memory::{
        ACCOUNT_DATA_LENGTH, ACCT_CODE_COMMITMENT_OFFSET, ACCT_ID_AND_NONCE_OFFSET,
        ACCT_PROCEDURES_SECTION_OFFSET, ACCT_STORAGE_COMMITMENT_OFFSET,
        ACCT_STORAGE_SLOTS_SECTION_OFFSET, ACCT_VAULT_ROOT_OFFSET, NATIVE_ACCOUNT_DATA_PTR,
        NOTE_MEM_SIZE, NUM_ACCT_PROCEDURES_OFFSET, NUM_ACCT_STORAGE_SLOTS_OFFSET,
        NUM_OUTPUT_NOTES_PTR, OUTPUT_NOTE_ASSETS_OFFSET, OUTPUT_NOTE_METADATA_OFFSET,
        OUTPUT_NOTE_RECIPIENT_OFFSET, OUTPUT_NOTE_SECTION_OFFSET,
    },
    TransactionKernel,
};
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        },
        Account, AccountCode, AccountProcedureInfo, AccountStorage, StorageSlot,
    },
    assets::NonFungibleAsset,
    notes::{
        Note, NoteAssets, NoteExecutionHint, NoteInputs, NoteMetadata, NoteRecipient, NoteTag,
        NoteType,
    },
    testing::{
        account::AccountBuilder, constants::NON_FUNGIBLE_ASSET_DATA_2, prepare_word,
        storage::STORAGE_LEAVES_2,
    },
    transaction::{OutputNote, OutputNotes},
    Digest, FieldElement,
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use vm_processor::AdviceInputs;

use super::{Felt, Process, ProcessState, Word, ONE, ZERO};
use crate::{
    assert_execution_error,
    errors::tx_kernel_errors::{
        ERR_NON_FUNGIBLE_ASSET_ALREADY_EXISTS, ERR_TX_NUMBER_OF_OUTPUT_NOTES_EXCEEDS_LIMIT,
    },
    testing::{
        mock_chain::{MockChain, MockChainBuilder},
        MockHost, TransactionContextBuilder,
    },
    tests::kernel_tests::{read_root_mem_value, try_read_root_mem_value},
};

#[test]
fn test_create_note() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();
    let account_id = tx_context.account().id();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let aux = Felt::new(27);
    let tag = Felt::new(4);

    let code = format!(
        "
        use.kernel::prologue
        use.miden::tx

        begin
            exec.prologue::prepare_transaction

            push.{recipient}
            push.{note_execution_hint}
            push.{PUBLIC_NOTE}
            push.{aux}
            push.{tag}

            call.tx::create_note

            # truncate the stack
            swapdw dropw dropw
        end
        ",
        recipient = prepare_word(&recipient),
        PUBLIC_NOTE = NoteType::Public as u8,
        note_execution_hint = Felt::from(NoteExecutionHint::after_block(23)),
        tag = tag,
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(
        read_root_mem_value(&process, NUM_OUTPUT_NOTES_PTR),
        [ONE, ZERO, ZERO, ZERO],
        "number of output notes must increment by 1",
    );

    assert_eq!(
        read_root_mem_value(&process, OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_RECIPIENT_OFFSET),
        recipient,
        "recipient must be stored at the correct memory location",
    );

    let expected_note_metadata: Word = NoteMetadata::new(
        account_id,
        NoteType::Public,
        tag.try_into().unwrap(),
        NoteExecutionHint::after_block(23),
        Felt::new(27),
    )
    .unwrap()
    .into();

    assert_eq!(
        read_root_mem_value(&process, OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_METADATA_OFFSET),
        [
            expected_note_metadata[0],
            expected_note_metadata[1],
            expected_note_metadata[2],
            expected_note_metadata[3]
        ],
        "metadata must be stored at the correct memory location",
    );

    assert_eq!(
        process.stack.get(0),
        ZERO,
        "top item on the stack is the index of the output note"
    );
}

#[test]
fn test_create_note_with_invalid_tag() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let invalid_tag = Felt::new((NoteType::Public as u64) << 62);
    let valid_tag: Felt = NoteTag::for_local_use_case(0, 0).unwrap().into();

    // Test invalid tag
    assert!(tx_context.execute_code(&note_creation_script(invalid_tag)).is_err());
    // Test valid tag
    assert!(tx_context.execute_code(&note_creation_script(valid_tag)).is_ok());

    fn note_creation_script(tag: Felt) -> String {
        format!(
            "
            use.kernel::prologue
            use.miden::tx
    
            begin
                exec.prologue::prepare_transaction
    
                push.{recipient}
                push.{execution_hint_always}
                push.{PUBLIC_NOTE}
                push.{aux}
                push.{tag}
    
                call.tx::create_note

                # clean the stack
                dropw dropw
            end
            ",
            recipient = prepare_word(&[ZERO, ONE, Felt::new(2), Felt::new(3)]),
            execution_hint_always = Felt::from(NoteExecutionHint::always()),
            PUBLIC_NOTE = NoteType::Public as u8,
            aux = Felt::ZERO,
        )
    }
}

#[test]
fn test_create_note_too_many_notes() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let code = format!(
        "
        use.kernel::constants
        use.kernel::memory
        use.miden::tx
        use.kernel::prologue

        begin
            exec.constants::get_max_num_output_notes
            exec.memory::set_num_output_notes
            exec.prologue::prepare_transaction
            
            push.{recipient}
            push.{execution_hint_always}
            push.{PUBLIC_NOTE}
            push.{aux}
            push.{tag}

            call.tx::create_note
        end
        ",
        tag = Felt::new(4),
        recipient = prepare_word(&[ZERO, ONE, Felt::new(2), Felt::new(3)]),
        execution_hint_always = Felt::from(NoteExecutionHint::always()),
        PUBLIC_NOTE = NoteType::Public as u8,
        aux = Felt::ZERO,
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_TX_NUMBER_OF_OUTPUT_NOTES_EXCEEDS_LIMIT);
}

#[test]
fn test_get_output_notes_hash() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved()
        .build();

    // extract input note data
    let input_note_1 = tx_context.tx_inputs().input_notes().get_note(0).note();
    let input_asset_1 = **input_note_1.assets().iter().take(1).collect::<Vec<_>>().first().unwrap();
    let input_note_2 = tx_context.tx_inputs().input_notes().get_note(1).note();
    let input_asset_2 = **input_note_2.assets().iter().take(1).collect::<Vec<_>>().first().unwrap();

    // create output note 1
    let output_serial_no_1 = [Felt::new(8); 4];
    let output_tag_1 = 8888.into();
    let assets = NoteAssets::new(vec![input_asset_1]).unwrap();
    let metadata = NoteMetadata::new(
        tx_context.tx_inputs().account().id(),
        NoteType::Public,
        output_tag_1,
        NoteExecutionHint::Always,
        ZERO,
    )
    .unwrap();
    let inputs = NoteInputs::new(vec![]).unwrap();
    let recipient = NoteRecipient::new(output_serial_no_1, input_note_1.script().clone(), inputs);
    let output_note_1 = Note::new(assets, metadata, recipient);

    // create output note 2
    let output_serial_no_2 = [Felt::new(11); 4];
    let output_tag_2 = 1111.into();
    let assets = NoteAssets::new(vec![input_asset_2]).unwrap();
    let metadata = NoteMetadata::new(
        tx_context.tx_inputs().account().id(),
        NoteType::Public,
        output_tag_2,
        NoteExecutionHint::after_block(123),
        ZERO,
    )
    .unwrap();
    let inputs = NoteInputs::new(vec![]).unwrap();
    let recipient = NoteRecipient::new(output_serial_no_2, input_note_2.script().clone(), inputs);
    let output_note_2 = Note::new(assets, metadata, recipient);

    // compute expected output notes hash
    let expected_output_notes_hash = OutputNotes::new(vec![
        OutputNote::Full(output_note_1.clone()),
        OutputNote::Full(output_note_2.clone()),
    ])
    .unwrap()
    .commitment();

    let code = format!(
        "
        use.std::sys

        use.kernel::prologue
        use.miden::tx

        begin
            # => [BH, acct_id, IAH, NC]
            exec.prologue::prepare_transaction
            # => []

            # create output note 1
            push.{recipient_1}
            push.{NOTE_EXECUTION_HINT_1}
            push.{PUBLIC_NOTE}
            push.{aux_1}
            push.{tag_1}
            call.tx::create_note
            # => [note_idx]

            push.{asset_1}
            call.tx::add_asset_to_note
            # => [ASSET, note_idx]
            
            dropw drop
            # => []

            # create output note 2
            push.{recipient_2}
            push.{NOTE_EXECUTION_HINT_2}
            push.{PUBLIC_NOTE}
            push.{aux_2}
            push.{tag_2}
            call.tx::create_note
            # => [note_idx]

            push.{asset_2} 
            call.tx::add_asset_to_note
            # => [ASSET, note_idx]

            dropw drop
            # => []

            # compute the output notes hash
            exec.tx::get_output_notes_hash
            # => [COM]

            # truncate the stack
            exec.sys::truncate_stack
            # => [COM]
        end
        ",
        PUBLIC_NOTE = NoteType::Public as u8,
        NOTE_EXECUTION_HINT_1 = Felt::from(output_note_1.metadata().execution_hint()),
        recipient_1 = prepare_word(&output_note_1.recipient().digest()),
        tag_1 = output_note_1.metadata().tag(),
        aux_1 = output_note_1.metadata().aux(),
        asset_1 = prepare_word(&Word::from(
            **output_note_1.assets().iter().take(1).collect::<Vec<_>>().first().unwrap()
        )),
        recipient_2 = prepare_word(&output_note_2.recipient().digest()),
        NOTE_EXECUTION_HINT_2 = Felt::from(output_note_2.metadata().execution_hint()),
        tag_2 = output_note_2.metadata().tag(),
        aux_2 = output_note_2.metadata().aux(),
        asset_2 = prepare_word(&Word::from(
            **output_note_2.assets().iter().take(1).collect::<Vec<_>>().first().unwrap()
        )),
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(
        read_root_mem_value(&process, NUM_OUTPUT_NOTES_PTR),
        [Felt::new(2), ZERO, ZERO, ZERO],
        "The test creates two notes",
    );
    assert_eq!(
        NoteMetadata::try_from(read_root_mem_value(
            &process,
            OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_METADATA_OFFSET
        ))
        .unwrap(),
        *output_note_1.metadata(),
        "Validate the output note 1 metadata",
    );
    assert_eq!(
        NoteMetadata::try_from(read_root_mem_value(
            &process,
            OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_METADATA_OFFSET + NOTE_MEM_SIZE
        ))
        .unwrap(),
        *output_note_2.metadata(),
        "Validate the output note 1 metadata",
    );

    assert_eq!(process.get_stack_word(0), *expected_output_notes_hash);
}

#[test]
fn test_create_note_and_add_asset() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let aux = Felt::new(27);
    let tag = Felt::new(4);
    let asset = [Felt::new(10), ZERO, ZERO, Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN)];

    let code = format!(
        "
        use.kernel::prologue
        use.miden::tx
        use.kernel::memory

        begin
            exec.prologue::prepare_transaction

            push.{recipient}
            push.{NOTE_EXECUTION_HINT}
            push.{PUBLIC_NOTE}
            push.{aux}
            push.{tag}

            call.tx::create_note
            # => [note_idx]

            push.{asset}
            call.tx::add_asset_to_note
            # => [ASSET, note_idx]

            dropw
            # => [note_idx]

            # truncate the stack
            swapdw dropw dropw
        end
        ",
        recipient = prepare_word(&recipient),
        PUBLIC_NOTE = NoteType::Public as u8,
        NOTE_EXECUTION_HINT = Felt::from(NoteExecutionHint::always()),
        tag = tag,
        asset = prepare_word(&asset),
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(
        read_root_mem_value(&process, OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_ASSETS_OFFSET),
        asset,
        "asset must be stored at the correct memory location",
    );

    assert_eq!(
        process.stack.get(0),
        ZERO,
        "top item on the stack is the index to the output note"
    );
}

#[test]
fn test_create_note_and_add_multiple_assets() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let aux = Felt::new(27);
    let tag = Felt::new(4);
    let asset = [Felt::new(10), ZERO, ZERO, Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN)];
    let asset_2 = [Felt::new(20), ZERO, ZERO, Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2)];
    let asset_3 = [Felt::new(30), ZERO, ZERO, Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2)];
    let asset_2_and_3 =
        [Felt::new(50), ZERO, ZERO, Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2)];
    let non_fungible_asset =
        NonFungibleAsset::mock(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &NON_FUNGIBLE_ASSET_DATA_2);
    let non_fungible_asset_encoded = Word::from(non_fungible_asset);

    let code = format!(
        "
        use.kernel::prologue
        use.miden::tx

        begin
            exec.prologue::prepare_transaction

            push.{recipient}
            push.{PUBLIC_NOTE}
            push.{aux}
            push.{tag}

            call.tx::create_note
            # => [note_idx]

            push.{asset}
            call.tx::add_asset_to_note dropw
            # => [note_idx]

            push.{asset_2}
            call.tx::add_asset_to_note dropw
            # => [note_idx]

            push.{asset_3}
            call.tx::add_asset_to_note dropw
            # => [note_idx]

            push.{nft}
            call.tx::add_asset_to_note dropw
            # => [note_idx]

            # truncate the stack
            swapdw dropw drop drop drop
        end
        ",
        recipient = prepare_word(&recipient),
        PUBLIC_NOTE = NoteType::Public as u8,
        tag = tag,
        asset = prepare_word(&asset),
        asset_2 = prepare_word(&asset_2),
        asset_3 = prepare_word(&asset_3),
        nft = prepare_word(&non_fungible_asset_encoded),
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(
        read_root_mem_value(&process, OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_ASSETS_OFFSET),
        asset,
        "asset must be stored at the correct memory location",
    );

    assert_eq!(
        read_root_mem_value(&process, OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_ASSETS_OFFSET + 1),
        asset_2_and_3,
        "asset_2 and asset_3 must be stored at the same correct memory location",
    );

    assert_eq!(
        read_root_mem_value(&process, OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_ASSETS_OFFSET + 2),
        Word::from(non_fungible_asset_encoded),
        "non_fungible_asset must be stored at the correct memory location",
    );

    assert_eq!(
        process.stack.get(0),
        ZERO,
        "top item on the stack is the index to the output note"
    );
}

#[test]
fn test_create_note_and_add_same_nft_twice() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);
    let non_fungible_asset =
        NonFungibleAsset::mock(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &[1, 2, 3]);
    let encoded = Word::from(non_fungible_asset);

    let code = format!(
        "
        use.kernel::prologue
        use.test::account
        use.miden::contracts::wallets::basic->wallet

        begin
            exec.prologue::prepare_transaction

            push.{recipient}
            push.{execution_hint_always}
            push.{PUBLIC_NOTE}
            push.{aux}
            push.{tag}

            call.wallet::create_note
            # => [note_idx]

            push.{nft} 
            call.account::add_asset_to_note
            dropw dropw dropw

            push.{nft} 
            call.account::add_asset_to_note
            # => [note_idx]
        end
        ",
        recipient = prepare_word(&recipient),
        PUBLIC_NOTE = NoteType::Public as u8,
        execution_hint_always = Felt::from(NoteExecutionHint::always()),
        aux = Felt::new(0),
        tag = tag,
        nft = prepare_word(&encoded),
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_NON_FUNGIBLE_ASSET_ALREADY_EXISTS);
}

#[test]
fn test_build_recipient_hash() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved()
        .build();

    let input_note_1 = tx_context.tx_inputs().input_notes().get_note(0).note();

    // create output note
    let output_serial_no = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let aux = Felt::new(27);
    let tag = 8888;
    let single_input = 2;
    let inputs = NoteInputs::new(vec![Felt::new(single_input)]).unwrap();
    let input_hash = inputs.commitment();

    let recipient = NoteRecipient::new(output_serial_no, input_note_1.script().clone(), inputs);
    let code = format!(
        "
        use.kernel::prologue
        use.miden::tx
        begin
            exec.prologue::prepare_transaction
            # input
            push.{input_hash}
            # SCRIPT_HASH
            push.{script_hash}
            # SERIAL_NUM
            push.{output_serial_no}
            call.tx::build_recipient_hash

            push.{execution_hint}
            push.{PUBLIC_NOTE}
            push.{aux}
            push.{tag}
            call.tx::create_note

            # clean the stack
            dropw dropw dropw dropw
        end
        ",
        input_hash = input_hash,
        script_hash = input_note_1.script().clone().hash(),
        output_serial_no = prepare_word(&output_serial_no),
        PUBLIC_NOTE = NoteType::Public as u8,
        tag = tag,
        execution_hint = Felt::from(NoteExecutionHint::after_block(2)),
        aux = aux,
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(
        read_root_mem_value(&process, NUM_OUTPUT_NOTES_PTR),
        [ONE, ZERO, ZERO, ZERO],
        "number of output notes must increment by 1",
    );

    let recipient_digest: Vec<Felt> = recipient.clone().digest().to_vec();

    assert_eq!(
        read_root_mem_value(&process, OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_RECIPIENT_OFFSET),
        recipient_digest.as_slice(),
        "recipient hash not correct",
    );
}

#[test]
fn test_load_foreign_account_basic() {
    // GET ITEM
    // --------------------------------------------------------------------------------------------
    let storage_slot = AccountStorage::mock_item_0().slot;
    let (foreign_account, _) = AccountBuilder::new(ChaCha20Rng::from_entropy())
        .add_storage_slot(storage_slot.clone())
        .code(AccountCode::mock_account_code(TransactionKernel::testing_assembler(), false))
        .nonce(ONE)
        .build()
        .unwrap();

    let account_id = foreign_account.id();
    let mock_chain = MockChainBuilder::default().accounts(vec![foreign_account.clone()]).build();
    let advice_inputs = get_mock_advice_inputs(&foreign_account, &mock_chain);

    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .mock_chain(mock_chain)
        .advice_inputs(advice_inputs.clone())
        .build();

    let code = format!(
        "
        use.std::sys
        
        use.kernel::prologue
        use.miden::tx
        use.miden::account
        use.miden::kernel_proc_offsets

        begin
            exec.prologue::prepare_transaction

            # pad the stack for the `execute_foreign_procedure`execution
            padw padw push.0.0.0
            # => [pad(11)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign` account procedure
            procref.account::get_item_foreign

            # push the foreign account id
            push.{account_id}
            # => [foreign_account_id, FOREIGN_PROC_ROOT, storage_item_index, pad(11)]

            exec.tx::execute_foreign_procedure
            # => [STORAGE_VALUE_1]

            # truncate the stack
            exec.sys::truncate_stack
        end
        "
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(
        process.stack.get_word(0),
        storage_slot.value(),
        "Value at the top of the stack (value in the storage at index 0) should be equal [1, 2, 3, 4]",
    );

    foreign_account_data_memory_assertions(&foreign_account, &process);

    // GET MAP ITEM
    // --------------------------------------------------------------------------------------------
    let storage_slot = AccountStorage::mock_item_2().slot;
    let (foreign_account, _) = AccountBuilder::new(ChaCha20Rng::from_entropy())
        .add_storage_slot(storage_slot.clone())
        .code(AccountCode::mock_account_code(TransactionKernel::testing_assembler(), false))
        .nonce(ONE)
        .build()
        .unwrap();

    let account_id = foreign_account.id();
    let mock_chain = MockChainBuilder::default().accounts(vec![foreign_account.clone()]).build();
    let advice_inputs = get_mock_advice_inputs(&foreign_account, &mock_chain);

    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .mock_chain(mock_chain)
        .advice_inputs(advice_inputs)
        .build();

    let code = format!(
        "
        use.std::sys

        use.kernel::prologue
        use.miden::tx
        use.miden::account
        use.miden::kernel_proc_offsets

        begin
            exec.prologue::prepare_transaction

            # pad the stack for the `execute_foreign_procedure`execution
            padw push.0.0.0
            # => [pad(7)]

            # push the key of desired storage item
            push.{map_key}

            # push the index of desired storage item
            push.0

            # get the hash of the `get_map_item_foreign` account procedure
            procref.account::get_map_item_foreign

            # push the foreign account id
            push.{account_id}
            # => [foreign_account_id, FOREIGN_PROC_ROOT, storage_item_index, MAP_ITEM_KEY, pad(7)]

            exec.tx::execute_foreign_procedure
            # => [MAP_VALUE]

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        map_key = STORAGE_LEAVES_2[0].0,
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(
        process.stack.get_word(0),
        STORAGE_LEAVES_2[0].1,
        "Value at the top of the stack should be equal [1, 2, 3, 4]",
    );

    foreign_account_data_memory_assertions(&foreign_account, &process);
}

/// This test checks that invoking two foreign procedures from the same account results in reuse of
/// the loaded account.
#[test]
fn test_load_foreign_account_twice() {
    let storage_slot = AccountStorage::mock_item_0().slot;
    let (foreign_account, _) = AccountBuilder::new(ChaCha20Rng::from_entropy())
        .add_storage_slot(storage_slot)
        .code(AccountCode::mock_account_code(TransactionKernel::testing_assembler(), false))
        .nonce(ONE)
        .build()
        .unwrap();

    let account_id = foreign_account.id();
    let mock_chain = MockChainBuilder::default().accounts(vec![foreign_account.clone()]).build();
    let advice_inputs = get_mock_advice_inputs(&foreign_account, &mock_chain);

    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .mock_chain(mock_chain)
        .advice_inputs(advice_inputs.clone())
        .build();

    let code = format!(
        "
        use.std::sys

        use.kernel::prologue
        use.miden::tx
        use.miden::account
        use.miden::kernel_proc_offsets

        begin
            exec.prologue::prepare_transaction

            ### Get the storage item at index 0 #####################
            # pad the stack for the `execute_foreign_procedure`execution
            padw padw push.0.0.0
            # => [pad(11)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign` account procedure
            procref.account::get_item_foreign

            # push the foreign account id
            push.{account_id}
            # => [foreign_account_id, FOREIGN_PROC_ROOT, storage_item_index, pad(11)]

            exec.tx::execute_foreign_procedure dropw
            # => []

            ### Get the storage item at index 0 again ###############
            # pad the stack for the `execute_foreign_procedure`execution
            padw push.0.0.0
            # => [pad(7)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign` account procedure
            procref.account::get_item_foreign

            # push the foreign account id
            push.{account_id}
            # => [foreign_account_id, FOREIGN_PROC_ROOT, storage_item_index, MAP_ITEM_KEY, pad(7)]

            exec.tx::execute_foreign_procedure

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(
        try_read_root_mem_value(&process, NATIVE_ACCOUNT_DATA_PTR + ACCOUNT_DATA_LENGTH as u32 * 2),
        None,
        "Memory starting from 6144 should stay uninitialized"
    );
}

// HELPER FUNCTIONS
// ================================================================================================

fn get_mock_advice_inputs(foreign_account: &Account, mock_chain: &MockChain) -> AdviceInputs {
    let foreign_id_root = Digest::from([foreign_account.id().into(), ZERO, ZERO, ZERO]);
    let foreign_id_and_nonce = [foreign_account.id().into(), ZERO, ZERO, foreign_account.nonce()];
    let foreign_vault_root = foreign_account.vault().commitment();
    let foreign_storage_root = foreign_account.storage().commitment();
    let foreign_code_root = foreign_account.code().commitment();

    AdviceInputs::default()
        .with_map([
            // ACCOUNT_ID |-> [ID_AND_NONCE, VAULT_ROOT, STORAGE_ROOT, CODE_ROOT]
            (
                foreign_id_root,
                [
                    &foreign_id_and_nonce,
                    foreign_vault_root.as_elements(),
                    foreign_storage_root.as_elements(),
                    foreign_code_root.as_elements(),
                ]
                .concat(),
            ),
            // STORAGE_ROOT |-> [[STORAGE_SLOT_DATA]]
            (foreign_storage_root, foreign_account.storage().as_elements()),
            // CODE_ROOT |-> [num_procs, [ACCOUNT_PROCEDURE_DATA]]
            (foreign_code_root, foreign_account.code().as_elements()),
        ])
        .with_merkle_store(mock_chain.accounts().into())
}

fn foreign_account_data_memory_assertions(foreign_account: &Account, process: &Process<MockHost>) {
    let foreign_account_data_ptr = NATIVE_ACCOUNT_DATA_PTR + ACCOUNT_DATA_LENGTH as u32;

    assert_eq!(
        read_root_mem_value(process, foreign_account_data_ptr + ACCT_ID_AND_NONCE_OFFSET),
        [foreign_account.id().into(), ZERO, ZERO, foreign_account.nonce()],
    );

    assert_eq!(
        read_root_mem_value(process, foreign_account_data_ptr + ACCT_VAULT_ROOT_OFFSET),
        foreign_account.vault().commitment().as_elements(),
    );

    assert_eq!(
        read_root_mem_value(process, foreign_account_data_ptr + ACCT_STORAGE_COMMITMENT_OFFSET),
        Word::from(foreign_account.storage().commitment()),
    );

    assert_eq!(
        read_root_mem_value(process, foreign_account_data_ptr + ACCT_CODE_COMMITMENT_OFFSET),
        foreign_account.code().commitment().as_elements(),
    );

    assert_eq!(
        read_root_mem_value(process, foreign_account_data_ptr + NUM_ACCT_STORAGE_SLOTS_OFFSET),
        [
            u16::try_from(foreign_account.storage().slots().len()).unwrap().into(),
            ZERO,
            ZERO,
            ZERO
        ],
    );

    for (i, elements) in foreign_account
        .storage()
        .as_elements()
        .chunks(StorageSlot::NUM_ELEMENTS_PER_STORAGE_SLOT / 2)
        .enumerate()
    {
        assert_eq!(
            read_root_mem_value(
                process,
                foreign_account_data_ptr + ACCT_STORAGE_SLOTS_SECTION_OFFSET + i as u32
            ),
            Word::try_from(elements).unwrap(),
        )
    }

    assert_eq!(
        read_root_mem_value(process, foreign_account_data_ptr + NUM_ACCT_PROCEDURES_OFFSET),
        [
            u16::try_from(foreign_account.code().procedures().len()).unwrap().into(),
            ZERO,
            ZERO,
            ZERO
        ],
    );

    for (i, elements) in foreign_account
        .code()
        .as_elements()
        .chunks(AccountProcedureInfo::NUM_ELEMENTS_PER_PROC / 2)
        .enumerate()
    {
        assert_eq!(
            read_root_mem_value(
                process,
                foreign_account_data_ptr + ACCT_PROCEDURES_SECTION_OFFSET + i as u32
            ),
            Word::try_from(elements).unwrap(),
        );
    }
}
