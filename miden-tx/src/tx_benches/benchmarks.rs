use miden_lib::notes::create_p2id_note;
use miden_lib::transaction::ToTransactionKernelInputs;
use miden_lib::utils::Serializable;
use miden_objects::{
    accounts::AccountId,
    assembly::ProgramAst,
    assets::{Asset, FungibleAsset},
    crypto::dsa::rpo_falcon512::SecretKey,
    crypto::rand::RpoRandomCoin,
    notes::NoteType,
    transaction::TransactionArgs,
    Felt,
};
use miden_tx::{TransactionExecutor, TransactionHost};
use vm_processor::{ExecutionOptions, RecAdviceProvider, Word};

use crate::utils::{MockDataStore, String, ToString, Vec, DEFAULT_AUTH_SCRIPT};

// BENCHMARKS
// ================================================================================================

/// Runs the default transaction with empty transaction script and two default notes.
pub fn benchmark_default_tx() -> Result<(), String> {
    let data_store = MockDataStore::default();
    let mut executor = TransactionExecutor::new(data_store.clone()).with_tracing();

    let account_id = data_store.account.id();
    executor.load_account(account_id).map_err(|e| e.to_string())?;

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let transaction = executor
        .prepare_transaction(account_id, block_ref, &note_ids, data_store.tx_args().clone())
        .map_err(|e| e.to_string())?;

    let (stack_inputs, advice_inputs) = transaction.get_kernel_inputs();
    let advice_recorder: RecAdviceProvider = advice_inputs.into();
    let mut host = TransactionHost::new(transaction.account().into(), advice_recorder);

    vm_processor::execute(
        transaction.program(),
        stack_inputs,
        &mut host,
        ExecutionOptions::default().with_tracing(),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
