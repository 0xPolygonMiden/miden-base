use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    sync::Arc,
    vec::Vec,
};

use ::assembly::{
    ast::{Module, ModuleKind},
    LibraryPath,
};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{
        AccountBuilder, AccountCode, AccountComponent, AccountStorage, AccountType, StorageSlot,
    },
    assembly::DefaultSourceManager,
    assets::{Asset, AssetVault, FungibleAsset, NonFungibleAsset},
    notes::{
        Note, NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteHeader, NoteId, NoteInputs,
        NoteMetadata, NoteRecipient, NoteScript, NoteTag, NoteType,
    },
    testing::{
        account_component::AccountMockComponent,
        account_id::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        },
        constants::{FUNGIBLE_ASSET_AMOUNT, NON_FUNGIBLE_ASSET_DATA},
        notes::DEFAULT_NOTE_CODE,
        prepare_word,
        storage::{STORAGE_INDEX_0, STORAGE_INDEX_2},
    },
    transaction::{ProvenTransaction, TransactionArgs, TransactionScript},
    Felt, Word, MIN_PROOF_SECURITY_LEVEL,
};
use miden_prover::ProvingOptions;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use vm_processor::{
    utils::{Deserializable, Serializable},
    Digest, MemAdviceProvider, ONE,
};

use super::{
    LocalTransactionProver, TransactionExecutor, TransactionHost, TransactionProver,
    TransactionVerifier,
};
use crate::{testing::TransactionContextBuilder, TransactionMastStore};

mod kernel_tests;

// TESTS
// ================================================================================================

#[test]
fn transaction_executor_witness() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved()
        .build();

    let executor = TransactionExecutor::new(Arc::new(tx_context.clone()), None);

    let account_id = tx_context.account().id();

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let executed_transaction = executor
        .execute_transaction(account_id, block_ref, &note_ids, tx_context.tx_args().clone())
        .unwrap();

    let tx_inputs = executed_transaction.tx_inputs();
    let tx_args = executed_transaction.tx_args();

    // use the witness to execute the transaction again
    let (stack_inputs, advice_inputs) = TransactionKernel::prepare_inputs(
        tx_inputs,
        tx_args,
        Some(executed_transaction.advice_witness().clone()),
    );
    let mem_advice_provider: MemAdviceProvider = advice_inputs.into();

    // load account/note/tx_script MAST to the mast_store
    let mast_store = Arc::new(TransactionMastStore::new());
    mast_store.load_transaction_code(tx_inputs, tx_args);

    let mut host: TransactionHost<MemAdviceProvider> = TransactionHost::new(
        tx_inputs.account().into(),
        mem_advice_provider,
        mast_store,
        None,
        BTreeSet::new(),
    )
    .unwrap();
    let result = vm_processor::execute(
        &TransactionKernel::main(),
        stack_inputs,
        &mut host,
        Default::default(),
    )
    .unwrap();

    let (advice_provider, _, output_notes, _signatures, _tx_progress) = host.into_parts();
    let (_, map, _) = advice_provider.into_parts();
    let tx_outputs = TransactionKernel::from_transaction_parts(
        result.stack_outputs(),
        &map.into(),
        output_notes,
    )
    .unwrap();

    assert_eq!(executed_transaction.final_account().hash(), tx_outputs.account.hash());
    assert_eq!(executed_transaction.output_notes(), &tx_outputs.output_notes);
}

