use miden_lib::transaction::{
    memory::{ACCT_CODE_ROOT_PTR, ACCT_NEW_CODE_ROOT_PTR},
    ToTransactionKernelInputs,
};
use miden_objects::{
    accounts::{
        AccountId, AccountType, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_INSUFFICIENT_ONES,
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
    },
    crypto::merkle::LeafIndex,
    Felt, Word, ONE, ZERO,
};
use mock::{
    constants::{
        storage_item_0, storage_item_1, CHILD_ROOT_PARENT_LEAF_INDEX, CHILD_SMT_DEPTH,
        CHILD_STORAGE_INDEX_0, CHILD_STORAGE_VALUE_0,
    },
    mock::{
        account::MockAccountType,
        host::MockHost,
        notes::AssetPreservationStatus,
        transaction::{mock_executed_tx, mock_inputs},
    },
    prepare_transaction,
    procedures::{output_notes_data_procedure, prepare_word},
    run_tx, run_within_host, run_within_tx_kernel,
};
use vm_processor::{ContextId, MemAdviceProvider, ProcessState, StackInputs};

// ACCOUNT CODE TESTS
// ================================================================================================

#[test]
pub fn test_set_code_is_not_immediate() {
    let tx_inputs =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let code = "
        use.miden::kernels::tx::prologue
        use.miden::account
        begin
            exec.prologue::prepare_transaction
            push.1.2.3.4
            exec.account::set_code
        end
        ";

    let transaction = prepare_transaction(tx_inputs, None, code, None);
    let process = run_tx(&transaction).unwrap();

    // assert the code root is not changed
    assert_eq!(
        process.get_mem_value(ContextId::root(), ACCT_CODE_ROOT_PTR).unwrap(),
        transaction.account().code().root().as_elements()
    );

    // assert the new code root is cached
    assert_eq!(
        process.get_mem_value(ContextId::root(), ACCT_NEW_CODE_ROOT_PTR).unwrap(),
        [ONE, Felt::new(2), Felt::new(3), Felt::new(4)]
    );
}

#[test]
pub fn test_set_code_succeeds() {
    let executed_transaction = mock_executed_tx(AssetPreservationStatus::Preserved);

    let output_notes_data_procedure =
        output_notes_data_procedure(executed_transaction.output_notes());

    let code = format!(
        "
        use.miden::account
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::epilogue

        {output_notes_data_procedure}
        begin
            exec.prologue::prepare_transaction

            push.0.1.2.3
            exec.account::set_code

            exec.create_mock_notes

            push.1
            exec.account::incr_nonce

            exec.epilogue::finalize_transaction
        end
        "
    );

    let (stack_inputs, advice_inputs) = executed_transaction.get_kernel_inputs();
    let host = MockHost::new(executed_transaction.initial_account().into(), advice_inputs);
    let process = run_within_host("", &code, stack_inputs, host, None).unwrap();

    // assert the code root is changed after the epilogue
    assert_eq!(
        process.get_mem_value(ContextId::root(), ACCT_CODE_ROOT_PTR).unwrap(),
        [ZERO, ONE, Felt::new(2), Felt::new(3)]
    );
}

// ACCOUNT ID TESTS
// ================================================================================================

