use alloc::vec::Vec;
use std::string::String;

use miden_lib::{
    errors::tx_kernel_errors::{
        ERR_NON_FUNGIBLE_ASSET_ALREADY_EXISTS, ERR_TX_NUMBER_OF_OUTPUT_NOTES_EXCEEDS_LIMIT,
    },
    transaction::memory::{
        NOTE_MEM_SIZE, NUM_OUTPUT_NOTES_PTR, OUTPUT_NOTE_ASSETS_OFFSET,
        OUTPUT_NOTE_METADATA_OFFSET, OUTPUT_NOTE_RECIPIENT_OFFSET, OUTPUT_NOTE_SECTION_OFFSET,
    },
    utils::word_to_masm_push_string,
};
use miden_objects::{
    FieldElement,
    account::AccountId,
    asset::NonFungibleAsset,
    note::{
        Note, NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteInputs, NoteMetadata,
        NoteRecipient, NoteTag, NoteType,
    },
    testing::{
        account_id::{ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET, ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_2},
        constants::NON_FUNGIBLE_ASSET_DATA_2,
    },
    transaction::{OutputNote, OutputNotes},
};

use super::{Felt, ONE, Process, ProcessState, Word, ZERO};
use super::{Felt, ONE, ProcessState, Word, ZERO, word_to_masm_push_string};
use crate::{
    assert_execution_error,
    testing::{MockChain, TransactionContextBuilder},
    tests::kernel_tests::{read_root_mem_word, try_read_root_mem_word},
    assert_execution_error, testing::TransactionContextBuilder,
    tests::kernel_tests::read_root_mem_word,
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
    let expected_output_notes_commitment = OutputNotes::new(vec![
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
            # => [OUTPUT_NOTES_COMMITMENT]

            # truncate the stack
            exec.sys::truncate_stack
            # => [OUTPUT_NOTES_COMMITMENT]
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

    assert_eq!(process_state.get_stack_word(0), *expected_output_notes_commitment);
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
            # SCRIPT_ROOT
            push.{script_root}
            # SERIAL_NUM
            push.{output_serial_no}
            # => [SERIAL_NUM, SCRIPT_ROOT, INPUT_COMMITMENT, pad(4)]

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
        script_root = input_note_1.script().clone().root(),
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
