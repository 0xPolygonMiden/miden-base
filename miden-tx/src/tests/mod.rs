use alloc::{string::ToString, vec::Vec};

use miden_lib::transaction::{ToTransactionKernelInputs, TransactionKernel};
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        },
        AccountCode,
    },
    assembly::ProgramAst,
    assets::{Asset, FungibleAsset},
    notes::{NoteAssets, NoteExecutionHint, NoteHeader, NoteId, NoteTag, NoteType},
    testing::{
        account_code::{
            ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT, ACCOUNT_CREATE_NOTE_MAST_ROOT,
            ACCOUNT_INCR_NONCE_MAST_ROOT, ACCOUNT_REMOVE_ASSET_MAST_ROOT,
            ACCOUNT_SEND_ASSET_MAST_ROOT, ACCOUNT_SET_CODE_MAST_ROOT, ACCOUNT_SET_ITEM_MAST_ROOT,
            ACCOUNT_SET_MAP_ITEM_MAST_ROOT,
        },
        constants::{FUNGIBLE_ASSET_AMOUNT, NON_FUNGIBLE_ASSET_DATA},
        prepare_word,
        storage::{STORAGE_INDEX_0, STORAGE_INDEX_2},
    },
    transaction::{ProvenTransaction, TransactionArgs, TransactionWitness},
    Felt, Word, MIN_PROOF_SECURITY_LEVEL,
};
use miden_prover::ProvingOptions;
use vm_processor::{
    utils::{Deserializable, Serializable},
    AdviceMap, Digest, MemAdviceProvider, ONE,
};

use super::{TransactionExecutor, TransactionHost, TransactionProver, TransactionVerifier};
use crate::testing::{ScriptAndInputs, TransactionContextBuilder};

mod kernel_tests;