#[test]
fn executed_transaction_account_delta_new() {
    let account_assets = AssetVault::mock().assets().collect::<Vec<Asset>>();
    let account = AccountBuilder::new()
        .init_seed(ChaCha20Rng::from_entropy().gen())
        .with_component(
            AccountMockComponent::new_with_slots(
                TransactionKernel::testing_assembler(),
                AccountStorage::mock_storage_slots(),
            )
            .unwrap(),
        )
        .with_assets(account_assets)
        .build_existing()
        .unwrap();

    let mut tx_context = TransactionContextBuilder::new(account)
        .with_mock_notes_preserved_with_account_vault_delta()
        .build();

    let new_acct_code_src = "\
    export.account_proc_1
        push.9.9.9.9
        dropw
    end
    ";

    let component = AccountComponent::compile(
        new_acct_code_src,
        TransactionKernel::testing_assembler(),
        vec![],
    )
    .unwrap()
    .with_supports_all_types();
    let new_acct_code =
        AccountCode::from_components(&[component], AccountType::RegularAccountUpdatableCode)
            .unwrap();

    // updated storage
    let updated_slot_value = [Felt::new(7), Felt::new(9), Felt::new(11), Felt::new(13)];

    // updated storage map
    let updated_map_key = [Felt::new(14), Felt::new(15), Felt::new(16), Felt::new(17)];
    let updated_map_value = [Felt::new(18), Felt::new(19), Felt::new(20), Felt::new(21)];

    // removed assets
    let removed_asset_1 = FungibleAsset::mock(FUNGIBLE_ASSET_AMOUNT / 2);
    let removed_asset_2 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT,
        )
        .expect("asset is valid"),
    );
    let removed_asset_3 = NonFungibleAsset::mock(&NON_FUNGIBLE_ASSET_DATA);
    let removed_assets = [removed_asset_1, removed_asset_2, removed_asset_3];

    let tag1 = NoteTag::from_account_id(
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
        NoteExecutionMode::Local,
    )
    .unwrap();
    let tag2 = NoteTag::for_local_use_case(0, 0).unwrap();
    let tag3 = NoteTag::for_local_use_case(0, 0).unwrap();
    let tags = [tag1, tag2, tag3];

    let aux_array = [Felt::new(27), Felt::new(28), Felt::new(29)];

    let note_types = [NoteType::Private; 3];

    tag1.validate(NoteType::Private)
        .expect("note tag 1 should support private notes");
    tag2.validate(NoteType::Private)
        .expect("note tag 2 should support private notes");
    tag3.validate(NoteType::Private)
        .expect("note tag 3 should support private notes");

    let execution_hint_1 = Felt::from(NoteExecutionHint::always());
    let execution_hint_2 = Felt::from(NoteExecutionHint::none());
    let execution_hint_3 = Felt::from(NoteExecutionHint::on_block_slot(1, 1, 1));
    let hints = [execution_hint_1, execution_hint_2, execution_hint_3];

    let mut send_asset_script = String::new();
    for i in 0..3 {
        send_asset_script.push_str(&format!(
            "
            ### note {i}
            # prepare the stack for a new note creation
            push.0.1.2.3            # recipient
            push.{EXECUTION_HINT} # note_execution_hint
            push.{NOTETYPE}        # note_type
            push.{aux}             # aux
            push.{tag}             # tag
            # => [tag, aux, note_type, execution_hint, RECIPIENT]

            # pad the stack before calling the `create_note`
            padw padw swapdw
            # => [tag, aux, note_type, execution_hint, RECIPIENT, pad(8)]

            # create the note
            call.::miden::contracts::wallets::basic::create_note
            # => [note_idx, pad(15)]

            # move an asset to the created note to partially deplete fungible asset balance
            swapw dropw push.{REMOVED_ASSET}
            call.::miden::contracts::wallets::basic::move_asset_to_note
            # => [ASSET, note_idx, pad(11)]

            # clear the stack
            dropw dropw dropw dropw

        ",
            EXECUTION_HINT = hints[i],
            NOTETYPE = note_types[i] as u8,
            aux = aux_array[i],
            tag = tags[i],
            REMOVED_ASSET = prepare_word(&Word::from(removed_assets[i]))
        ));
    }

    let tx_script_src = format!(
        "\
        use.test::account

        ## TRANSACTION SCRIPT
        ## ========================================================================================
        begin
            ## Update account storage item
            ## ------------------------------------------------------------------------------------
            # push a new value for the storage slot onto the stack
            push.{UPDATED_SLOT_VALUE}
            # => [13, 11, 9, 7]

            # get the index of account storage slot
            push.{STORAGE_INDEX_0}
            # => [idx, 13, 11, 9, 7]
            # update the storage value
            call.account::set_item dropw dropw
            # => []

            ## Update account storage map
            ## ------------------------------------------------------------------------------------
            # push a new VALUE for the storage map onto the stack
            push.{UPDATED_MAP_VALUE}
            # => [18, 19, 20, 21]

            # push a new KEY for the storage map onto the stack
            push.{UPDATED_MAP_KEY}
            # => [14, 15, 16, 17, 18, 19, 20, 21]

            # get the index of account storage slot
            push.{STORAGE_INDEX_2}
            # => [idx, 14, 15, 16, 17, 18, 19, 20, 21]

            # update the storage value
            call.account::set_map_item dropw dropw dropw
            # => []

            ## Send some assets from the account vault
            ## ------------------------------------------------------------------------------------
            {send_asset_script}

            ## Update account code
            ## ------------------------------------------------------------------------------------
            push.{NEW_ACCOUNT_COMMITMENT} call.account::set_code dropw
            # => []
            dropw dropw dropw dropw
            ## Update the account nonce
            ## ------------------------------------------------------------------------------------
            push.1 call.account::incr_nonce drop
            # => []
        end
    ",
        NEW_ACCOUNT_COMMITMENT = prepare_word(&new_acct_code.commitment()),
        UPDATED_SLOT_VALUE = prepare_word(&Word::from(updated_slot_value)),
        UPDATED_MAP_VALUE = prepare_word(&Word::from(updated_map_value)),
        UPDATED_MAP_KEY = prepare_word(&Word::from(updated_map_key)),
    );

    let tx_script = TransactionScript::compile(
        tx_script_src,
        [],
        TransactionKernel::testing_assembler_with_mock_account(),
    )
    .unwrap();

    let tx_args = TransactionArgs::new(
        Some(tx_script),
        None,
        tx_context.tx_args().advice_inputs().clone().map,
    );

    tx_context.set_tx_args(tx_args);

    // expected delta
    // --------------------------------------------------------------------------------------------
    // execute the transaction and get the witness
    let executed_transaction = tx_context.clone().execute().unwrap();

    // nonce delta
    // --------------------------------------------------------------------------------------------
    assert_eq!(executed_transaction.account_delta().nonce(), Some(Felt::new(2)));

    // storage delta
    // --------------------------------------------------------------------------------------------
    // We expect one updated item and one updated map
    assert_eq!(executed_transaction.account_delta().storage().values().len(), 1);
    assert_eq!(
        executed_transaction.account_delta().storage().values().get(&STORAGE_INDEX_0),
        Some(&updated_slot_value)
    );

    assert_eq!(executed_transaction.account_delta().storage().maps().len(), 1);
    assert_eq!(
        executed_transaction
            .account_delta()
            .storage()
            .maps()
            .get(&STORAGE_INDEX_2)
            .unwrap()
            .leaves(),
        &Some((updated_map_key.into(), updated_map_value))
            .into_iter()
            .collect::<BTreeMap<Digest, _>>()
    );

    // vault delta
    // --------------------------------------------------------------------------------------------
    // assert that added assets are tracked
    let added_assets = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .find(|n| n.note().assets().num_assets() == 3)
        .unwrap()
        .note()
        .assets()
        .iter()
        .cloned()
        .collect::<Vec<_>>();

    assert!(executed_transaction
        .account_delta()
        .vault()
        .added_assets()
        .all(|x| added_assets.contains(&x)));
    assert_eq!(
        added_assets.len(),
        executed_transaction.account_delta().vault().added_assets().count()
    );

    // assert that removed assets are tracked
    assert!(executed_transaction
        .account_delta()
        .vault()
        .removed_assets()
        .all(|x| removed_assets.contains(&x)));
    assert_eq!(
        removed_assets.len(),
        executed_transaction.account_delta().vault().removed_assets().count()
    );
}

