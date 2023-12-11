use miden_lib::{assembler::assembler, faucets::create_basic_fungible_faucet, AuthScheme};
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountVault},
    assembly::{ModuleAst, ProgramAst},
    assets::{Asset, FungibleAsset, TokenSymbol},
    crypto::{
        dsa::rpo_falcon512::{KeyPair, PublicKey},
        merkle::MerkleStore,
    },
    notes::{NoteMetadata, NoteStub, NoteVault},
    Felt, StarkField, Word, ZERO,
};
use mock::{constants::ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, utils::prepare_word};

use miden_tx::TransactionExecutor;

mod common;
use common::{
    get_new_key_pair_with_advice_map, get_note_with_fungible_asset_and_script, MockDataStore,
};

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

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins =
        data_store.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    let recipient = [Felt::new(0), Felt::new(1), Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);
    let amount = Felt::new(100);

    let tx_script_code = ProgramAst::parse(
        format!(
            "
            use.miden::faucets::basic_fungible->faucet
            use.miden::eoa::basic->auth_tx

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
            tag = tag,
            amount = amount,
        )
        .as_str(),
    )
    .unwrap();
    let tx_script = executor
        .compile_tx_script(tx_script_code, vec![(faucet_pub_key, faucet_keypair_felts)], vec![])
        .unwrap();

    // Execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(faucet_account.id(), block_ref, &note_origins, Some(tx_script))
        .unwrap();

    let fungible_asset: Asset =
        FungibleAsset::new(faucet_account.id(), amount.into()).unwrap().into();

    let expected_note = NoteStub::new(
        recipient.into(),
        NoteVault::new(&[fungible_asset]).unwrap(),
        NoteMetadata::new(faucet_account.id(), tag, Felt::new(1)),
    )
    .unwrap();

    let created_note = transaction_result.created_notes().notes()[0].clone();
    assert!(created_note.recipient() == expected_note.recipient());
    assert!(created_note.vault() == expected_note.vault());
    assert!(created_note.metadata() == expected_note.metadata());
}

