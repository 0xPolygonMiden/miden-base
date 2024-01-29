use miden_lib::{
    accounts::faucets::create_basic_fungible_faucet,
    transaction::{memory::FAUCET_STORAGE_DATA_SLOT, TransactionKernel},
    AuthScheme,
};
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, StorageSlotType},
    assembly::{ModuleAst, ProgramAst},
    assets::{Asset, AssetVault, FungibleAsset, TokenSymbol},
    crypto::dsa::rpo_falcon512::{KeyPair, PublicKey},
    notes::{NoteAssets, NoteMetadata},
    transaction::OutputNote,
    Felt, Word, ZERO,
};
use miden_tx::TransactionExecutor;
use mock::{constants::ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, procedures::prepare_word};

mod common;
use common::{
    get_new_key_pair_with_advice_map, get_note_with_fungible_asset_and_script, MockDataStore,
};

// TESTS MINT FUNGIBLE ASSET
// ================================================================================================

#[test]
fn test_faucet_contract_mint_fungible_asset_succeeds() {
    let (faucet_pub_key, faucet_keypair_felts) = get_new_key_pair_with_advice_map();
    let faucet_account =
        get_faucet_account_with_max_supply_and_total_issuance(faucet_pub_key, 200, None);

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store = MockDataStore::with_existing(Some(faucet_account.clone()), Some(vec![]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(faucet_account.id()).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let recipient = [Felt::new(0), Felt::new(1), Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);
    let amount = Felt::new(100);

    let tx_script_code = ProgramAst::parse(
        format!(
            "
            use.miden::contracts::faucets::basic_fungible->faucet
            use.miden::contracts::auth::basic->auth_tx

            begin

                push.{recipient}
                push.{tag}
                push.{amount}
                call.faucet::distribute

                call.auth_tx::auth_tx_rpo_falcon512
                dropw dropw

            end
            ",
            recipient = prepare_word(&recipient),
        )
        .as_str(),
    )
    .unwrap();

    let tx_script = executor
        .compile_tx_script(tx_script_code, vec![(faucet_pub_key, faucet_keypair_felts)], vec![])
        .unwrap();

    // Execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(faucet_account.id(), block_ref, &note_ids, Some(tx_script))
        .unwrap();

    let fungible_asset: Asset =
        FungibleAsset::new(faucet_account.id(), amount.into()).unwrap().into();

    let expected_note = OutputNote::new(
        recipient.into(),
        NoteAssets::new(&[fungible_asset]).unwrap(),
        NoteMetadata::new(faucet_account.id(), tag),
    );

    let created_note = transaction_result.output_notes().get_note(0).clone();
    assert_eq!(created_note.recipient(), expected_note.recipient());
    assert_eq!(created_note.assets(), expected_note.assets());
    assert_eq!(created_note.metadata(), expected_note.metadata());
}

#[test]
fn test_faucet_contract_mint_fungible_asset_fails_exceeds_max_supply() {
    let (faucet_pub_key, faucet_keypair_felts) = get_new_key_pair_with_advice_map();
    let faucet_account =
        get_faucet_account_with_max_supply_and_total_issuance(faucet_pub_key, 200, None);

    // CONSTRUCT AND EXECUTE TX (Failure)
    // --------------------------------------------------------------------------------------------
    let data_store = MockDataStore::with_existing(Some(faucet_account.clone()), Some(vec![]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(faucet_account.id()).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let recipient = [Felt::new(0), Felt::new(1), Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);
    let amount = Felt::new(250);

    let tx_script_code = ProgramAst::parse(
        format!(
            "
            use.miden::contracts::faucets::basic_fungible->faucet
            use.miden::contracts::auth::basic->auth_tx

            begin

                push.{recipient}
                push.{tag}
                push.{amount}
                call.faucet::distribute

                call.auth_tx::auth_tx_rpo_falcon512
                dropw dropw

            end
            ",
            recipient = prepare_word(&recipient),
        )
        .as_str(),
    )
    .unwrap();
    let tx_script = executor
        .compile_tx_script(tx_script_code, vec![(faucet_pub_key, faucet_keypair_felts)], vec![])
        .unwrap();

    // Execute the transaction and get the witness
    let transaction_result =
        executor.execute_transaction(faucet_account.id(), block_ref, &note_ids, Some(tx_script));

    assert!(transaction_result.is_err());
}

// TESTS BURN FUNGIBLE ASSET
// ================================================================================================

#[test]
fn test_faucet_contract_burn_fungible_asset_succeeds() {
    let (faucet_pub_key, _faucet_keypair_felts) = get_new_key_pair_with_advice_map();
    let faucet_account =
        get_faucet_account_with_max_supply_and_total_issuance(faucet_pub_key, 200, Some(100));

    let fungible_asset = FungibleAsset::new(faucet_account.id(), 100).unwrap();

    // check that max_supply (slot 1) is 200 and amount already issued (slot 255) is 100
    assert_eq!(
        faucet_account.storage().get_item(1),
        [Felt::new(200), Felt::new(0), Felt::new(0), Felt::new(0)].into()
    );
    assert_eq!(
        faucet_account.storage().get_item(FAUCET_STORAGE_DATA_SLOT),
        [Felt::new(0), Felt::new(0), Felt::new(0), Felt::new(100)].into()
    );

    // need to create a note with the fungible asset to be burned
    let note_script = ProgramAst::parse(
        "
        use.miden::contracts::faucets::basic_fungible->faucet_contract
        use.miden::note

        # burn the asset
        begin
            dropw
            exec.note::get_assets drop
            mem_loadw
            call.faucet_contract::burn
        end
        ",
    )
    .unwrap();

    let note = get_note_with_fungible_asset_and_script(fungible_asset, note_script);

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store =
        MockDataStore::with_existing(Some(faucet_account.clone()), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(faucet_account.id()).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(faucet_account.id(), block_ref, &note_ids, None)
        .unwrap();

    // check that the account burned the asset
    assert_eq!(transaction_result.account_delta().nonce(), Some(Felt::new(2)));
    assert_eq!(transaction_result.input_notes().get_note(0).id(), note.id());
}

// TESTS FUNGIBLE CONTRACT CONSTRUCTION
// ================================================================================================

#[test]
fn test_faucet_contract_creation() {
    // we need a Falcon Public Key to create the wallet account
    let key_pair: KeyPair = KeyPair::new().unwrap();
    let pub_key: PublicKey = key_pair.public_key();
    let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key };

    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = [
        90, 110, 209, 94, 84, 105, 250, 242, 223, 203, 216, 124, 22, 159, 14, 132, 215, 85, 183,
        204, 149, 90, 166, 68, 100, 73, 106, 168, 125, 237, 138, 16,
    ];

    let max_supply = Felt::new(123);
    let token_symbol_string = "POL";
    let token_symbol = TokenSymbol::try_from(token_symbol_string).unwrap();
    let decimals = 2u8;

    let (faucet_account, _) =
        create_basic_fungible_faucet(init_seed, token_symbol, decimals, max_supply, auth_scheme)
            .unwrap();

    // check that max_supply (slot 1) is 123
    assert_eq!(
        faucet_account.storage().get_item(1),
        [Felt::new(123), Felt::new(2), token_symbol.into(), ZERO].into()
    );

    assert!(faucet_account.is_faucet());

    let exp_faucet_account_code_src =
        include_str!("../../miden-lib/asm/miden/contracts/faucets/basic_fungible.masm");
    let exp_faucet_account_code_ast = ModuleAst::parse(exp_faucet_account_code_src).unwrap();
    let account_assembler = TransactionKernel::assembler();

    let exp_faucet_account_code =
        AccountCode::new(exp_faucet_account_code_ast.clone(), &account_assembler).unwrap();

    assert_eq!(faucet_account.code(), &exp_faucet_account_code);
}

fn get_faucet_account_with_max_supply_and_total_issuance(
    public_key: Word,
    max_supply: u64,
    total_issuance: Option<u64>,
) -> Account {
    let faucet_account_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let faucet_account_code_src =
        include_str!("../../miden-lib/asm/miden/contracts/faucets/basic_fungible.masm");
    let faucet_account_code_ast = ModuleAst::parse(faucet_account_code_src).unwrap();
    let account_assembler = TransactionKernel::assembler();

    let faucet_account_code =
        AccountCode::new(faucet_account_code_ast.clone(), &account_assembler).unwrap();

    let faucet_storage_slot_1 = [Felt::new(max_supply), Felt::new(0), Felt::new(0), Felt::new(0)];
    let mut faucet_account_storage = AccountStorage::new(vec![
        (0, (StorageSlotType::Value { value_arity: 0 }, public_key)),
        (1, (StorageSlotType::Value { value_arity: 0 }, faucet_storage_slot_1)),
    ])
    .unwrap();

    if total_issuance.is_some() {
        let faucet_storage_slot_254 =
            [Felt::new(0), Felt::new(0), Felt::new(0), Felt::new(total_issuance.unwrap())];
        faucet_account_storage
            .set_item(FAUCET_STORAGE_DATA_SLOT, faucet_storage_slot_254)
            .unwrap();
    };

    Account::new(
        faucet_account_id,
        AssetVault::new(&[]).unwrap(),
        faucet_account_storage.clone(),
        faucet_account_code.clone(),
        Felt::new(1),
    )
}
