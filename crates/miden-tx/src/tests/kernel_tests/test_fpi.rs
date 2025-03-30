use alloc::{string::String, vec::Vec};
use std::{string::ToString, vec};

use miden_lib::{
    errors::tx_kernel_errors::{
        ERR_FOREIGN_ACCOUNT_CONTEXT_AGAINST_NATIVE_ACCOUNT, ERR_FOREIGN_ACCOUNT_MAX_NUMBER_EXCEEDED,
    },
    transaction::{
        TransactionKernel,
        memory::{
            ACCOUNT_DATA_LENGTH, ACCT_CODE_COMMITMENT_OFFSET, ACCT_ID_AND_NONCE_OFFSET,
            ACCT_PROCEDURES_SECTION_OFFSET, ACCT_STORAGE_COMMITMENT_OFFSET,
            ACCT_STORAGE_SLOTS_SECTION_OFFSET, ACCT_VAULT_ROOT_OFFSET, ASSET_BOOKKEEPING_SIZE,
            ASSET_ISSUER_PREFIX_OFFSET, ASSET_MIN_PTR, ASSET_NEXT_PTR, NATIVE_ACCOUNT_DATA_PTR,
            NUM_ACCT_PROCEDURES_OFFSET, NUM_ACCT_STORAGE_SLOTS_OFFSET,
        },
    },
};
use miden_objects::{
    ACCOUNT_TREE_DEPTH,
    account::{
        Account, AccountBuilder, AccountComponent, AccountProcedureInfo, AccountStorage,
        StorageSlot,
    },
    assembly::{Compile, LibraryPath},
    crypto::merkle::{LeafIndex, MerklePath},
    testing::{account_component::AccountMockComponent, storage::STORAGE_LEAVES_2},
    transaction::TransactionScript,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use vm_processor::{AdviceInputs, ContextId, Felt, ProcessState};

use super::{Process, Word, ZERO};
use crate::{
    TransactionExecutor, TransactionExecutorError, assert_execution_error,
    testing::MockChain,
    tests::kernel_tests::{read_root_mem_word, try_read_root_mem_word},
};

// SIMPLE FPI TESTS
// ================================================================================================

// FOREIGN PROCEDURE INVOCATION TESTS
// ================================================================================================

#[test]
fn test_fpi_memory() {
    // Prepare the test data
    let storage_slots =
        vec![AccountStorage::mock_item_0().slot, AccountStorage::mock_item_2().slot];
    let foreign_account_code_source = "
        use.miden::account

        export.get_item_foreign
            # make this foreign procedure unique to make sure that we invoke the procedure of the 
            # foreign account, not the native one
            push.1 drop
            exec.account::get_item

            # truncate the stack
            movup.6 movup.6 movup.6 drop drop drop
        end

        export.get_map_item_foreign
            # make this foreign procedure unique to make sure that we invoke the procedure of the 
            # foreign account, not the native one
            push.2 drop
            exec.account::get_map_item
        end
    ";

    let foreign_account_component = AccountComponent::compile(
        foreign_account_code_source,
        TransactionKernel::testing_assembler(),
        storage_slots.clone(),
    )
    .unwrap()
    .with_supports_all_types();

    let foreign_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_component(foreign_account_component)
        .build_existing()
        .unwrap();

    let native_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_component(
            AccountMockComponent::new_with_slots(
                TransactionKernel::testing_assembler(),
                vec![AccountStorage::mock_item_2().slot],
            )
            .unwrap(),
        )
        .build_existing()
        .unwrap();

    let mut mock_chain =
        MockChain::with_accounts(&[native_account.clone(), foreign_account.clone()]);
    mock_chain.seal_next_block();
    let advice_inputs = get_mock_fpi_adv_inputs(vec![&foreign_account], &mock_chain);

    let tx_context = mock_chain
        .build_tx_context(native_account.id(), &[], &[])
        .foreign_account_codes(vec![foreign_account.code().clone()])
        .advice_inputs(advice_inputs.clone())
        .build();

    // GET ITEM
    // --------------------------------------------------------------------------------------------
    // Check the correctness of the memory layout after `get_item_foreign` account procedure
    // invocation

    let code = format!(
        "
        use.std::sys
        
        use.kernel::prologue
        use.miden::tx

        begin
            exec.prologue::prepare_transaction

            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign` procedure of the foreign account 
            push.{get_item_foreign_hash}

            # push the foreign account ID
            push.{foreign_suffix}.{foreign_prefix}
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(11)]

            exec.tx::execute_foreign_procedure
            # => [STORAGE_VALUE_1]

            # truncate the stack
            exec.sys::truncate_stack
            end
            ",
        foreign_prefix = foreign_account.id().prefix().as_felt(),
        foreign_suffix = foreign_account.id().suffix(),
        get_item_foreign_hash = foreign_account.code().procedures()[0].mast_root(),
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(
        process.stack.get_word(0),
        storage_slots[0].value(),
        "Value at the top of the stack (value in the storage at index 0) should be equal [1, 2, 3, 4]",
    );

    foreign_account_data_memory_assertions(&foreign_account, &process);

    // GET MAP ITEM
    // --------------------------------------------------------------------------------------------
    // Check the correctness of the memory layout after `get_map_item` account procedure invocation

    let code = format!(
        "
        use.std::sys

        use.kernel::prologue
        use.miden::tx

        begin
            exec.prologue::prepare_transaction

            # pad the stack for the `execute_foreign_procedure` execution
            padw padw push.0.0
            # => [pad(10)]

            # push the key of desired storage item
            push.{map_key}

            # push the index of desired storage item
            push.1

            # get the hash of the `get_map_item_foreign` account procedure
            push.{get_map_item_foreign_hash}

            # push the foreign account ID
            push.{foreign_suffix}.{foreign_prefix}
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, MAP_ITEM_KEY, pad(10)]

            exec.tx::execute_foreign_procedure
            # => [MAP_VALUE]

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        foreign_prefix = foreign_account.id().prefix().as_felt(),
        foreign_suffix = foreign_account.id().suffix(),
        map_key = STORAGE_LEAVES_2[0].0,
        get_map_item_foreign_hash = foreign_account.code().procedures()[1].mast_root(),
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(
        process.stack.get_word(0),
        STORAGE_LEAVES_2[0].1,
        "Value at the top of the stack should be equal [1, 2, 3, 4]",
    );

    foreign_account_data_memory_assertions(&foreign_account, &process);

    // GET ITEM TWICE
    // --------------------------------------------------------------------------------------------
    // Check the correctness of the memory layout after two consecutive invocations of the
    // `get_item` account procedures. Invoking two foreign procedures from the same account should
    // result in reuse of the loaded account.

    let code = format!(
        "
        use.std::sys

        use.kernel::prologue
        use.miden::tx

        begin
            exec.prologue::prepare_transaction

            ### Get the storage item at index 0 #####################
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign` procedure of the foreign account 
            push.{get_item_foreign_hash}

            # push the foreign account ID
            push.{foreign_suffix}.{foreign_prefix}
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure dropw
            # => []

            ### Get the storage item at index 0 again ###############
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign` procedure of the foreign account 
            push.{get_item_foreign_hash}

            # push the foreign account ID
            push.{foreign_suffix}.{foreign_prefix}
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        foreign_prefix = foreign_account.id().prefix().as_felt(),
        foreign_suffix = foreign_account.id().suffix(),
        get_item_foreign_hash = foreign_account.code().procedures()[0].mast_root(),
    );

    let process = &tx_context.execute_code(&code).unwrap();

    // Check that the second invocation of the foreign procedure from the same account does not load
    // the account data again: already loaded data should be reused.
    //
    // Native account:    [8192; 16383]  <- initialized during prologue
    // Foreign account:   [16384; 24575] <- initialized during first FPI
    // Next account slot: [24576; 32767] <- should not be initialized
    assert_eq!(
        try_read_root_mem_word(
            &process.into(),
            NATIVE_ACCOUNT_DATA_PTR + ACCOUNT_DATA_LENGTH as u32 * 2
        ),
        None,
        "Memory starting from 24576 should stay uninitialized"
    );
}

#[test]
fn test_fpi_memory_two_accounts() {
    // Prepare the test data
    let storage_slots_1 = vec![AccountStorage::mock_item_0().slot];
    let storage_slots_2 = vec![AccountStorage::mock_item_1().slot];

    let foreign_account_code_source_1 = "
        use.miden::account

        export.get_item_foreign_1
            # make this foreign procedure unique to make sure that we invoke the procedure of the 
            # foreign account, not the native one
            push.1 drop
            exec.account::get_item

            # truncate the stack
            movup.6 movup.6 movup.6 drop drop drop
        end
    ";
    let foreign_account_code_source_2 = "
        use.miden::account

        export.get_item_foreign_2
            # make this foreign procedure unique to make sure that we invoke the procedure of the 
            # foreign account, not the native one
            push.2 drop
            exec.account::get_item

            # truncate the stack
            movup.6 movup.6 movup.6 drop drop drop
        end
    ";

    let foreign_account_component_1 = AccountComponent::compile(
        foreign_account_code_source_1,
        TransactionKernel::testing_assembler(),
        storage_slots_1.clone(),
    )
    .unwrap()
    .with_supports_all_types();

    let foreign_account_component_2 = AccountComponent::compile(
        foreign_account_code_source_2,
        TransactionKernel::testing_assembler(),
        storage_slots_2.clone(),
    )
    .unwrap()
    .with_supports_all_types();

    let foreign_account_1 = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_component(foreign_account_component_1)
        .build_existing()
        .unwrap();

    let foreign_account_2 = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_component(foreign_account_component_2)
        .build_existing()
        .unwrap();

    let native_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_component(
            AccountMockComponent::new_with_empty_slots(TransactionKernel::testing_assembler())
                .unwrap(),
        )
        .build_existing()
        .unwrap();

    let mut mock_chain = MockChain::with_accounts(&[
        native_account.clone(),
        foreign_account_1.clone(),
        foreign_account_2.clone(),
    ]);
    mock_chain.seal_next_block();
    let advice_inputs =
        get_mock_fpi_adv_inputs(vec![&foreign_account_1, &foreign_account_2], &mock_chain);

    let tx_context = mock_chain
        .build_tx_context(native_account.id(), &[], &[])
        .foreign_account_codes(vec![
            foreign_account_1.code().clone(),
            foreign_account_2.code().clone(),
        ])
        .advice_inputs(advice_inputs.clone())
        .build();

    // GET ITEM TWICE WITH TWO ACCOUNTS
    // --------------------------------------------------------------------------------------------
    // Check the correctness of the memory layout after two invocations of the `get_item` account
    // procedures separated by the call of this procedure against another foreign account. Invoking
    // two foreign procedures from the same account should result in reuse of the loaded account.

    let code = format!(
        "
        use.std::sys

        use.kernel::prologue
        use.miden::tx

        begin
            exec.prologue::prepare_transaction

            ### Get the storage item at index 0 from the first account 
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign_1` procedure of the foreign account 1
            push.{get_item_foreign_1_hash}

            # push the foreign account ID
            push.{foreign_1_suffix}.{foreign_1_prefix}
            # => [foreign_account_1_id_prefix, foreign_account_1_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure dropw
            # => []

            ### Get the storage item at index 0 from the second account 
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign_2` procedure of the foreign account 2
            push.{get_item_foreign_2_hash}

            # push the foreign account ID
            push.{foreign_2_suffix}.{foreign_2_prefix}
            # => [foreign_account_2_id_prefix, foreign_account_2_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure dropw
            # => []

            ### Get the storage item at index 0 from the first account again
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item_foreign_1` procedure of the foreign account 1
            push.{get_item_foreign_1_hash}

            # push the foreign account ID
            push.{foreign_1_suffix}.{foreign_1_prefix}
            # => [foreign_account_1_id_prefix, foreign_account_1_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        get_item_foreign_1_hash = foreign_account_1.code().procedures()[0].mast_root(),
        get_item_foreign_2_hash = foreign_account_2.code().procedures()[0].mast_root(),

        foreign_1_prefix = foreign_account_1.id().prefix().as_felt(),
        foreign_1_suffix = foreign_account_1.id().suffix(),

        foreign_2_prefix = foreign_account_2.id().prefix().as_felt(),
        foreign_2_suffix = foreign_account_2.id().suffix(),
    );

    let process = &tx_context.execute_code(&code).unwrap();

    // Check the correctness of the memory layout after multiple foreign procedure invocations from
    // different foreign accounts
    //
    // Native account:    [8192; 16383]  <- initialized during prologue
    // Foreign account 1: [16384; 24575] <- initialized during first FPI
    // Foreign account 2: [24576; 32767] <- initialized during second FPI
    // Next account slot: [32768; 40959] <- should not be initialized

    // check that the first word of the first foreign account slot is correct
    assert_eq!(
        read_root_mem_word(&process.into(), NATIVE_ACCOUNT_DATA_PTR + ACCOUNT_DATA_LENGTH as u32),
        [
            foreign_account_1.id().suffix(),
            foreign_account_1.id().prefix().as_felt(),
            ZERO,
            foreign_account_1.nonce()
        ]
    );

    // check that the first word of the second foreign account slot is correct
    assert_eq!(
        read_root_mem_word(
            &process.into(),
            NATIVE_ACCOUNT_DATA_PTR + ACCOUNT_DATA_LENGTH as u32 * 2
        ),
        [
            foreign_account_2.id().suffix(),
            foreign_account_2.id().prefix().as_felt(),
            ZERO,
            foreign_account_2.nonce()
        ]
    );

    // check that the first word of the third foreign account slot was not initialized
    assert_eq!(
        try_read_root_mem_word(
            &process.into(),
            NATIVE_ACCOUNT_DATA_PTR + ACCOUNT_DATA_LENGTH as u32 * 3
        ),
        None,
        "Memory starting from 32768 should stay uninitialized"
    );
}

#[test]
fn test_fpi_asset_memory() {
    // Num fields must satisfy num_fields % 8 == 0.

    const TREASURY_CAP_ASSET_TYPE: u32 = 3;
    const TREASURY_CAP_NUM_FIELDS: u32 = 8;
    const TREASURY_CAP_FIELD_TYPE_ID_PREFIX: u32 = 0;
    const TREASURY_CAP_FIELD_TYPE_ID_SUFFIX: u32 = 1;
    const TREASURY_CAP_FIELD_OTW_ID: u32 = 2;
    const TREASURY_CAP_FIELD_TOTAL_SUPPLY: u32 = 3;

    const TOKEN_ASSET_TYPE: u32 = 5;
    const TOKEN_NUM_FIELDS: u32 = 8;
    const TOKEN_FIELD_TYPE_ID_PREFIX: u32 = 0;
    const TOKEN_FIELD_TYPE_ID_SUFFIX: u32 = 1;
    const TOKEN_FIELD_OTW_ID: u32 = 2;
    const TOKEN_FIELD_AMOUNT: u32 = 3;

    let miden_std_account_code = format!(
        "
        use.miden::account
        use.miden::asset

        const.TREASURY_CAP_ASSET_TYPE={TREASURY_CAP_ASSET_TYPE}
        const.TREASURY_CAP_NUM_FIELDS={TREASURY_CAP_NUM_FIELDS}
        const.TREASURY_CAP_FIELD_TYPE_ID_PREFIX={TREASURY_CAP_FIELD_TYPE_ID_PREFIX}
        const.TREASURY_CAP_FIELD_TYPE_ID_SUFFIX={TREASURY_CAP_FIELD_TYPE_ID_SUFFIX}
        const.TREASURY_CAP_FIELD_OTW_ID={TREASURY_CAP_FIELD_OTW_ID}
        const.TREASURY_CAP_FIELD_TOTAL_SUPPLY={TREASURY_CAP_FIELD_TOTAL_SUPPLY}

        const.TOKEN_ASSET_TYPE={TOKEN_ASSET_TYPE}
        const.TOKEN_NUM_FIELDS={TOKEN_NUM_FIELDS}
        const.TOKEN_FIELD_TYPE_ID_PREFIX={TOKEN_FIELD_TYPE_ID_PREFIX}
        const.TOKEN_FIELD_TYPE_ID_SUFFIX={TOKEN_FIELD_TYPE_ID_SUFFIX}
        const.TOKEN_FIELD_OTW_ID={TOKEN_FIELD_OTW_ID}
        const.TOKEN_FIELD_AMOUNT={TOKEN_FIELD_AMOUNT}

        #! Inputs:  []
        #! Outputs: [treasury_cap_ptr]
        export.create
            push.TREASURY_CAP_ASSET_TYPE.TREASURY_CAP_NUM_FIELDS
            exec.asset::create
            # => [treasury_cap_ptr, otw_id]

            # consume OTW or abort if it was already consumed
            dup.1 exec.asset::consume_one_time_witness
            # => [treasury_cap_ptr, otw_id]

            swap push.TREASURY_CAP_FIELD_OTW_ID dup.2
            # => [treasury_cap_ptr, field_idx, otw_id, treasury_cap_ptr]
            exec.asset::set_field
            # => [treasury_cap_ptr]

            exec.account::get_native_id
            # => [native_id_prefix, native_id_suffix, treasury_cap_ptr]
            dup.2
            # => [treasury_cap_ptr, native_id_prefix, native_id_suffix, treasury_cap_ptr]

            push.TREASURY_CAP_FIELD_TYPE_ID_PREFIX swap
            # => [treasury_cap_ptr, field_idx, native_id_prefix, native_id_suffix, treasury_cap_ptr]
            exec.asset::set_field
            # => [native_id_suffix, treasury_cap_ptr]

            push.TREASURY_CAP_FIELD_TYPE_ID_SUFFIX dup.2
            # => [treasury_cap_ptr, field_idx, native_id_suffix, treasury_cap_ptr]
            exec.asset::set_field
            # => [treasury_cap_ptr]

            # truncate the stack
            swap drop
        end

        #! Inputs:  [treasury_cap_ptr, amount]
        #! Outputs: [token_ptr]
        export.mint
            dup exec.assert_treasury_cap
            # => [treasury_cap_ptr, amount]

            push.TOKEN_ASSET_TYPE.TOKEN_NUM_FIELDS
            exec.asset::create
            # => [token_ptr, treasury_cap_ptr, amount]

            # copy token flavour from treasury cap to token
            push.TREASURY_CAP_FIELD_OTW_ID dup.2 exec.asset::get_field
            # => [otw_id, token_ptr, treasury_cap_ptr, amount]
            push.TOKEN_FIELD_OTW_ID dup.2
            exec.asset::set_field
            # => [token_ptr, treasury_cap_ptr, amount]

            push.TREASURY_CAP_FIELD_TYPE_ID_PREFIX dup.2 exec.asset::get_field
            # => [issuer_prefix, token_ptr, treasury_cap_ptr, amount]
            push.TOKEN_FIELD_TYPE_ID_PREFIX dup.2
            exec.asset::set_field
            # => [token_ptr, treasury_cap_ptr, amount]

            push.TREASURY_CAP_FIELD_TYPE_ID_SUFFIX dup.2 exec.asset::get_field
            # => [issuer_prefix, token_ptr, treasury_cap_ptr, amount]
            push.TOKEN_FIELD_TYPE_ID_SUFFIX dup.2
            exec.asset::set_field
            # => [token_ptr, treasury_cap_ptr, amount]

            # set amount on token
            dup.2 push.TOKEN_FIELD_AMOUNT dup.2
            # => [token_ptr, amount_field_idx, amount, token_ptr, treasury_cap_ptr, amount]
            exec.asset::set_field
            # => [token_ptr, treasury_cap_ptr, amount]

            # increase total supply in treasury
            push.TREASURY_CAP_FIELD_TOTAL_SUPPLY dup.2 exec.asset::get_field
            # => [total_supply, token_ptr, treasury_cap_ptr, amount]
            movup.3 add
            # => [new_total_supply, token_ptr, treasury_cap_ptr]
            push.TREASURY_CAP_FIELD_TOTAL_SUPPLY movup.3 exec.asset::set_field
            # => [token_ptr]
        end

        #! Inputs:  [asset_ptr]
        #! Outputs: []
        export.move_to_account
            # before allowing the move we could check that the calling account's ID
            # is in the storage of this account to implement a regulated token
            # we could also prevent moving entirely by not exposing a procedure that wraps move_to_account
            # (or an equivalent move_to_note)
            exec.asset::move_to_account
            # => []
        end

        #! Inputs:  [treasury_cap_ptr]
        #! Outputs: []
        proc.assert_treasury_cap
          dup exec.assert_asset_issuer
          # => [treasury_cap_ptr]

          exec.asset::get_asset_type push.TREASURY_CAP_ASSET_TYPE assert_eq.err=13844
          # => []
        end

        #! Inputs:  [asset_ptr]
        #! Outputs: []
        proc.assert_asset_issuer
          exec.asset::get_asset_issuer
          # => [asset_account_id_prefix, asset_account_id_suffix]

          exec.account::get_id
          # => [current_account_id_prefix, current_account_id_suffix, asset_account_id_prefix, asset_account_id_suffix]

          exec.account::is_id_equal assert.err=3421
          # => []
        end
    "
    );

    let miden_std =
        NamedModule::new(LibraryPath::new("miden_std::token").unwrap(), miden_std_account_code);
    let miden_std = TransactionKernel::testing_assembler()
        .with_debug_mode(true)
        .assemble_library([miden_std])
        .unwrap();

    let miden_std_account_component = AccountComponent::new(miden_std.clone(), vec![])
        .unwrap()
        .with_supports_all_types();

    let miden_std_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_component(miden_std_account_component)
        .build_existing()
        .unwrap();

    let mut mock_chain = MockChain::with_accounts(&[miden_std_account.clone()]);
    let native_account = mock_chain.add_new_wallet(crate::testing::Auth::BasicAuth);
    mock_chain.seal_next_block();
    let advice_inputs = get_mock_fpi_adv_inputs(vec![&miden_std_account], &mock_chain);

    const BOB_TOKEN_OTW: u32 = 8;

    let tx_code = format!(
        "
        use.miden::tx
        use.miden::asset
        use.miden_std::token
        use.kernel::prologue

        #! Inputs:  []
        #! Outputs: [treasury_cap_ptr]
        proc.create_bob_treasury_cap
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # Push OTW
            push.{BOB_TOKEN_OTW}
            # => [otw_id, pad(14)]

            # get the hash of the `create` procedure of the miden_std account
            procref.token::create

            # push the miden_std account ID
            push.{miden_std_suffix}.{miden_std_prefix}
            # => [miden_std_id_prefix, miden_std_id_suffix, FOREIGN_PROC_ROOT, pad(15)]

            exec.tx::execute_foreign_procedure
            # => [treasury_cap_ptr]

            # truncate the stack
            movdn.15 dropw dropw dropw drop drop drop
            # => [treasury_cap_ptr]
        end

        #! Inputs:  [treasury_cap_ptr]
        #! Outputs: [token_ptr]
        proc.mint_bob_token
            # pad the stack for the `execute_foreign_procedure` execution
            push.0 padw padw padw movup.13
            # => [treasury_cap_ptr, pad(13)]

            # Push the amount of tokens to mint
            push.100 swap
            # => [treasury_cap_ptr, 100, pad(13)]

            # get the hash of the `mint` procedure of the miden_std account
            procref.token::mint

            # push the miden_std account ID
            push.{miden_std_suffix}.{miden_std_prefix}
            # => [miden_std_id_prefix, miden_std_id_suffix, FOREIGN_PROC_ROOT, treasury_cap_ptr, 100, pad(13)]

            exec.tx::execute_foreign_procedure
            # => [token_ptr]

            # truncate the stack
            movdn.13 dropw dropw dropw drop
            # => [token_ptr]
        end

        #! Inputs:  [asset_ptr]
        #! Outputs: []
        proc.move_bob_token_to_account
            # pad the stack for the `execute_foreign_procedure` execution
            #push.0.0 movdn.2 movdn.2 padw padw padw swapdw swapw
            padw padw padw push.0.0.0 movup.15
            # => [asset_ptr, pad(15)]

            # get the hash of the `move_to_account` procedure of the miden_std account
            procref.token::move_to_account

            # push the miden_std account ID
            push.{miden_std_suffix}.{miden_std_prefix}
            # => [miden_std_id_prefix, miden_std_id_suffix, FOREIGN_PROC_ROOT, asset_ptr, pad(15)]

            exec.tx::execute_foreign_procedure
            # => []

            # truncate the stack
            dropw dropw dropw dropw
            # => []
        end

        begin
            exec.prologue::prepare_transaction

            exec.create_bob_treasury_cap
            # => [treasury_cap_ptr]

            dup exec.mint_bob_token
            # => [token_ptr, treasury_cap_ptr]

            dup exec.move_bob_token_to_account
            # => [token_ptr, treasury_cap_ptr]

            # truncate stack
            swapw dropw
        end
        ",
        miden_std_prefix = miden_std_account.id().prefix().as_felt(),
        miden_std_suffix = miden_std_account.id().suffix(),
    );

    let mut tx_context = mock_chain
        .build_tx_context(native_account.id(), &[], &[])
        .foreign_account_codes(vec![miden_std_account.code().clone()])
        .advice_inputs(advice_inputs.clone())
        .build();

    tx_context.assembler_mut().set_debug_mode(true);
    tx_context.assembler_mut().add_library(miden_std).unwrap();
    let process = &tx_context.execute_code(&tx_code).unwrap();

    let token_ptr = u32::try_from(process.stack.get(0)).unwrap();
    let treasury_cap_ptr = u32::try_from(process.stack.get(1)).unwrap();

    assert_eq!(
        read_mem_felt(process, ASSET_NEXT_PTR),
        Felt::from(
            ASSET_MIN_PTR + 2 * ASSET_BOOKKEEPING_SIZE + TOKEN_NUM_FIELDS + TREASURY_CAP_NUM_FIELDS
        )
    );

    // TREASURY CAP MEMORY ASSERTIONS
    assert_eq!(
        read_root_mem_word(&process.into(), treasury_cap_ptr + ASSET_ISSUER_PREFIX_OFFSET),
        [
            miden_std_account.id().prefix().as_felt(),
            miden_std_account.id().suffix(),
            Felt::from(TREASURY_CAP_ASSET_TYPE),
            Felt::from(TREASURY_CAP_NUM_FIELDS),
        ]
    );
    assert_eq!(
        read_mem_felt(
            process,
            treasury_cap_ptr + ASSET_BOOKKEEPING_SIZE + TREASURY_CAP_FIELD_TYPE_ID_PREFIX
        ),
        native_account.id().prefix().as_felt()
    );
    assert_eq!(
        read_mem_felt(
            process,
            treasury_cap_ptr + ASSET_BOOKKEEPING_SIZE + TREASURY_CAP_FIELD_TYPE_ID_SUFFIX
        ),
        native_account.id().suffix()
    );
    assert_eq!(
        read_mem_felt(
            process,
            treasury_cap_ptr + ASSET_BOOKKEEPING_SIZE + TREASURY_CAP_FIELD_OTW_ID
        ),
        Felt::from(BOB_TOKEN_OTW)
    );
    assert_eq!(
        read_mem_felt(
            process,
            treasury_cap_ptr + ASSET_BOOKKEEPING_SIZE + TREASURY_CAP_FIELD_TOTAL_SUPPLY
        ),
        Felt::from(100u32)
    );

    // TOKEN MEMORY ASSERTIONS
    assert_eq!(
        read_root_mem_word(&process.into(), token_ptr + ASSET_ISSUER_PREFIX_OFFSET),
        [
            miden_std_account.id().prefix().as_felt(),
            miden_std_account.id().suffix(),
            Felt::from(TOKEN_ASSET_TYPE),
            Felt::from(TOKEN_NUM_FIELDS),
        ]
    );
    assert_eq!(
        read_mem_felt(process, token_ptr + ASSET_BOOKKEEPING_SIZE + TOKEN_FIELD_TYPE_ID_PREFIX),
        native_account.id().prefix().as_felt()
    );
    assert_eq!(
        read_mem_felt(process, token_ptr + ASSET_BOOKKEEPING_SIZE + TOKEN_FIELD_TYPE_ID_SUFFIX),
        native_account.id().suffix()
    );
    assert_eq!(
        read_mem_felt(process, token_ptr + ASSET_BOOKKEEPING_SIZE + TOKEN_FIELD_OTW_ID),
        Felt::from(BOB_TOKEN_OTW)
    );
}

pub fn read_mem_felt<'process>(process: impl Into<ProcessState<'process>>, addr: u32) -> Felt {
    process.into().get_mem_value(ContextId::root(), addr).unwrap()
}

