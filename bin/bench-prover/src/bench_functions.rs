use miden_objects::{
    Felt,
    account::Account,
    asset::{Asset, AssetVault, FungibleAsset},
    note::NoteType,
    testing::account_id::ACCOUNT_ID_SENDER,
};
use miden_testing::{Auth, MockChain, utils::prove_and_verify_transaction};

pub fn prove_consume_note_with_new_account() {
    let mut mock_chain = MockChain::new();

    // Create assets
    let fungible_asset: Asset = FungibleAsset::mock(123);

    // Create faucet account and target account
    let faucet_account = mock_chain.add_pending_existing_wallet(Auth::BasicAuth, vec![]);
    let target_account = mock_chain.add_pending_new_wallet(Auth::BasicAuth);

    // Create the note
    let note = mock_chain
        .add_pending_p2id_note(
            faucet_account.id(),
            target_account.id(),
            &[fungible_asset],
            NoteType::Public,
            None,
        )
        .unwrap();

    mock_chain.prove_next_block();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------

    // Execute the transaction and get the witness
    let executed_transaction = mock_chain
        .build_tx_context(target_account.id(), &[note.id()], &[])
        .build()
        .execute()
        .unwrap();

    // Apply delta to the target account to verify it is no longer new
    let target_account_after: Account = Account::from_parts(
        target_account.id(),
        AssetVault::new(&[fungible_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(1),
    );

    assert_eq!(
        executed_transaction.final_account().commitment(),
        target_account_after.commitment()
    );
    prove_and_verify_transaction(executed_transaction).unwrap();
}

pub fn prove_consume_multiple_notes() {
    let mut mock_chain = MockChain::new();
    let mut account = mock_chain.add_pending_existing_wallet(Auth::BasicAuth, vec![]);

    let fungible_asset_1: Asset = FungibleAsset::mock(100);
    let fungible_asset_2: Asset = FungibleAsset::mock(23);

    let note_1 = mock_chain
        .add_pending_p2id_note(
            ACCOUNT_ID_SENDER.try_into().unwrap(),
            account.id(),
            &[fungible_asset_1],
            NoteType::Private,
            None,
        )
        .unwrap();
    let note_2 = mock_chain
        .add_pending_p2id_note(
            ACCOUNT_ID_SENDER.try_into().unwrap(),
            account.id(),
            &[fungible_asset_2],
            NoteType::Private,
            None,
        )
        .unwrap();

    mock_chain.prove_next_block();

    let tx_context = mock_chain
        .build_tx_context(account.id(), &[note_1.id(), note_2.id()], &[])
        .build();

    let executed_transaction = tx_context.execute().unwrap();

    account.apply_delta(executed_transaction.account_delta()).unwrap();
    let resulting_asset = account.vault().assets().next().unwrap();
    if let Asset::Fungible(asset) = resulting_asset {
        assert_eq!(asset.amount(), 123u64);
    } else {
        panic!("Resulting asset should be fungible");
    }

    prove_and_verify_transaction(executed_transaction).unwrap();
}