#[test]
fn test_empty_delta_nonce_update() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let executor = TransactionExecutor::new(Arc::new(tx_context.clone()), None);
    let account_id = tx_context.tx_inputs().account().id();

    let tx_script_src = "
        use.test::account
        begin
            push.1

            call.account::incr_nonce
            # => [0, 1]

            drop drop
            # => []
        end
    ";

    let tx_script = TransactionScript::compile(
        tx_script_src,
        [],
        TransactionKernel::testing_assembler_with_mock_account(),
    )
    .unwrap();
    let tx_args = TransactionArgs::new(
        Some(tx_script),
        None,
        tx_context.tx_args().advice_inputs().clone().map,
    );

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    // expected delta
    // --------------------------------------------------------------------------------------------
    // execute the transaction and get the witness
    let executed_transaction =
        executor.execute_transaction(account_id, block_ref, &note_ids, tx_args).unwrap();

    // nonce delta
    // --------------------------------------------------------------------------------------------
    assert_eq!(executed_transaction.account_delta().nonce(), Some(Felt::new(2)));

    // storage delta
    // --------------------------------------------------------------------------------------------
    // We expect one updated item and one updated map
    assert_eq!(executed_transaction.account_delta().storage().values().len(), 0);

    assert_eq!(executed_transaction.account_delta().storage().maps().len(), 0);
}

