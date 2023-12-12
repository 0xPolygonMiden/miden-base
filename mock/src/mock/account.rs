use crate::constants::{
    generate_account_seed, non_fungible_asset, non_fungible_asset_2, storage_item_0,
    storage_item_1, AccountSeedType, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
    ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
    ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
    CHILD_ROOT_PARENT_LEAF_INDEX, CHILD_SMT_DEPTH, CHILD_STORAGE_INDEX_0, CHILD_STORAGE_VALUE_0,
    FUNGIBLE_ASSET_AMOUNT, FUNGIBLE_FAUCET_INITIAL_BALANCE,
};
use miden_lib::memory::FAUCET_STORAGE_DATA_SLOT;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountVault, StorageSlotType},
    assembly::{Assembler, ModuleAst},
    assets::{Asset, FungibleAsset},
    crypto::merkle::{SimpleSmt, TieredSmt},
    Felt, FieldElement, Word, ZERO,
};
use vm_processor::AdviceInputs;

fn mock_account_vault() -> AccountVault {
    // prepare fungible asset
    let faucet_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let fungible_asset =
        Asset::Fungible(FungibleAsset::new(faucet_id, FUNGIBLE_ASSET_AMOUNT).unwrap());

    // prepare second fungible asset
    let faucet_id_1: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1.try_into().unwrap();
    let fungible_asset_1 =
        Asset::Fungible(FungibleAsset::new(faucet_id_1, FUNGIBLE_ASSET_AMOUNT).unwrap());

    // prepare third fungible asset
    let faucet_id_2: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().unwrap();
    let fungible_asset_2 =
        Asset::Fungible(FungibleAsset::new(faucet_id_2, FUNGIBLE_ASSET_AMOUNT).unwrap());

    // prepare non fungible asset
    let non_fungible_asset = non_fungible_asset(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);
    AccountVault::new(&[fungible_asset, fungible_asset_1, fungible_asset_2, non_fungible_asset])
        .unwrap()
}

pub fn mock_account_storage(auxiliary_data: &mut AdviceInputs) -> AccountStorage {
    // Create an account merkle store
    let child_smt =
        SimpleSmt::with_leaves(CHILD_SMT_DEPTH, [(CHILD_STORAGE_INDEX_0, CHILD_STORAGE_VALUE_0)])
            .unwrap();
    auxiliary_data.extend_merkle_store(child_smt.inner_nodes());

    // create account storage
    AccountStorage::new(vec![
        storage_item_0(),
        storage_item_1(),
        (
            CHILD_ROOT_PARENT_LEAF_INDEX,
            (StorageSlotType::Value { value_arity: 0 }, *child_smt.root()),
        ),
    ])
    .unwrap()
}

// Constants that define the indexes of the account procedures of interest
pub const ACCOUNT_PROCEDURE_INCR_NONCE_PROC_IDX: usize = 2;
pub const ACCOUNT_PROCEDURE_SET_ITEM_PROC_IDX: usize = 3;
pub const ACCOUNT_PROCEDURE_SET_CODE_PROC_IDX: usize = 4;

pub fn mock_account_code(assembler: &Assembler) -> AccountCode {
    let account_code = "\
            use.miden::sat::account
            use.miden::sat::tx
            use.miden::wallets::basic->wallet

            # acct proc 0
            export.wallet::receive_asset
            # acct proc 1
            export.wallet::send_asset

            # acct proc 2
            export.incr_nonce
                push.0 swap
                # => [value, 0]

                exec.account::incr_nonce
                # => [0]
            end

            # acct proc 3
            export.set_item
                exec.account::set_item
                # => [R', V, 0, 0, 0]

                movup.8 drop movup.8 drop movup.8 drop
                # => [R', V]
            end

            # acct proc 4
            export.set_code
                padw swapw
                # => [CODE_ROOT, 0, 0, 0, 0]

                exec.account::set_code
                # => [0, 0, 0, 0]
            end

            # acct proc 5
            export.create_note
                # apply padding
                repeat.8
                    push.0 movdn.9
                end

                # create note
                exec.tx::create_note
                # => [ptr, 0, 0, 0, 0, 0, 0, 0, 0]
            end

            # acct proc 6
            export.account_procedure_1
                push.1.2
                add
            end

            # acct proc 7
            export.account_procedure_2
                push.2.1
                sub
            end
            ";
    let account_module_ast = ModuleAst::parse(account_code).unwrap();
    AccountCode::new(account_module_ast, assembler).unwrap()
}

