pub mod common;
use common::{
    data::mock_inputs, procedures::prepare_word, run_within_tx_kernel, Felt, MemAdviceProvider,
};

#[test]
fn test_get_sender_no_sender() {
    let inputs = mock_inputs();

    // calling get_sender should return sender
    let code = "
        use.miden::sat::prologue
        use.miden::sat::note_setup
        use.miden::sat::note

        begin
            exec.prologue::prepare_transaction
            exec.note::get_sender
        end
        ";
    let process = run_within_tx_kernel(
        "",
        code,
        inputs.stack_inputs(),
        MemAdviceProvider::from(inputs.advice_provider_inputs()),
        None,
        None,
    );
    assert!(process.is_err());
}

#[test]
fn test_get_sender() {
    let inputs = mock_inputs();

    // calling get_sender should return sender
    let code = "
        use.miden::sat::prologue
        use.miden::sat::note_setup
        use.miden::sat::note

        begin
            exec.prologue::prepare_transaction
            push.0
            exec.note_setup::prepare_note
            dropw dropw dropw dropw
            exec.note::get_sender
        end
        ";
    let process = run_within_tx_kernel(
        "",
        code,
        inputs.stack_inputs(),
        MemAdviceProvider::from(inputs.advice_provider_inputs()),
        None,
        None,
    )
    .unwrap();

    let sender = inputs.consumed_notes()[0].metadata().sender().into();
    assert_eq!(process.stack.get(0), sender);
}

#[test]
fn test_get_vault_data() {
    let inputs = mock_inputs();

    // for (i, note) in inputs.consumed_notes().iter().enumerate() {
    let notes = &inputs.consumed_notes();
    // calling get_vault_data should return vault data
    let code = format!(
        "
        use.miden::sat::prologue
        use.miden::sat::note_setup
        use.miden::sat::note
        use.miden::sat::layout

        begin
            exec.prologue::prepare_transaction

            # prepare note 0
            push.0
            exec.note_setup::prepare_note
            
            # drop the note inputs
            dropw dropw dropw dropw

            # get the vault data
            exec.note::get_vault_data

            # assert the vault data is correct
            push.{note_0_vault_root} assert_eqw
            push.{note_0_num_assets} assert_eq

            # prepare note 1
            push.1
            exec.note_setup::prepare_note

            # drop the note inputs
            dropw dropw dropw dropw

            # get the vault data
            exec.note::get_vault_data

            # assert the vault data is correct
            push.{note_1_vault_root} assert_eqw
            push.{note_1_num_assets} assert_eq
        end
        ",
        note_0_vault_root = prepare_word(&notes[0].vault().hash()),
        note_0_num_assets = notes[0].vault().num_assets(),
        note_1_vault_root = prepare_word(&notes[1].vault().hash()),
        note_1_num_assets = notes[1].vault().num_assets(),
    );

    // run to ensure success
    let _process = run_within_tx_kernel(
        "",
        &code,
        inputs.stack_inputs(),
        MemAdviceProvider::from(inputs.advice_provider_inputs()),
        None,
        None,
    )
    .unwrap();
}

#[test]
fn test_get_assets() {
    let inputs = mock_inputs();

    const DEST_POINTER_NOTE_0: u64 = 100000000;
    const DEST_POINTER_NOTE_1: u64 = 200000000;

    let notes = inputs.consumed_notes();

    // calling get_assets should return assets at the specified address
    let code = format!(
        "
        use.miden::sat::prologue
        use.miden::sat::note_setup
        use.miden::sat::note

        begin
            # prepare tx
            exec.prologue::prepare_transaction

            # prepare note 0
            push.0
            exec.note_setup::prepare_note

            # drop the note inputs
            dropw dropw dropw dropw

            # set the destination pointer for note 0 assets
            push.{DEST_POINTER_NOTE_0}

            # get the assets
            exec.note::get_assets

            # assert the number of assets is correct
            eq.{note_0_num_assets} assert

            # assert the pointer is returned
            eq.{DEST_POINTER_NOTE_0} assert

            # prepare note 1
            push.1
            exec.note_setup::prepare_note

            # drop the note inputs
            dropw dropw dropw dropw

            # set the destination pointer for note 1 assets
            push.{DEST_POINTER_NOTE_1}

            # get the assets
            exec.note::get_assets

            # assert the number of assets is correct
            eq.{note_1_num_assets} assert

            # assert the pointer is returned
            eq.{DEST_POINTER_NOTE_1} assert
        end
        ",
        note_0_num_assets = notes[0].vault().num_assets(),
        note_1_num_assets = notes[1].vault().num_assets(),
    );

    let process = run_within_tx_kernel(
        "",
        &code,
        inputs.stack_inputs(),
        MemAdviceProvider::from(inputs.advice_provider_inputs()),
        None,
        None,
    )
    .unwrap();

    // check the assets saved to memory for note 0 are correct
    for (asset, idx) in notes[0].vault().iter().zip(0u64..) {
        assert_eq!(
            process.get_memory_value(0, DEST_POINTER_NOTE_0 + idx).unwrap(),
            <[Felt; 4]>::from(*asset)
        );
    }

    // check the assets saved to memory for note 1 are correct
    for (asset, idx) in notes[1].vault().iter().zip(0u64..) {
        assert_eq!(
            process.get_memory_value(0, DEST_POINTER_NOTE_1 + idx).unwrap(),
            <[Felt; 4]>::from(*asset)
        );
    }
}