#[test]
fn test_send_note_proc() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved_with_account_vault_delta()
        .build();

    let executor = TransactionExecutor::new(Arc::new(tx_context.clone()), None).with_debug_mode();
    let account_id = tx_context.tx_inputs().account().id();

    // removed assets
    let removed_asset_1 = FungibleAsset::mock(FUNGIBLE_ASSET_AMOUNT / 2);
    let removed_asset_2 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT,
        )
        .expect("asset is valid"),
    );
    let removed_asset_3 = NonFungibleAsset::mock(&NON_FUNGIBLE_ASSET_DATA);

    let tag = NoteTag::from_account_id(
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
        NoteExecutionMode::Local,
    )
    .unwrap();
    let aux = Felt::new(27);
    let note_type = NoteType::Private;

    tag.validate(note_type).expect("note tag should support private notes");

    // prepare the asset vector to be removed for each test variant
    let assets_matrix = vec![
        vec![],
        vec![removed_asset_1],
        vec![removed_asset_1, removed_asset_2],
        vec![removed_asset_1, removed_asset_2, removed_asset_3],
    ];

    for removed_assets in assets_matrix {
        // Prepare the string containing the procedures required for adding assets to the note.
        // Depending on the number of the assets to remove, the resulting string will be extended
        // with the corresponding number of procedure "blocks"
        let mut assets_to_remove = String::new();
        for asset in removed_assets.iter() {
            assets_to_remove.push_str(&format!(
                "\n
            # prepare the stack for the next call
            dropw

            # push the asset to be removed
            push.{ASSET}
            # => [ASSET, note_idx, GARBAGE(11)]

            call.wallet::move_asset_to_note
            # => [ASSET, note_idx, GARBAGE(11)]\n",
                ASSET = prepare_word(&asset.into())
            ))
        }

        let tx_script_src = format!(
            "\
            use.miden::contracts::wallets::basic->wallet
            use.miden::tx
            use.test::account

            ## TRANSACTION SCRIPT
            ## ========================================================================================
            begin
                # prepare the values for note creation
                push.1.2.3.4      # recipient
                push.1            # note_execution_hint (NoteExecutionHint::Always)
                push.{note_type}  # note_type
                push.{aux}        # aux
                push.{tag}        # tag
                # => [tag, aux, note_type, RECIPIENT, ...]

                # pad the stack with zeros before calling the `create_note`.
                padw padw swapdw
                # => [tag, aux, execution_hint, note_type, RECIPIENT, pad(8) ...]

                call.wallet::create_note
                # => [note_idx, GARBAGE(15)]

                movdn.4
                # => [GARBAGE(4), note_idx, GARBAGE(11)]

                {assets_to_remove}

                dropw dropw dropw dropw

                ## Update the account nonce
                ## ------------------------------------------------------------------------------------
                push.1 call.account::incr_nonce drop
                # => []
            end
        ",
            note_type = note_type as u8,
        );

        let tx_script = TransactionScript::compile(
            tx_script_src,
            [],
            TransactionKernel::testing_assembler_with_mock_account(),
        )
        .unwrap();
        let tx_args = TransactionArgs::new(
            Some(tx_script),
            None,
            tx_context.tx_args().advice_inputs().clone().map,
        );

        let block_ref = tx_context.tx_inputs().block_header().block_num();
        let note_ids = tx_context
            .tx_inputs()
            .input_notes()
            .iter()
            .map(|note| note.id())
            .collect::<Vec<_>>();

        // expected delta
        // --------------------------------------------------------------------------------------------
        // execute the transaction and get the witness
        let executed_transaction =
            executor.execute_transaction(account_id, block_ref, &note_ids, tx_args).unwrap();

        // nonce delta
        // --------------------------------------------------------------------------------------------
        assert_eq!(executed_transaction.account_delta().nonce(), Some(Felt::new(2)));

        // vault delta
        // --------------------------------------------------------------------------------------------
        // assert that removed assets are tracked
        assert!(executed_transaction
            .account_delta()
            .vault()
            .removed_assets()
            .all(|x| removed_assets.contains(&x)));
        assert_eq!(
            removed_assets.len(),
            executed_transaction.account_delta().vault().removed_assets().count()
        );
    }
}