/// [TransactionWitness] must produce the same result as its [miden_objects::transaction::PreparedTransaction].
#[test]
fn transaction_executor_witness() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .with_mock_notes_preserved()
    .build();

    let executed_transaction = tx_context
        .execute_transaction(None, AdviceMap::default(), ScriptAndInputs::empty())
        .unwrap();
    let tx_witness: TransactionWitness = executed_transaction.clone().into();

    // use the witness to execute the transaction again
    let (stack_inputs, advice_inputs) = tx_witness.get_kernel_inputs();
    let mem_advice_provider: MemAdviceProvider = advice_inputs.into();
    let mut host: TransactionHost<MemAdviceProvider, ()> =
        TransactionHost::new(tx_witness.account().into(), mem_advice_provider, None);
    let result =
        vm_processor::execute(tx_witness.program(), stack_inputs, &mut host, Default::default())
            .unwrap();

    let (advice_provider, _, output_notes, _signatures) = host.into_parts();
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
fn executed_transaction_account_delta() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .with_mock_notes_preserved_with_account_vault_delta()
    .build();

    let mut executor: TransactionExecutor<_, ()> =
        TransactionExecutor::new(tx_context.clone(), None);
    let account_id = tx_context.tx_inputs().account().id();
    executor.load_account(account_id).unwrap();

    let new_acct_code = AccountCode::from_code(
        "
        export.account_proc_1
            push.9.9.9.9
            dropw
        end
        ",
    )
    .unwrap();

    // updated storage
    let updated_slot_value = [Felt::new(7), Felt::new(9), Felt::new(11), Felt::new(13)];

    // updated storage map
    let updated_map_key = [Felt::new(14), Felt::new(15), Felt::new(16), Felt::new(17)];
    let updated_map_value = [Felt::new(18), Felt::new(19), Felt::new(20), Felt::new(21)];

    // removed assets
    let removed_asset_1 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT / 2,
        )
        .expect("asset is valid"),
    );
    let removed_asset_2 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT,
        )
        .expect("asset is valid"),
    );
    let removed_asset_3 =
        Asset::mock_non_fungible(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &NON_FUNGIBLE_ASSET_DATA);
    let removed_assets = [removed_asset_1, removed_asset_2, removed_asset_3];

    let tag1 = NoteTag::from_account_id(
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
        NoteExecutionHint::Local,
    )
    .unwrap();
    let tag2 = NoteTag::for_local_use_case(0, 0).unwrap();
    let tag3 = NoteTag::for_local_use_case(0, 0).unwrap();

    let aux1 = Felt::new(27);
    let aux2 = Felt::new(28);
    let aux3 = Felt::new(29);

    let note_type1 = NoteType::OffChain;
    let note_type2 = NoteType::OffChain;
    let note_type3 = NoteType::OffChain;

    assert_eq!(tag1.validate(note_type1), Ok(tag1));
    assert_eq!(tag2.validate(note_type2), Ok(tag2));
    assert_eq!(tag3.validate(note_type3), Ok(tag3));
    let tx_script = format!(
        "
        use.miden::account
        use.miden::contracts::wallets::basic->wallet

        # asserts the stack is currently empty
        # if note, there are issues with the stack management
        proc.stack_empty
            assertz assertz assertz assertz
            assertz assertz assertz assertz
            assertz assertz assertz assertz
            assertz assertz assertz assertz
        end

        begin
            # => [TX_SCRIPT_ROOT]
            dropw

            # Update account storage item
            # -------------------------------------------------------------------------------------

            push.0.0.0
            push.{UPDATED_SLOT_VALUE}
            push.{STORAGE_INDEX_0}
            # => [idx, VALUE, 0, 0, 0]

            call.{ACCOUNT_SET_ITEM_MAST_ROOT}
            # => [R', V]

            dropw dropw exec.stack_empty
            # => []

            # Update account storage map
            # -------------------------------------------------------------------------------------

            push.{UPDATED_MAP_VALUE}
            push.{UPDATED_MAP_KEY}
            push.{STORAGE_INDEX_2}
            # => [idx, KEY, VALUE]

            call.{ACCOUNT_SET_MAP_ITEM_MAST_ROOT}
            # => [R', V]

            dropw dropw exec.stack_empty
            # => []

            # Send some assets from the account vault
            # -------------------------------------------------------------------------------------
            # partially deplete fungible asset balance
            push.0.1.2.3            # recipient
            push.{NOTETYPE1}        # note_type
            push.{aux1}             # aux
            push.{tag1}             # tag
            push.{REMOVED_ASSET_1}  # asset
            call.{ACCOUNT_SEND_ASSET_MAST_ROOT}
            # => [note_idx, EMPTY_WORD, EMPTY_WORD, 0, 0, ...]

            dropw dropw drop drop drop exec.stack_empty
            # => []

            # totally deplete fungible asset balance
            push.0.1.2.3            # recipient
            push.{NOTETYPE2}        # note_type
            push.{aux2}             # aux
            push.{tag2}             # tag
            push.{REMOVED_ASSET_2}  # asset
            call.{ACCOUNT_SEND_ASSET_MAST_ROOT}
            # => [note_idx, EMPTY_WORD, EMPTY_WORD, 0, 0, ...]

            dropw dropw drop drop drop exec.stack_empty
            # => []

            # send non-fungible asset
            push.0.1.2.3            # recipient
            push.{NOTETYPE3}        # note_type
            push.{aux3}             # aux
            push.{tag3}             # tag
            push.{REMOVED_ASSET_3}  # asset
            call.{ACCOUNT_SEND_ASSET_MAST_ROOT}
            # => [note_idx, EMPTY_WORD, EMPTY_WORD, 0, 0, ...]

            dropw dropw drop drop drop exec.stack_empty
            # => []

            # Update account code
            # -------------------------------------------------------------------------------------
            push.{NEW_ACCOUNT_ROOT}
            call.{ACCOUNT_SET_CODE_MAST_ROOT}
            exec.stack_empty
            # => []

            # Update the account nonce
            # -------------------------------------------------------------------------------------
            push.1
            call.{ACCOUNT_INCR_NONCE_MAST_ROOT}
            exec.stack_empty
            # => []
        end
        ",
        NEW_ACCOUNT_ROOT = prepare_word(&new_acct_code.root()),
        UPDATED_SLOT_VALUE = prepare_word(&Word::from(updated_slot_value)),
        UPDATED_MAP_VALUE = prepare_word(&Word::from(updated_map_value)),
        UPDATED_MAP_KEY = prepare_word(&Word::from(updated_map_key)),
        REMOVED_ASSET_1 = prepare_word(&Word::from(removed_asset_1)),
        REMOVED_ASSET_2 = prepare_word(&Word::from(removed_asset_2)),
        REMOVED_ASSET_3 = prepare_word(&Word::from(removed_asset_3)),
        NOTETYPE1 = note_type1 as u8,
        NOTETYPE2 = note_type2 as u8,
        NOTETYPE3 = note_type3 as u8,
    );
    let tx_script_code = ProgramAst::parse(&tx_script).unwrap();
    let tx_script = executor.compile_tx_script(tx_script_code, vec![], vec![]).unwrap();
    let mut tx_args = TransactionArgs::new(Some(tx_script), None, AdviceMap::default());
    let context_output_notes = tx_context.expected_output_notes().to_vec();
    tx_args.extend_expected_output_notes(context_output_notes);

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
    assert_eq!(executed_transaction.account_delta().storage().updated_items.len(), 1);
    assert_eq!(
        executed_transaction.account_delta().storage().updated_items[0].0,
        STORAGE_INDEX_0
    );
    assert_eq!(
        executed_transaction.account_delta().storage().updated_items[0].1,
        updated_slot_value
    );

    assert_eq!(executed_transaction.account_delta().storage().updated_maps.len(), 1);
    assert_eq!(
        executed_transaction.account_delta().storage().updated_maps[0].0,
        STORAGE_INDEX_2
    );
    assert_eq!(
        executed_transaction.account_delta().storage().updated_maps[0].1.updated_leaves[0],
        (updated_map_key, updated_map_value)
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
        .added_assets
        .iter()
        .all(|x| added_assets.contains(x)));
    assert_eq!(
        added_assets.len(),
        executed_transaction.account_delta().vault().added_assets.len()
    );

    // assert that removed assets are tracked
    assert!(executed_transaction
        .account_delta()
        .vault()
        .removed_assets
        .iter()
        .all(|x| removed_assets.contains(x)));
    assert_eq!(
        removed_assets.len(),
        executed_transaction.account_delta().vault().removed_assets.len()
    );
}

