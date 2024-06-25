use alloc::{string::String, vec::Vec};

use miden_lib::transaction::{
    memory::{CREATED_NOTE_ASSET_HASH_OFFSET, CREATED_NOTE_SECTION_OFFSET, NOTE_MEM_SIZE},
    ToTransactionKernelInputs,
};
use miden_objects::testing::notes::AssetPreservationStatus;

use super::{
    build_module_path, output_notes_data_procedure, MemAdviceProvider, TX_KERNEL_DIR, ZERO,
};
use crate::{
    testing::{executor::CodeExecutor, utils::mock_executed_tx},
    tests::kernel_tests::read_root_mem_value,
};

const EPILOGUE_FILE: &str = "epilogue.masm";

/// Loads epilogue file and returns the complete code formatted as
/// "{imports}{epilogue_code}{code}"`
#[cfg(feature = "std")]
fn insert_epilogue(imports: &str, code: &str) -> String {
    let assembly_file = build_module_path(TX_KERNEL_DIR, EPILOGUE_FILE);
    use std::fs::File;

    let mut module = String::new();
    std::io::Read::read_to_string(&mut File::open(assembly_file).unwrap(), &mut module).unwrap();
    let complete_code = format!("{imports}{module}{code}");

    // This hack is going around issue #686 on miden-vm
    complete_code.replace("export", "proc")
}

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
    let code = insert_epilogue(imports, &code);

    let process = CodeExecutor::with_advice_provider(MemAdviceProvider::from(advice_inputs))
        .stack_inputs(stack_inputs)
        .run(&code)
        .unwrap();

    let mut expected_stack = Vec::with_capacity(16);
    expected_stack
        .extend(executed_transaction.output_notes().commitment().as_elements().iter().rev());
    expected_stack.extend(executed_transaction.final_account().hash().as_elements().iter().rev());
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
        let test = insert_epilogue(imports, &test);

        let process = CodeExecutor::with_advice_provider(MemAdviceProvider::from(advice_inputs))
            .stack_inputs(stack_inputs)
            .run(&test)
            .unwrap();

        let expected_asset_hash =
            note.assets().expect("Output note should be full note").commitment();
        let asset_hash_memory_address =
            CREATED_NOTE_SECTION_OFFSET + i * NOTE_MEM_SIZE + CREATED_NOTE_ASSET_HASH_OFFSET;
        let actual_asset_hash = read_root_mem_value(&process, asset_hash_memory_address);
        assert_eq!(
            expected_asset_hash.as_elements(),
            actual_asset_hash,
            "Asset hash didn't match expected value"
        );

        let expected_id = note.id();
        let note_id_memory_address = CREATED_NOTE_SECTION_OFFSET + i * NOTE_MEM_SIZE;
        let actual_note_id = read_root_mem_value(&process, note_id_memory_address);
        assert_eq!(
            &actual_note_id,
            expected_id.as_elements(),
            "note id didn't match expected value"
        );
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
                push.1
                exec.account::incr_nonce
                exec.finalize_transaction
            end
            "
        );

        let (stack_inputs, advice_inputs) = executed_transaction.get_kernel_inputs();
        let code = insert_epilogue(imports, &code);

        let process = CodeExecutor::with_advice_provider(MemAdviceProvider::from(advice_inputs))
            .stack_inputs(stack_inputs)
            .run(&code);

        assert!(process.is_err(), "Violating asset preservation must result in a failure");
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

            push.1.2.3.4
            push.0
            exec.account::set_item
            dropw

            push.1
            exec.account::incr_nonce

            exec.finalize_transaction
        end
        "
    );

    let (stack_inputs, advice_inputs) = executed_transaction.get_kernel_inputs();
    let code = insert_epilogue(imports, &code);

    let _process = CodeExecutor::with_advice_provider(MemAdviceProvider::from(advice_inputs))
        .stack_inputs(stack_inputs)
        .run(&code)
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

            push.1.2.3.4
            push.0
            exec.account::set_item
            dropw

            exec.finalize_transaction
        end
        "
    );

    let (stack_inputs, advice_inputs) = executed_transaction.get_kernel_inputs();
    let code = insert_epilogue(imports, &code);

    let process = CodeExecutor::with_advice_provider(MemAdviceProvider::from(advice_inputs))
        .stack_inputs(stack_inputs)
        .run(&code);

    assert!(process.is_err());
}
