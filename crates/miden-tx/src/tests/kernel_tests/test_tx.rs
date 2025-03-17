use alloc::vec::Vec;
use std::string::{String, ToString};

use miden_lib::{
    errors::tx_kernel_errors::{
        ERR_NON_FUNGIBLE_ASSET_ALREADY_EXISTS, ERR_TX_NUMBER_OF_OUTPUT_NOTES_EXCEEDS_LIMIT,
    },
    transaction::{
        memory::{
            ACCOUNT_DATA_LENGTH, ACCT_CODE_COMMITMENT_OFFSET, ACCT_ID_AND_NONCE_OFFSET,
            ACCT_PROCEDURES_SECTION_OFFSET, ACCT_STORAGE_COMMITMENT_OFFSET,
            ACCT_STORAGE_SLOTS_SECTION_OFFSET, ACCT_VAULT_ROOT_OFFSET, NATIVE_ACCOUNT_DATA_PTR,
            NOTE_MEM_SIZE, NUM_ACCT_PROCEDURES_OFFSET, NUM_ACCT_STORAGE_SLOTS_OFFSET,
            NUM_OUTPUT_NOTES_PTR, OUTPUT_NOTE_ASSETS_OFFSET, OUTPUT_NOTE_METADATA_OFFSET,
            OUTPUT_NOTE_RECIPIENT_OFFSET, OUTPUT_NOTE_SECTION_OFFSET,
        },
        TransactionKernel,
    },
};
use miden_objects::{
    account::{
        Account, AccountBuilder, AccountComponent, AccountId, AccountProcedureInfo, AccountStorage,
        StorageSlot,
    },
    asset::NonFungibleAsset,
    crypto::merkle::{LeafIndex, MerklePath},
    note::{
        Note, NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteInputs, NoteMetadata,
        NoteRecipient, NoteTag, NoteType,
    },
    testing::{
        account_component::AccountMockComponent,
        account_id::{ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET, ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_2},
        constants::NON_FUNGIBLE_ASSET_DATA_2,
        storage::STORAGE_LEAVES_2,
    },
    transaction::{OutputNote, OutputNotes, TransactionScript},
    FieldElement, ACCOUNT_TREE_DEPTH,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use vm_processor::AdviceInputs;

use super::{word_to_masm_push_string, Felt, Process, ProcessState, Word, ONE, ZERO};
use crate::{
    assert_execution_error,
    testing::{MockChain, TransactionContextBuilder},
    tests::kernel_tests::{read_root_mem_word, try_read_root_mem_word},
    TransactionExecutor,
};

#[test]
fn test_create_note() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();
    let account_id = tx_context.account().id();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let aux = Felt::new(27);
    let tag = NoteTag::from_account_id(account_id, NoteExecutionMode::Local).unwrap();

    let code = format!(
        "
        use.miden::contracts::wallets::basic->wallet
        
        use.kernel::prologue

        begin
            exec.prologue::prepare_transaction

            push.{recipient}
            push.{note_execution_hint}
            push.{PUBLIC_NOTE}
            push.{aux}
            push.{tag}

            call.wallet::create_note

            # truncate the stack
            swapdw dropw dropw
        end
        ",
        recipient = word_to_masm_push_string(&recipient),
        PUBLIC_NOTE = NoteType::Public as u8,
        note_execution_hint = Felt::from(NoteExecutionHint::after_block(23.into()).unwrap()),
        tag = tag,
    );

    let process = &tx_context.execute_code(&code).unwrap();

    assert_eq!(
        read_root_mem_word(&process.into(), NUM_OUTPUT_NOTES_PTR),
        [ONE, ZERO, ZERO, ZERO],
        "number of output notes must increment by 1",
    );

    assert_eq!(
        read_root_mem_word(
            &process.into(),
            OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_RECIPIENT_OFFSET
        ),
        recipient,
        "recipient must be stored at the correct memory location",
    );

    let expected_note_metadata: Word = NoteMetadata::new(
        account_id,
        NoteType::Public,
        tag,
        NoteExecutionHint::after_block(23.into()).unwrap(),
        Felt::new(27),
    )
    .unwrap()
    .into();

    assert_eq!(
        read_root_mem_word(
            &process.into(),
            OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_METADATA_OFFSET
        ),
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
            use.miden::contracts::wallets::basic->wallet
            use.kernel::prologue
    
            begin
                exec.prologue::prepare_transaction
    
                push.{recipient}
                push.{execution_hint_always}
                push.{PUBLIC_NOTE}
                push.{aux}
                push.{tag}
    
                call.wallet::create_note

                # clean the stack
                dropw dropw
            end
            ",
            recipient = word_to_masm_push_string(&[ZERO, ONE, Felt::new(2), Felt::new(3)]),
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
        use.miden::contracts::wallets::basic->wallet
        use.kernel::constants
        use.kernel::memory
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

            call.wallet::create_note
        end
        ",
        tag = Felt::new(4),
        recipient = word_to_masm_push_string(&[ZERO, ONE, Felt::new(2), Felt::new(3)]),
        execution_hint_always = Felt::from(NoteExecutionHint::always()),
        PUBLIC_NOTE = NoteType::Public as u8,
        aux = Felt::ZERO,
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_TX_NUMBER_OF_OUTPUT_NOTES_EXCEEDS_LIMIT);
}