#[test]
fn executed_transaction_output_notes() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved_with_account_vault_delta()
        .build();

    let executor = TransactionExecutor::new(Arc::new(tx_context.clone()), None).with_debug_mode();
    let account_id = tx_context.tx_inputs().account().id();

    // removed assets
    let removed_asset_1 = FungibleAsset::mock(FUNGIBLE_ASSET_AMOUNT / 2);
    let removed_asset_2 = FungibleAsset::mock(FUNGIBLE_ASSET_AMOUNT / 2);

    let combined_asset = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT,
        )
        .expect("asset is valid"),
    );
    let removed_asset_3 = NonFungibleAsset::mock(&NON_FUNGIBLE_ASSET_DATA);
    let removed_asset_4 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT / 2,
        )
        .expect("asset is valid"),
    );

    let tag1 = NoteTag::from_account_id(
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
        NoteExecutionMode::Local,
    )
    .unwrap();
    let tag2 = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local).unwrap();
    let tag3 = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local).unwrap();
    let aux1 = Felt::new(27);
    let aux2 = Felt::new(28);
    let aux3 = Felt::new(29);

    let note_type1 = NoteType::Private;
    let note_type2 = NoteType::Public;
    let note_type3 = NoteType::Public;

    tag1.validate(note_type1).expect("note tag 1 should support private notes");
    tag2.validate(note_type2).expect("note tag 2 should support public notes");
    tag3.validate(note_type3).expect("note tag 3 should support public notes");

    // In this test we create 3 notes. Note 1 is private, Note 2 is public and Note 3 is public
    // without assets.

    // Create the expected output note for Note 2 which is public
    let serial_num_2 = Word::from([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]);
    let note_script_2 =
        NoteScript::compile(DEFAULT_NOTE_CODE, TransactionKernel::testing_assembler()).unwrap();
    let inputs_2 = NoteInputs::new(vec![]).unwrap();
    let metadata_2 =
        NoteMetadata::new(account_id, note_type2, tag2, NoteExecutionHint::none(), aux2).unwrap();
    let vault_2 = NoteAssets::new(vec![removed_asset_3, removed_asset_4]).unwrap();
    let recipient_2 = NoteRecipient::new(serial_num_2, note_script_2, inputs_2);
    let expected_output_note_2 = Note::new(vault_2, metadata_2, recipient_2);

    // Create the expected output note for Note 3 which is public
    let serial_num_3 = Word::from([Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)]);
    let note_script_3 =
        NoteScript::compile(DEFAULT_NOTE_CODE, TransactionKernel::testing_assembler()).unwrap();
    let inputs_3 = NoteInputs::new(vec![]).unwrap();
    let metadata_3 = NoteMetadata::new(
        account_id,
        note_type3,
        tag3,
        NoteExecutionHint::on_block_slot(1, 2, 3),
        aux3,
    )
    .unwrap();
    let vault_3 = NoteAssets::new(vec![]).unwrap();
    let recipient_3 = NoteRecipient::new(serial_num_3, note_script_3, inputs_3);
    let expected_output_note_3 = Note::new(vault_3, metadata_3, recipient_3);

    let tx_script_src = format!(
        "\
        use.miden::contracts::wallets::basic->wallet
        use.test::account

        # Inputs:  [tag, aux, note_type, execution_hint, RECIPIENT]
        # Outputs: [note_idx]
        proc.create_note
            # pad the stack before the call to prevent accidental modification of the deeper stack
            # elements
            padw padw swapdw
            # => [tag, aux, execution_hint, note_type, RECIPIENT, pad(8)]

            call.wallet::create_note
            # => [note_idx, pad(15)]

            # remove excess PADs from the stack
            swapdw dropw dropw movdn.7 dropw drop drop drop
            # => [note_idx]
        end

        # Inputs:  [ASSET, note_idx]
        # Outputs: [ASSET, note_idx]
        proc.move_asset_to_note
            # pad the stack before call
            push.0.0.0 movdn.7 movdn.7 movdn.7 padw padw swapdw
            # => [ASSET, note_idx, pad(11)]

            call.wallet::move_asset_to_note
            # => [ASSET, note_idx, pad(11)]

            # remove excess PADs from the stack
            swapdw dropw dropw swapw movdn.7 drop drop drop
            # => [ASSET, note_idx]
        end

        ## TRANSACTION SCRIPT
        ## ========================================================================================
        begin
            ## Send some assets from the account vault
            ## ------------------------------------------------------------------------------------
            # partially deplete fungible asset balance
            push.0.1.2.3                        # recipient
            push.{EXECUTION_HINT_1}             # note execution hint
            push.{NOTETYPE1}                    # note_type
            push.{aux1}                         # aux
            push.{tag1}                         # tag
            exec.create_note
            # => [note_idx]
            
            push.{REMOVED_ASSET_1}              # asset_1
            # => [ASSET, note_idx]

            exec.move_asset_to_note dropw
            # => [note_idx]

            push.{REMOVED_ASSET_2}              # asset_2
            exec.move_asset_to_note dropw drop
            # => []

            # send non-fungible asset
            push.{RECIPIENT2}                   # recipient
            push.{EXECUTION_HINT_2}             # note execution hint
            push.{NOTETYPE2}                    # note_type
            push.{aux2}                         # aux
            push.{tag2}                         # tag
            exec.create_note
            # => [note_idx]

            push.{REMOVED_ASSET_3}              # asset_3
            exec.move_asset_to_note dropw
            # => [note_idx]

            push.{REMOVED_ASSET_4}              # asset_4
            exec.move_asset_to_note dropw drop
            # => []

            # create a public note without assets
            push.{RECIPIENT3}                   # recipient
            push.{EXECUTION_HINT_3}             # note execution hint
            push.{NOTETYPE3}                    # note_type
            push.{aux3}                         # aux
            push.{tag3}                         # tag
            exec.create_note drop
            # => []

            ## Update the account nonce
            ## ------------------------------------------------------------------------------------
            push.1 call.account::incr_nonce drop
            # => []
        end
    ",
        REMOVED_ASSET_1 = prepare_word(&Word::from(removed_asset_1)),
        REMOVED_ASSET_2 = prepare_word(&Word::from(removed_asset_2)),
        REMOVED_ASSET_3 = prepare_word(&Word::from(removed_asset_3)),
        REMOVED_ASSET_4 = prepare_word(&Word::from(removed_asset_4)),
        RECIPIENT2 = prepare_word(&Word::from(expected_output_note_2.recipient().digest())),
        RECIPIENT3 = prepare_word(&Word::from(expected_output_note_3.recipient().digest())),
        NOTETYPE1 = note_type1 as u8,
        NOTETYPE2 = note_type2 as u8,
        NOTETYPE3 = note_type3 as u8,
        EXECUTION_HINT_1 = Felt::from(NoteExecutionHint::always()),
        EXECUTION_HINT_2 = Felt::from(NoteExecutionHint::none()),
        EXECUTION_HINT_3 = Felt::from(NoteExecutionHint::on_block_slot(11, 22, 33)),
    );

    let tx_script = TransactionScript::compile(
        tx_script_src,
        [],
        TransactionKernel::testing_assembler_with_mock_account(),
    )
    .unwrap();
    let mut tx_args = TransactionArgs::new(
        Some(tx_script),
        None,
        tx_context.tx_args().advice_inputs().clone().map,
    );

    tx_args.add_expected_output_note(&expected_output_note_2);
    tx_args.add_expected_output_note(&expected_output_note_3);

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    // expected delta
    // --------------------------------------------------------------------------------------------
    // execute the transaction and get the witness

    let executed_transaction =
        executor.execute_transaction(account_id, block_ref, &note_ids, tx_args).unwrap();

    // output notes
    // --------------------------------------------------------------------------------------------
    let output_notes = executed_transaction.output_notes();

    // assert that the expected output note is present
    // NOTE: the mock state already contains 3 output notes
    assert_eq!(output_notes.num_notes(), 6);

    let output_note_id_3 = executed_transaction.output_notes().get_note(3).id();
    let recipient_3 = Digest::from([Felt::new(0), Felt::new(1), Felt::new(2), Felt::new(3)]);
    let note_assets_3 = NoteAssets::new(vec![combined_asset]).unwrap();
    let expected_note_id_3 = NoteId::new(recipient_3, note_assets_3.commitment());
    assert_eq!(output_note_id_3, expected_note_id_3);

    // assert that the expected output note 2 is present
    let output_note = executed_transaction.output_notes().get_note(4);
    let note_id = expected_output_note_2.id();
    let note_metadata = expected_output_note_2.metadata();
    assert_eq!(NoteHeader::from(output_note), NoteHeader::new(note_id, *note_metadata));

    // assert that the expected output note 3 is present and has no assets
    let output_note_3 = executed_transaction.output_notes().get_note(5);
    assert_eq!(expected_output_note_3.id(), output_note_3.id());
    assert_eq!(expected_output_note_3.assets(), output_note_3.assets().unwrap());
}

