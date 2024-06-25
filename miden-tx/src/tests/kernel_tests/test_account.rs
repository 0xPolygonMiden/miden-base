use alloc::vec::Vec;

use miden_lib::transaction::{
    memory::{ACCT_CODE_ROOT_PTR, ACCT_NEW_CODE_ROOT_PTR},
    ToTransactionKernelInputs,
};
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_INSUFFICIENT_ONES,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        },
        AccountId, AccountStorage, AccountType, StorageSlotType,
    },
    crypto::{hash::rpo::RpoDigest, merkle::LeafIndex},
    notes::Note,
    testing::{notes::AssetPreservationStatus, prepare_word, storage::STORAGE_LEAVES_2},
    transaction::OutputNote,
};
use vm_processor::{Felt, MemAdviceProvider};

use super::{ProcessState, StackInputs, Word, ONE, ZERO};
use crate::{
    testing::{
        executor::CodeExecutor, utils::mock_executed_tx, MockHost, TransactionContextBuilder,
    },
    tests::kernel_tests::{output_notes_data_procedure, read_root_mem_value},
};

// ACCOUNT CODE TESTS
// ================================================================================================

#[test]
pub fn test_set_code_is_not_immediate() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .build();
    let code = "
        use.miden::kernels::tx::prologue
        use.miden::account
        begin
            exec.prologue::prepare_transaction
            push.1.2.3.4
            exec.account::set_code
        end
        ";

    let process = tx_context.execute_code(code).unwrap();

    assert_eq!(
        read_root_mem_value(&process, ACCT_CODE_ROOT_PTR),
        tx_context.account().code().root().as_elements(),
        "the code root must not change immediatelly",
    );

    assert_eq!(
        read_root_mem_value(&process, ACCT_NEW_CODE_ROOT_PTR),
        [ONE, Felt::new(2), Felt::new(3), Felt::new(4)],
        "the code root must be cached",
    );
}

#[test]
pub fn test_set_code_succeeds() {
    let executed_transaction = mock_executed_tx(AssetPreservationStatus::Preserved);

    let output_notes: Vec<Note> = executed_transaction
        .output_notes()
        .iter()
        .filter_map(|note| {
            if let OutputNote::Full(note) = note {
                Some(note)
            } else {
                None
            }
        })
        .cloned()
        .collect();

    let output_notes_data_procedure = output_notes_data_procedure(&output_notes);

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
    let process = CodeExecutor::new(host).stack_inputs(stack_inputs).run(&code).unwrap();

    assert_eq!(
        read_root_mem_value(&process, ACCT_CODE_ROOT_PTR),
        [ZERO, ONE, Felt::new(2), Felt::new(3)],
        "the code root must change after the epilogue",
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
        let mut has_type = false;

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

            let process = CodeExecutor::with_advice_provider(MemAdviceProvider::default())
                .stack_inputs(StackInputs::new(vec![account_id.into()]).unwrap())
                .run(&code)
                .unwrap();

            let type_matches = account_id.account_type() == expected_type;
            let expected_result = Felt::from(type_matches);
            has_type |= type_matches;

            assert_eq!(
                process.stack.get(0),
                expected_result,
                "Rust and Masm check on account type diverge. proc: {} account_id: {} account_type: {:?} expected_type: {:?}",
                procedure,
                account_id,
                account_id.account_type(),
                expected_type,
            );
        }

        assert!(has_type, "missing test for type {:?}", expected_type);
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

    let result = CodeExecutor::with_advice_provider(MemAdviceProvider::default()).run(&code);

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

        let code = format!(
            "
            use.miden::kernels::tx::account

            begin
                push.{account_id}
                exec.account::is_faucet
            end
            ",
            account_id = account_id,
        );

        let process = CodeExecutor::with_advice_provider(MemAdviceProvider::default())
            .run(&code)
            .unwrap();

        let is_faucet = account_id.is_faucet();
        assert_eq!(
            process.stack.get(0),
            Felt::new(is_faucet as u64),
            "Rust and Masm is_faucet diverged. account_id: {}",
            account_id
        );
    }
}

// ACCOUNT STORAGE TESTS
// ================================================================================================

#[test]
fn test_get_item() {
    for storage_item in [AccountStorage::mock_item_0(), AccountStorage::mock_item_1()] {
        let tx_context = TransactionContextBuilder::with_standard_account(
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            ONE,
        )
        .build();

        let code = format!(
            "
            use.miden::account
            use.miden::kernels::tx::prologue

            begin
                exec.prologue::prepare_transaction

                # push the account storage item index
                push.{item_index}

                # assert the item value is correct
                exec.account::get_item
                push.{item_value}
                assert_eqw
            end
            ",
            item_index = storage_item.index,
            item_value = prepare_word(&storage_item.slot.value)
        );

        tx_context.execute_code(&code).unwrap();
    }
}

#[test]
fn test_set_item() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .build();

    // copy the initial account slots (SMT)
    let mut account_smt = tx_context.account().storage().slots().clone();
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
            exec.prologue::prepare_transaction

            # set the storage item
            push.{new_value}
            push.{new_item_index}
            exec.account::set_item

            # assert old value was empty
            padw assert_eqw

            # assert the new item value is properly stored
            exec.memory::get_acct_storage_root
            push.{new_root} assert_eqw
        end
        ",
        new_value = prepare_word(&new_item_value),
        new_item_index = new_item_index.value(),
        new_root = prepare_word(&account_smt.root()),
    );

    tx_context.execute_code(&code).unwrap();
}