#[test]
fn test_get_output_notes_commitment() {
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
        NoteExecutionHint::after_block(123.into()).unwrap(),
        ZERO,
    )
    .unwrap();
    let inputs = NoteInputs::new(vec![]).unwrap();
    let recipient = NoteRecipient::new(output_serial_no_2, input_note_2.script().clone(), inputs);
    let output_note_2 = Note::new(assets, metadata, recipient);

    // compute expected output notes commitment
    let expected_output_notes_hash = OutputNotes::new(vec![
        OutputNote::Full(output_note_1.clone()),
        OutputNote::Full(output_note_2.clone()),
    ])
    .unwrap()
    .commitment();

    let code = format!(
        "
        use.std::sys

        use.miden::contracts::wallets::basic->wallet
        use.miden::tx

        use.kernel::prologue
        use.test::account

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
            call.wallet::create_note
            # => [note_idx]

            push.{asset_1}
            call.account::add_asset_to_note
            # => [ASSET, note_idx]
            
            dropw drop
            # => []

            # create output note 2
            push.{recipient_2}
            push.{NOTE_EXECUTION_HINT_2}
            push.{PUBLIC_NOTE}
            push.{aux_2}
            push.{tag_2}
            call.wallet::create_note
            # => [note_idx]

            push.{asset_2} 
            call.account::add_asset_to_note
            # => [ASSET, note_idx]

            dropw drop
            # => []

            # compute the output notes commitment
            exec.tx::get_output_notes_commitment
            # => [COM]

            # truncate the stack
            exec.sys::truncate_stack
            # => [COM]
        end
        ",
        PUBLIC_NOTE = NoteType::Public as u8,
        NOTE_EXECUTION_HINT_1 = Felt::from(output_note_1.metadata().execution_hint()),
        recipient_1 = word_to_masm_push_string(&output_note_1.recipient().digest()),
        tag_1 = output_note_1.metadata().tag(),
        aux_1 = output_note_1.metadata().aux(),
        asset_1 = word_to_masm_push_string(&Word::from(
            **output_note_1.assets().iter().take(1).collect::<Vec<_>>().first().unwrap()
        )),
        recipient_2 = word_to_masm_push_string(&output_note_2.recipient().digest()),
        NOTE_EXECUTION_HINT_2 = Felt::from(output_note_2.metadata().execution_hint()),
        tag_2 = output_note_2.metadata().tag(),
        aux_2 = output_note_2.metadata().aux(),
        asset_2 = word_to_masm_push_string(&Word::from(
            **output_note_2.assets().iter().take(1).collect::<Vec<_>>().first().unwrap()
        )),
    );

    let process = &tx_context.execute_code(&code).unwrap();
    let process_state: ProcessState = process.into();

    assert_eq!(
        read_root_mem_word(&process_state, NUM_OUTPUT_NOTES_PTR),
        [Felt::new(2), ZERO, ZERO, ZERO],
        "The test creates two notes",
    );
    assert_eq!(
        NoteMetadata::try_from(read_root_mem_word(
            &process_state,
            OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_METADATA_OFFSET
        ))
        .unwrap(),
        *output_note_1.metadata(),
        "Validate the output note 1 metadata",
    );
    assert_eq!(
        NoteMetadata::try_from(read_root_mem_word(
            &process_state,
            OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_METADATA_OFFSET + NOTE_MEM_SIZE
        ))
        .unwrap(),
        *output_note_2.metadata(),
        "Validate the output note 1 metadata",
    );

    assert_eq!(process_state.get_stack_word(0), *expected_output_notes_hash);
}

