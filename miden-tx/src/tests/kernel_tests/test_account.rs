use miden_lib::{
    errors::tx_kernel_errors::ERR_ACCOUNT_INSUFFICIENT_NUMBER_OF_ONES,
    transaction::{
        memory::{NATIVE_ACCT_CODE_COMMITMENT_PTR, NEW_CODE_ROOT_PTR},
        TransactionKernel,
    },
};
use miden_objects::{
    accounts::{
        AccountBuilder, AccountCode, AccountComponent, AccountId, AccountStorage, AccountType,
        StorageSlot,
    },
    assembly::Library,
    testing::{
        account_component::AccountMockComponent,
        account_id::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_INSUFFICIENT_ONES,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        },
        prepare_word,
        storage::STORAGE_LEAVES_2,
    },
    transaction::TransactionScript,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use vm_processor::{Digest, MemAdviceProvider, ProcessState};

use super::{Felt, StackInputs, Word, ONE, ZERO};
use crate::{
    assert_execution_error,
    testing::{executor::CodeExecutor, TransactionContextBuilder},
    tests::kernel_tests::{output_notes_data_procedure, read_root_mem_value},
};

// ACCOUNT CODE TESTS
// ================================================================================================

#[test]
pub fn test_set_code_is_not_immediate() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();
    let code = "
        use.kernel::prologue
        use.kernel::account
        begin
            exec.prologue::prepare_transaction
            push.1.2.3.4
            exec.account::set_code
        end
        ";

    let process = tx_context.execute_code(code).unwrap();

    assert_eq!(
        read_root_mem_value(&process, NATIVE_ACCT_CODE_COMMITMENT_PTR),
        tx_context.account().code().commitment().as_elements(),
        "the code commitment must not change immediately",
    );

    assert_eq!(
        read_root_mem_value(&process, NEW_CODE_ROOT_PTR),
        [ONE, Felt::new(2), Felt::new(3), Felt::new(4)],
        "the code commitment must be cached",
    );
}

#[test]
pub fn test_set_code_succeeds() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved()
        .build();

    let output_notes_data_procedure =
        output_notes_data_procedure(tx_context.expected_output_notes());

    let code = format!(
        "
        use.kernel::account
        use.kernel::prologue
        use.kernel::epilogue

        {output_notes_data_procedure}
        begin
            exec.prologue::prepare_transaction

            push.0.1.2.3
            exec.account::set_code

            exec.create_mock_notes

            push.1
            exec.account::incr_nonce

            exec.epilogue::finalize_transaction

            # clean the stack
            dropw dropw dropw dropw
        end
        "
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(
        read_root_mem_value(&process, NATIVE_ACCT_CODE_COMMITMENT_PTR),
        [ZERO, ONE, Felt::new(2), Felt::new(3)],
        "the code commitment must change after the epilogue",
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
                use.kernel::account

                begin
                    exec.account::{}
                end
                ",
                procedure
            );

            let process = CodeExecutor::with_advice_provider(MemAdviceProvider::default())
                .stack_inputs(StackInputs::new(vec![account_id.first_felt()]).unwrap())
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
    // Split account ID into u64 limbs manually since we can't use AccountId constructors.
    let second_felt = ACCOUNT_ID_INSUFFICIENT_ONES % (1u128 << 64);
    let first_felt = ACCOUNT_ID_INSUFFICIENT_ONES / (1u128 << 64);

    let code = format!(
        "
        use.kernel::account

        begin
            push.{second_felt}.{first_felt}
            exec.account::validate_id
        end
        "
    );

    let result = CodeExecutor::with_advice_provider(MemAdviceProvider::default()).run(&code);

    assert_execution_error!(result, ERR_ACCOUNT_INSUFFICIENT_NUMBER_OF_ONES);
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
            use.kernel::account

            begin
                push.{first_felt}
                exec.account::is_faucet
                # => [is_faucet, account_id_hi]

                # truncate the stack
                swap drop
            end
            ",
            first_felt = account_id.first_felt(),
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
        let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

        let code = format!(
            "
            use.kernel::account
            use.kernel::prologue

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
            item_value = prepare_word(&storage_item.slot.value())
        );

        tx_context.execute_code(&code).unwrap();
    }
}

#[test]
fn test_get_map_item() {
    let account = AccountBuilder::new()
        .init_seed(ChaCha20Rng::from_entropy().gen())
        .with_component(
            AccountMockComponent::new_with_slots(
                TransactionKernel::testing_assembler(),
                vec![AccountStorage::mock_item_2().slot],
            )
            .unwrap(),
        )
        .build_existing()
        .unwrap();

    let tx_context = TransactionContextBuilder::new(account).build();

    for (key, value) in STORAGE_LEAVES_2 {
        let code = format!(
            "
            use.kernel::prologue

            begin
                exec.prologue::prepare_transaction

                # get the map item
                push.{map_key}
                push.{item_index}
                call.::test::account::get_map_item

                # truncate the stack 
                swapw dropw movup.4 drop
            end
            ",
            item_index = 0,
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
            "The rest of the stack must be cleared",
        );
        assert_eq!(
            Word::default(),
            process.get_stack_word(2),
            "The rest of the stack must be cleared",
        );
        assert_eq!(
            Word::default(),
            process.get_stack_word(3),
            "The rest of the stack must be cleared",
        );
    }
}