/// Test the correctness of the foreign procedure execution.
///
/// It checks the foreign account code loading, providing the mast forest to the executor,
/// construction of the account procedure maps and execution the foreign procedure in order to
/// obtain the data from the foreign account's storage slot.
#[test]
fn test_fpi_execute_foreign_procedure() {
    // Prepare the test data
    let storage_slots =
        vec![AccountStorage::mock_item_0().slot, AccountStorage::mock_item_2().slot];
    let foreign_account_code_source = "
        use.miden::account

        export.get_item_foreign
            # make this foreign procedure unique to make sure that we invoke the procedure of the 
            # foreign account, not the native one
            push.1 drop
            exec.account::get_item

            # truncate the stack
            movup.6 movup.6 movup.6 drop drop drop
        end

        export.get_map_item_foreign
            # make this foreign procedure unique to make sure that we invoke the procedure of the 
            # foreign account, not the native one
            push.2 drop
            exec.account::get_map_item
        end
    ";

    let foreign_account_component = AccountComponent::compile(
        foreign_account_code_source,
        TransactionKernel::testing_assembler(),
        storage_slots,
    )
    .unwrap()
    .with_supports_all_types();

    let foreign_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_component(foreign_account_component)
        .build_existing()
        .unwrap();

    let native_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_component(
            AccountMockComponent::new_with_slots(TransactionKernel::testing_assembler(), vec![])
                .unwrap(),
        )
        .build_existing()
        .unwrap();

    let mut mock_chain =
        MockChain::with_accounts(&[native_account.clone(), foreign_account.clone()]);
    mock_chain.seal_next_block();
    let advice_inputs = get_mock_fpi_adv_inputs(vec![&foreign_account], &mock_chain);

    let code = format!(
        "
        use.std::sys

        use.miden::tx

        begin
            # get the storage item at index 0
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item` account procedure
            push.{get_item_foreign_hash}

            # push the foreign account ID
            push.{foreign_suffix}.{foreign_prefix}
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure
            # => [STORAGE_VALUE]

            # assert the correctness of the obtained value
            push.1.2.3.4 assert_eqw
            # => []

            # get the storage map at index 1
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw push.0.0
            # => [pad(10)]

            # push the key of desired storage item
            push.{map_key}

            # push the index of desired storage item
            push.1

            # get the hash of the `get_map_item_foreign` account procedure
            push.{get_map_item_foreign_hash}

            # push the foreign account ID
            push.{foreign_suffix}.{foreign_prefix}
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, MAP_ITEM_KEY, pad(10)]

            exec.tx::execute_foreign_procedure
            # => [MAP_VALUE]

            # assert the correctness of the obtained value
            push.1.2.3.4 assert_eqw
            # => []

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        foreign_prefix = foreign_account.id().prefix().as_felt(),
        foreign_suffix = foreign_account.id().suffix(),
        get_item_foreign_hash = foreign_account.code().procedures()[0].mast_root(),
        get_map_item_foreign_hash = foreign_account.code().procedures()[1].mast_root(),
        map_key = STORAGE_LEAVES_2[0].0,
    );

    let tx_script =
        TransactionScript::compile(code, vec![], TransactionKernel::testing_assembler()).unwrap();

    let tx_context = mock_chain
        .build_tx_context(native_account.id(), &[], &[])
        .advice_inputs(advice_inputs.clone())
        .tx_script(tx_script)
        .build();

    let block_ref = tx_context.tx_inputs().block_header().block_num();

    let mut executor = TransactionExecutor::new(tx_context.get_data_store(), None).with_tracing();

    // load the mast forest of the foreign account's code to be able to create an account procedure
    // index map and execute the specified foreign procedure
    executor.load_account_code(foreign_account.code());

    let _executed_transaction = executor
        .execute_transaction(native_account.id(), block_ref, &[], tx_context.tx_args().clone())
        .map_err(|e| e.to_string())
        .unwrap();
}

// NESTED FPI TESTS
// ================================================================================================

/// Test the correctness of the cyclic foreign procedure calls.
///
/// It checks that the account data pointers are correctly added and removed from the account data
/// stack.
///
/// The call chain in this test looks like so:
/// `Native -> First FA -> Second FA -> First FA`
#[test]
fn test_nested_fpi_cyclic_invocation() {
    // ------ SECOND FOREIGN ACCOUNT ---------------------------------------------------------------
    let storage_slots = vec![AccountStorage::mock_item_0().slot];
    let second_foreign_account_code_source = "
        use.miden::tx
        use.miden::account
        
        use.std::sys

        export.second_account_foreign_proc
            # get the storage item at index 1
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.1

            # get the hash of the `get_item_foreign` account procedure from the advice stack
            adv_push.4

            # push the foreign account ID from the advice stack
            adv_push.2
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure
            # => [storage_value]

            # make sure that the resulting value equals 5
            dup push.5 assert_eq.err=5678

            # get the first element of the 0'th storage slot (it should be 1) and add it to the 
            # obtained foreign value.
            push.0 exec.account::get_item drop drop drop
            add

            # assert that the resulting value equals 6
            dup push.6 assert_eq.err=9012

            exec.sys::truncate_stack
        end
    ";

    let second_foreign_account_component = AccountComponent::compile(
        second_foreign_account_code_source,
        TransactionKernel::testing_assembler(),
        storage_slots,
    )
    .unwrap()
    .with_supports_all_types();

    let second_foreign_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_component(second_foreign_account_component)
        .build_existing()
        .unwrap();

    // ------ FIRST FOREIGN ACCOUNT ---------------------------------------------------------------
    let storage_slots =
        vec![AccountStorage::mock_item_0().slot, AccountStorage::mock_item_1().slot];
    let first_foreign_account_code_source = "
        use.miden::tx
        use.miden::account

        use.std::sys

        export.first_account_foreign_proc
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0.0
            # => [pad(15)]

            # get the hash of the `second_account_foreign_proc` account procedure from the advice stack
            adv_push.4

            # push the ID of the second foreign account from the advice stack
            adv_push.2
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure
            # => [storage_value]

            # get the second element of the 0'th storage slot (it should be 2) and add it to the 
            # obtained foreign value.
            push.0 exec.account::get_item drop drop swap drop
            add

            # assert that the resulting value equals 8
            dup push.8 assert_eq.err=3456

            exec.sys::truncate_stack
        end

        export.get_item_foreign
            # make this foreign procedure unique to make sure that we invoke the procedure of the 
            # foreign account, not the native one
            push.1 drop
            exec.account::get_item

            # return the first element of the resulting word
            drop drop drop
        end
    ";

    let first_foreign_account_component = AccountComponent::compile(
        first_foreign_account_code_source,
        TransactionKernel::testing_assembler(),
        storage_slots,
    )
    .unwrap()
    .with_supports_all_types();

    let first_foreign_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_component(first_foreign_account_component)
        .build_existing()
        .unwrap();

    // ------ NATIVE ACCOUNT ---------------------------------------------------------------
    let native_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_component(
            AccountMockComponent::new_with_slots(TransactionKernel::testing_assembler(), vec![])
                .unwrap(),
        )
        .build_existing()
        .unwrap();

    let mut mock_chain = MockChain::with_accounts(&[
        native_account.clone(),
        first_foreign_account.clone(),
        second_foreign_account.clone(),
    ]);
    mock_chain.seal_block(None, None);
    let mut advice_inputs =
        get_mock_fpi_adv_inputs(vec![&first_foreign_account, &second_foreign_account], &mock_chain);

    // push the hashes of the foreign procedures and account IDs to the advice stack to be able to
    // call them dynamically.
    advice_inputs.extend_stack(*second_foreign_account.code().procedures()[0].mast_root());
    advice_inputs.extend_stack([
        second_foreign_account.id().suffix(),
        second_foreign_account.id().prefix().as_felt(),
    ]);
    advice_inputs.extend_stack(*first_foreign_account.code().procedures()[1].mast_root());
    advice_inputs.extend_stack([
        first_foreign_account.id().suffix(),
        first_foreign_account.id().prefix().as_felt(),
    ]);

    let code = format!(
        "
        use.std::sys

        use.miden::tx

        begin
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0.0
            # => [pad(15)]

            # get the hash of the `get_item` account procedure
            push.{first_account_foreign_proc_hash}

            # push the foreign account ID
            push.{foreign_suffix}.{foreign_prefix}
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure
            # => [storage_value]
            
            # add 10 to the returning value
            add.10

            # assert that the resulting value equals 18
            push.18 assert_eq.err=1234
            # => []

            exec.sys::truncate_stack
        end
        ",
        foreign_prefix = first_foreign_account.id().prefix().as_felt(),
        foreign_suffix = first_foreign_account.id().suffix(),
        first_account_foreign_proc_hash = first_foreign_account.code().procedures()[0].mast_root(),
    );

    let tx_script = TransactionScript::compile(
        code,
        vec![],
        TransactionKernel::testing_assembler().with_debug_mode(true),
    )
    .unwrap();

    let tx_context = mock_chain
        .build_tx_context(native_account.id(), &[], &[])
        .advice_inputs(advice_inputs.clone())
        .tx_script(tx_script)
        .build();

    let block_ref = tx_context.tx_inputs().block_header().block_num();

    let mut executor = TransactionExecutor::new(tx_context.get_data_store(), None)
        .with_tracing()
        .with_debug_mode();

    // load the mast forest of the foreign account's code to be able to create an account procedure
    // index map and execute the specified foreign procedure
    executor.load_account_code(first_foreign_account.code());
    executor.load_account_code(second_foreign_account.code());

    let _executed_transaction = executor
        .execute_transaction(native_account.id(), block_ref, &[], tx_context.tx_args().clone())
        .map_err(|e| e.to_string())
        .unwrap();
}

/// Test that code will panic in attempt to create more than 63 foreign accounts.
///
/// Attempt to create a 64th foreign account first triggers the assert during the account data
/// loading, but we have an additional assert during the account stack push just in case.
#[test]
fn test_nested_fpi_stack_overflow() {
    // use a custom thread to increase its stack capacity
    std::thread::Builder::new().stack_size(8 * 1_048_576).spawn(||{
        let mut foreign_accounts = Vec::new();

        let last_foreign_account_code_source = "
                use.miden::account

                export.get_item_foreign
                    # make this foreign procedure unique to make sure that we invoke the procedure 
                    # of the foreign account, not the native one
                    push.1 drop

                    # push the index of desired storage item
                    push.0

                    exec.account::get_item

                    # return the first element of the resulting word
                    drop drop drop

                    # make sure that the resulting value equals 1
                    assert
                end
        ";

        let storage_slots = vec![AccountStorage::mock_item_0().slot];
        let last_foreign_account_component = AccountComponent::compile(
            last_foreign_account_code_source,
            TransactionKernel::testing_assembler(),
            storage_slots,
        )
        .unwrap()
        .with_supports_all_types();

        let last_foreign_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
            .with_component(last_foreign_account_component)
            .build_existing()
            .unwrap();

        foreign_accounts.push(last_foreign_account);

        for foreign_account_index in 0..63 {
            let next_account = foreign_accounts.last().unwrap();

            let foreign_account_code_source = format!("
                use.miden::tx
                use.std::sys

                export.read_first_foreign_storage_slot_{foreign_account_index}
                    # pad the stack for the `execute_foreign_procedure` execution
                    padw padw padw push.0.0.0
                    # => [pad(15)]

                    # get the hash of the `get_item` account procedure
                    push.{next_account_proc_hash}

                    # push the foreign account ID
                    push.{next_foreign_suffix}.{next_foreign_prefix}
                    # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

                    exec.tx::execute_foreign_procedure
                    # => [storage_value]

                    exec.sys::truncate_stack
                end
            ", 
                next_account_proc_hash = next_account.code().procedures()[0].mast_root(),
                next_foreign_suffix = next_account.id().suffix(),
                next_foreign_prefix = next_account.id().prefix().as_felt(),
            );

            let foreign_account_component = AccountComponent::compile(
                foreign_account_code_source,
                TransactionKernel::testing_assembler(),
                vec![],
            )
            .unwrap()
            .with_supports_all_types();

            let foreign_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
                .with_component(foreign_account_component)
                .build_existing()
                .unwrap();

            foreign_accounts.push(foreign_account)
        }

        // ------ NATIVE ACCOUNT ---------------------------------------------------------------
        let native_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
            .with_component(
                AccountMockComponent::new_with_slots(TransactionKernel::testing_assembler(), vec![])
                    .unwrap(),
            )
            .build_existing()
            .unwrap();

        let mut mock_chain = MockChain::with_accounts(&[
            vec![native_account.clone()], foreign_accounts.clone()
        ].concat());

        mock_chain.seal_block(None, None);

        let advice_inputs =
            get_mock_fpi_adv_inputs(foreign_accounts.iter().collect::<Vec<&Account>>(), &mock_chain);

        let code = format!(
            "
            use.std::sys

            use.miden::tx

            begin
                # pad the stack for the `execute_foreign_procedure` execution
                padw padw padw push.0.0.0
                # => [pad(15)]

                # get the hash of the `get_item` account procedure
                push.{foreign_account_proc_hash}

                # push the foreign account ID
                push.{foreign_suffix}.{foreign_prefix}
                # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

                exec.tx::execute_foreign_procedure
                # => [storage_value]

                exec.sys::truncate_stack
            end
            ",
            foreign_account_proc_hash = foreign_accounts.last().unwrap().code().procedures()[0].mast_root(),
            foreign_prefix = foreign_accounts.last().unwrap().id().prefix().as_felt(),
            foreign_suffix = foreign_accounts.last().unwrap().id().suffix(),
        );

        let tx_script = TransactionScript::compile(
            code,
            vec![],
            TransactionKernel::testing_assembler().with_debug_mode(true),
        )
        .unwrap();

        let tx_context = mock_chain
            .build_tx_context(native_account.id(), &[], &[])
            .advice_inputs(advice_inputs.clone())
            .tx_script(tx_script)
            .build();

        let block_ref = tx_context.tx_inputs().block_header().block_num();

        let mut executor = TransactionExecutor::new(tx_context.get_data_store(), None)
            .with_tracing()
            .with_debug_mode();

        // load the mast forest of the foreign account's code to be able to create an account 
        // procedure index map and execute the specified foreign procedure
        for foreign_account in foreign_accounts {
            executor.load_account_code(foreign_account.code());
        }

        let err = executor
            .execute_transaction(
                native_account.id(),
                block_ref,
                &[],
                tx_context.tx_args().clone(),
            ).unwrap_err();

        let TransactionExecutorError::TransactionProgramExecutionFailed(err) = err else {
            panic!("unexpected error")
        };

        assert_execution_error!(Err::<(), _>(err), ERR_FOREIGN_ACCOUNT_MAX_NUMBER_EXCEEDED);
    }).expect("thread panic external").join().expect("thread panic internal");
}

/// Test that code will panic in attempt to call a procedure from the native account.
#[test]
fn test_nested_fpi_native_account_invocation() {
    // ------ FIRST FOREIGN ACCOUNT ---------------------------------------------------------------
    let foreign_account_code_source = "
        use.miden::tx

        use.std::sys

        export.first_account_foreign_proc
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0.0
            # => [pad(15)]

            # get the hash of the native account procedure from the advice stack
            adv_push.4

            # push the ID of the native account from the advice stack
            adv_push.2
            # => [native_account_id_prefix, native_account_id_suffix, NATIVE_PROC_ROOT, pad(15)]

            exec.tx::execute_foreign_procedure
            # => [storage_value]

            exec.sys::truncate_stack
        end
    ";

    let foreign_account_component = AccountComponent::compile(
        foreign_account_code_source,
        TransactionKernel::testing_assembler(),
        vec![],
    )
    .unwrap()
    .with_supports_all_types();

    let foreign_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_component(foreign_account_component)
        .build_existing()
        .unwrap();

    // ------ NATIVE ACCOUNT ---------------------------------------------------------------
    let native_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_component(
            AccountMockComponent::new_with_slots(TransactionKernel::testing_assembler(), vec![])
                .unwrap(),
        )
        .build_existing()
        .unwrap();

    let mut mock_chain =
        MockChain::with_accounts(&[native_account.clone(), foreign_account.clone()]);
    mock_chain.seal_block(None, None);
    let mut advice_inputs = get_mock_fpi_adv_inputs(vec![&foreign_account], &mock_chain);

    // push the hash of the native procedure and native account IDs to the advice stack to be able
    // to call them dynamically.
    advice_inputs.extend_stack(*native_account.code().procedures()[2].mast_root());
    advice_inputs
        .extend_stack([native_account.id().suffix(), native_account.id().prefix().as_felt()]);

    let code = format!(
        "
        use.std::sys

        use.miden::tx

        begin
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0.0
            # => [pad(15)]

            # get the hash of the `get_item` account procedure
            push.{first_account_foreign_proc_hash}

            # push the foreign account ID
            push.{foreign_suffix}.{foreign_prefix}
            # => [foreign_account_id_prefix, foreign_account_id_suffix, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure
            # => [storage_value]

            exec.sys::truncate_stack
        end
        ",
        foreign_prefix = foreign_account.id().prefix().as_felt(),
        foreign_suffix = foreign_account.id().suffix(),
        first_account_foreign_proc_hash = foreign_account.code().procedures()[0].mast_root(),
    );

    let tx_script = TransactionScript::compile(
        code,
        vec![],
        TransactionKernel::testing_assembler().with_debug_mode(true),
    )
    .unwrap();

    let tx_context = mock_chain
        .build_tx_context(native_account.id(), &[], &[])
        .advice_inputs(advice_inputs.clone())
        .tx_script(tx_script)
        .build();

    let block_ref = tx_context.tx_inputs().block_header().block_num();

    let mut executor = TransactionExecutor::new(tx_context.get_data_store(), None)
        .with_tracing()
        .with_debug_mode();

    // load the mast forest of the foreign account's code to be able to create an account procedure
    // index map and execute the specified foreign procedure
    executor.load_account_code(foreign_account.code());

    let err = executor
        .execute_transaction(native_account.id(), block_ref, &[], tx_context.tx_args().clone())
        .unwrap_err();

    let TransactionExecutorError::TransactionProgramExecutionFailed(err) = err else {
        panic!("unexpected error")
    };

    assert_execution_error!(Err::<(), _>(err), ERR_FOREIGN_ACCOUNT_CONTEXT_AGAINST_NATIVE_ACCOUNT);
}

// HELPER FUNCTIONS
// ================================================================================================

fn get_mock_fpi_adv_inputs(
    foreign_accounts: Vec<&Account>,
    mock_chain: &MockChain,
) -> AdviceInputs {
    let mut advice_inputs = AdviceInputs::default();

    for foreign_account in foreign_accounts {
        TransactionKernel::extend_advice_inputs_for_account(
            &mut advice_inputs,
            &foreign_account.into(),
            foreign_account.code(),
            &foreign_account.storage().get_header(),
            // Provide the merkle path of the foreign account to be able to verify that the account
            // tree has the commitment of this foreign account. Verification is done during the
            // execution of the `kernel::account::validate_current_foreign_account` procedure.
            &MerklePath::new(
                mock_chain
                    .accounts()
                      // TODO: Update.
                    .open(&LeafIndex::<ACCOUNT_TREE_DEPTH>::new(foreign_account.id().prefix().as_felt().as_int()).unwrap())
                    .path
                    .into(),
            ),
        )
        .unwrap();

        for slot in foreign_account.storage().slots() {
            // if there are storage maps, we populate the merkle store and advice map
            if let StorageSlot::Map(map) = slot {
                // extend the merkle store and map with the storage maps
                advice_inputs.extend_merkle_store(map.inner_nodes());
                // populate advice map with Sparse Merkle Tree leaf nodes
                advice_inputs
                    .extend_map(map.leaves().map(|(_, leaf)| (leaf.hash(), leaf.to_elements())));
            }
        }
    }

    advice_inputs
}

fn foreign_account_data_memory_assertions(foreign_account: &Account, process: &Process) {
    let foreign_account_data_ptr = NATIVE_ACCOUNT_DATA_PTR + ACCOUNT_DATA_LENGTH as u32;

    assert_eq!(
        read_root_mem_word(&process.into(), foreign_account_data_ptr + ACCT_ID_AND_NONCE_OFFSET),
        [
            foreign_account.id().suffix(),
            foreign_account.id().prefix().as_felt(),
            ZERO,
            foreign_account.nonce()
        ],
    );

    assert_eq!(
        read_root_mem_word(&process.into(), foreign_account_data_ptr + ACCT_VAULT_ROOT_OFFSET),
        foreign_account.vault().root().as_elements(),
    );

    assert_eq!(
        read_root_mem_word(
            &process.into(),
            foreign_account_data_ptr + ACCT_STORAGE_COMMITMENT_OFFSET
        ),
        Word::from(foreign_account.storage().commitment()),
    );

    assert_eq!(
        read_root_mem_word(&process.into(), foreign_account_data_ptr + ACCT_CODE_COMMITMENT_OFFSET),
        foreign_account.code().commitment().as_elements(),
    );

    assert_eq!(
        read_root_mem_word(
            &process.into(),
            foreign_account_data_ptr + NUM_ACCT_STORAGE_SLOTS_OFFSET
        ),
        [
            u16::try_from(foreign_account.storage().slots().len()).unwrap().into(),
            ZERO,
            ZERO,
            ZERO
        ],
    );

    for (i, elements) in foreign_account
        .storage()
        .as_elements()
        .chunks(StorageSlot::NUM_ELEMENTS_PER_STORAGE_SLOT / 2)
        .enumerate()
    {
        assert_eq!(
            read_root_mem_word(
                &process.into(),
                foreign_account_data_ptr + ACCT_STORAGE_SLOTS_SECTION_OFFSET + (i as u32) * 4
            ),
            Word::try_from(elements).unwrap(),
        )
    }

    assert_eq!(
        read_root_mem_word(&process.into(), foreign_account_data_ptr + NUM_ACCT_PROCEDURES_OFFSET),
        [
            u16::try_from(foreign_account.code().num_procedures()).unwrap().into(),
            ZERO,
            ZERO,
            ZERO
        ],
    );

    for (i, elements) in foreign_account
        .code()
        .as_elements()
        .chunks(AccountProcedureInfo::NUM_ELEMENTS_PER_PROC / 2)
        .enumerate()
    {
        assert_eq!(
            read_root_mem_word(
                &process.into(),
                foreign_account_data_ptr + ACCT_PROCEDURES_SECTION_OFFSET + (i as u32) * 4
            ),
            Word::try_from(elements).unwrap(),
        );
    }
}

struct NamedModule {
    lib_path: LibraryPath,
    code: String,
}

impl NamedModule {
    pub fn new(lib_path: LibraryPath, code: impl Into<String>) -> Self {
        Self { lib_path, code: code.into() }
    }
}

impl Compile for NamedModule {
    fn compile_with_options(
        self,
        source_manager: &dyn assembly::SourceManager,
        mut options: assembly::CompileOptions,
    ) -> Result<std::prelude::v1::Box<miden_objects::assembly::Module>, assembly::Report> {
        options.path = Some(self.lib_path);
        self.code.compile_with_options(source_manager, options)
    }
}
