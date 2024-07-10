use miden_lib::notes::create_swap_note;
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN, ACCOUNT_ID_SENDER,
        },
        Account, AccountId,
    },
    assembly::ProgramAst,
    assets::{Asset, AssetVault, FungibleAsset, NonFungibleAsset, NonFungibleAssetDetails},
    crypto::rand::RpoRandomCoin,
    notes::{NoteAssets, NoteExecutionHint, NoteHeader, NoteId, NoteMetadata, NoteTag, NoteType},
    testing::account_code::DEFAULT_AUTH_SCRIPT,
    transaction::TransactionArgs,
    Felt, ZERO,
};
use miden_tx::{testing::TransactionContextBuilder, TransactionExecutor};

use crate::{
    get_account_with_default_account_code, get_new_pk_and_authenticator,
    prove_and_verify_transaction,
};

#[test]
fn prove_swap_script() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let offered_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let requested_asset: Asset = NonFungibleAsset::new(
        &NonFungibleAssetDetails::new(faucet_id_2, vec![1, 2, 3, 4]).unwrap(),
    )
    .unwrap()
    .into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN).unwrap();
    let (target_pub_key, target_falcon_auth) = get_new_pk_and_authenticator();
    let target_account = get_account_with_default_account_code(
        target_account_id,
        target_pub_key,
        Some(requested_asset),
    );

    // Create the note containing the SWAP script
    let (note, payback_note) = create_swap_note(
        sender_account_id,
        offered_asset,
        requested_asset,
        NoteType::Public,
        Felt::new(27),
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let tx_context = TransactionContextBuilder::new(target_account.clone())
        .input_notes(vec![note.clone()])
        .build();

    let mut executor =
        TransactionExecutor::new(tx_context.clone(), Some(target_falcon_auth.clone()));
    executor.load_account(target_account_id).unwrap();

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(DEFAULT_AUTH_SCRIPT).unwrap();
    let tx_script_target =
        executor.compile_tx_script(tx_script_code.clone(), vec![], vec![]).unwrap();
    let tx_args_target = TransactionArgs::with_tx_script(tx_script_target);

    let executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, tx_args_target)
        .expect("Transaction consuming swap note failed");

    // target account vault delta
    let target_account_after: Account = Account::from_parts(
        target_account.id(),
        AssetVault::new(&[offered_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );

    // Check that the target account has received the asset from the note
    assert_eq!(executed_transaction.final_account().hash(), target_account_after.hash());

    // Check if only one `Note` has been created
    assert_eq!(executed_transaction.output_notes().num_notes(), 1);

    // Check if the output `Note` is what we expect
    let recipient = payback_note.recipient().clone();
    let tag = NoteTag::from_account_id(sender_account_id, NoteExecutionHint::Local).unwrap();
    let note_metadata = NoteMetadata::new(target_account_id, NoteType::Private, tag, ZERO).unwrap();
    let assets = NoteAssets::new(vec![requested_asset]).unwrap();
    let note_id = NoteId::new(recipient.digest(), assets.commitment());

    let output_note = executed_transaction.output_notes().get_note(0);
    assert_eq!(NoteHeader::from(output_note), NoteHeader::new(note_id, note_metadata));

    // Prove, serialize/deserialize and verify the transaction
    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());
}
