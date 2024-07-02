use alloc::{collections::BTreeMap, string::String};

use miden_lib::transaction::memory::CURRENT_CONSUMED_NOTE_PTR;
use miden_objects::{
    accounts::account_id::testing::ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
    notes::Note, testing::prepare_word, transaction::TransactionArgs, WORD_SIZE,
};
use vm_processor::{EMPTY_WORD, ONE};

use super::{Felt, Process, ZERO};
use crate::{
    testing::{
        utils::consumed_note_data_ptr, MockHost, TransactionContext, TransactionContextBuilder,
    },
    tests::kernel_tests::read_root_mem_value,
};

#[test]
fn test_get_sender_no_sender() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .build();
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

    let process = tx_context.execute_code(code);

    assert!(process.is_err());
}

#[test]
fn test_get_sender() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .with_mock_notes_preserved()
    .build();

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

    let process = tx_context.execute_code(code).unwrap();

    let sender = tx_context.input_notes().get_note(0).note().metadata().sender().into();
    assert_eq!(process.stack.get(0), sender);
}

#[test]
fn test_get_vault_data() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .with_mock_notes_preserved()
    .build();

    let notes = tx_context.input_notes();

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

    tx_context.execute_code(&code).unwrap();
}
#[test]
fn test_get_assets() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .with_mock_notes_preserved()
    .build();

    let notes = tx_context.input_notes();

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

    tx_context.execute_code(&code).unwrap();
}

#[test]
fn test_get_inputs() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .with_mock_notes_preserved()
    .build();

    let notes = tx_context.mock_chain().available_notes();

    fn construct_input_assertions(note: &Note) -> String {
        let mut code = String::new();
        for input_chunk in note.inputs().values().chunks(WORD_SIZE) {
            let mut input_word = EMPTY_WORD;
            input_word[..input_chunk.len()].copy_from_slice(input_chunk);

            code += &format!(
                "
                # assert the input is correct
                dup padw movup.4 mem_loadw push.{input_word} assert_eqw push.1 add
                ",
                input_word = prepare_word(&input_word)
            );
        }
        code
    }

    let note0 = notes[0].note();

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::note->note_internal
        use.miden::note

        begin
            # => [BH, acct_id, IAH, NC]
            exec.prologue::prepare_transaction
            # => []

            exec.note_internal::prepare_note
            # => [NOTE_SCRIPT_ROOT, NOTE_ARGS]

            # drop the note inputs
            dropw dropw
            # => []

            push.{NOTE_0_PTR} exec.note::get_inputs
            # => [num_inputs, dest_ptr]

            eq.{num_inputs} assert
            # => [dest_ptr]

            dup eq.{NOTE_0_PTR} assert
            # => [dest_ptr]

            # apply note 1 input assertions
            {input_assertions}
            # => [dest_ptr]

            # clean the pointer
            drop
            # => []
        end
        ",
        num_inputs = note0.inputs().num_values(),
        input_assertions = construct_input_assertions(note0),
        NOTE_0_PTR = 100000000,
    );

    tx_context.execute_code(&code).unwrap();
}

#[test]
fn test_note_setup() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .with_mock_notes_preserved()
    .build();

    let code = "
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::note

        begin
            exec.prologue::prepare_transaction
            exec.note::prepare_note
        end
        ";

    let process = tx_context.execute_code(code).unwrap();

    note_setup_stack_assertions(&process, &tx_context);
    note_setup_memory_assertions(&process);
}

#[test]
fn test_note_script_and_note_args() {
    let note_args = [
        [Felt::new(91), Felt::new(91), Felt::new(91), Felt::new(91)],
        [Felt::new(92), Felt::new(92), Felt::new(92), Felt::new(92)],
    ];

    let mut tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .with_mock_notes_preserved()
    .build();

    let code = "
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::memory
        use.miden::kernels::tx::note

        begin
            exec.prologue::prepare_transaction
            exec.memory::get_total_num_consumed_notes push.2 assert_eq
            exec.note::prepare_note dropw
            exec.note::increment_current_consumed_note_ptr drop
            exec.note::prepare_note dropw
        end
        ";

    let note_args_map = BTreeMap::from([
        (tx_context.input_notes().get_note(0).note().id(), note_args[1]),
        (tx_context.input_notes().get_note(1).note().id(), note_args[0]),
    ]);

    let tx_args =
        TransactionArgs::new(None, Some(note_args_map), tx_context.tx_args().advice_map().clone());

    tx_context.set_tx_args(tx_args);
    let process = tx_context.execute_code(code).unwrap();

    assert_eq!(process.stack.get_word(0), note_args[0]);

    assert_eq!(process.stack.get_word(1), note_args[1]);
}

fn note_setup_stack_assertions(process: &Process<MockHost>, inputs: &TransactionContext) {
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
        read_root_mem_value(process, CURRENT_CONSUMED_NOTE_PTR)[0],
        Felt::from(consumed_note_data_ptr(0))
    );
}

#[test]
fn test_get_note_serial_number() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .with_mock_notes_preserved()
    .build();

    // calling get_serial_number should return the serial number of the note
    let code = "
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::note->note_internal
        use.miden::note

        begin
            exec.prologue::prepare_transaction
            exec.note_internal::prepare_note
            dropw dropw dropw dropw
            exec.note::get_serial_number
        end
        ";

    let process = tx_context.execute_code(code).unwrap();

    let serial_number = tx_context.input_notes().get_note(0).note().serial_num();
    assert_eq!(process.stack.get_word(0), serial_number);
}
