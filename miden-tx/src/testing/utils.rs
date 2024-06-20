use alloc::vec::Vec;

use miden_lib::transaction::memory;
#[cfg(not(target_family = "wasm"))]
use miden_lib::transaction::TransactionKernel;
#[cfg(feature = "std")]
use miden_objects::Felt;
use miden_objects::{
    accounts::{
        account_id::testing::ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN, Account,
        AccountCode, AccountDelta,
    },
    notes::Note,
    testing::{
        block::{MockChain, MockChainBuilder},
        notes::AssetPreservationStatus,
    },
    transaction::{ExecutedTransaction, OutputNote, OutputNotes, TransactionOutputs},
    vm::CodeBlock,
    FieldElement,
};
use vm_processor::{AdviceInputs, Operation, Program, ZERO};

use super::TransactionContextBuilder;

// TEST HELPERS
// ================================================================================================

pub fn consumed_note_data_ptr(note_idx: u32) -> memory::MemoryAddress {
    memory::CONSUMED_NOTE_DATA_SECTION_OFFSET + note_idx * memory::NOTE_MEM_SIZE
}

pub fn mock_executed_tx(asset_preservation: AssetPreservationStatus) -> ExecutedTransaction {
    let assembler = TransactionKernel::assembler().with_debug_mode(true);

    let initial_account = Account::mock(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        Felt::ONE,
        AccountCode::mock_wallet(&assembler),
    );

    // nonce incremented by 1
    let final_account = Account::mock(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        Felt::new(2),
        initial_account.code().clone(),
    );

    let tx_context = TransactionContextBuilder::new(initial_account)
        .assembler(assembler)
        .with_mock_notes(asset_preservation)
        .build();

    let output_notes = tx_context
        .expected_output_notes()
        .iter()
        .cloned()
        .map(OutputNote::Full)
        .collect();

    let tx_outputs = TransactionOutputs {
        account: final_account.into(),
        output_notes: OutputNotes::new(output_notes).unwrap(),
    };

    let program = build_dummy_tx_program();
    let account_delta = AccountDelta::default();
    let advice_witness = AdviceInputs::default();

    ExecutedTransaction::new(
        program,
        tx_context.tx_inputs().clone(),
        tx_outputs,
        account_delta,
        tx_context.tx_args().clone(),
        advice_witness,
    )
}

pub fn create_test_chain(created_notes: Vec<Note>) -> MockChain {
    let mut mock_chain = MockChainBuilder::new().notes(created_notes).build();
    mock_chain.seal_block();
    mock_chain.seal_block();
    mock_chain.seal_block();

    mock_chain
}

pub fn build_dummy_tx_program() -> Program {
    let operations = vec![Operation::Push(ZERO), Operation::Drop];
    let span = CodeBlock::new_span(operations);
    Program::new(span)
}