#[test]
fn test_faucet_contract_mint_fungible_asset_fails_exceeds_max_supply() {
    let (faucet_pub_key, faucet_keypair_felts) = get_new_key_pair_with_advice_map();
    let faucet_account =
        get_faucet_account_with_max_supply_and_total_issuance(faucet_pub_key.clone(), 200, None);

    // CONSTRUCT AND EXECUTE TX (Failure)
    // --------------------------------------------------------------------------------------------
    let data_store = MockDataStore::with_existing(Some(faucet_account.clone()), Some(vec![]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(faucet_account.id()).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins =
        data_store.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    let recipient = [Felt::new(0), Felt::new(1), Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);
    let amount = Felt::new(250);

    let tx_script_code = ProgramAst::parse(
        format!(
            "
            use.miden::faucets::basic_fungible->faucet
            use.miden::eoa::basic->auth_tx

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
            tag = tag,
            amount = amount,
        )
        .as_str(),
    )
    .unwrap();
    let tx_script = executor
        .compile_tx_script(tx_script_code, vec![(faucet_pub_key, faucet_keypair_felts)], vec![])
        .unwrap();

    // Execute the transaction and get the witness
    let transaction_result = executor.execute_transaction(
        faucet_account.id(),
        block_ref,
        &note_origins,
        Some(tx_script),
    );

    assert!(transaction_result.is_err());
}

#[test]
fn test_faucet_contract_burn_fungible_asset_succeeds() {
    let (faucet_pub_key, _faucet_keypair_felts) = get_new_key_pair_with_advice_map();
    let faucet_account = get_faucet_account_with_max_supply_and_total_issuance(
        faucet_pub_key.clone(),
        200,
        Some(100),
    );

    let fungible_asset = FungibleAsset::new(faucet_account.id(), 100).unwrap();

    // check that max_supply (slot 1) is 200 and amount already issued (slot 255) is 100
    assert!(
        faucet_account.storage().get_item(1)
            == [Felt::new(200), Felt::new(0), Felt::new(0), Felt::new(0)].into()
    );
    assert!(
        faucet_account.storage().get_item(255)
            == [Felt::new(0), Felt::new(0), Felt::new(0), Felt::new(100)].into()
    );

    // need to create a note with the fungible asset to be burned
    let note_script = ProgramAst::parse(
        format!(
            "
        use.miden::faucets::basic_fungible->faucet_contract
        use.miden::sat::note

        # burn the asset
        begin
            dropw
            exec.note::get_assets drop
            mem_loadw
            call.faucet_contract::burn
        end
        "
        )
        .as_str(),
    )
    .unwrap();

    let note = get_note_with_fungible_asset_and_script(fungible_asset.clone(), note_script);

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store =
        MockDataStore::with_existing(Some(faucet_account.clone()), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(faucet_account.id()).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins =
        data_store.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(faucet_account.id(), block_ref, &note_origins, None)
        .unwrap();

    // check that the account burned the asset
    assert!(transaction_result.account_delta().nonce.unwrap() == Felt::new(2));
    assert!(transaction_result.consumed_notes().notes()[0].note().hash() == note.hash());
}

#[test]
fn test_faucet_contract_creation() {
    // we need a Falcon Public Key to create the wallet account
    let key_pair: KeyPair = KeyPair::new().unwrap();
    let pub_key: PublicKey = key_pair.public_key();
    let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 {
        pub_key: pub_key.into(),
    };

    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = [
        90, 110, 209, 94, 84, 105, 250, 242, 223, 203, 216, 124, 22, 159, 14, 132, 215, 85, 183,
        204, 149, 90, 166, 68, 100, 73, 106, 168, 125, 237, 138, 16,
    ];

    let max_supply = Felt::new(123);
    let token_symbol_string = "POL";
    let token_symbol = TokenSymbol::try_from(token_symbol_string).unwrap();
    let decimals = 2u8;

    let (faucet_account, _) = create_basic_fungible_faucet(
        init_seed,
        token_symbol.clone(),
        decimals,
        max_supply,
        auth_scheme,
    )
    .unwrap();

    // check that max_supply (slot 1) is 123
    assert!(
        faucet_account.storage().get_item(1)
            == [Felt::new(123), Felt::new(2), TokenSymbol::from(token_symbol).into(), ZERO].into()
    );

    assert!(faucet_account.is_faucet() == true);

    let exp_faucet_account_code_src =
        include_str!("../../miden-lib/asm/miden/faucets/basic_fungible.masm");
    let exp_faucet_account_code_ast = ModuleAst::parse(exp_faucet_account_code_src).unwrap();
    let mut account_assembler = assembler();

    let exp_faucet_account_code =
        AccountCode::new(exp_faucet_account_code_ast.clone(), &mut account_assembler).unwrap();

    assert!(faucet_account.code() == &exp_faucet_account_code);
}

fn get_faucet_account_with_max_supply_and_total_issuance(
    public_key: Word,
    max_supply: u64,
    total_issuance: Option<u64>,
) -> Account {
    let faucet_account_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let faucet_account_code_src =
        include_str!("../../miden-lib/asm/miden/faucets/basic_fungible.masm");
    let faucet_account_code_ast = ModuleAst::parse(faucet_account_code_src).unwrap();
    let mut account_assembler = assembler();

    let faucet_account_code =
        AccountCode::new(faucet_account_code_ast.clone(), &mut account_assembler).unwrap();

    let faucet_storage_slot_1 = [Felt::new(max_supply), Felt::new(0), Felt::new(0), Felt::new(0)];
    let mut faucet_account_storage =
        AccountStorage::new(vec![(0, public_key), (1, faucet_storage_slot_1)], MerkleStore::new())
            .unwrap();

    if total_issuance.is_some() {
        let faucet_storage_slot_255 =
            [Felt::new(0), Felt::new(0), Felt::new(0), Felt::new(total_issuance.unwrap())];
        faucet_account_storage.set_item(255, faucet_storage_slot_255);
    };

    Account::new(
        faucet_account_id,
        AccountVault::new(&vec![]).unwrap(),
        faucet_account_storage.clone(),
        faucet_account_code.clone(),
        Felt::new(1),
    )
}