pub fn mock_new_account(assembler: &Assembler, auxiliary_data: &mut AdviceInputs) -> Account {
    let (acct_id, _account_seed) =
        generate_account_seed(AccountSeedType::RegularAccountUpdatableCodeOnChain);
    let account_storage = mock_account_storage(auxiliary_data);
    let account_code = mock_account_code(assembler);
    Account::new(acct_id, AccountVault::default(), account_storage, account_code, Felt::ZERO)
}

pub fn mock_account(
    account_id: Option<u64>,
    nonce: Felt,
    code: Option<AccountCode>,
    assembler: &Assembler,
    auxiliary_data: &mut AdviceInputs,
) -> Account {
    // mock account storage
    let account_storage = mock_account_storage(auxiliary_data);

    // mock account code
    let account_code = match code {
        Some(code) => code,
        None => mock_account_code(assembler),
    };

    // Create account vault
    let account_vault = mock_account_vault();

    // Create an account with storage items
    let account_id = account_id.unwrap_or(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN);
    let account_id = AccountId::try_from(account_id).unwrap();
    Account::new(account_id, account_vault, account_storage, account_code, nonce)
}

pub fn mock_fungible_faucet(
    account_id: u64,
    nonce: Felt,
    empty_reserved_slot: bool,
    assembler: &Assembler,
) -> Account {
    let initial_balance = if empty_reserved_slot {
        ZERO
    } else {
        Felt::new(FUNGIBLE_FAUCET_INITIAL_BALANCE)
    };
    let account_storage = AccountStorage::new(vec![(
        FAUCET_STORAGE_DATA_SLOT,
        (StorageSlotType::Value { value_arity: 0 }, [ZERO, ZERO, ZERO, initial_balance]),
    )])
    .unwrap();
    let account_id = AccountId::try_from(account_id).unwrap();
    let account_code = mock_account_code(assembler);
    Account::new(account_id, AccountVault::default(), account_storage, account_code, nonce)
}

pub fn mock_non_fungible_faucet(
    account_id: u64,
    nonce: Felt,
    empty_reserved_slot: bool,
    assembler: &Assembler,
    auxiliary_data: &mut AdviceInputs,
) -> Account {
    let entires = match empty_reserved_slot {
        true => vec![],
        false => vec![(
            Word::from(non_fungible_asset_2(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN)).into(),
            non_fungible_asset_2(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN).into(),
        )],
    };

    // construct nft tree
    let nft_tree = TieredSmt::with_entries(entires).unwrap();

    // add nft tree data to auxiliary data inputs
    auxiliary_data.extend_merkle_store(nft_tree.inner_nodes());

    let account_storage = AccountStorage::new(vec![(
        FAUCET_STORAGE_DATA_SLOT,
        (StorageSlotType::Map { value_arity: 0 }, *nft_tree.root()),
    )])
    .unwrap();
    let account_id = AccountId::try_from(account_id).unwrap();
    let account_code = mock_account_code(assembler);
    Account::new(account_id, AccountVault::default(), account_storage, account_code, nonce)
}

#[derive(Debug, PartialEq)]
pub enum MockAccountType {
    StandardNew,
    StandardExisting,
    FungibleFaucet {
        acct_id: u64,
        nonce: Felt,
        empty_reserved_slot: bool,
    },
    NonFungibleFaucet {
        acct_id: u64,
        nonce: Felt,
        empty_reserved_slot: bool,
    },
}
