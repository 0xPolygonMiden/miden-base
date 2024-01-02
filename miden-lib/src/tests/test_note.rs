use miden_objects::{notes::Note, transaction::PreparedTransaction, WORD_SIZE};
use mock::{
    consumed_note_data_ptr,
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    prepare_transaction,
    procedures::prepare_word,
    run_tx,
};

use super::{
    build_tx_inputs, AdviceProvider, ContextId, DefaultHost, Felt, Process, ProcessState, ZERO,
};
use crate::transaction::memory::CURRENT_CONSUMED_NOTE_PTR;

#[test]
fn test_get_sender_no_sender() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    // calling get_sender should return sender
    let code = "
        use.miden::sat::internal::layout
        use.miden::sat::internal::prologue
        use.miden::sat::note

        begin
            exec.prologue::prepare_transaction

            # force the current consumed note pointer to 0
            push.0 exec.layout::set_current_consumed_note_ptr

            # get the sender
            exec.note::get_sender
        end
        ";

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
}

#[test]
fn test_get_sender() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    // calling get_sender should return sender
    let code = "
        use.miden::sat::internal::prologue
        use.miden::sat::internal::note->note_internal
        use.miden::sat::note

        begin
            exec.prologue::prepare_transaction
            exec.note_internal::prepare_note
            dropw dropw dropw dropw
            exec.note::get_sender
        end
        ";

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider).unwrap();

    let sender = transaction.input_notes().get_note(0).note().metadata().sender().into();
    assert_eq!(process.stack.get(0), sender);
}

#[test]
fn test_get_vault_data() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    // calling get_vault_info should return vault info
    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::internal::note

        begin
            exec.prologue::prepare_transaction

            # prepare note 0
            exec.note::prepare_note

            # get the vault data
            exec.note::get_vault_info

            # assert the vault data is correct
            push.{note_0_vault_root} assert_eqw
            push.{note_0_num_assets} assert_eq

            # increment current consumed note pointer
            exec.note::increment_current_consumed_note_ptr

            # prepare note 1
            exec.note::prepare_note

            # get the vault data
            exec.note::get_vault_info

            # assert the vault data is correct
            push.{note_1_vault_root} assert_eqw
            push.{note_1_num_assets} assert_eq
        end
        ",
        note_0_vault_root = prepare_word(&notes[0].note().vault().hash()),
        note_0_num_assets = notes[0].note().vault().num_assets(),
        note_1_vault_root = prepare_word(&notes[1].note().vault().hash()),
        note_1_num_assets = notes[1].note().vault().num_assets(),
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let _process = run_tx(program, stack_inputs, advice_provider).unwrap();
}

#[test]
fn test_get_assets() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    const DEST_POINTER_NOTE_0: u32 = 100000000;
    const DEST_POINTER_NOTE_1: u32 = 200000000;

    fn construct_asset_assertions(note: &Note) -> String {
        let mut code = String::new();
        for asset in note.vault().iter() {
            code += &format!(
                "
                # assert the asset is correct
                dup padw movup.4 mem_loadw push.{asset} assert_eqw push.1 add
                ",
                asset = prepare_word(&<[Felt; 4]>::from(*asset))
            );
        }
        code
    }

    // calling get_assets should return assets at the specified address
    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::internal::note->note_internal
        use.miden::sat::note

        proc.process_note_0
            # drop the note inputs
            dropw dropw dropw dropw

            # set the destination pointer for note 0 assets
            push.{DEST_POINTER_NOTE_0}

            # get the assets
            exec.note::get_assets

            # assert the number of assets is correct
            eq.{note_0_num_assets} assert

            # assert the pointer is returned
            dup eq.{DEST_POINTER_NOTE_0} assert

            # asset memory assertions
            {NOTE_0_ASSET_ASSERTIONS}

            # clean pointer
            drop
        end

        proc.process_note_1
            # drop the note inputs
            dropw dropw dropw dropw

            # set the destination pointer for note 1 assets
            push.{DEST_POINTER_NOTE_1}

            # get the assets
            exec.note::get_assets

            # assert the number of assets is correct
            eq.{note_1_num_assets} assert

            # assert the pointer is returned
            dup eq.{DEST_POINTER_NOTE_1} assert

            # asset memory assertions
            {NOTE_1_ASSET_ASSERTIONS}

            # clean pointer
            drop
        end

        begin
            # prepare tx
            exec.prologue::prepare_transaction

            # prepare note 0
            exec.note_internal::prepare_note

            # process note 0
            call.process_note_0

            # increment current consumed note pointer
            exec.note_internal::increment_current_consumed_note_ptr

            # prepare note 1
            exec.note_internal::prepare_note

            # process note 1
            call.process_note_1
        end
        ",
        note_0_num_assets = notes[0].note().vault().num_assets(),
        note_1_num_assets = notes[1].note().vault().num_assets(),
        NOTE_0_ASSET_ASSERTIONS = construct_asset_assertions(notes[0].note()),
        NOTE_1_ASSET_ASSERTIONS = construct_asset_assertions(notes[1].note()),
    );

    let transaction = prepare_transaction(
        account,
        None,
        block_header,
        chain,
        notes.clone(),
        None,
        &code,
        "",
        None,
    );
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let _process = run_tx(program, stack_inputs, advice_provider).unwrap();
}

