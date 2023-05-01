pub mod common;
use common::{
    data::mock_inputs,
    memory::{ACCT_CODE_ROOT_PTR, ACCT_NEW_CODE_ROOT_PTR},
    run_within_tx_kernel, AccountId, AccountType, Felt, MemAdviceProvider, ONE, ZERO,
};
use vm_core::StackInputs;

// MOCK DATA
// ================================================================================================

const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN: u64 = 0b0110011011u64 << 54;
const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN: u64 = 0b0001101110 << 54;
const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1010011100 << 54;
const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN: u64 = 0b1101100110 << 54;
const ACCOUNT_ID_INSUFFICIENT_ONES: u64 = 0b1100000110 << 54;

// TESTS
// ================================================================================================

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
    )
    .unwrap();

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
    )
    .unwrap();

    // assert the code root is changed after the epilogue
    assert_eq!(
        process.get_memory_value(0, ACCT_CODE_ROOT_PTR).unwrap(),
        [ZERO, ONE, Felt::new(2), Felt::new(3)]
    );
}

#[test]
pub fn test_account_type() {
    let procedures = vec![
        ("is_fungible_faucet", AccountType::FungibleFaucet),
        ("is_non_fungible_faucet", AccountType::NonFungibleFaucet),
        ("is_updatable_account", AccountType::RegularAccountUpdatableCode),
        ("is_immutable_account", AccountType::RegularAccountImmutableCode),
    ];

    let test_cases = vec![
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
    ];

    for (procedure, expected_type) in procedures {
        for account_id in test_cases.iter() {
            let account_id = AccountId::try_from(*account_id).unwrap();

            let code = format!(
                "
                use.miden::sat::layout
                use.miden::sat::account

                begin
                    exec.account::{}
                end
                ",
                procedure
            );

            let process = run_within_tx_kernel(
                "",
                &code,
                StackInputs::new(vec![account_id.into()]),
                MemAdviceProvider::default(),
                None,
                None,
            )
            .unwrap();

            let expected_result = if account_id.account_type() == expected_type {
                ONE
            } else {
                ZERO
            };
            assert_eq!(process.stack.get(0), expected_result);
        }
    }
}

#[test]
fn test_validate_id_fails_on_insuficcient_ones() {
    let code = format!(
        "
        use.miden::sat::account
    
        begin
            push.{ACCOUNT_ID_INSUFFICIENT_ONES}
            exec.account::validate_id
        end
        "
    );

    let result = run_within_tx_kernel(
        "",
        &code,
        StackInputs::default(),
        MemAdviceProvider::default(),
        None,
        None,
    );

    assert!(result.is_err());
}
