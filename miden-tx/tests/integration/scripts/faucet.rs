extern crate alloc;

use miden_lib::transaction::{memory::FAUCET_STORAGE_DATA_SLOT, TransactionKernel};
use miden_objects::{
    accounts::{AccountCode, AccountType, StorageSlot},
    assets::{Asset, FungibleAsset},
    notes::{NoteAssets, NoteExecutionHint, NoteId, NoteMetadata, NoteTag, NoteType},
    testing::{account::AccountBuilder, prepare_word},
    transaction::TransactionScript,
    Felt, Word, ZERO,
};
use miden_tx::testing::mock_chain::{Auth, MockChain};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::{get_note_with_fungible_asset_and_script, prove_and_verify_transaction};

const FUNGIBLE_FAUCET_SOURCE: &str = "
export.::miden::contracts::faucets::basic_fungible::distribute
export.::miden::contracts::faucets::basic_fungible::burn
export.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
";

// TESTS MINT FUNGIBLE ASSET
// ================================================================================================

#[test]
fn prove_faucet_contract_mint_fungible_asset_succeeds() {
    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let faucet_builder = get_faucet_account_with_max_supply_and_total_issuance(200, None);
    let mut mock_chain = MockChain::new();
    let faucet = mock_chain.add_from_account_builder(Auth::BasicAuth, faucet_builder);

    let recipient = [Felt::new(0), Felt::new(1), Felt::new(2), Felt::new(3)];
    let tag = NoteTag::for_local_use_case(0, 0).unwrap();
    let aux = Felt::new(27);
    let note_execution_hint = NoteExecutionHint::on_block_slot(5, 6, 7);
    let note_type = NoteType::Private;
    let amount = Felt::new(100);

    assert_eq!(tag.validate(note_type), Ok(tag));

    let tx_script_code = format!(
        "
            begin

                push.{recipient}
                push.{note_execution_hint}
                push.{note_type}
                push.{aux}
                push.{tag}
                push.{amount}
                call.::miden::contracts::faucets::basic_fungible::distribute

                call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
                dropw dropw drop
            end
            ",
        note_type = note_type as u8,
        recipient = prepare_word(&recipient),
        aux = aux,
        tag = u32::from(tag),
        note_execution_hint = Felt::from(note_execution_hint)
    );

    let tx_script =
        TransactionScript::compile(tx_script_code, vec![], TransactionKernel::testing_assembler())
            .unwrap();
    let tx_context =
        mock_chain.build_tx_context(faucet.id(), &[], &[]).tx_script(tx_script).build();

    let executed_transaction = tx_context.execute().unwrap();

    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    let fungible_asset: Asset = FungibleAsset::new(faucet.id(), amount.into()).unwrap().into();

    let output_note = executed_transaction.output_notes().get_note(0).clone();

    let assets = NoteAssets::new(vec![fungible_asset]).unwrap();
    let id = NoteId::new(recipient.into(), assets.commitment());

    assert_eq!(output_note.id(), id);
    assert_eq!(
        output_note.metadata(),
        &NoteMetadata::new(faucet.id(), NoteType::Private, tag, note_execution_hint, aux).unwrap()
    );
}

#[test]
fn faucet_contract_mint_fungible_asset_fails_exceeds_max_supply() {
    // CONSTRUCT AND EXECUTE TX (Failure)
    // --------------------------------------------------------------------------------------------
    let mut mock_chain = MockChain::new();
    let faucet: miden_tx::testing::mock_chain::MockFungibleFaucet =
        mock_chain.add_new_faucet(Auth::BasicAuth, "TST", 200u64);

    let recipient = [Felt::new(0), Felt::new(1), Felt::new(2), Felt::new(3)];
    let aux = Felt::new(27);
    let tag = Felt::new(4);
    let amount = Felt::new(250);

    let tx_script_code = format!(
        "
            begin

                push.{recipient}
                push.{note_type}
                push.{aux}
                push.{tag}
                push.{amount}
                call.::miden::contracts::faucets::basic_fungible::distribute

                call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
                dropw dropw

            end
            ",
        note_type = NoteType::Private as u8,
        recipient = prepare_word(&recipient),
    );

    let tx_script =
        TransactionScript::compile(tx_script_code, vec![], TransactionKernel::testing_assembler())
            .unwrap();
    let tx = mock_chain
        .build_tx_context(faucet.account().id(), &[], &[])
        .tx_script(tx_script)
        .build()
        .execute();

    // Execute the transaction and get the witness
    assert!(tx.is_err());
}

