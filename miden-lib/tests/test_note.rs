pub mod common;
use common::{
    data::mock_inputs, prepare_transaction, procedures::prepare_word, run_tx, Felt,
    MemAdviceProvider,
};

#[test]
fn test_get_sender_no_sender() {
    let (account, block_header, chain, notes) = mock_inputs();

    // calling get_sender should return sender
    let code = "
        use.miden::sat::internal::prologue
        use.miden::sat::internal::note_setup
        use.miden::sat::note

        begin
            exec.prologue::prepare_transaction
            exec.note::get_sender
        end
        ";

    let transaction =
        prepare_transaction(account, block_header, chain, notes, &code, "", None, None);

    let process = run_tx(
        transaction.tx_program().clone(),
        transaction.stack_inputs(),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    );
    assert!(process.is_err());
}

#[test]
fn test_get_sender() {
    let (account, block_header, chain, notes) = mock_inputs();

    // calling get_sender should return sender
    let code = "
        use.miden::sat::internal::prologue
        use.miden::sat::internal::note_setup
        use.miden::sat::note

        begin
            exec.prologue::prepare_transaction
            exec.note_setup::prepare_note
            dropw dropw dropw dropw
            exec.note::get_sender
        end
        ";

    let transaction =
        prepare_transaction(account, block_header, chain, notes, &code, "", None, None);

    let process = run_tx(
        transaction.tx_program().clone(),
        transaction.stack_inputs(),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();

    let sender = transaction.consumed_notes().notes()[0].metadata().sender().into();
    assert_eq!(process.stack.get(0), sender);
}

#[test]
fn test_get_vault_data() {
    let (account, block_header, chain, notes) = mock_inputs();

    // calling get_vault_data should return vault data
    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::internal::note_setup
        use.miden::sat::internal::note
        use.miden::sat::internal::layout

        begin
            exec.prologue::prepare_transaction

            # prepare note 0
            exec.note_setup::prepare_note

            # drop the note inputs
            dropw dropw dropw dropw

            # get the vault data
            exec.note::get_vault_data

            # assert the vault data is correct
            push.{note_0_vault_root} assert_eqw
            push.{note_0_num_assets} assert_eq

            # prepare note 1
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

    let transaction =
        prepare_transaction(account, block_header, chain, notes, &code, "", None, None);

    // run to ensure success
    let _process = run_tx(
        transaction.tx_program().clone(),
        transaction.stack_inputs(),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();
}

#[test]
fn test_get_assets() {
    let (account, block_header, chain, notes) = mock_inputs();

    const DEST_POINTER_NOTE_0: u32 = 100000000;
    const DEST_POINTER_NOTE_1: u32 = 200000000;

    // calling get_assets should return assets at the specified address
    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::internal::note_setup
        use.miden::sat::note

        begin
            # prepare tx
            exec.prologue::prepare_transaction

            # prepare note 0
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

    let inputs =
        prepare_transaction(account, block_header, chain, notes.clone(), &code, "", None, None);

    let process = run_tx(
        inputs.tx_program().clone(),
        inputs.stack_inputs(),
        MemAdviceProvider::from(inputs.advice_provider_inputs()),
    )
    .unwrap();

    // check the assets saved to memory for note 0 are correct
    for (asset, idx) in notes[0].vault().iter().zip(0u32..) {
        assert_eq!(
            process.get_memory_value(0, DEST_POINTER_NOTE_0 + idx).unwrap(),
            <[Felt; 4]>::from(*asset)
        );
    }

    // check the assets saved to memory for note 1 are correct
    for (asset, idx) in notes[1].vault().iter().zip(0u32..) {
        assert_eq!(
            process.get_memory_value(0, DEST_POINTER_NOTE_1 + idx).unwrap(),
            <[Felt; 4]>::from(*asset)
        );
    }
}