#[test]
fn executed_transaction_output_notes() {
    // Assets
    // --------------------------------------------------------------------------------------------
    let removed_asset_1 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT / 2,
        )
        .expect("asset is valid"),
    );
    let removed_asset_2 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT / 2,
        )
        .expect("asset is valid"),
    );
    let combined_asset = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT,
        )
        .expect("asset is valid"),
    );
    let removed_asset_3 =
        Asset::mock_non_fungible(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &NON_FUNGIBLE_ASSET_DATA);
    let removed_asset_4 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT / 2,
        )
        .expect("asset is valid"),
    );

    // Notes
    // --------------------------------------------------------------------------------------------
    //
    // Output notes:
    // - Note 1 is private
    // - Note 2 is public
    // - Note 3 is public without assets

    let mut tx_context_builder = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .with_mock_notes_preserved_with_account_vault_delta();

    let tag1 = NoteTag::from_account_id(
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
        NoteExecutionHint::Local,
    )
    .unwrap();
    let note_type1 = NoteType::OffChain;
    assert_eq!(tag1.validate(note_type1), Ok(tag1));
    let aux1 = Felt::new(27);

    let note2 = tx_context_builder.add_output_note(
        vec![],
        vec![removed_asset_3, removed_asset_4],
        NoteTag::for_public_use_case(0, 0, NoteExecutionHint::Local).unwrap(),
    );
    let note3 = tx_context_builder.add_output_note(
        vec![],
        vec![],
        NoteTag::for_public_use_case(0, 0, NoteExecutionHint::Local).unwrap(),
    );

    let tx_script = format!(
        "
        proc.stack_empty
            assertz assertz assertz assertz
            assertz assertz assertz assertz
            assertz assertz assertz assertz
            assertz assertz assertz assertz
        end

        begin
            # => [TX_SCRIPT_ROOT]
            dropw

            # Output Note 1
            # -------------------------------------------------------------------------------------
            #
            # Partially deplete fungible asset balance

            push.0.1.2.3
            push.{type1}
            push.{aux1}
            push.{tag1}
            # => [tag, aux, note_type, RECIPIENT]

            call.{ACCOUNT_CREATE_NOTE_MAST_ROOT}
            push.{ASSET1}
            call.{ACCOUNT_REMOVE_ASSET_MAST_ROOT}
            # => [ASSET1, idx1]

            movup.4
            call.{ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT}
            # => [idx1]

            push.{ASSET2}
            call.{ACCOUNT_REMOVE_ASSET_MAST_ROOT}
            # => [ASSET2, note_ptr]

            movup.4
            call.{ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT}
            # => [idx1]

            drop exec.stack_empty
            # => []

            # Output Note 2
            # -------------------------------------------------------------------------------------
            #
            # Send non-fungible asset

            push.{RECIPIENT2}
            push.{type2}
            push.{aux2}
            push.{tag2}
            # => [tag, aux, note_type, RECIPIENT]

            call.{ACCOUNT_CREATE_NOTE_MAST_ROOT}
            # => [idx2]

            push.{ASSET3}
            call.{ACCOUNT_REMOVE_ASSET_MAST_ROOT}
            movup.4
            call.{ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT}
            # => [ASSET3, idx2]

            push.{ASSET4}
            call.{ACCOUNT_REMOVE_ASSET_MAST_ROOT}
            movup.4
            call.{ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT}
            # => [ASSET3, idx2]

            drop exec.stack_empty
            # => []

            # Output Note 3
            # -------------------------------------------------------------------------------------
            #
            # create a public note without assets

            push.{RECIPIENT3}
            push.{type3}
            push.{aux3}
            push.{tag3}
            # => [tag, aux, note_type, RECIPIENT]

            call.{ACCOUNT_CREATE_NOTE_MAST_ROOT}
            # => [idx3]

            drop exec.stack_empty
            # => []

            # Update the account nonce
            # -------------------------------------------------------------------------------------
            push.1
            call.{ACCOUNT_INCR_NONCE_MAST_ROOT}
            drop exec.stack_empty
            # => []
        end
        ",
        ASSET1 = prepare_word(&Word::from(removed_asset_1)),
        ASSET2 = prepare_word(&Word::from(removed_asset_2)),
        ASSET3 = prepare_word(&Word::from(removed_asset_3)),
        ASSET4 = prepare_word(&Word::from(removed_asset_4)),
        RECIPIENT2 = prepare_word(&Word::from(note2.recipient().digest())),
        RECIPIENT3 = prepare_word(&Word::from(note3.recipient().digest())),
        type1 = note_type1 as u8,
        type2 = note2.metadata().note_type() as u8,
        type3 = note3.metadata().note_type() as u8,
        aux2 = note2.metadata().aux().as_int(),
        aux3 = note2.metadata().aux().as_int(),
        tag2 = note2.metadata().tag(),
        tag3 = note3.metadata().tag(),
    );

    // Execute transaction and asserts
    // --------------------------------------------------------------------------------------------

    let tx_context = tx_context_builder.build();
    let executed_transaction = tx_context
        .execute_transaction(
            None,
            AdviceMap::default(),
            ScriptAndInputs::new(tx_script, vec![], vec![]),
        )
        .unwrap();

    let output_notes = executed_transaction.output_notes();

    // NOTE: the mock state already contains 3 output notes
    assert_eq!(output_notes.num_notes(), 6);

    let created_note_id_3 = executed_transaction.output_notes().get_note(3).id();
    let recipient_3 = Digest::from([Felt::new(0), Felt::new(1), Felt::new(2), Felt::new(3)]);
    let note_assets_3 = NoteAssets::new(vec![combined_asset]).unwrap();
    let expected_note_id_3 = NoteId::new(recipient_3, note_assets_3.commitment());
    assert_eq!(created_note_id_3, expected_note_id_3);

    // assert that the expected output note 2 is present
    let created_note = executed_transaction.output_notes().get_note(4);
    let note_id = note2.id();
    let note_metadata = note2.metadata();
    assert_eq!(NoteHeader::from(created_note), NoteHeader::new(note_id, *note_metadata));

    // assert that the expected output note 3 is present and has no assets
    let created_note_3 = executed_transaction.output_notes().get_note(5);
    assert_eq!(note3.id(), created_note_3.id());
    assert_eq!(note3.assets(), created_note_3.assets().unwrap());
}

