use miden_lib::{
    accounts::faucets::create_basic_fungible_faucet,
    transaction::{memory::FAUCET_STORAGE_DATA_SLOT, TransactionKernel},
    AuthScheme,
};
use miden_objects::{
    accounts::{
        Account, AccountCode, AccountId, AccountStorage, AccountStorageType, SlotItem, StorageSlot,
        ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN,
    },
    assembly::{ModuleAst, ProgramAst},
    assets::{Asset, AssetVault, FungibleAsset, TokenSymbol},
    crypto::dsa::rpo_falcon512::SecretKey,
    notes::{NoteAssets, NoteId, NoteMetadata, NoteType},
    transaction::TransactionArgs,
    Felt, Word, ZERO,
};
use miden_tx::TransactionExecutor;
use mock::utils::prepare_word;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

use crate::{
    get_new_key_pair_with_advice_map, get_note_with_fungible_asset_and_script,
    prove_and_verify_transaction, MockDataStore,
};

// TESTS MINT FUNGIBLE ASSET
// ================================================================================================

#[test]
fn prove_faucet_contract_mint_fungible_asset_succeeds() {
    let (faucet_pub_key, faucet_keypair_felts) = get_new_key_pair_with_advice_map();
    let faucet_account =
        get_faucet_account_with_max_supply_and_total_issuance(faucet_pub_key, 200, None);

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store = MockDataStore::with_existing(Some(faucet_account.clone()), Some(vec![]));

    let mut executor = TransactionExecutor::new();
    executor.load_account(faucet_account.id(), &data_store).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let recipient = [Felt::new(0), Felt::new(1), Felt::new(2), Felt::new(3)];
    let tag = 4.into();
    let amount = Felt::new(100);

    let tx_script_code = ProgramAst::parse(
        format!(
            "
            use.miden::contracts::faucets::basic_fungible->faucet
            use.miden::contracts::auth::basic->auth_tx

            begin

                push.{recipient}
                push.{note_type}
                push.{tag}
                push.{amount}
                call.faucet::distribute

                call.auth_tx::auth_tx_rpo_falcon512
                dropw dropw

            end
            ",
            note_type = NoteType::OffChain as u8,
            recipient = prepare_word(&recipient),
        )
        .as_str(),
    )
    .unwrap();

    let tx_script = executor
        .compile_tx_script(tx_script_code, vec![(faucet_pub_key, faucet_keypair_felts)], vec![])
        .unwrap();
    let tx_args = TransactionArgs::with_tx_script(tx_script);

    let executed_transaction = executor
        .execute_transaction(faucet_account.id(), block_ref, &note_ids, tx_args, &data_store)
        .unwrap();

    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    let fungible_asset: Asset =
        FungibleAsset::new(faucet_account.id(), amount.into()).unwrap().into();

    let created_note = executed_transaction.output_notes().get_note(0).clone();

    let assets = NoteAssets::new(vec![fungible_asset]).unwrap();
    let id = NoteId::new(recipient.into(), assets.commitment());

    assert_eq!(created_note.id(), id);
    assert_eq!(
        created_note.metadata(),
        &NoteMetadata::new(faucet_account.id(), NoteType::OffChain, tag, ZERO).unwrap()
    );
}

#[test]
fn faucet_contract_mint_fungible_asset_fails_exceeds_max_supply() {
    let (faucet_pub_key, faucet_keypair_felts) = get_new_key_pair_with_advice_map();
    let faucet_account =
        get_faucet_account_with_max_supply_and_total_issuance(faucet_pub_key, 200, None);

    // CONSTRUCT AND EXECUTE TX (Failure)
    // --------------------------------------------------------------------------------------------
    let data_store = MockDataStore::with_existing(Some(faucet_account.clone()), Some(vec![]));

    let mut executor = TransactionExecutor::new();
    executor.load_account(faucet_account.id(), &data_store).unwrap();

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
                push.{note_type}
                push.{tag}
                push.{amount}
                call.faucet::distribute

                call.auth_tx::auth_tx_rpo_falcon512
                dropw dropw

            end
            ",
            note_type = NoteType::OffChain as u8,
            recipient = prepare_word(&recipient),
        )
        .as_str(),
    )
    .unwrap();
    let tx_script = executor
        .compile_tx_script(tx_script_code, vec![(faucet_pub_key, faucet_keypair_felts)], vec![])
        .unwrap();

    let tx_args = TransactionArgs::with_tx_script(tx_script);

    // Execute the transaction and get the witness
    let executed_transaction = executor.execute_transaction(
        faucet_account.id(),
        block_ref,
        &note_ids,
        tx_args,
        &data_store,
    );

    assert!(executed_transaction.is_err());
}

// TESTS BURN FUNGIBLE ASSET
// ================================================================================================

#[test]
fn prove_faucet_contract_burn_fungible_asset_succeeds() {
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

    let mut executor = TransactionExecutor::new();
    executor.load_account(faucet_account.id(), &data_store).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction = executor
        .execute_transaction(
            faucet_account.id(),
            block_ref,
            &note_ids,
            data_store.tx_args.clone(),
            &data_store,
        )
        .unwrap();

    // Prove, serialize/deserialize and verify the transaction
    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // check that the account burned the asset
    assert_eq!(executed_transaction.account_delta().nonce(), Some(Felt::new(2)));
    assert_eq!(executed_transaction.input_notes().get_note(0).id(), note.id());
}

// TESTS FUNGIBLE CONTRACT CONSTRUCTION
// ================================================================================================

#[test]
fn faucet_contract_creation() {
    // we need a Falcon Public Key to create the wallet account
    let seed = [0_u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);

    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key = sec_key.public_key();
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
    let storage_type = AccountStorageType::OffChain;

    let (faucet_account, _) = create_basic_fungible_faucet(
        init_seed,
        token_symbol,
        decimals,
        max_supply,
        storage_type,
        auth_scheme,
    )
    .unwrap();

    // check that max_supply (slot 1) is 123
    assert_eq!(
        faucet_account.storage().get_item(1),
        [Felt::new(123), Felt::new(2), token_symbol.into(), ZERO].into()
    );

    assert!(faucet_account.is_faucet());

    let exp_faucet_account_code_src =
        include_str!("../../../../miden-lib/asm/miden/contracts/faucets/basic_fungible.masm");
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
    let faucet_account_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap();
    let faucet_account_code_src =
        include_str!("../../../../miden-lib/asm/miden/contracts/faucets/basic_fungible.masm");
    let faucet_account_code_ast = ModuleAst::parse(faucet_account_code_src).unwrap();
    let account_assembler = TransactionKernel::assembler();

    let faucet_account_code =
        AccountCode::new(faucet_account_code_ast.clone(), &account_assembler).unwrap();

    let faucet_storage_slot_1 = [Felt::new(max_supply), Felt::new(0), Felt::new(0), Felt::new(0)];
    let mut faucet_account_storage = AccountStorage::new(vec![
        SlotItem {
            index: 0,
            slot: StorageSlot::new_value(public_key),
        },
        SlotItem {
            index: 1,
            slot: StorageSlot::new_value(faucet_storage_slot_1),
        },
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
