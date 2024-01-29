use common::{build_module_path, TX_KERNEL_DIR};
use miden_lib::transaction::{
    memory::{CREATED_NOTE_ASSET_HASH_OFFSET, CREATED_NOTE_SECTION_OFFSET, NOTE_MEM_SIZE},
    ToTransactionKernelInputs, FINAL_ACCOUNT_HASH_WORD_IDX, OUTPUT_NOTES_COMMITMENT_WORD_IDX,
    TX_SCRIPT_ROOT_WORD_IDX,
};
use mock::{
    mock::{notes::AssetPreservationStatus, transaction::mock_executed_tx},
    procedures::output_notes_data_procedure,
    run_within_tx_kernel,
};
use vm_processor::{ContextId, MemAdviceProvider, ProcessState, Word, ZERO};

mod common;

const EPILOGUE_FILE: &str = "epilogue.masm";

#[test]
fn test_epilogue() {
    let executed_transaction = mock_executed_tx(AssetPreservationStatus::Preserved);

    let output_notes_data_procedure =
        output_notes_data_procedure(executed_transaction.output_notes());

    let imports = "use.miden::kernels::tx::prologue\n";
    let code = format!(
        "
        {output_notes_data_procedure}
        begin
            exec.prologue::prepare_transaction
            exec.create_mock_notes
            push.1 exec.account::incr_nonce
            exec.finalize_transaction
        end
        "
    );

    let (stack_inputs, advice_inputs) = executed_transaction.get_kernel_inputs();
    let assembly_file = build_module_path(TX_KERNEL_DIR, EPILOGUE_FILE);
    let process = run_within_tx_kernel(
        imports,
        &code,
        stack_inputs,
        MemAdviceProvider::from(advice_inputs),
        Some(assembly_file),
    )
    .unwrap();

    // assert tx script root is correct
    assert_eq!(
        process.stack.get_word(TX_SCRIPT_ROOT_WORD_IDX),
        executed_transaction
            .tx_script()
            .as_ref()
            .map_or_else(Word::default, |s| **s.hash())
    );

    // assert created notes commitment is correct
    assert_eq!(
        process.stack.get_word(OUTPUT_NOTES_COMMITMENT_WORD_IDX),
        executed_transaction.output_notes().commitment().as_elements()
    );

    // assert final account hash is correct
    assert_eq!(
        process.stack.get_word(FINAL_ACCOUNT_HASH_WORD_IDX),
        executed_transaction.final_account().hash().as_elements(),
    );

    // assert stack has been truncated correctly
    assert_eq!(process.stack.depth(), 16);

    // assert the bottom of the stack is filled with zeros
    for i in 12..16 {
        assert_eq!(process.stack.get(i), ZERO);
    }
}

#[test]
fn test_compute_created_note_id() {
    let executed_transaction = mock_executed_tx(AssetPreservationStatus::Preserved);

    let output_notes_data_procedure =
        output_notes_data_procedure(executed_transaction.output_notes());

    for (note, i) in executed_transaction.output_notes().iter().zip(0u32..) {
        let imports = "use.miden::kernels::tx::prologue\n";
        let test = format!(
            "
        {output_notes_data_procedure}
        begin
            exec.prologue::prepare_transaction
            exec.create_mock_notes
            exec.finalize_transaction
        end
        "
        );

        let (stack_inputs, advice_inputs) = executed_transaction.get_kernel_inputs();
        let assembly_file = build_module_path(TX_KERNEL_DIR, EPILOGUE_FILE);
        let process = run_within_tx_kernel(
            imports,
            &test,
            stack_inputs,
            MemAdviceProvider::from(advice_inputs),
            Some(assembly_file),
        )
        .unwrap();

        // assert the note asset hash is correct
        let expected_asset_hash = note.assets().commitment();
        let asset_hash_memory_address =
            CREATED_NOTE_SECTION_OFFSET + i * NOTE_MEM_SIZE + CREATED_NOTE_ASSET_HASH_OFFSET;
        let actual_asset_hash =
            process.get_mem_value(ContextId::root(), asset_hash_memory_address).unwrap();
        assert_eq!(expected_asset_hash.as_elements(), actual_asset_hash);

        // assert the note ID is correct
        let expected_id = note.id();
        let note_id_memory_address = CREATED_NOTE_SECTION_OFFSET + i * NOTE_MEM_SIZE;
        let actual_note_id =
            process.get_mem_value(ContextId::root(), note_id_memory_address).unwrap();
        assert_eq!(&actual_note_id, expected_id.as_elements());
    }
}

#[test]
fn test_epilogue_asset_preservation_violation() {
    for asset_preservation in [
        AssetPreservationStatus::TooFewInput,
        AssetPreservationStatus::TooManyFungibleInput,
    ] {
        let executed_transaction = mock_executed_tx(asset_preservation);

        let output_notes_data_procedure =
            output_notes_data_procedure(executed_transaction.output_notes());

        let imports = "use.miden::kernels::tx::prologue\n";
        let code = format!(
            "
        {output_notes_data_procedure}
        begin
            exec.prologue::prepare_transaction
            exec.create_mock_notes
            push.1 exec.account::incr_nonce
            exec.finalize_transaction
        end
        "
        );

        let (stack_inputs, advice_inputs) = executed_transaction.get_kernel_inputs();
        let assembly_file = build_module_path(TX_KERNEL_DIR, EPILOGUE_FILE);
        let process = run_within_tx_kernel(
            imports,
            &code,
            stack_inputs,
            MemAdviceProvider::from(advice_inputs),
            Some(assembly_file),
        );

        // assert the process results in error
        assert!(process.is_err());
    }
}

#[test]
fn test_epilogue_increment_nonce_success() {
    let executed_transaction = mock_executed_tx(AssetPreservationStatus::Preserved);

    let output_notes_data_procedure =
        output_notes_data_procedure(executed_transaction.output_notes());

    let imports = "use.miden::kernels::tx::prologue\n";
    let code = format!(
        "
        {output_notes_data_procedure}
        begin
            exec.prologue::prepare_transaction
            exec.create_mock_notes
            push.1.2.3.4 push.0 exec.account::set_item dropw
            push.1 exec.account::incr_nonce
            exec.finalize_transaction
        end
        "
    );

    let (stack_inputs, advice_inputs) = executed_transaction.get_kernel_inputs();
    let assembly_file = build_module_path(TX_KERNEL_DIR, EPILOGUE_FILE);
    let _process = run_within_tx_kernel(
        imports,
        &code,
        stack_inputs,
        MemAdviceProvider::from(advice_inputs),
        Some(assembly_file),
    )
    .unwrap();
}

#[test]
fn test_epilogue_increment_nonce_violation() {
    let executed_transaction = mock_executed_tx(AssetPreservationStatus::Preserved);

    let output_notes_data_procedure =
        output_notes_data_procedure(executed_transaction.output_notes());

    let imports = "use.miden::kernels::tx::prologue\n";
    let code = format!(
        "
        {output_notes_data_procedure}
        begin
            exec.prologue::prepare_transaction
            exec.create_mock_notes
            push.1.2.3.4 push.0 exec.account::set_item dropw
            exec.finalize_transaction
        end
        "
    );

    let (stack_inputs, advice_inputs) = executed_transaction.get_kernel_inputs();
    let assembly_file = build_module_path(TX_KERNEL_DIR, EPILOGUE_FILE);
    let process = run_within_tx_kernel(
        imports,
        &code,
        stack_inputs,
        MemAdviceProvider::from(advice_inputs),
        Some(assembly_file),
    );

    assert!(process.is_err());
}