#[test]
fn test_create_note_and_add_asset() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let faucet_id = AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET).unwrap();
    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let aux = Felt::new(27);
    let tag = Felt::new(4);
    let asset = [Felt::new(10), ZERO, faucet_id.suffix(), faucet_id.prefix().as_felt()];

    let code = format!(
        "
        use.miden::contracts::wallets::basic->wallet

        use.kernel::prologue
        use.test::account

        begin
            exec.prologue::prepare_transaction

            push.{recipient}
            push.{NOTE_EXECUTION_HINT}
            push.{PUBLIC_NOTE}
            push.{aux}
            push.{tag}

            call.wallet::create_note
            # => [note_idx]

            push.{asset}
            call.account::add_asset_to_note
            # => [ASSET, note_idx]

            dropw
            # => [note_idx]

            # truncate the stack
            swapdw dropw dropw
        end
        ",
        recipient = word_to_masm_push_string(&recipient),
        PUBLIC_NOTE = NoteType::Public as u8,
        NOTE_EXECUTION_HINT = Felt::from(NoteExecutionHint::always()),
        tag = tag,
        asset = word_to_masm_push_string(&asset),
    );

    let process = &tx_context.execute_code(&code).unwrap();
    let process_state: ProcessState = process.into();

    assert_eq!(
        read_root_mem_word(&process_state, OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_ASSETS_OFFSET),
        asset,
        "asset must be stored at the correct memory location",
    );

    assert_eq!(
        process_state.get_stack_item(0),
        ZERO,
        "top item on the stack is the index to the output note"
    );
}

#[test]
fn test_create_note_and_add_multiple_assets() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let faucet = AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET).unwrap();
    let faucet_2 = AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_2).unwrap();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let aux = Felt::new(27);
    let tag = Felt::new(4);

    let asset = [Felt::new(10), ZERO, faucet.suffix(), faucet.prefix().as_felt()];
    let asset_2 = [Felt::new(20), ZERO, faucet_2.suffix(), faucet_2.prefix().as_felt()];
    let asset_3 = [Felt::new(30), ZERO, faucet_2.suffix(), faucet_2.prefix().as_felt()];
    let asset_2_and_3 = [Felt::new(50), ZERO, faucet_2.suffix(), faucet_2.prefix().as_felt()];

    let non_fungible_asset = NonFungibleAsset::mock(&NON_FUNGIBLE_ASSET_DATA_2);
    let non_fungible_asset_encoded = Word::from(non_fungible_asset);

    let code = format!(
        "
        use.miden::contracts::wallets::basic->wallet

        use.kernel::prologue
        use.test::account

        begin
            exec.prologue::prepare_transaction

            push.{recipient}
            push.{PUBLIC_NOTE}
            push.{aux}
            push.{tag}

            call.wallet::create_note
            # => [note_idx]

            push.{asset}
            call.account::add_asset_to_note dropw
            # => [note_idx]

            push.{asset_2}
            call.account::add_asset_to_note dropw
            # => [note_idx]

            push.{asset_3}
            call.account::add_asset_to_note dropw
            # => [note_idx]

            push.{nft}
            call.account::add_asset_to_note dropw
            # => [note_idx]

            # truncate the stack
            swapdw dropw drop drop drop
        end
        ",
        recipient = word_to_masm_push_string(&recipient),
        PUBLIC_NOTE = NoteType::Public as u8,
        tag = tag,
        asset = word_to_masm_push_string(&asset),
        asset_2 = word_to_masm_push_string(&asset_2),
        asset_3 = word_to_masm_push_string(&asset_3),
        nft = word_to_masm_push_string(&non_fungible_asset_encoded),
    );

    let process = &tx_context.execute_code(&code).unwrap();
    let process_state: ProcessState = process.into();

    assert_eq!(
        read_root_mem_word(&process_state, OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_ASSETS_OFFSET),
        asset,
        "asset must be stored at the correct memory location",
    );

    assert_eq!(
        read_root_mem_word(
            &process_state,
            OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_ASSETS_OFFSET + 4
        ),
        asset_2_and_3,
        "asset_2 and asset_3 must be stored at the same correct memory location",
    );

    assert_eq!(
        read_root_mem_word(
            &process_state,
            OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_ASSETS_OFFSET + 8
        ),
        Word::from(non_fungible_asset_encoded),
        "non_fungible_asset must be stored at the correct memory location",
    );

    assert_eq!(
        process_state.get_stack_item(0),
        ZERO,
        "top item on the stack is the index to the output note"
    );
}

