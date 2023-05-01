pub mod common;
use common::{
    consumed_note_data_ptr, data::mock_inputs, memory::CURRENT_CONSUMED_NOTE_PTR,
    run_within_tx_kernel, AdviceProvider, Felt, FieldElement, MemAdviceProvider, Process,
    TX_KERNEL_DIR,
};
use miden_objects::transaction::TransactionInputs;

const NOTE_SETUP_FILE: &str = "note_setup.masm";

#[test]
fn test_note_setup() {
    let (merkle_store, inputs) = mock_inputs();
    let imports = "use.miden::sat::prologue\n";
    let code = "
        begin
            exec.prologue::prepare_transaction
            exec.prepare_note
        end
        ";
    let process = run_within_tx_kernel(
        imports,
        code,
        inputs.stack_inputs(),
        MemAdviceProvider::from(inputs.advice_provider_inputs().with_merkle_store(merkle_store)),
        Some(TX_KERNEL_DIR),
        Some(NOTE_SETUP_FILE),
    )
    .unwrap();
    note_setup_stack_assertions(&process, &inputs);
    note_setup_memory_assertions(&process);
}

fn note_setup_stack_assertions<A: AdviceProvider>(
    process: &Process<A>,
    inputs: &TransactionInputs,
) {
    let mut note_inputs = [Felt::ZERO; 16];
    note_inputs.copy_from_slice(inputs.consumed_notes()[0].inputs().inputs());
    note_inputs.reverse();

    // assert that the stack contains the note inputs at the end of execution
    assert_eq!(process.stack.trace_state(), note_inputs)
}

fn note_setup_memory_assertions<A: AdviceProvider>(process: &Process<A>) {
    // assert that the correct pointer is stored in bookkeeping memory
    assert_eq!(
        process.get_memory_value(0, CURRENT_CONSUMED_NOTE_PTR).unwrap()[0],
        Felt::try_from(consumed_note_data_ptr(0)).unwrap()
    );
}