#[test]
pub fn test_account_type() {
    let procedures = vec![
        ("is_fungible_faucet", AccountType::FungibleFaucet),
        ("is_non_fungible_faucet", AccountType::NonFungibleFaucet),
        ("is_updatable_account", AccountType::RegularAccountUpdatableCode),
        ("is_immutable_account", AccountType::RegularAccountImmutableCode),
    ];

    let test_cases = [
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
                use.miden::kernels::tx::memory
                use.miden::kernels::tx::account

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
fn test_validate_id_fails_on_insufficient_ones() {
    let code = format!(
        "
        use.miden::kernels::tx::account

        begin
            push.{ACCOUNT_ID_INSUFFICIENT_ONES}
            exec.account::validate_id
        end
        "
    );

    let result =
        run_within_tx_kernel("", &code, StackInputs::default(), MemAdviceProvider::default(), None);

    assert!(result.is_err());
}

#[test]
fn test_is_faucet_procedure() {
    let test_cases = [
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
    ];

    for account_id in test_cases.iter() {
        let account_id = AccountId::try_from(*account_id).unwrap();

        // assembly codes that checks if an account is a faucet
        let code = format!(
            "
        use.miden::kernels::tx::account

        begin
            # push the account id on to the stack
            push.{account_id}

            # execute is_faucet procedure
            exec.account::is_faucet

            # assert it matches expected result
            eq.{expected} assert
        end
    ",
            account_id = account_id,
            expected = if account_id.is_faucet() { 1 } else { 0 },
        );

        let _process = run_within_tx_kernel(
            "",
            &code,
            StackInputs::default(),
            MemAdviceProvider::default(),
            None,
        )
        .unwrap();
    }
}

// ACCOUNT STORAGE TESTS
// ================================================================================================

#[test]
fn test_get_item() {
    for storage_item in [storage_item_0(), storage_item_1()] {
        let tx_inputs =
            mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

        let code = format!(
            "
        use.miden::account
        use.miden::kernels::tx::prologue


        begin
            # prepare the transaction
            exec.prologue::prepare_transaction

            # push the account storage item index
            push.{item_index}

            # get the item
            exec.account::get_item

            # assert the item value is correct
            push.{item_value} assert_eqw
        end
        ",
            item_index = storage_item.0,
            item_value = prepare_word(&storage_item.1 .1)
        );

        let transaction = prepare_transaction(tx_inputs, None, &code, None);
        let _process = run_tx(&transaction).unwrap();
    }
}

#[test]
fn test_set_item() {
    let tx_inputs =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    // copy the initial account slots (SMT)
    let mut account_smt = tx_inputs.account().storage().slots().clone();
    let init_root = account_smt.root();

    // insert a new leaf value
    let new_item_index = LeafIndex::new(12).unwrap();
    let new_item_value: Word = [Felt::new(91), Felt::new(92), Felt::new(93), Felt::new(94)];
    account_smt.insert(new_item_index, new_item_value);
    assert_ne!(account_smt.root(), init_root);

    let code = format!(
        "
    use.miden::account
    use.miden::kernels::tx::memory
    use.miden::kernels::tx::prologue

    begin
        # prepare the transaction
        exec.prologue::prepare_transaction

        # push the new storage item onto the stack
        push.{new_value}

        # push the account storage item index
        push.{new_item_index}

        # get the item
        exec.account::set_item

        # assert empty old value
        padw assert_eqw

        # get the new storage root
        exec.memory::get_acct_storage_root

        # assert the item value is correct
        push.{new_root} assert_eqw
    end
    ",
        new_value = prepare_word(&new_item_value),
        new_item_index = new_item_index.value(),
        new_root = prepare_word(&account_smt.root()),
    );

    let transaction = prepare_transaction(tx_inputs, None, &code, None);
    let _process = run_tx(&transaction).unwrap();
}

// TODO: reenable once storage map support is implemented
#[ignore]
#[test]
fn test_get_map_item() {
    let tx_inputs =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let code = format!(
        "
        use.miden::account
        use.miden::kernels::tx::prologue

        begin
            # prepare the transaction
            exec.prologue::prepare_transaction

            # push the account storage index the child root is stored at
            push.{CHILD_ROOT_PARENT_LEAF_INDEX}

            # get the child root
            exec.account::get_item

            # get a value from the child tree
            push.{CHILD_STORAGE_INDEX_0}

            # get the item
            push.{CHILD_SMT_DEPTH} mtree_get

            # assert the child value is correct
            push.{child_value} assert_eqw
        end
        ",
        child_value = prepare_word(&CHILD_STORAGE_VALUE_0)
    );

    let transaction = prepare_transaction(tx_inputs, None, code.as_str(), None);
    let _process = run_tx(&transaction).unwrap();
}

// ACCOUNT VAULT TESTS
// ================================================================================================

#[test]
fn test_get_vault_commitment() {
    let tx_inputs =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let account = tx_inputs.account();
    let code = format!(
        "
    use.miden::account
    use.miden::kernels::tx::prologue

    begin
        # prepare the transaction
        exec.prologue::prepare_transaction

        # push the new storage item onto the stack
        exec.account::get_vault_commitment
        push.{expected_vault_commitment}
        assert_eqw
    end
    ",
        expected_vault_commitment = prepare_word(&account.vault().commitment()),
    );

    let transaction = prepare_transaction(tx_inputs, None, &code, None);
    let _process = run_tx(&transaction).unwrap();
}

// PROCEDURE AUTHENTICATION TESTS
// ================================================================================================

#[test]
fn test_authenticate_procedure() {
    let tx_inputs =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let account = tx_inputs.account();

    let proc0_index = LeafIndex::new(0).unwrap();
    let proc1_index = LeafIndex::new(1).unwrap();

    let test_cases = vec![
        (account.code().procedure_tree().get_leaf(&proc0_index), true),
        (account.code().procedure_tree().get_leaf(&proc1_index), true),
        ([ONE, ZERO, ONE, ZERO], false),
    ];

    for (root, valid) in test_cases.into_iter() {
        let tx_inputs =
            mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

        let code = format!(
            "\
            use.miden::kernels::tx::account
            use.miden::kernels::tx::prologue

            begin
                # prepare the transaction
                exec.prologue::prepare_transaction

                # push test procedure root onto stack
                push.{root}

                # authenticate procedure
                exec.account::authenticate_procedure
            end
        ",
            root = prepare_word(&root)
        );

        let transaction = prepare_transaction(tx_inputs, None, &code, None);
        let process = run_tx(&transaction);

        match valid {
            true => assert!(process.is_ok()),
            false => assert!(process.is_err()),
        }
    }
}