#[test]
fn test_create_note_and_add_same_nft_twice() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);
    let non_fungible_asset = NonFungibleAsset::mock(&[1, 2, 3]);
    let encoded = Word::from(non_fungible_asset);

    let code = format!(
        "
        use.kernel::prologue
        use.test::account
        use.miden::contracts::wallets::basic->wallet

        begin
            exec.prologue::prepare_transaction
            # => []

            padw padw
            push.{recipient}
            push.{execution_hint_always}
            push.{PUBLIC_NOTE}
            push.{aux}
            push.{tag}

            call.wallet::create_note
            # => [note_idx, pad(15)]

            push.{nft} 
            call.account::add_asset_to_note
            # => [NFT, note_idx, pad(15)]
            dropw

            push.{nft} 
            call.account::add_asset_to_note
            # => [NFT, note_idx, pad(15)]

            repeat.5 dropw end
        end
        ",
        recipient = word_to_masm_push_string(&recipient),
        PUBLIC_NOTE = NoteType::Public as u8,
        execution_hint_always = Felt::from(NoteExecutionHint::always()),
        aux = Felt::new(0),
        tag = tag,
        nft = word_to_masm_push_string(&encoded),
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
    let input_commitment = inputs.commitment();

    let recipient = NoteRecipient::new(output_serial_no, input_note_1.script().clone(), inputs);
    let code = format!(
        "
        use.miden::contracts::wallets::basic->wallet
        use.miden::tx
        use.kernel::prologue

        proc.build_recipient_hash
            exec.tx::build_recipient_hash
        end

        begin
            exec.prologue::prepare_transaction

            # pad the stack before call
            padw

            # input
            push.{input_commitment}
            # SCRIPT_COMMITMENT
            push.{script_commitment}
            # SERIAL_NUM
            push.{output_serial_no}
            # => [SERIAL_NUM, SCRIPT_COMMITMENT, INPUT_COMMITMENT, pad(4)]

            call.build_recipient_hash
            # => [RECIPIENT, pad(12)]

            push.{execution_hint}
            push.{PUBLIC_NOTE}
            push.{aux}
            push.{tag}
            # => [tag, aux, note_type, execution_hint, RECIPIENT, pad(12)]

            call.wallet::create_note
            # => [note_idx, pad(19)]

            # clean the stack
            dropw dropw dropw dropw dropw
        end
        ",
        script_commitment = input_note_1.script().clone().commitment(),
        output_serial_no = word_to_masm_push_string(&output_serial_no),
        PUBLIC_NOTE = NoteType::Public as u8,
        tag = tag,
        execution_hint = Felt::from(NoteExecutionHint::after_block(2.into()).unwrap()),
        aux = aux,
    );

    let process = &tx_context.execute_code(&code).unwrap();

    assert_eq!(
        read_root_mem_word(&process.into(), NUM_OUTPUT_NOTES_PTR),
        [ONE, ZERO, ZERO, ZERO],
        "number of output notes must increment by 1",
    );

    let recipient_digest: Vec<Felt> = recipient.clone().digest().to_vec();

    assert_eq!(
        read_root_mem_word(
            &process.into(),
            OUTPUT_NOTE_SECTION_OFFSET + OUTPUT_NOTE_RECIPIENT_OFFSET
        ),
        recipient_digest.as_slice(),
        "recipient hash not correct",
    );
}

// BLOCK TESTS
// ================================================================================================

#[test]
fn test_block_procedures() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let code = "
        use.miden::tx
        use.kernel::prologue

        begin
            exec.prologue::prepare_transaction

            # get the block data
            exec.tx::get_block_number
            exec.tx::get_block_timestamp
            exec.tx::get_block_commitment
            # => [BLOCK_COMMITMENT, block_timestamp, block_number]

            # truncate the stack
            swapdw dropw dropw
        end
        ";

    let process = &tx_context.execute_code(code).unwrap();

    assert_eq!(
        process.stack.get_word(0),
        tx_context.tx_inputs().block_header().commitment().as_elements(),
        "top word on the stack should be equal to the block header commitment"
    );

    assert_eq!(
        process.stack.get(4).as_int(),
        tx_context.tx_inputs().block_header().timestamp() as u64,
        "fifth element on the stack should be equal to the timestamp of the last block creation"
    );

    assert_eq!(
        process.stack.get(5).as_int(),
        tx_context.tx_inputs().block_header().block_num().as_u64(),
        "sixth element on the stack should be equal to the block number"
    );
}

// FOREIGN PROCEDURE INVOCATION TESTS
// ================================================================================================

