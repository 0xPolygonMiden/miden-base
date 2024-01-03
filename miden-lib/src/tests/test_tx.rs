use miden_objects::{
    notes::Note,
    transaction::{OutputNote, OutputNotes},
};
use mock::{
    constants::ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    prepare_transaction,
    procedures::prepare_word,
    run_tx, run_within_tx_kernel,
};

use super::{
    build_tx_inputs, ContextId, Felt, MemAdviceProvider, ProcessState, StackInputs, Word, ONE, ZERO,
};
use crate::transaction::memory::{
    CREATED_NOTE_ASSETS_OFFSET, CREATED_NOTE_METADATA_OFFSET, CREATED_NOTE_RECIPIENT_OFFSET,
    CREATED_NOTE_SECTION_OFFSET, NUM_CREATED_NOTES_PTR,
};

#[test]
fn test_create_note() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let account_id = account.id();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);
    let asset = [Felt::new(10), ZERO, ZERO, Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN)];

    let code = format!(
        "
    use.miden::kernels::tx::prologue
    use.miden::tx

    begin
        exec.prologue::prepare_transaction

        push.{recipient}
        push.{tag}
        push.{asset}

        exec.tx::create_note
    end
    ",
        recipient = prepare_word(&recipient),
        tag = tag,
        asset = prepare_word(&asset)
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider).unwrap();

    // assert the number of created notes has been incremented to 1.
    assert_eq!(
        process.get_mem_value(ContextId::root(), NUM_CREATED_NOTES_PTR).unwrap(),
        [ONE, ZERO, ZERO, ZERO]
    );

    // assert the recipient is stored at the correct memory location.
    assert_eq!(
        process
            .get_mem_value(
                ContextId::root(),
                CREATED_NOTE_SECTION_OFFSET + CREATED_NOTE_RECIPIENT_OFFSET
            )
            .unwrap(),
        recipient
    );

    // assert the metadata is stored at the correct memory location.
    assert_eq!(
        process
            .get_mem_value(
                ContextId::root(),
                CREATED_NOTE_SECTION_OFFSET + CREATED_NOTE_METADATA_OFFSET
            )
            .unwrap(),
        [ONE, tag, Felt::from(account_id), ZERO]
    );

    // assert the asset is stored at the correct memory location.
    assert_eq!(
        process
            .get_mem_value(
                ContextId::root(),
                CREATED_NOTE_SECTION_OFFSET + CREATED_NOTE_ASSETS_OFFSET
            )
            .unwrap(),
        asset
    );

    // assert there top item on the stack is a pointer to the created note.
    assert_eq!(process.stack.get(0), Felt::new(10000));
}

#[test]
fn test_create_note_too_many_notes() {
    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);
    let asset = [Felt::new(10), ZERO, ZERO, Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN)];

    let code = format!(
        "
    use.miden::kernels::tx::constants
    use.miden::kernels::tx::memory
    use.miden::tx

    begin
        exec.constants::get_max_num_created_notes
        exec.memory::set_num_created_notes

        push.{recipient}
        push.{tag}
        push.{asset}

        exec.tx::create_note
    end
    ",
        recipient = prepare_word(&recipient),
        tag = tag,
        asset = prepare_word(&asset)
    );

    let process =
        run_within_tx_kernel("", &code, StackInputs::default(), MemAdviceProvider::default(), None);

    // assert the process failed
    assert!(process.is_err());
}

#[test]
fn test_get_output_notes_hash() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    // extract input note data
    let input_note_1 = notes.first().unwrap().note();
    let input_asset_1 = **input_note_1.vault().iter().take(1).collect::<Vec<_>>().first().unwrap();
    let input_note_2 = notes.last().unwrap().note();
    let input_asset_2 = **input_note_2.vault().iter().take(1).collect::<Vec<_>>().first().unwrap();

    // create output note 1
    let output_serial_no_1 = [Felt::new(8); 4];
    let output_tag_1 = Felt::new(8888);
    let output_note_1 = Note::new(
        input_note_1.script().clone(),
        &[],
        &[input_asset_1],
        output_serial_no_1,
        account.id(),
        output_tag_1,
    )
    .unwrap();

    // create output note 2
    let output_serial_no_2 = [Felt::new(11); 4];
    let output_tag_2 = Felt::new(1111);
    let output_note_2 = Note::new(
        input_note_2.script().clone(),
        &[],
        &[input_asset_2],
        output_serial_no_2,
        account.id(),
        output_tag_2,
    )
    .unwrap();

    // compute expected output notes hash
    let expected_output_notes_hash = OutputNotes::new(vec![
        OutputNote::from(output_note_1.clone()),
        OutputNote::from(output_note_2.clone()),
    ])
    .unwrap()
    .commitment();

    let code = format!(
        "
    use.miden::kernels::tx::prologue
    use.miden::tx

    begin
        exec.prologue::prepare_transaction

        # create output note 1
        push.{recipient_1}
        push.{tag_1}
        push.{asset_1}
        exec.tx::create_note
        drop

        # create output note 2
        push.{recipient_2}
        push.{tag_2}
        push.{asset_2}
        exec.tx::create_note
        drop

        # compute the output notes hash
        exec.tx::get_output_notes_hash
        push.{expected} assert_eqw
    end
    ",
        recipient_1 = prepare_word(&output_note_1.recipient()),
        tag_1 = output_note_1.metadata().tag(),
        asset_1 = prepare_word(&Word::from(
            **output_note_1.vault().iter().take(1).collect::<Vec<_>>().first().unwrap()
        )),
        recipient_2 = prepare_word(&output_note_2.recipient()),
        tag_2 = output_note_2.metadata().tag(),
        asset_2 = prepare_word(&Word::from(
            **output_note_2.vault().iter().take(1).collect::<Vec<_>>().first().unwrap()
        )),
        expected = prepare_word(&expected_output_notes_hash)
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let _process = run_tx(program, stack_inputs, advice_provider).unwrap();
}