#[test]
fn test_get_inputs() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    const DEST_POINTER_NOTE_0: u32 = 100000000;

    fn construct_input_assertions(note: &Note) -> String {
        let mut code = String::new();
        for input_word in note.inputs().inputs().chunks(WORD_SIZE) {
            code += &format!(
                "
                # assert the asset is correct
                dup padw movup.4 mem_loadw push.{input_word} assert_eqw push.1 add
                ",
                input_word = prepare_word(input_word.try_into().unwrap())
            );
        }
        code
    }

    // calling get_assets should return assets at the specified address
    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::internal::note->note_internal
        use.miden::sat::note

        proc.process_note_0
            # drop the note inputs
            dropw

            # set the destination pointer for note 0 assets
            push.{DEST_POINTER_NOTE_0}

            # get the assets
            exec.note::get_inputs

            # assert the correct pointer is returned
            dup eq.{DEST_POINTER_NOTE_0} assert

            # apply note 1 input assertions
            {NOTE_1_INPUT_ASSERTIONS}

            # clean the pointer
            drop
        end

        begin
            # prepare tx
            exec.prologue::prepare_transaction

            # prepare note 0
            exec.note_internal::prepare_note

            # process note 0
            call.process_note_0
        end
        ",
        NOTE_1_INPUT_ASSERTIONS = construct_input_assertions(notes[0].note()),
    );

    let transaction = prepare_transaction(
        account,
        None,
        block_header,
        chain,
        notes.clone(),
        None,
        &code,
        "",
        None,
    );
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let _process = run_tx(program, stack_inputs, advice_provider).unwrap();
}

#[test]
fn test_note_setup() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let code = "
        use.miden::sat::internal::prologue
        use.miden::sat::internal::note

        begin
            exec.prologue::prepare_transaction
            exec.note::prepare_note
        end
        ";

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider).unwrap();

    note_setup_stack_assertions(&process, &transaction);
    note_setup_memory_assertions(&process);
}

fn note_setup_stack_assertions<A: AdviceProvider>(
    process: &Process<DefaultHost<A>>,
    inputs: &PreparedTransaction,
) {
    let mut expected_stack = [ZERO; 16];

    // replace the top four elements with the tx script root
    let mut note_script_root = *inputs.input_notes().get_note(0).note().script().hash();
    note_script_root.reverse();
    expected_stack[..4].copy_from_slice(&note_script_root);

    // assert that the stack contains the note inputs at the end of execution
    assert_eq!(process.stack.trace_state(), expected_stack)
}

fn note_setup_memory_assertions<A: AdviceProvider>(process: &Process<DefaultHost<A>>) {
    // assert that the correct pointer is stored in bookkeeping memory
    assert_eq!(
        process.get_mem_value(ContextId::root(), CURRENT_CONSUMED_NOTE_PTR).unwrap()[0],
        Felt::from(consumed_note_data_ptr(0))
    );
}