#[test]
fn test_fpi_memory() {
    // Prepare the test data
    let storage_slots =
        vec![AccountStorage::mock_item_0().slot, AccountStorage::mock_item_2().slot];
    let foreign_account_code_source = "
        use.miden::account

        export.get_item_foreign
            # make this foreign procedure unique to make sure that we invoke the procedure of the 
            # foreign account, not the native one
            push.1 drop
            exec.account::get_item

            # truncate the stack
            movup.6 movup.6 movup.6 drop drop drop
        end

        export.get_map_item_foreign
            # make this foreign procedure unique to make sure that we invoke the procedure of the 
            # foreign account, not the native one
            push.2 drop
            exec.account::get_map_item
        end
    ";

    let foreign_account_component = AccountComponent::compile(
        foreign_account_code_source,
        TransactionKernel::testing_assembler(),
        storage_slots.clone(),
    )
    .unwrap()
    .with_supports_all_types();

    let foreign_account = AccountBuilder::new(ChaCha20Rng::from_entropy().gen())
        .with_component(foreign_account_component)
        .build_existing()
        .unwrap();

    let native_account = AccountBuilder::new(ChaCha20Rng::from_entropy().gen())
        .with_component(
            AccountMockComponent::new_with_slots(
                TransactionKernel::testing_assembler(),
                vec![AccountStorage::mock_item_2().slot],
            )
            .unwrap(),
        )
        .build_existing()
        .unwrap();

    let mut mock_chain =
        MockChain::with_accounts(&[native_account.clone(), foreign_account.clone()]);
    mock_chain.seal_next_block();
    let advice_inputs = get_mock_fpi_adv_inputs(vec![&foreign_account], &mock_chain);

    let tx_context = mock_chain
        .build_tx_context(native_account.id(), &[], &[])
        .foreign_account_codes(vec![foreign_account.code().clone()])
        .advice_inputs(advice_inputs.clone())
        .build();

    // GET ITEM
    // --------------------------------------------------------------------------------------------
    // Check the correctness of the memory layout after `get_item_foreign` account procedure
    // invocation

    let code = format!(
        "
        use.std::sys
        
        use.kernel::prologue
        use.miden::tx

        begin
            exec.prologue::prepare_transaction

            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign` procedure of the foreign account 
            push.{get_item_foreign_hash}

            # push the foreign account ID
            push.{foreign_suffix}.{foreign_prefix}
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(11)]

            exec.tx::execute_foreign_procedure
            # => [STORAGE_VALUE_1]

            # truncate the stack
            exec.sys::truncate_stack
            end
            ",
        foreign_prefix = foreign_account.id().prefix().as_felt(),
        foreign_suffix = foreign_account.id().suffix(),
        get_item_foreign_hash = foreign_account.code().procedures()[0].mast_root(),
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(
        process.stack.get_word(0),
        storage_slots[0].value(),
        "Value at the top of the stack (value in the storage at index 0) should be equal [1, 2, 3, 4]",
    );

    foreign_account_data_memory_assertions(&foreign_account, &process);

    // GET MAP ITEM
    // --------------------------------------------------------------------------------------------
    // Check the correctness of the memory layout after `get_map_item` account procedure invocation

    let code = format!(
        "
        use.std::sys

        use.kernel::prologue
        use.miden::tx

        begin
            exec.prologue::prepare_transaction

            # pad the stack for the `execute_foreign_procedure` execution
            padw padw push.0.0
            # => [pad(10)]

            # push the key of desired storage item
            push.{map_key}

            # push the index of desired storage item
            push.1

            # get the hash of the `get_map_item_foreign` account procedure
            push.{get_map_item_foreign_hash}

            # push the foreign account ID
            push.{foreign_suffix}.{foreign_prefix}
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, MAP_ITEM_KEY, pad(10)]

            exec.tx::execute_foreign_procedure
            # => [MAP_VALUE]

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        foreign_prefix = foreign_account.id().prefix().as_felt(),
        foreign_suffix = foreign_account.id().suffix(),
        map_key = STORAGE_LEAVES_2[0].0,
        get_map_item_foreign_hash = foreign_account.code().procedures()[1].mast_root(),
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(
        process.stack.get_word(0),
        STORAGE_LEAVES_2[0].1,
        "Value at the top of the stack should be equal [1, 2, 3, 4]",
    );

    foreign_account_data_memory_assertions(&foreign_account, &process);

    // GET ITEM TWICE
    // --------------------------------------------------------------------------------------------
    // Check the correctness of the memory layout after two consecutive invocations of the
    // `get_item` account procedures. Invoking two foreign procedures from the same account should
    // result in reuse of the loaded account.

    let code = format!(
        "
        use.std::sys

        use.kernel::prologue
        use.miden::tx

        begin
            exec.prologue::prepare_transaction

            ### Get the storage item at index 0 #####################
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign` procedure of the foreign account 
            push.{get_item_foreign_hash}

            # push the foreign account ID
            push.{foreign_suffix}.{foreign_prefix}
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure dropw
            # => []

            ### Get the storage item at index 0 again ###############
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign` procedure of the foreign account 
            push.{get_item_foreign_hash}

            # push the foreign account ID
            push.{foreign_suffix}.{foreign_prefix}
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        foreign_prefix = foreign_account.id().prefix().as_felt(),
        foreign_suffix = foreign_account.id().suffix(),
        get_item_foreign_hash = foreign_account.code().procedures()[0].mast_root(),
    );

    let process = &tx_context.execute_code(&code).unwrap();

    // Check that the second invocation of the foreign procedure from the same account does not load
    // the account data again: already loaded data should be reused.
    //
    // Native account:    [8192; 16383]  <- initialized during prologue
    // Foreign account:   [16384; 24575] <- initialized during first FPI
    // Next account slot: [24576; 32767] <- should not be initialized
    assert_eq!(
        try_read_root_mem_word(
            &process.into(),
            NATIVE_ACCOUNT_DATA_PTR + ACCOUNT_DATA_LENGTH as u32 * 2
        ),
        None,
        "Memory starting from 24576 should stay uninitialized"
    );
}

#[test]
fn test_fpi_memory_two_accounts() {
    // Prepare the test data
    let storage_slots_1 = vec![AccountStorage::mock_item_0().slot];
    let storage_slots_2 = vec![AccountStorage::mock_item_1().slot];

    let foreign_account_code_source_1 = "
        use.miden::account

        export.get_item_foreign_1
            # make this foreign procedure unique to make sure that we invoke the procedure of the 
            # foreign account, not the native one
            push.1 drop
            exec.account::get_item

            # truncate the stack
            movup.6 movup.6 movup.6 drop drop drop
        end
    ";
    let foreign_account_code_source_2 = "
        use.miden::account

        export.get_item_foreign_2
            # make this foreign procedure unique to make sure that we invoke the procedure of the 
            # foreign account, not the native one
            push.2 drop
            exec.account::get_item

            # truncate the stack
            movup.6 movup.6 movup.6 drop drop drop
        end
    ";

    let foreign_account_component_1 = AccountComponent::compile(
        foreign_account_code_source_1,
        TransactionKernel::testing_assembler(),
        storage_slots_1.clone(),
    )
    .unwrap()
    .with_supports_all_types();

    let foreign_account_component_2 = AccountComponent::compile(
        foreign_account_code_source_2,
        TransactionKernel::testing_assembler(),
        storage_slots_2.clone(),
    )
    .unwrap()
    .with_supports_all_types();

    let foreign_account_1 = AccountBuilder::new(ChaCha20Rng::from_entropy().gen())
        .with_component(foreign_account_component_1)
        .build_existing()
        .unwrap();

    let foreign_account_2 = AccountBuilder::new(ChaCha20Rng::from_entropy().gen())
        .with_component(foreign_account_component_2)
        .build_existing()
        .unwrap();

    let native_account = AccountBuilder::new(ChaCha20Rng::from_entropy().gen())
        .with_component(
            AccountMockComponent::new_with_empty_slots(TransactionKernel::testing_assembler())
                .unwrap(),
        )
        .build_existing()
        .unwrap();

    let mut mock_chain = MockChain::with_accounts(&[
        native_account.clone(),
        foreign_account_1.clone(),
        foreign_account_2.clone(),
    ]);
    mock_chain.seal_next_block();
    let advice_inputs =
        get_mock_fpi_adv_inputs(vec![&foreign_account_1, &foreign_account_2], &mock_chain);

    let tx_context = mock_chain
        .build_tx_context(native_account.id(), &[], &[])
        .foreign_account_codes(vec![
            foreign_account_1.code().clone(),
            foreign_account_2.code().clone(),
        ])
        .advice_inputs(advice_inputs.clone())
        .build();

    // GET ITEM TWICE WITH TWO ACCOUNTS
    // --------------------------------------------------------------------------------------------
    // Check the correctness of the memory layout after two invocations of the `get_item` account
    // procedures separated by the call of this procedure against another foreign account. Invoking
    // two foreign procedures from the same account should result in reuse of the loaded account.

    let code = format!(
        "
        use.std::sys

        use.kernel::prologue
        use.miden::tx

        begin
            exec.prologue::prepare_transaction

            ### Get the storage item at index 0 from the first account 
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign_1` procedure of the foreign account 1
            push.{get_item_foreign_1_hash}

            # push the foreign account ID
            push.{foreign_1_suffix}.{foreign_1_prefix}
            # => [foreign_account_1_id_prefix, foreign_account_1_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure dropw
            # => []

            ### Get the storage item at index 0 from the second account 
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign_2` procedure of the foreign account 2
            push.{get_item_foreign_2_hash}

            # push the foreign account ID
            push.{foreign_2_suffix}.{foreign_2_prefix}
            # => [foreign_account_2_id_prefix, foreign_account_2_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure dropw
            # => []

            ### Get the storage item at index 0 from the first account again
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign_1` procedure of the foreign account 1
            push.{get_item_foreign_1_hash}

            # push the foreign account ID
            push.{foreign_1_suffix}.{foreign_1_prefix}
            # => [foreign_account_1_id_prefix, foreign_account_1_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        get_item_foreign_1_hash = foreign_account_1.code().procedures()[0].mast_root(),
        get_item_foreign_2_hash = foreign_account_2.code().procedures()[0].mast_root(),

        foreign_1_prefix = foreign_account_1.id().prefix().as_felt(),
        foreign_1_suffix = foreign_account_1.id().suffix(),

        foreign_2_prefix = foreign_account_2.id().prefix().as_felt(),
        foreign_2_suffix = foreign_account_2.id().suffix(),
    );

    let process = &tx_context.execute_code(&code).unwrap();

    // Check the correctness of the memory layout after multiple foreign procedure invocations from
    // different foreign accounts
    //
    // Native account:    [8192; 16383]  <- initialized during prologue
    // Foreign account 1: [16384; 24575] <- initialized during first FPI
    // Foreign account 2: [24576; 32767] <- initialized during second FPI
    // Next account slot: [32768; 40959] <- should not be initialized

    // check that the first word of the first foreign account slot is correct
    assert_eq!(
        read_root_mem_word(&process.into(), NATIVE_ACCOUNT_DATA_PTR + ACCOUNT_DATA_LENGTH as u32),
        [
            foreign_account_1.id().suffix(),
            foreign_account_1.id().prefix().as_felt(),
            ZERO,
            foreign_account_1.nonce()
        ]
    );

    // check that the first word of the second foreign account slot is correct
    assert_eq!(
        read_root_mem_word(
            &process.into(),
            NATIVE_ACCOUNT_DATA_PTR + ACCOUNT_DATA_LENGTH as u32 * 2
        ),
        [
            foreign_account_2.id().suffix(),
            foreign_account_2.id().prefix().as_felt(),
            ZERO,
            foreign_account_2.nonce()
        ]
    );

    // check that the first word of the third foreign account slot was not initialized
    assert_eq!(
        try_read_root_mem_word(
            &process.into(),
            NATIVE_ACCOUNT_DATA_PTR + ACCOUNT_DATA_LENGTH as u32 * 3
        ),
        None,
        "Memory starting from 32768 should stay uninitialized"
    );
}