#[test]
fn test_get_storage_slot_type() {
    for storage_item in [
        AccountStorage::mock_item_0(),
        AccountStorage::mock_item_1(),
        AccountStorage::mock_item_2(),
    ] {
        let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

        let code = format!(
            "
            use.kernel::account
            use.kernel::prologue

            begin
                exec.prologue::prepare_transaction

                # push the account storage item index
                push.{item_index}

                # get the type of the respective storage slot
                exec.account::get_storage_slot_type

                # truncate the stack
                swap drop
            end
            ",
            item_index = storage_item.index,
        );

        let process = tx_context.execute_code(&code).unwrap();

        let storage_slot_type = storage_item.slot.slot_type();

        assert_eq!(storage_slot_type, process.get_stack_item(0).try_into().unwrap());
        assert_eq!(process.get_stack_item(1), ZERO, "the rest of the stack is empty");
        assert_eq!(process.get_stack_item(2), ZERO, "the rest of the stack is empty");
        assert_eq!(process.get_stack_item(3), ZERO, "the rest of the stack is empty");
        assert_eq!(Word::default(), process.get_stack_word(1), "the rest of the stack is empty");
        assert_eq!(Word::default(), process.get_stack_word(2), "the rest of the stack is empty");
        assert_eq!(Word::default(), process.get_stack_word(3), "the rest of the stack is empty");
    }
}

#[test]
fn test_set_item() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let new_storage_item: Word = [Felt::new(91), Felt::new(92), Felt::new(93), Felt::new(94)];

    let code = format!(
        "
        use.kernel::account
        use.kernel::memory
        use.kernel::prologue

        begin
            exec.prologue::prepare_transaction

            # set the storage item
            push.{new_storage_item}
            push.{new_storage_item_index}
            exec.account::set_item

            # assert old value was correctly returned
            push.1.2.3.4 assert_eqw

            # assert new value has been correctly set
            push.{new_storage_item_index}
            exec.account::get_item
            push.{new_storage_item}
            assert_eqw
        end
        ",
        new_storage_item = prepare_word(&new_storage_item),
        new_storage_item_index = 0,
    );

    tx_context.execute_code(&code).unwrap();
}

#[test]
fn test_set_map_item() {
    let (new_key, new_value) = (
        Digest::new([Felt::new(109), Felt::new(110), Felt::new(111), Felt::new(112)]),
        [Felt::new(9_u64), Felt::new(10_u64), Felt::new(11_u64), Felt::new(12_u64)],
    );

    let account = AccountBuilder::new()
        .init_seed(ChaCha20Rng::from_entropy().gen())
        .with_component(
            AccountMockComponent::new_with_slots(
                TransactionKernel::testing_assembler(),
                vec![AccountStorage::mock_item_2().slot],
            )
            .unwrap(),
        )
        .build_existing()
        .unwrap();

    let tx_context = TransactionContextBuilder::new(account).build();
    let storage_item = AccountStorage::mock_item_2();

    let code = format!(
        "
        use.std::sys

        use.test::account
        use.kernel::prologue

        begin
            exec.prologue::prepare_transaction

            # set the map item
            push.{new_value}
            push.{new_key}
            push.{item_index}
            call.account::set_map_item

            # double check that on storage slot is indeed the new map
            push.{item_index}
            call.account::get_item

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        item_index = 0,
        new_key = prepare_word(&new_key),
        new_value = prepare_word(&new_value),
    );

    let process = tx_context.execute_code(&code).unwrap();

    let mut new_storage_map = AccountStorage::mock_map();
    new_storage_map.insert(new_key, new_value);

    assert_eq!(
        new_storage_map.root(),
        Digest::from(process.get_stack_word(0)),
        "get_item must return the new updated value",
    );
    assert_eq!(
        storage_item.slot.value(),
        process.get_stack_word(1),
        "The original value stored in the map doesn't match the expected value",
    );
}