// TESTS BURN FUNGIBLE ASSET
// ================================================================================================

#[test]
fn prove_faucet_contract_burn_fungible_asset_succeeds() {
    let faucet_builder = get_faucet_account_with_max_supply_and_total_issuance(200, Some(100));
    let mut mock_chain = MockChain::new();
    let faucet_account = mock_chain.add_from_account_builder(Auth::BasicAuth, faucet_builder);

    let fungible_asset = FungibleAsset::new(faucet_account.id(), 100).unwrap();

    // check that the faucet reserved slot has been correctly initialised
    assert_eq!(
        faucet_account.storage().get_item(FAUCET_STORAGE_DATA_SLOT).unwrap(),
        [Felt::new(0), Felt::new(0), Felt::new(0), Felt::new(100)].into()
    );

    // check that max_supply (slot 2) is 200 and amount already issued (slot 0) is 100
    assert_eq!(
        faucet_account.storage().get_item(2).unwrap(),
        [Felt::new(200), Felt::new(0), Felt::new(0), Felt::new(0)].into()
    );

    // need to create a note with the fungible asset to be burned
    let note_script = "
        # burn the asset
        begin
            dropw
            exec.::miden::note::get_assets drop
            mem_loadw
            call.::miden::contracts::faucets::basic_fungible::burn
        end
        ";

    let note = get_note_with_fungible_asset_and_script(fungible_asset, note_script);

    mock_chain.add_note(note.clone());
    mock_chain.seal_block(None);

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    // Execute the transaction and get the witness
    let executed_transaction = mock_chain
        .build_tx_context(faucet_account.id(), &[note.id()], &[])
        .build()
        .execute()
        .unwrap();

    // Prove, serialize/deserialize and verify the transaction
    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // check that the account burned the asset
    assert_eq!(executed_transaction.account_delta().nonce(), Some(Felt::new(3)));
    assert_eq!(executed_transaction.input_notes().get_note(0).id(), note.id());
}

// HELPER FUNCTIONS
// ================================================================================================

fn get_faucet_account_with_max_supply_and_total_issuance(
    max_supply: u64,
    total_issuance: Option<u64>,
) -> AccountBuilder<ChaCha20Rng> {
    let assembler = TransactionKernel::assembler();
    let faucet_account_code =
        AccountCode::compile(FUNGIBLE_FAUCET_SOURCE, assembler, true).unwrap();

    let faucet_storage_slot_0 = match total_issuance {
        Some(issuance) => StorageSlot::Value([ZERO, ZERO, ZERO, Felt::new(issuance)]),
        None => StorageSlot::Value(Word::default()),
    };
    let faucet_storage_slot_1 =
        StorageSlot::Value([Felt::new(max_supply), Felt::new(0), Felt::new(0), Felt::new(0)]);
    let faucet_storage_slot_2 =
        StorageSlot::Value([Felt::new(0), Felt::new(0), Felt::new(0), Felt::new(0)]);

    AccountBuilder::new(ChaCha20Rng::from_seed(Default::default()))
        .nonce(Felt::new(1))
        .code(faucet_account_code)
        .account_type(AccountType::FungibleFaucet)
        .add_storage_slots([faucet_storage_slot_0, faucet_storage_slot_1, faucet_storage_slot_2])
}