// Test different account storage types
#[test]
fn test_get_storage_data_type() {
    for storage_item in [
        AccountStorage::mock_item_0(),
        AccountStorage::mock_item_1(),
        AccountStorage::mock_item_2(),
    ] {
        let tx_context = TransactionContextBuilder::with_standard_account(
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            ONE,
        )
        .build();

        let code = format!(
            "
            use.miden::kernels::tx::account
            use.miden::kernels::tx::prologue

            begin
                exec.prologue::prepare_transaction

                # push the account storage item index
                push.{item_index}

                # get the data type of the respective storage slot
                exec.account::get_storage_slot_type_info
            end
            ",
            item_index = storage_item.index,
        );

        let process = tx_context.execute_code(&code).unwrap();

        let storage_slot_data_type = match storage_item.slot.slot_type {
            StorageSlotType::Value { value_arity } => (value_arity, 0),
            StorageSlotType::Map { value_arity } => (value_arity, 1),
            StorageSlotType::Array { value_arity, depth } => (value_arity, depth),
        };

        assert_eq!(
            process.get_stack_item(0),
            Felt::from(storage_slot_data_type.0),
            "Arity must match",
        );
        assert_eq!(
            process.get_stack_item(1),
            Felt::from(storage_slot_data_type.1),
            "Depth must match",
        );
        assert_eq!(process.get_stack_item(2), ZERO, "the rest of the stack is empty");
        assert_eq!(process.get_stack_item(3), ZERO, "the rest of the stack is empty");
        assert_eq!(Word::default(), process.get_stack_word(1), "the rest of the stack is empty");
        assert_eq!(Word::default(), process.get_stack_word(2), "the rest of the stack is empty");
        assert_eq!(Word::default(), process.get_stack_word(3), "the rest of the stack is empty");
    }
}

#[test]
fn test_get_map_item() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .build();

    let storage_item = AccountStorage::mock_item_2();
    for (key, value) in STORAGE_LEAVES_2 {
        let code = format!(
            "
            use.miden::account
            use.miden::kernels::tx::prologue

            begin
                exec.prologue::prepare_transaction

                # get the map item
                push.{map_key}
                push.{item_index}
                exec.account::get_map_item
            end
            ",
            item_index = storage_item.index,
            map_key = prepare_word(&key),
        );
        let process = tx_context.execute_code(&code).unwrap();

        assert_eq!(
            value,
            process.get_stack_word(0),
            "get_map_item result doesn't match the expected value",
        );
        assert_eq!(
            Word::default(),
            process.get_stack_word(1),
            "The the rest of the stack must be cleared",
        );
        assert_eq!(
            Word::default(),
            process.get_stack_word(2),
            "The the rest of the stack must be cleared",
        );
        assert_eq!(
            Word::default(),
            process.get_stack_word(3),
            "The the rest of the stack must be cleared",
        );
    }
}

#[test]
fn test_set_map_item() {
    let (new_key, new_value) = (
        RpoDigest::new([Felt::new(109), Felt::new(110), Felt::new(111), Felt::new(112)]),
        [Felt::new(9_u64), Felt::new(10_u64), Felt::new(11_u64), Felt::new(12_u64)],
    );

    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .build();

    let storage_item = AccountStorage::mock_item_2();

    let code = format!(
        "
        use.miden::account
        use.miden::kernels::tx::prologue

        begin
            exec.prologue::prepare_transaction

            # set the map item
            push.{new_value}
            push.{new_key}
            push.{item_index}
            exec.account::set_map_item

            # double check that on storage slot is indeed the new map
            push.{item_index}
            exec.account::get_item
        end
        ",
        item_index = storage_item.index,
        new_key = prepare_word(&new_key),
        new_value = prepare_word(&new_value),
    );

    let process = tx_context.execute_code(&code).unwrap();

    let mut new_storage_map = AccountStorage::mock_map_2();
    new_storage_map.insert(new_key, new_value);

    assert_eq!(
        new_storage_map.root(),
        RpoDigest::from(process.get_stack_word(0)),
        "get_item must return the new updated value",
    );
    assert_eq!(
        storage_item.slot.value,
        process.get_stack_word(1),
        "The original value stored in the map doesn't match the expected value",
    );
}

// ACCOUNT VAULT TESTS
// ================================================================================================

#[test]
fn test_get_vault_commitment() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .build();

    let account = tx_context.account();
    let code = format!(
        "
        use.miden::account
        use.miden::kernels::tx::prologue

        begin
            exec.prologue::prepare_transaction

            # push the new storage item onto the stack
            exec.account::get_vault_commitment
            push.{expected_vault_commitment}
            assert_eqw
        end
        ",
        expected_vault_commitment = prepare_word(&account.vault().commitment()),
    );

    tx_context.execute_code(&code).unwrap();
}

// PROCEDURE AUTHENTICATION TESTS
// ================================================================================================

#[test]
fn test_authenticate_procedure() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .build();
    let account = tx_context.tx_inputs().account();

    let proc0_index = LeafIndex::new(0).unwrap();
    let proc1_index = LeafIndex::new(1).unwrap();

    let test_cases = vec![
        (account.code().procedure_tree().get_leaf(&proc0_index), true),
        (account.code().procedure_tree().get_leaf(&proc1_index), true),
        ([ONE, ZERO, ONE, ZERO], false),
    ];

    for (root, valid) in test_cases.into_iter() {
        let tx_context = TransactionContextBuilder::with_standard_account(
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            ONE,
        )
        .build();

        let code = format!(
            "
            use.miden::kernels::tx::account
            use.miden::kernels::tx::prologue

            begin
                exec.prologue::prepare_transaction

                # authenticate procedure
                push.{root}
                exec.account::authenticate_procedure
            end
            ",
            root = prepare_word(&root)
        );

        let process = tx_context.execute_code(&code);

        match valid {
            true => assert!(process.is_ok(), "A valid procedure must successfully authenticate"),
            false => assert!(process.is_err(), "An invalid procedure must fail to authenticate"),
        }
    }
}