#[test]
fn prove_witness_and_verify() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved()
        .build();

    let account_id = tx_context.tx_inputs().account().id();

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let executor = TransactionExecutor::new(Arc::new(tx_context.clone()), None);
    let executed_transaction = executor
        .execute_transaction(account_id, block_ref, &note_ids, tx_context.tx_args().clone())
        .unwrap();
    let executed_transaction_id = executed_transaction.id();

    let proof_options = ProvingOptions::default();
    let prover = LocalTransactionProver::new(proof_options);
    let proven_transaction = prover.prove(executed_transaction.into()).unwrap();

    assert_eq!(proven_transaction.id(), executed_transaction_id);

    let serialized_transaction = proven_transaction.to_bytes();
    let proven_transaction = ProvenTransaction::read_from_bytes(&serialized_transaction).unwrap();
    let verifier = TransactionVerifier::new(MIN_PROOF_SECURITY_LEVEL);
    assert!(verifier.verify(proven_transaction).is_ok());
}

// TEST TRANSACTION SCRIPT
// ================================================================================================

#[test]
fn test_tx_script() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved()
        .build();
    let executor = TransactionExecutor::new(Arc::new(tx_context.clone()), None);

    let account_id = tx_context.tx_inputs().account().id();

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let tx_script_input_key = [Felt::new(9999), Felt::new(8888), Felt::new(9999), Felt::new(8888)];
    let tx_script_input_value = [Felt::new(9), Felt::new(8), Felt::new(7), Felt::new(6)];
    let tx_script_src = format!(
        "
    begin
        # push the tx script input key onto the stack
        push.{key}

        # load the tx script input value from the map and read it onto the stack
        adv.push_mapval push.16073 drop         # TODO: remove line, see miden-vm/#1122
        adv_loadw

        # assert that the value is correct
        push.{value} assert_eqw
    end
",
        key = prepare_word(&tx_script_input_key),
        value = prepare_word(&tx_script_input_value)
    );

    let tx_script = TransactionScript::compile(
        tx_script_src,
        [(tx_script_input_key, tx_script_input_value.into())],
        TransactionKernel::testing_assembler(),
    )
    .unwrap();
    let tx_args = TransactionArgs::new(
        Some(tx_script),
        None,
        tx_context.tx_args().advice_inputs().clone().map,
    );

    let executed_transaction =
        executor.execute_transaction(account_id, block_ref, &note_ids, tx_args);

    assert!(
        executed_transaction.is_ok(),
        "Transaction execution failed {:?}",
        executed_transaction,
    );
}

