use miden_lib::{notes::create_swap_note, transaction::TransactionKernel};
use miden_objects::{
    accounts::{account_id::testing::ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, Account, AccountId},
    assembly::ProgramAst,
    assets::{Asset, AssetVault, NonFungibleAsset, NonFungibleAssetDetails},
    crypto::rand::RpoRandomCoin,
    notes::{NoteAssets, NoteExecutionHint, NoteHeader, NoteId, NoteMetadata, NoteTag, NoteType},
    testing::account_code::DEFAULT_AUTH_SCRIPT,
    transaction::TransactionScript,
    Felt, ZERO,
};
use miden_tx::testing::mock_chain::{Auth, MockChain};

use crate::prove_and_verify_transaction;

#[test]
fn prove_swap_script() {
    // Create assets
    let assembler = &TransactionKernel::assembler();
    let mut chain = MockChain::new();
    let faucet = chain.add_existing_faucet(Auth::NoAuth, "POL", 100000u64);
    let offered_asset = faucet.mint(100);

    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let requested_asset: Asset = NonFungibleAsset::new(
        &NonFungibleAssetDetails::new(faucet_id_2, vec![1, 2, 3, 4]).unwrap(),
    )
    .unwrap()
    .into();

    // Create sender and target account
    let sender_account = chain.add_new_wallet(Auth::RpoAuth, vec![offered_asset]);
    let target_account = chain.add_existing_wallet(Auth::RpoAuth, vec![requested_asset]);

    // Create the note containing the SWAP script
    let (note, payback_note) = create_swap_note(
        sender_account.id(),
        offered_asset,
        requested_asset,
        NoteType::Public,
        Felt::new(27),
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    chain.add_note(note.clone());
    chain.seal_block(None);

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let tx_script_code = ProgramAst::parse(DEFAULT_AUTH_SCRIPT).unwrap();
    let (tx_script, _) = TransactionScript::new(tx_script_code, vec![], assembler).unwrap();
    let executed_transaction = chain
        .build_tx_context(target_account.id())
        .tx_script(tx_script)
        .build()
        .execute()
        .unwrap();

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
    let tag = NoteTag::from_account_id(sender_account.id(), NoteExecutionHint::Local).unwrap();
    let note_metadata =
        NoteMetadata::new(target_account.id(), NoteType::Private, tag, ZERO).unwrap();
    let assets = NoteAssets::new(vec![requested_asset]).unwrap();
    let note_id = NoteId::new(recipient.digest(), assets.commitment());

    let output_note = executed_transaction.output_notes().get_note(0);
    assert_eq!(NoteHeader::from(output_note), NoteHeader::new(note_id, note_metadata));

    // Prove, serialize/deserialize and verify the transaction
    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());
}
