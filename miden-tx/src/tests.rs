use super::{mock::MockDataStore, TransactionExecutor, TransactionProver};
use crypto::StarkField;
use miden_core::ProgramInfo;
use miden_objects::{transaction::TransactionOutputs, TryFromVmResult};
use miden_prover::ProofOptions;
use processor::MemAdviceProvider;

#[test]
fn test_transaction_executor_witness() {
    let data_store = MockDataStore::new();
    let mut executor = TransactionExecutor::new(data_store.clone());

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins = data_store
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(account_id, block_ref, &note_origins, None, None)
        .unwrap();
    let witness = transaction_result.clone().into_witness();

    // use the witness to execute the transaction again
    let mut mem_advice_provider: MemAdviceProvider = witness.advice_inputs().clone().into();
    let result =
        processor::execute(witness.program(), witness.get_stack_inputs(), &mut mem_advice_provider)
            .unwrap();

    let reexecution_result =
        TransactionOutputs::try_from_vm_result(result.stack_outputs(), &mem_advice_provider)
            .unwrap();

    assert_eq!(
        transaction_result.final_account_hash(),
        reexecution_result.final_account_stub.0.hash()
    );
    assert_eq!(transaction_result.created_notes(), &reexecution_result.created_notes);
}

#[test]
fn prove_witness() {
    let data_store = MockDataStore::new();
    let mut executor = TransactionExecutor::new(data_store.clone());

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins = data_store
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(account_id, block_ref, &note_origins, None, None)
        .unwrap();
    let witness = transaction_result.clone().into_witness();

    // prove the transaction with the witness
    let proof_options = ProofOptions::default();
    let prover = TransactionProver::new(proof_options);
    let proven_transaction = prover.prove_transaction_witness(witness).unwrap();

    println!("proven transaction: {:?}", proven_transaction);
}

#[test]
fn test_prove_and_verify_with_tx_executor() {
    let data_store = MockDataStore::new();
    let mut executor = TransactionExecutor::new(data_store.clone());

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins = data_store
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // prove the transaction with the executor
    let prepared_transaction = executor
        .prepare_transaction(account_id, block_ref, &note_origins, None)
        .unwrap();

    // extract transaction data for later consumption
    let program_hash = prepared_transaction.tx_program().hash();
    let kernel = prepared_transaction.tx_program().kernel().clone();

    // prove transaction
    let proof_options = ProofOptions::default();
    let prover = TransactionProver::new(proof_options);
    let proven_transaction = prover.prove_prepared_transaction(prepared_transaction).unwrap();

    let stack_inputs = proven_transaction.stack_inputs();
    let stack_outputs = proven_transaction.stack_outputs();
    let program_info = ProgramInfo::new(program_hash, kernel);
    let _result: u32 = miden_verifier::verify(
        program_info,
        stack_inputs,
        stack_outputs,
        proven_transaction.proof().clone(),
    )
    .unwrap();
}
