use crate::constants::{
    non_fungible_asset_2, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
    ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
    ACCOUNT_SEED_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN, CHILD_ROOT_PARENT_LEAF_INDEX,
    CHILD_SMT_DEPTH, CHILD_STORAGE_INDEX_0, CHILD_STORAGE_VALUE_0, FUNGIBLE_ASSET_AMOUNT,
    FUNGIBLE_FAUCET_INITIAL_BALANCE, NON_FUNGIBLE_ASSET_DATA, STORAGE_ITEM_0, STORAGE_ITEM_1,
};
use assembly::{ast::ModuleAst, Assembler};
use crypto::{
    merkle::{SimpleSmt, TieredSmt},
    utils::collections::Vec,
    Felt, Word, ZERO,
};
use miden_lib::memory::FAUCET_STORAGE_DATA_SLOT;
use miden_objects::accounts::{Account, AccountCode, AccountId, AccountStorage, AccountVault};
use miden_objects::assets::{Asset, FungibleAsset, NonFungibleAsset, NonFungibleAssetDetails};
use vm_core::{crypto::merkle::MerkleStore, FieldElement};

fn mock_account_vault() -> AccountVault {
    // prepare fungible asset
    let faucet_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let fungible_asset =
        Asset::Fungible(FungibleAsset::new(faucet_id, FUNGIBLE_ASSET_AMOUNT).unwrap());

    // prepare non fungible asset
    let faucet_id: AccountId = ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let non_fungible_asset_details =
        NonFungibleAssetDetails::new(faucet_id, NON_FUNGIBLE_ASSET_DATA.to_vec()).unwrap();
    let non_fungible_asset =
        Asset::NonFungible(NonFungibleAsset::new(&non_fungible_asset_details).unwrap());

    AccountVault::new(&[fungible_asset, non_fungible_asset]).unwrap()
}

pub fn mock_account_storage() -> AccountStorage {
    // Create an account merkle store
    let mut account_merkle_store = MerkleStore::new();
    let child_smt =
        SimpleSmt::with_leaves(CHILD_SMT_DEPTH, [(CHILD_STORAGE_INDEX_0, CHILD_STORAGE_VALUE_0)])
            .unwrap();
    account_merkle_store.extend(child_smt.inner_nodes());

    // create account storage
    AccountStorage::new(
        vec![
            STORAGE_ITEM_0,
            STORAGE_ITEM_1,
            (CHILD_ROOT_PARENT_LEAF_INDEX, *child_smt.root()),
        ],
        account_merkle_store,
    )
    .unwrap()
}

fn mock_account_code(assembler: &Assembler) -> AccountCode {
    let account_code = "\
            use.miden::sat::account
            use.miden::sat::tx

            export.incr_nonce
                push.0 swap
                # => [value, 0]

                exec.account::incr_nonce
                # => [0]
            end

            export.set_item
                exec.account::set_item
                # => [R', V, 0, 0, 0]

                movup.8 drop movup.8 drop movup.8 drop
                # => [R', V]
            end

            export.set_code
                padw swapw
                # => [CODE_ROOT, 0, 0, 0, 0]

                exec.account::set_code
                # => [0, 0, 0, 0]
            end

            export.create_note
                # apply padding
                repeat.8
                    push.0 movdn.9
                end

                # create note
                exec.tx::create_note
                # => [ptr, 0, 0, 0, 0, 0, 0, 0, 0]
            end

            export.account_procedure_1
                push.1.2
                add
            end

            export.account_procedure_2
                push.2.1
                sub
            end
            ";
    let account_module_ast = ModuleAst::parse(account_code).unwrap();
    AccountCode::new(account_module_ast, assembler).unwrap()
}

pub fn mock_new_account(assembler: &Assembler) -> Account {
    let account_storage = mock_account_storage();
    let account_code = mock_account_code(assembler);
    let account_seed: Word = ACCOUNT_SEED_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN
        .iter()
        .map(|x| Felt::new(*x))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let account_id =
        AccountId::new(account_seed, account_code.root(), account_storage.root()).unwrap();
    Account::new(account_id, AccountVault::default(), account_storage, account_code, Felt::ZERO)
}

pub fn mock_account(nonce: Felt, code: Option<AccountCode>, assembler: &Assembler) -> Account {
    // mock account storage
    let account_storage = mock_account_storage();

    // mock account code
    let account_code = match code {
        Some(code) => code,
        None => mock_account_code(assembler),
    };

    // Create account vault
    let account_vault = mock_account_vault();

    // Create an account with storage items
    let account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    Account::new(account_id, account_vault, account_storage, account_code, nonce)
}

pub fn mock_fungible_faucet(account_id: u64, assembler: &Assembler) -> Account {
    let account_storage = AccountStorage::new(
        vec![(
            FAUCET_STORAGE_DATA_SLOT,
            [ZERO, ZERO, ZERO, Felt::new(FUNGIBLE_FAUCET_INITIAL_BALANCE)],
        )],
        Default::default(),
    )
    .unwrap();
    let account_id = AccountId::try_from(account_id).unwrap();
    let account_code = mock_account_code(assembler);
    Account::new(account_id, AccountVault::default(), account_storage, account_code, Felt::ONE)
}

pub fn mock_non_fungible_faucet(assembler: &Assembler) -> Account {
    let non_fungible_asset = non_fungible_asset_2(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);
    let nft_tree = TieredSmt::with_entries(vec![(
        Word::from(non_fungible_asset).into(),
        non_fungible_asset.into(),
    )])
    .unwrap();

    let account_storage = AccountStorage::new(
        vec![(FAUCET_STORAGE_DATA_SLOT, nft_tree.root().into())],
        (&nft_tree).into(),
    )
    .unwrap();
    let account_id = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let account_code = mock_account_code(assembler);
    Account::new(account_id, AccountVault::default(), account_storage, account_code, Felt::ONE)
}

#[derive(Debug, PartialEq)]
pub enum MockAccountType {
    StandardNew,
    StandardExisting,
    FungibleFaucet(u64),
    NonFungibleFaucet,
}