#[test]
fn prove_witness_and_verify() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .with_mock_notes_preserved()
    .build();

    let executed_transaction = tx_context
        .execute_transaction(None, AdviceMap::default(), ScriptAndInputs::empty())
        .unwrap();
    let executed_transaction_id = executed_transaction.id();

    let proof_options = ProvingOptions::default();
    let prover = TransactionProver::new(proof_options);
    let proven_transaction = prover.prove_transaction(executed_transaction).unwrap();

    assert_eq!(proven_transaction.id(), executed_transaction_id);

    let serialised_transaction = proven_transaction.to_bytes();
    let proven_transaction = ProvenTransaction::read_from_bytes(&serialised_transaction).unwrap();
    let verifier = TransactionVerifier::new(MIN_PROOF_SECURITY_LEVEL);
    assert!(verifier.verify(proven_transaction).is_ok());
}

// TEST TRANSACTION SCRIPT
// ================================================================================================

#[test]
fn test_tx_script() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .with_mock_notes_preserved()
    .build();

    let tx_input_key = [Felt::new(9999), Felt::new(8888), Felt::new(9999), Felt::new(8888)];
    let tx_input_value = [Felt::new(9), Felt::new(8), Felt::new(7), Felt::new(6)];
    let tx_source = format!(
        "
        begin
            # => [TX_SCRIPT_ROOT]
            dropw

            # push the tx script input key onto the stack
            push.{key}

            # load the tx script input value from the map and read it onto the stack
            adv.push_mapval adv_loadw

            # assert that the value is correct
            push.{value} assert_eqw
        end
        ",
        key = prepare_word(&tx_input_key),
        value = prepare_word(&tx_input_value)
    );

    let executed_transaction = tx_context.execute_transaction(
        None,
        AdviceMap::default(),
        ScriptAndInputs::new(tx_source, vec![(tx_input_key, tx_input_value.into())], vec![]),
    );

    assert!(
        executed_transaction.is_ok(),
        "Transaction execution failed {:?}",
        executed_transaction,
    );
}

// Checks the stack is empty when calling the transaction script.
#[test]
fn test_tx_script_stack_is_empty() {
    let tx_context = TransactionContextBuilder::with_standard_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ONE,
    )
    .with_mock_notes_preserved()
    .build();

    let tx_source = "
        begin
            # => [TX_SCRIPT_ROOT]
            dropw

            assertz assertz assertz assertz
            assertz assertz assertz assertz
            assertz assertz assertz assertz
        end
    ";

    // [TransactionKernel::parse_output_stack] validated the final stack of the VM. If the
    // [ExecutedTransaction] successfully parsed the program's final stack, then it is empty.
    let executed_transaction = tx_context.execute_transaction(
        None,
        AdviceMap::default(),
        ScriptAndInputs::new(tx_source.to_string(), vec![], vec![]),
    );

    assert!(
        executed_transaction.is_ok(),
        "Transaction execution failed {:?}",
        executed_transaction,
    );
}
