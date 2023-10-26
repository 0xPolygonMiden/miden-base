use super::{
    build_module_path, AdviceProvider, ContextId, DefaultHost, Felt, MemAdviceProvider, Process,
    ProcessState, TX_KERNEL_DIR, ZERO,
};
use crate::memory::CURRENT_CONSUMED_NOTE_PTR;
use miden_objects::transaction::PreparedTransaction;
use mock::{
    consumed_note_data_ptr,
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    prepare_transaction, run_tx,
};

const NOTE_SETUP_FILE: &str = "note_setup.masm";

#[test]
fn test_note_setup() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let imports = "use.miden::sat::internal::prologue\n";
    let code = "
        begin
            exec.prologue::prepare_transaction
            exec.prepare_note
        end
        ";

    let assembly_file = build_module_path(TX_KERNEL_DIR, NOTE_SETUP_FILE);
    let inputs = prepare_transaction(
        account,
        None,
        block_header,
        chain,
        notes,
        &code,
        imports,
        Some(assembly_file),
    );

    let process = run_tx(
        inputs.tx_program().clone(),
        inputs.stack_inputs(),
        MemAdviceProvider::from(inputs.advice_provider_inputs()),
    )
    .unwrap();
    note_setup_stack_assertions(&process, &inputs);
    note_setup_memory_assertions(&process);
}

fn note_setup_stack_assertions<A: AdviceProvider>(
    process: &Process<DefaultHost<A>>,
    inputs: &PreparedTransaction,
) {
    let mut note_inputs = [ZERO; 16];
    note_inputs.copy_from_slice(inputs.consumed_notes().notes()[0].inputs().inputs());
    note_inputs.reverse();

    // assert that the stack contains the note inputs at the end of execution
    assert_eq!(process.stack.trace_state(), note_inputs)
}

fn note_setup_memory_assertions<A: AdviceProvider>(process: &Process<DefaultHost<A>>) {
    // assert that the correct pointer is stored in bookkeeping memory
    assert_eq!(
        process.get_mem_value(ContextId::root(), CURRENT_CONSUMED_NOTE_PTR).unwrap()[0],
        Felt::try_from(consumed_note_data_ptr(0)).unwrap()
    );
}
