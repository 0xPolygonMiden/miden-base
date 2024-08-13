use alloc::vec::Vec;

use miden_lib::transaction::{
    memory::{NOTE_MEM_SIZE, OUTPUT_NOTE_ASSET_HASH_OFFSET, OUTPUT_NOTE_SECTION_OFFSET},
    TransactionKernel,
};
use miden_objects::{
    accounts::Account,
    transaction::{OutputNote, OutputNotes},
};
use vm_processor::ONE;

use super::{output_notes_data_procedure, ZERO};
use crate::{testing::TransactionContextBuilder, tests::kernel_tests::read_root_mem_value};

#[test]
fn test_epilogue() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved()
        .build();

    let output_notes_data_procedure =
        output_notes_data_procedure(tx_context.expected_output_notes());

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::account
        use.miden::kernels::tx::epilogue

        {output_notes_data_procedure}

        begin
            exec.prologue::prepare_transaction

            exec.create_mock_notes

            push.1
            exec.account::incr_nonce

            exec.epilogue::finalize_transaction
        end
        "
    );

    let process = tx_context.execute_code(&code).unwrap();

    let final_account = Account::mock(
        tx_context.account().id().into(),
        tx_context.account().nonce() + ONE,
        TransactionKernel::assembler(),
    );

    let output_notes = OutputNotes::new(
        tx_context
            .expected_output_notes()
            .iter()
            .cloned()
            .map(OutputNote::Full)
            .collect(),
    )
    .unwrap();

    let mut expected_stack = Vec::with_capacity(16);
    expected_stack.extend(output_notes.commitment().as_elements().iter().rev());
    expected_stack.extend(final_account.hash().as_elements().iter().rev());
    expected_stack.extend((8..16).map(|_| ZERO));

    assert_eq!(
        process.stack.build_stack_outputs().stack(),
        &expected_stack,
        "Stack state after finalize_transaction does not contain the expected values"
    );

    assert_eq!(
        process.stack.depth(),
        16,
        "The stack must be truncated to 16 elements after finalize_transaction"
    );
}

#[test]
fn test_compute_output_note_id() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved()
        .build();

    let output_notes_data_procedure =
        output_notes_data_procedure(tx_context.expected_output_notes());

    for (note, i) in tx_context.expected_output_notes().iter().zip(0u32..) {
        let code = format!(
            "
            use.miden::kernels::tx::prologue
            use.miden::kernels::tx::epilogue

            {output_notes_data_procedure}

            begin
                exec.prologue::prepare_transaction
                exec.create_mock_notes
                exec.epilogue::finalize_transaction
            end
            "
        );

        let process = tx_context.execute_code(&code).unwrap();

        assert_eq!(
            note.assets().commitment().as_elements(),
            read_root_mem_value(
                &process,
                OUTPUT_NOTE_SECTION_OFFSET + i * NOTE_MEM_SIZE + OUTPUT_NOTE_ASSET_HASH_OFFSET
            ),
            "ASSET_HASH didn't match expected value",
        );

        assert_eq!(
            note.id().as_elements(),
            &read_root_mem_value(&process, OUTPUT_NOTE_SECTION_OFFSET + i * NOTE_MEM_SIZE),
            "NOTE_ID didn't match expected value",
        );
    }
}

#[test]
fn test_epilogue_asset_preservation_violation_too_few_input() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_too_few_input()
        .build();

    let output_notes_data_procedure =
        output_notes_data_procedure(tx_context.expected_output_notes());

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::account
        use.miden::kernels::tx::epilogue

        {output_notes_data_procedure}

        begin
            exec.prologue::prepare_transaction
            exec.create_mock_notes
            push.1
            exec.account::incr_nonce
            exec.epilogue::finalize_transaction
        end
        "
    );

    let process = tx_context.execute_code(&code);
    assert!(process.is_err(), "Violating asset preservation must result in a failure");
}

#[test]
fn test_epilogue_asset_preservation_violation_too_many_fungible_input() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_too_many_fungible_input()
        .build();

    let output_notes_data_procedure =
        output_notes_data_procedure(tx_context.expected_output_notes());

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::account
        use.miden::kernels::tx::epilogue

        {output_notes_data_procedure}

        begin
            exec.prologue::prepare_transaction
            exec.create_mock_notes
            push.1
            exec.account::incr_nonce
            exec.epilogue::finalize_transaction
        end
        "
    );

    let process = tx_context.execute_code(&code);
    assert!(process.is_err(), "Violating asset preservation must result in a failure");
}

#[test]
fn test_epilogue_increment_nonce_success() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved()
        .build();

    let output_notes_data_procedure =
        output_notes_data_procedure(tx_context.expected_output_notes());

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::account
        use.miden::kernels::tx::epilogue

        {output_notes_data_procedure}

        begin
            exec.prologue::prepare_transaction

            exec.create_mock_notes

            push.1.2.3.4
            push.0
            exec.account::set_item
            dropw

            push.1
            exec.account::incr_nonce

            exec.epilogue::finalize_transaction
        end
        "
    );

    let process = tx_context.execute_code(&code);
    assert!(process.is_ok(), "Calling incr_nonce should succeed");
}

#[test]
fn test_epilogue_increment_nonce_violation() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved()
        .build();

    let output_notes_data_procedure =
        output_notes_data_procedure(tx_context.expected_output_notes());

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::account
        use.miden::kernels::tx::epilogue

        {output_notes_data_procedure}

        begin
            exec.prologue::prepare_transaction

            exec.create_mock_notes

            push.1.2.3.4
            push.0
            exec.account::set_item
            dropw

            exec.epilogue::finalize_transaction
        end
        "
    );

    let process = tx_context.execute_code(&code);
    assert!(
        process.is_err(),
        "Not incrementing the nonce when the state changes must be an error",
    );
}
