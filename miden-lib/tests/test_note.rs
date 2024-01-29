use miden_lib::transaction::memory::CURRENT_CONSUMED_NOTE_PTR;
use miden_objects::{notes::Note, transaction::PreparedTransaction, Felt, WORD_SIZE, ZERO};
use mock::{
    consumed_note_data_ptr,
    mock::{
        account::MockAccountType, host::MockHost, notes::AssetPreservationStatus,
        transaction::mock_inputs,
    },
    prepare_transaction,
    procedures::prepare_word,
    run_tx,
};
use vm_processor::{ContextId, Process, ProcessState};

#[test]
fn test_get_sender_no_sender() {
    let tx_inputs =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    // calling get_sender should return sender
    let code = "
        use.miden::kernels::tx::memory
        use.miden::kernels::tx::prologue
        use.miden::note

        begin
            exec.prologue::prepare_transaction

            # force the current consumed note pointer to 0
            push.0 exec.memory::set_current_consumed_note_ptr

            # get the sender
            exec.note::get_sender
        end
        ";

    let transaction = prepare_transaction(tx_inputs, None, code, None);
    let process = run_tx(&transaction);

    assert!(process.is_err());
}

#[test]
fn test_get_sender() {
    let tx_inputs =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    // calling get_sender should return sender
    let code = "
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::note->note_internal
        use.miden::note

        begin
            exec.prologue::prepare_transaction
            exec.note_internal::prepare_note
            dropw dropw dropw dropw
            exec.note::get_sender
        end
        ";

    let transaction = prepare_transaction(tx_inputs, None, code, None);
    let process = run_tx(&transaction).unwrap();

    let sender = transaction.input_notes().get_note(0).note().metadata().sender().into();
    assert_eq!(process.stack.get(0), sender);
}

#[test]
fn test_get_vault_data() {
    let tx_inputs =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let notes = tx_inputs.input_notes();

    // calling get_vault_info should return vault info
    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::note

        begin
            exec.prologue::prepare_transaction

            # prepare note 0
            exec.note::prepare_note

            # get the vault data
            exec.note::get_vault_info

            # assert the vault data is correct
            push.{note_0_asset_hash} assert_eqw
            push.{note_0_num_assets} assert_eq

            # increment current consumed note pointer
            exec.note::increment_current_consumed_note_ptr

            # prepare note 1
            exec.note::prepare_note

            # get the vault data
            exec.note::get_vault_info

            # assert the vault data is correct
            push.{note_1_asset_hash} assert_eqw
            push.{note_1_num_assets} assert_eq
        end
        ",
        note_0_asset_hash = prepare_word(&notes.get_note(0).note().assets().commitment()),
        note_0_num_assets = notes.get_note(0).note().assets().num_assets(),
        note_1_asset_hash = prepare_word(&notes.get_note(1).note().assets().commitment()),
        note_1_num_assets = notes.get_note(1).note().assets().num_assets(),
    );

    let transaction = prepare_transaction(tx_inputs, None, &code, None);
    let _process = run_tx(&transaction).unwrap();
}

#[test]
fn test_get_assets() {
    let tx_inputs =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let notes = tx_inputs.input_notes();

    const DEST_POINTER_NOTE_0: u32 = 100000000;
    const DEST_POINTER_NOTE_1: u32 = 200000000;

    fn construct_asset_assertions(note: &Note) -> String {
        let mut code = String::new();
        for asset in note.assets().iter() {
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
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::note->note_internal
        use.miden::note

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
        note_0_num_assets = notes.get_note(0).note().assets().num_assets(),
        note_1_num_assets = notes.get_note(1).note().assets().num_assets(),
        NOTE_0_ASSET_ASSERTIONS = construct_asset_assertions(notes.get_note(0).note()),
        NOTE_1_ASSET_ASSERTIONS = construct_asset_assertions(notes.get_note(1).note()),
    );

    let transaction = prepare_transaction(tx_inputs, None, &code, None);
    let _process = run_tx(&transaction).unwrap();
}

#[test]
fn test_get_inputs() {
    let tx_inputs =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let notes = tx_inputs.input_notes();

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
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::note->note_internal
        use.miden::note

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
        NOTE_1_INPUT_ASSERTIONS = construct_input_assertions(notes.get_note(0).note()),
    );

    let transaction = prepare_transaction(tx_inputs, None, &code, None);
    let _process = run_tx(&transaction).unwrap();
}

#[test]
fn test_note_setup() {
    let tx_inputs =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let code = "
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::note

        begin
            exec.prologue::prepare_transaction
            exec.note::prepare_note
        end
        ";

    let transaction = prepare_transaction(tx_inputs, None, code, None);
    let process = run_tx(&transaction).unwrap();

    note_setup_stack_assertions(&process, &transaction);
    note_setup_memory_assertions(&process);
}

fn note_setup_stack_assertions(process: &Process<MockHost>, inputs: &PreparedTransaction) {
    let mut expected_stack = [ZERO; 16];

    // replace the top four elements with the tx script root
    let mut note_script_root = *inputs.input_notes().get_note(0).note().script().hash();
    note_script_root.reverse();
    expected_stack[..4].copy_from_slice(&note_script_root);

    // assert that the stack contains the note inputs at the end of execution
    assert_eq!(process.stack.trace_state(), expected_stack)
}

fn note_setup_memory_assertions(process: &Process<MockHost>) {
    // assert that the correct pointer is stored in bookkeeping memory
    assert_eq!(
        process.get_mem_value(ContextId::root(), CURRENT_CONSUMED_NOTE_PTR).unwrap()[0],
        Felt::from(consumed_note_data_ptr(0))
    );
}
