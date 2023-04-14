pub mod common;
use common::{
    data::mock_inputs,
    memory::{ACCT_CODE_ROOT_PTR, ACCT_NEW_CODE_ROOT_PTR},
    run_within_tx_kernel, Felt, MemAdviceProvider, ONE, ZERO,
};

#[test]
pub fn test_set_code_is_not_immediate() {
    let (merkle_store, inputs) = mock_inputs();
    let code = "
        use.miden::sat::prologue
        use.miden::sat::account
        begin
            exec.prologue::prepare_transaction
            push.1.2.3.4
            exec.account::set_code
        end
        ";
    let process = run_within_tx_kernel(
        "",
        code,
        inputs.stack_inputs(),
        MemAdviceProvider::from(inputs.advice_provider_inputs().with_merkle_store(merkle_store)),
        None,
        None,
    );

    // assert the code root is not changed
    assert_eq!(
        process.get_memory_value(0, ACCT_CODE_ROOT_PTR).unwrap(),
        inputs.account().code().root().as_elements()
    );

    // assert the new code root is cached
    assert_eq!(
        process.get_memory_value(0, ACCT_NEW_CODE_ROOT_PTR).unwrap(),
        [ONE, Felt::new(2), Felt::new(3), Felt::new(4)]
    );
}

#[test]
pub fn test_set_code_succeeds() {
    let (merkle_store, inputs) = mock_inputs();
    let code = "
        use.miden::sat::account
        use.miden::sat::prologue
        use.miden::sat::epilogue

        begin
            exec.prologue::prepare_transaction
            push.0.1.2.3
            exec.account::set_code
            exec.epilogue::finalize_transaction
        end
        ";
    let process = run_within_tx_kernel(
        "",
        code,
        inputs.stack_inputs(),
        MemAdviceProvider::from(inputs.advice_provider_inputs().with_merkle_store(merkle_store)),
        None,
        None,
    );

    // assert the code root is changed after the epilogue
    assert_eq!(
        process.get_memory_value(0, ACCT_CODE_ROOT_PTR).unwrap(),
        [ZERO, ONE, Felt::new(2), Felt::new(3)]
    );
}
