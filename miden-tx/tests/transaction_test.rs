// use miden_lib::transaction::TransactionKernel;
// use miden_objects::{
//     accounts::AccountCode,
//     assembly::{Assembler, ModuleAst, ProgramAst},
//     assets::{Asset, FungibleAsset},
//     transaction::TransactionWitness,
//     Felt, Word,
// };
// use miden_tx::{TransactionExecutor, TransactionHost};
// use mock::{
//     constants::{
//         non_fungible_asset, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
//         ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
//         ACCOUNT_PROCEDURE_INCR_NONCE_PROC_IDX, ACCOUNT_PROCEDURE_SET_CODE_PROC_IDX,
//         ACCOUNT_PROCEDURE_SET_ITEM_PROC_IDX, FUNGIBLE_ASSET_AMOUNT, STORAGE_INDEX_0,
//     },
//     mock::notes::AssetPreservationStatus,
//     utils::prepare_word,
// };
// use vm_processor::MemAdviceProvider;

// // TESTS
// // ================================================================================================

// #[test]
// fn test_transaction_executor_witness() {
//     let data_store = MockDataStore::default();
//     let mut executor = TransactionExecutor::new(data_store.clone());

//     let account_id = data_store.account.id();
//     executor.load_account(account_id).unwrap();

//     let block_ref = data_store.block_header.block_num();
//     let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

//     // execute the transaction and get the witness
//     let executed_transaction =
//         executor.execute_transaction(account_id, block_ref, &note_ids, None).unwrap();
//     let tx_witness: TransactionWitness = executed_transaction.clone().into();

//     // use the witness to execute the transaction again
//     let (stack_inputs, advice_inputs) = tx_witness.get_kernel_inputs();
//     let mem_advice_provider: MemAdviceProvider = advice_inputs.into();
//     let mut host = TransactionHost::new(tx_witness.account().into(), mem_advice_provider);
//     let result =
//         vm_processor::execute(tx_witness.program(), stack_inputs, &mut host, Default::default())
//             .unwrap();

//     let (advice_provider, _event_handler) = host.into_parts();
//     let (_, map, _) = advice_provider.into_parts();
//     let tx_outputs =
//         TransactionKernel::parse_transaction_outputs(result.stack_outputs(), &map.into()).unwrap();

//     assert_eq!(executed_transaction.final_account().hash(), tx_outputs.account.hash());
//     assert_eq!(executed_transaction.output_notes(), &tx_outputs.output_notes);
// }

// #[test]
// fn test_transaction_result_account_delta() {
//     let data_store = MockDataStore::new(AssetPreservationStatus::PreservedWithAccountVaultDelta);
//     let mut executor = TransactionExecutor::new(data_store.clone());
//     let account_id = data_store.account.id();
//     executor.load_account(account_id).unwrap();

//     let new_acct_code_src = "\
//     export.account_proc_1
//         push.9.9.9.9
//         dropw
//     end
//     ";
//     let new_acct_code_ast = ModuleAst::parse(new_acct_code_src).unwrap();
//     let new_acct_code = AccountCode::new(new_acct_code_ast.clone(), &Assembler::default()).unwrap();

//     // updated storage
//     let updated_slot_value = [Felt::new(7), Felt::new(9), Felt::new(11), Felt::new(13)];

//     // removed assets
//     let removed_asset_1 = Asset::Fungible(
//         FungibleAsset::new(
//             ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().expect("id is valid"),
//             FUNGIBLE_ASSET_AMOUNT / 2,
//         )
//         .expect("asset is valid"),
//     );
//     let removed_asset_2 = Asset::Fungible(
//         FungibleAsset::new(
//             ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().expect("id is valid"),
//             FUNGIBLE_ASSET_AMOUNT,
//         )
//         .expect("asset is valid"),
//     );
//     let removed_asset_3 = non_fungible_asset(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);
//     let removed_assets = [removed_asset_1, removed_asset_2, removed_asset_3];

//     let account_procedure_incr_nonce_mast_root =
//         &data_store.account.code().procedures()[ACCOUNT_PROCEDURE_INCR_NONCE_PROC_IDX].to_hex();
//     let account_procedure_set_code_mast_root =
//         &data_store.account.code().procedures()[ACCOUNT_PROCEDURE_SET_CODE_PROC_IDX].to_hex();
//     let account_procedure_set_item_mast_root =
//         &data_store.account.code().procedures()[ACCOUNT_PROCEDURE_SET_ITEM_PROC_IDX].to_hex();

//     let tx_script = format!(
//         "\
//         use.miden::account
//         use.miden::contracts::wallets::basic->wallet

