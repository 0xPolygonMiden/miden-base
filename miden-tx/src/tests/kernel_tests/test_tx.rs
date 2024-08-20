use alloc::vec::Vec;

use miden_lib::transaction::memory::{
        NOTE_MEM_SIZE, NUM_OUTPUT_NOTES_PTR, OUTPUT_NOTE_ASSETS_OFFSET,
        OUTPUT_NOTE_METADATA_OFFSET, OUTPUT_NOTE_RECIPIENT_OFFSET, OUTPUT_NOTE_SECTION_OFFSET,
    };
use miden_objects::{
    accounts::account_id::testing::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
    },
    assets::Asset,
    notes::{
        Note, NoteAssets, NoteExecutionHint, NoteInputs, NoteMetadata, NoteRecipient, NoteType,
    },
    testing::{constants::NON_FUNGIBLE_ASSET_DATA_2, prepare_word},
    transaction::{OutputNote, OutputNotes},
};

use super::{Felt, MemAdviceProvider, ProcessState, Word, ONE, ZERO};
use crate::{
    testing::{executor::CodeExecutor, testing_assembler, TransactionContextBuilder},
    tests::kernel_tests::read_root_mem_value,
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

            exec.tx::create_note
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

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new((NoteType::Public as u64) << 62);

    let code = format!(
        "
        use.kernel::prologue
        use.miden::tx

        begin
            exec.prologue::prepare_transaction

            push.{recipient}
            push.{note_execution_hint}
            push.{PUBLIC_NOTE}
            push.{tag}

            exec.tx::create_note
        end
        ",
        recipient = prepare_word(&recipient),
        note_execution_hint = Felt::from(NoteExecutionHint::always()),
        PUBLIC_NOTE = NoteType::Public as u8,
        tag = tag,
    );

    let process = tx_context.execute_code(&code);

    assert!(process.is_err(), "Transaction should have failed because the tag is invalid");
}

#[test]
fn test_create_note_too_many_notes() {
    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);

    let code = format!(
        "
        use.kernel::constants
        use.kernel::memory
        use.miden::tx

        begin
            exec.constants::get_max_num_output_notes
            exec.memory::set_num_output_notes

            push.{recipient}
            push.{PUBLIC_NOTE}
            push.{tag}

            exec.tx::create_note
        end
        ",
        recipient = prepare_word(&recipient),
        tag = tag,
        PUBLIC_NOTE = NoteType::Public as u8,
    );

    let process = CodeExecutor::with_advice_provider(MemAdviceProvider::default())
        .run(&code, testing_assembler::instance().clone());

    // assert the process failed
    assert!(process.is_err());
}

#[test]
#[ignore = "stack oveflow bug"]
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
            exec.tx::create_note
            # => [note_idx]

            push.{asset_1}
            exec.tx::add_asset_to_note
            # => [ASSET, note_idx]
            
            dropw drop
            # => []

            # create output note 2
            push.{recipient_2}
            push.{NOTE_EXECUTION_HINT_2}
            push.{PUBLIC_NOTE}
            push.{aux_2}
            push.{tag_2}
            exec.tx::create_note
            # => [note_idx]

            push.{asset_2} 
            exec.tx::add_asset_to_note
            # => [ASSET, note_idx]

            dropw drop
            # => []

            # compute the output notes hash
            exec.tx::get_output_notes_hash
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
#[ignore = "stack overflow bug"]
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

            exec.tx::create_note
            # => [note_idx]

            push.{asset}
            exec.tx::add_asset_to_note
            # => [ASSET, note_idx]

            dropw
            # => [note_idx]
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
#[ignore = "stack overflow bug"]
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
    let non_fungible_asset = Asset::mock_non_fungible(
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        &NON_FUNGIBLE_ASSET_DATA_2,
    );
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

            exec.tx::create_note
            # => [note_idx]

            push.{asset}
            exec.tx::add_asset_to_note dropw
            # => [note_idx]

            push.{asset_2}
            exec.tx::add_asset_to_note dropw
            # => [note_idx]

            push.{asset_3}
            exec.tx::add_asset_to_note dropw
            # => [note_idx]

            push.{nft}
            exec.tx::add_asset_to_note dropw
            # => [note_idx]
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
        Asset::mock_non_fungible(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &[1, 2, 3]);
    let encoded = Word::from(non_fungible_asset);

    let code = format!(
        "
        use.kernel::prologue
        use.miden::tx

        begin
            exec.prologue::prepare_transaction

            push.{recipient}
            push.{PUBLIC_NOTE}
            push.{tag}
            push.{nft}

            exec.tx::create_note
            # => [note_idx]

            push.{nft} movup.4
            exec.tx::add_asset_to_note
            # => [note_idx]
        end
        ",
        recipient = prepare_word(&recipient),
        PUBLIC_NOTE = NoteType::Public as u8,
        tag = tag,
        nft = prepare_word(&encoded),
    );

    let process = tx_context.execute_code(&code);

    assert!(
        process.is_err(),
        "Transaction should have failed because the same NFT is added twice"
    );
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
            exec.tx::build_recipient_hash

            push.{execution_hint}
            push.{PUBLIC_NOTE}
            push.{aux}
            push.{tag}
            exec.tx::create_note
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