/// Test the correctness of the foreign procedure execution.
///
/// It checks the foreign account code loading, providing the mast forest to the executor,
/// construction of the account procedure maps and execution the foreign procedure in order to
/// obtain the data from the foreign account's storage slot.
#[test]
fn test_fpi_execute_foreign_procedure() {
    // Prepare the test data
    let storage_slots =
        vec![AccountStorage::mock_item_0().slot, AccountStorage::mock_item_2().slot];
    let foreign_account_code_source = "
        use.miden::account

        export.get_item_foreign
            # make this foreign procedure unique to make sure that we invoke the procedure of the 
            # foreign account, not the native one
            push.1 drop
            exec.account::get_item

            # truncate the stack
            movup.6 movup.6 movup.6 drop drop drop
        end

        export.get_map_item_foreign
            # make this foreign procedure unique to make sure that we invoke the procedure of the 
            # foreign account, not the native one
            push.2 drop
            exec.account::get_map_item
        end
    ";

    let foreign_account_component = AccountComponent::compile(
        foreign_account_code_source,
        TransactionKernel::testing_assembler(),
        storage_slots,
    )
    .unwrap()
    .with_supports_all_types();

    let foreign_account = AccountBuilder::new(ChaCha20Rng::from_entropy().gen())
        .with_component(foreign_account_component)
        .build_existing()
        .unwrap();

    let native_account = AccountBuilder::new(ChaCha20Rng::from_entropy().gen())
        .with_component(
            AccountMockComponent::new_with_slots(TransactionKernel::testing_assembler(), vec![])
                .unwrap(),
        )
        .build_existing()
        .unwrap();

    let mut mock_chain =
        MockChain::with_accounts(&[native_account.clone(), foreign_account.clone()]);
    mock_chain.seal_next_block();
    let advice_inputs = get_mock_fpi_adv_inputs(vec![&foreign_account], &mock_chain);

    let code = format!(
        "
        use.std::sys

        use.miden::tx

        begin
            # get the storage item at index 0
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item` account procedure
            push.{get_item_foreign_hash}

            # push the foreign account ID
            push.{foreign_suffix}.{foreign_prefix}
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure
            # => [STORAGE_VALUE]

            # assert the correctness of the obtained value
            push.1.2.3.4 assert_eqw
            # => []

            # get the storage map at index 1
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw push.0.0
            # => [pad(10)]

            # push the key of desired storage item
            push.{map_key}

            # push the index of desired storage item
            push.1

            # get the hash of the `get_map_item_foreign` account procedure
            push.{get_map_item_foreign_hash}

            # push the foreign account ID
            push.{foreign_suffix}.{foreign_prefix}
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, MAP_ITEM_KEY, pad(10)]

            exec.tx::execute_foreign_procedure
            # => [MAP_VALUE]

            # assert the correctness of the obtained value
            push.1.2.3.4 assert_eqw
            # => []

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        foreign_prefix = foreign_account.id().prefix().as_felt(),
        foreign_suffix = foreign_account.id().suffix(),
        get_item_foreign_hash = foreign_account.code().procedures()[0].mast_root(),
        get_map_item_foreign_hash = foreign_account.code().procedures()[1].mast_root(),
        map_key = STORAGE_LEAVES_2[0].0,
    );

    let tx_script =
        TransactionScript::compile(code, vec![], TransactionKernel::testing_assembler()).unwrap();

    let tx_context = mock_chain
        .build_tx_context(native_account.id(), &[], &[])
        .advice_inputs(advice_inputs.clone())
        .tx_script(tx_script)
        .build();

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let mut executor = TransactionExecutor::new(tx_context.get_data_store(), None).with_tracing();

    // load the mast forest of the foreign account's code to be able to create an account procedure
    // index map and execute the specified foreign procedure
    executor.load_account_code(foreign_account.code());

    let _executed_transaction = executor
        .execute_transaction(
            native_account.id(),
            block_ref,
            &note_ids,
            tx_context.tx_args().clone(),
        )
        .map_err(|e| e.to_string())
        .unwrap();
}

// HELPER FUNCTIONS
// ================================================================================================

fn get_mock_fpi_adv_inputs(
    foreign_accounts: Vec<&Account>,
    mock_chain: &MockChain,
) -> AdviceInputs {
    let mut advice_inputs = AdviceInputs::default();

    for foreign_account in foreign_accounts {
        TransactionKernel::extend_advice_inputs_for_account(
            &mut advice_inputs,
            &foreign_account.into(),
            foreign_account.code(),
            &foreign_account.storage().get_header(),
            // Provide the merkle path of the foreign account to be able to verify that the account
            // tree has the commitment of this foreign account. Verification is done during the
            // execution of the `kernel::account::validate_current_foreign_account` procedure.
            &MerklePath::new(
                mock_chain
                    .accounts()
                      // TODO: Update.
                    .open(&LeafIndex::<ACCOUNT_TREE_DEPTH>::new(foreign_account.id().prefix().as_felt().as_int()).unwrap())
                    .path
                    .into(),
            ),
        )
        .unwrap();

        for slot in foreign_account.storage().slots() {
            // if there are storage maps, we populate the merkle store and advice map
            if let StorageSlot::Map(map) = slot {
                // extend the merkle store and map with the storage maps
                advice_inputs.extend_merkle_store(map.inner_nodes());
                // populate advice map with Sparse Merkle Tree leaf nodes
                advice_inputs
                    .extend_map(map.leaves().map(|(_, leaf)| (leaf.hash(), leaf.to_elements())));
            }
        }
    }

    advice_inputs
}

fn foreign_account_data_memory_assertions(foreign_account: &Account, process: &Process) {
    let foreign_account_data_ptr = NATIVE_ACCOUNT_DATA_PTR + ACCOUNT_DATA_LENGTH as u32;

    assert_eq!(
        read_root_mem_word(&process.into(), foreign_account_data_ptr + ACCT_ID_AND_NONCE_OFFSET),
        [
            foreign_account.id().suffix(),
            foreign_account.id().prefix().as_felt(),
            ZERO,
            foreign_account.nonce()
        ],
    );

    assert_eq!(
        read_root_mem_word(&process.into(), foreign_account_data_ptr + ACCT_VAULT_ROOT_OFFSET),
        foreign_account.vault().root().as_elements(),
    );

    assert_eq!(
        read_root_mem_word(
            &process.into(),
            foreign_account_data_ptr + ACCT_STORAGE_COMMITMENT_OFFSET
        ),
        Word::from(foreign_account.storage().commitment()),
    );

    assert_eq!(
        read_root_mem_word(&process.into(), foreign_account_data_ptr + ACCT_CODE_COMMITMENT_OFFSET),
        foreign_account.code().commitment().as_elements(),
    );

    assert_eq!(
        read_root_mem_word(
            &process.into(),
            foreign_account_data_ptr + NUM_ACCT_STORAGE_SLOTS_OFFSET
        ),
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
            read_root_mem_word(
                &process.into(),
                foreign_account_data_ptr + ACCT_STORAGE_SLOTS_SECTION_OFFSET + (i as u32) * 4
            ),
            Word::try_from(elements).unwrap(),
        )
    }

    assert_eq!(
        read_root_mem_word(&process.into(), foreign_account_data_ptr + NUM_ACCT_PROCEDURES_OFFSET),
        [
            u16::try_from(foreign_account.code().num_procedures()).unwrap().into(),
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
            read_root_mem_word(
                &process.into(),
                foreign_account_data_ptr + ACCT_PROCEDURES_SECTION_OFFSET + (i as u32) * 4
            ),
            Word::try_from(elements).unwrap(),
        );
    }
}