//         ## ACCOUNT PROCEDURE WRAPPERS
//         ## ========================================================================================
//         #TODO: Move this into an account library
//         proc.set_item
//             push.0 movdn.5 push.0 movdn.5 push.0 movdn.5
//             # => [index, V', 0, 0, 0]

//             call.{account_procedure_set_item_mast_root}
//             # => [R', V]
//         end

//         proc.set_code
//             call.{account_procedure_set_code_mast_root}
//             # => [0, 0, 0, 0]

//             dropw
//             # => []
//         end

//         proc.incr_nonce
//             call.{account_procedure_incr_nonce_mast_root}
//             # => [0]

//             drop
//             # => []
//         end

//         ## TRANSACTION SCRIPT
//         ## ========================================================================================
//         begin
//             ## Update account storage item
//             ## ------------------------------------------------------------------------------------
//             # push a new value for the storage slot onto the stack
//             push.{UPDATED_SLOT_VALUE}
//             # => [13, 11, 9, 7]

//             # get the index of account storage slot
//             push.{STORAGE_INDEX_0}
//             # => [idx, 13, 11, 9, 7]

//             # update the storage value
//             exec.set_item dropw dropw
//             # => []

//             ## Send some assets from the account vault
//             ## ------------------------------------------------------------------------------------
//             # partially deplete fungible asset balance
//             push.0.1.2.3
//             push.999
//             push.{REMOVED_ASSET_1}
//             call.wallet::send_asset drop dropw dropw

//             # totally deplete fungible asset balance
//             push.0.1.2.3
//             push.999
//             push.{REMOVED_ASSET_2}
//             call.wallet::send_asset drop dropw dropw

//             # send non-fungible asset
//             push.0.1.2.3
//             push.999
//             push.{REMOVED_ASSET_3}
//             call.wallet::send_asset drop dropw dropw

//             ## Update account code
//             ## ------------------------------------------------------------------------------------
//             push.{NEW_ACCOUNT_ROOT} exec.set_code
//             # => []

//             ## Update the account nonce
//             ## ------------------------------------------------------------------------------------
//             push.1 exec.incr_nonce
//         end
//     ",
//         NEW_ACCOUNT_ROOT = prepare_word(&new_acct_code.root()),
//         UPDATED_SLOT_VALUE = prepare_word(&Word::from(updated_slot_value)),
//         REMOVED_ASSET_1 = prepare_word(&Word::from(removed_asset_1)),
//         REMOVED_ASSET_2 = prepare_word(&Word::from(removed_asset_2)),
//         REMOVED_ASSET_3 = prepare_word(&Word::from(removed_asset_3)),
//     );
//     let tx_script_code = ProgramAst::parse(&tx_script).unwrap();
//     let tx_script = executor.compile_tx_script(tx_script_code, vec![], vec![]).unwrap();

//     let block_ref = data_store.block_header.block_num();
//     let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

//     // expected delta
//     // --------------------------------------------------------------------------------------------
//     // execute the transaction and get the witness
//     let transaction_result = executor
//         .execute_transaction(account_id, block_ref, &note_ids, Some(tx_script))
//         .unwrap();

//     // nonce delta
//     // --------------------------------------------------------------------------------------------
//     assert_eq!(transaction_result.account_delta().nonce(), Some(Felt::new(2)));

//     // storage delta
//     // --------------------------------------------------------------------------------------------
//     assert_eq!(transaction_result.account_delta().storage().updated_items.len(), 1);
//     assert_eq!(transaction_result.account_delta().storage().updated_items[0].0, STORAGE_INDEX_0);
//     assert_eq!(
//         transaction_result.account_delta().storage().updated_items[0].1,
//         updated_slot_value
//     );

//     // vault delta
//     // --------------------------------------------------------------------------------------------
//     // assert that added assets are tracked
//     let added_assets = data_store
//         .notes
//         .last()
//         .unwrap()
//         .note()
//         .assets()
//         .iter()
//         .cloned()
//         .collect::<Vec<_>>();
//     assert!(transaction_result
//         .account_delta()
//         .vault()
//         .added_assets
//         .iter()
//         .all(|x| added_assets.contains(x)));
//     assert_eq!(
//         added_assets.len(),
//         transaction_result.account_delta().vault().added_assets.len()
//     );

//     // assert that removed assets are tracked
//     assert!(transaction_result
//         .account_delta()
//         .vault()
//         .removed_assets
//         .iter()
//         .all(|x| removed_assets.contains(x)));
//     assert_eq!(
//         removed_assets.len(),
//         transaction_result.account_delta().vault().removed_assets.len()
//     );
// }