#[test]
fn test_account_component_storage_offset() {
    // setup assembler
    let assembler = TransactionKernel::testing_assembler();

    // The following code will execute the following logic that will be asserted during the test:
    //
    // 1. foo_write will set word [1, 2, 3, 4] in storage at location 0 (0 offset by 0)
    // 2. foo_read will read word [1, 2, 3, 4] in storage from location 0 (0 offset by 0)
    // 3. bar_write will set word [5, 6, 7, 8] in storage at location 1 (0 offset by 1)
    // 4. bar_read will read word [5, 6, 7, 8] in storage from location 1 (0 offset by 1)
    //
    // We will then assert that we are able to retrieve the correct elements from storage
    // insuring consistent "set" and "get" using offsets.
    let source_code_component1 = "
        use.miden::account

        export.foo_write
            push.1.2.3.4.0
            exec.account::set_item

            dropw dropw
        end

        export.foo_read
            push.0
            exec.account::get_item
            push.1.2.3.4 eqw assert

            dropw dropw
        end
    ";

    let source_code_component2 = "
        use.miden::account

        export.bar_write
            push.5.6.7.8.0
            exec.account::set_item

            dropw dropw
        end

        export.bar_read
            push.0
            exec.account::get_item
            push.5.6.7.8 eqw assert

            push.1 exec.account::incr_nonce
            dropw dropw
        end
    ";

    // Compile source code to find MAST roots of procedures.
    let code1 = assembler.clone().assemble_library([source_code_component1]).unwrap();
    let code2 = assembler.clone().assemble_library([source_code_component2]).unwrap();
    let find_procedure_digest_by_name = |name: &str, lib: &Library| {
        lib.exports().find_map(|export| {
            if export.name.as_str() == name {
                Some(lib.mast_forest()[lib.get_export_node_id(export)].digest())
            } else {
                None
            }
        })
    };

    let foo_write = find_procedure_digest_by_name("foo_write", &code1).unwrap();
    let foo_read = find_procedure_digest_by_name("foo_read", &code1).unwrap();
    let bar_write = find_procedure_digest_by_name("bar_write", &code2).unwrap();
    let bar_read = find_procedure_digest_by_name("bar_read", &code2).unwrap();

    // Compile source code into components.
    let component1 = AccountComponent::compile(
        source_code_component1,
        assembler.clone(),
        vec![StorageSlot::Value(Word::default())],
    )
    .unwrap()
    .with_supported_type(AccountType::RegularAccountUpdatableCode);

    let component2 = AccountComponent::compile(
        source_code_component2,
        assembler.clone(),
        vec![StorageSlot::Value(Word::default())],
    )
    .unwrap()
    .with_supported_type(AccountType::RegularAccountUpdatableCode);

    let mut account = AccountBuilder::new()
        .init_seed(ChaCha20Rng::from_entropy().gen())
        .with_component(component1)
        .with_component(component2)
        .build_existing()
        .unwrap();

    // Assert that the storage offset and size have been set correctly.
    for (procedure_digest, expected_offset, expected_size) in
        [(foo_write, 0, 1), (foo_read, 0, 1), (bar_write, 1, 1), (bar_read, 1, 1)]
    {
        let procedure_info = account
            .code()
            .procedures()
            .iter()
            .find(|proc| proc.mast_root() == &procedure_digest)
            .unwrap();
        assert_eq!(
            procedure_info.storage_offset(),
            expected_offset,
            "failed for procedure {procedure_digest}"
        );
        assert_eq!(
            procedure_info.storage_size(),
            expected_size,
            "failed for procedure {procedure_digest}"
        );
    }

    // setup transaction script
    let tx_script_source_code = format!(
        "
    begin
        call.{foo_write}
        call.{foo_read}
        call.{bar_write}
        call.{bar_read}
    end
    "
    );
    let tx_script_program = assembler.assemble_program(tx_script_source_code).unwrap();
    let tx_script = TransactionScript::new(tx_script_program, vec![]);

    // setup transaction context
    let tx_context = TransactionContextBuilder::new(account.clone()).tx_script(tx_script).build();

    // execute code in context
    let tx = tx_context.execute().unwrap();
    account.apply_delta(tx.account_delta()).unwrap();

    // assert that elements have been set at the correct locations in storage
    assert_eq!(
        account.storage().get_item(0).unwrap(),
        [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)].into()
    );

    assert_eq!(
        account.storage().get_item(1).unwrap(),
        [Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)].into()
    );
}

// ACCOUNT VAULT TESTS
// ================================================================================================

#[test]
fn test_get_vault_commitment() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let account = tx_context.account();
    let code = format!(
        "
        use.miden::account
        use.kernel::prologue

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
    let mock_component =
        AccountMockComponent::new_with_empty_slots(TransactionKernel::assembler()).unwrap();
    let account_code = AccountCode::from_components(
        &[mock_component.into()],
        AccountType::RegularAccountUpdatableCode,
    )
    .unwrap();

    let tc_0: [Felt; 4] =
        account_code.procedures()[0].mast_root().as_elements().try_into().unwrap();
    let tc_1: [Felt; 4] =
        account_code.procedures()[1].mast_root().as_elements().try_into().unwrap();
    let tc_2: [Felt; 4] =
        account_code.procedures()[2].mast_root().as_elements().try_into().unwrap();

    let test_cases =
        vec![(tc_0, true), (tc_1, true), (tc_2, true), ([ONE, ZERO, ONE, ZERO], false)];

    for (root, valid) in test_cases.into_iter() {
        let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

        let code = format!(
            "
            use.kernel::account
            use.kernel::prologue

            begin
                exec.prologue::prepare_transaction

                # authenticate procedure
                push.{root}
                exec.account::authenticate_procedure

                # truncate the stack
                dropw
            end
            ",
            root = prepare_word(&root)
        );

        // Execution of this code will return an EventError(UnknownAccountProcedure) for procs
        // that are not in the advice provider.
        let process = tx_context.execute_code(&code);

        match valid {
            true => assert!(process.is_ok(), "A valid procedure must successfully authenticate"),
            false => assert!(process.is_err(), "An invalid procedure should fail to authenticate"),
        }
    }
}