/// Tests that an account can call code in a custom library when loading that library into the
/// executor.
///
/// The call chain and dependency graph in this test is:
/// `tx script -> account code -> external library`
#[test]
fn transaction_executor_account_code_using_custom_library() {
    const EXTERNAL_LIBRARY_CODE: &str = "
      use.miden::account

      export.incr_nonce_by_four
        dup eq.4 assert.err=42 exec.account::incr_nonce
      end";

    const ACCOUNT_COMPONENT_CODE: &str = "
      use.external_library::external_module

      export.custom_nonce_incr
        push.4 exec.external_module::incr_nonce_by_four
      end";

    let source_manager = Arc::new(DefaultSourceManager::default());
    let external_library_module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new("external_library::external_module").unwrap(),
            EXTERNAL_LIBRARY_CODE,
            &source_manager,
        )
        .unwrap();
    let external_library = TransactionKernel::assembler()
        .assemble_library([external_library_module])
        .unwrap();

    let mut assembler = TransactionKernel::assembler();
    assembler.add_library(&external_library).unwrap();

    let account_component_module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new("account_component::account_module").unwrap(),
            ACCOUNT_COMPONENT_CODE,
            &source_manager,
        )
        .unwrap();

    let account_component_lib =
        assembler.clone().assemble_library([account_component_module]).unwrap();

    let tx_script_src = "\
          use.account_component::account_module

          begin
            call.account_module::custom_nonce_incr
          end";

    let account_component =
        AccountComponent::new(account_component_lib.clone(), vec![StorageSlot::empty_value()])
            .unwrap()
            .with_supports_all_types();

    // Build an existing account with nonce 1.
    let native_account = AccountBuilder::new()
        .init_seed(ChaCha20Rng::from_entropy().gen())
        .with_component(account_component)
        .build_existing()
        .unwrap();

    let tx_context = TransactionContextBuilder::new(native_account).build();

    let tx_script = TransactionScript::compile(
        tx_script_src,
        [],
        // Add the account component library since the transaction script is calling the account's
        // procedure.
        assembler.with_library(&account_component_lib).unwrap(),
    )
    .unwrap();

    let tx_args = TransactionArgs::new(
        Some(tx_script),
        None,
        tx_context.tx_args().advice_inputs().clone().map,
    );

    let mut executor = TransactionExecutor::new(Arc::new(tx_context.clone()), None);
    // Load the external library into the executor to make it available during transaction
    // execution.
    executor.load_library(&external_library);

    let account_id = tx_context.account().id();
    let block_ref = tx_context.tx_inputs().block_header().block_num();

    let executed_tx = executor.execute_transaction(account_id, block_ref, &[], tx_args).unwrap();

    // Account's initial nonce of 1 should have been incremented by 4.
    assert_eq!(executed_tx.account_delta().nonce().unwrap(), Felt::new(5));
}
