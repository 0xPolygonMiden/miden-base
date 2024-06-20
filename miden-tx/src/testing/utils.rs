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
    testing::notes::AssetPreservationStatus,
    transaction::{ExecutedTransaction, OutputNote, OutputNotes, TransactionOutputs},
    FieldElement,
};
use vm_processor::AdviceInputs;

use super::TransactionContextBuilder;

// TEST HELPERS
// ================================================================================================

pub fn consumed_note_data_ptr(note_idx: u32) -> memory::MemoryAddress {
    memory::CONSUMED_NOTE_DATA_SECTION_OFFSET + note_idx * memory::NOTE_MEM_SIZE
}

pub fn mock_executed_tx(asset_preservation: AssetPreservationStatus) -> ExecutedTransaction {
    let assembler = TransactionKernel::assembler().with_debug_mode(true);

    // use empty main program to produce the mock transaction
    let program = assembler.compile("begin push.0 drop end").unwrap();

    // simulate a transaction that modifies the account state, and increases the nonce by one
    let initial_nonce = Felt::ONE;
    let final_nonce = initial_nonce + Felt::ONE;

    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        initial_nonce,
    )
    .with_mock_notes(asset_preservation)
    .build();

    let final_account = Account::mock(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        final_nonce,
        AccountCode::mock_wallet(&assembler),
    );

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
