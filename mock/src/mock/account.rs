use miden_lib::transaction::memory::FAUCET_STORAGE_DATA_SLOT;
use miden_objects::{
    accounts::{
        get_account_seed_single, Account, AccountCode, AccountId, AccountStorage, AccountType,
        SlotItem, StorageSlot,
    },
    assembly::{Assembler, ModuleAst},
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::merkle::Smt,
    Felt, FieldElement, Word, ZERO,
};

use crate::{
    constants::{
        non_fungible_asset, non_fungible_asset_2, FUNGIBLE_ASSET_AMOUNT,
        FUNGIBLE_FAUCET_INITIAL_BALANCE,
    },
    TransactionKernel,
};

// ACCOUNT IDs
// ================================================================================================

pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u64 = 3238098370154045919;
pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN: u64 = 932255360940351967;

pub const ACCOUNT_ID_SENDER: u64 = 0b0110111011u64 << 54;
pub const ACCOUNT_ID_OFF_CHAIN_SENDER: u64 = 0b0100111011u64 << 54;
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1010111100 << 54;
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN: u64 = 0b1000111100 << 54;
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1110011100 << 54;
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1: u64 = 0b1110011101 << 54;

pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1: u64 =
    0b1010010001111111010110100011011110101011010001101111110110111100u64;
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2: u64 =
    0b1010000101101010101101000110111101010110100011011110100011011101u64;
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3: u64 =
    0b1010011001011010101101000110111101010110100011011101000110111100u64;

// ACCOUNT STORAGE
// ================================================================================================

pub const STORAGE_INDEX_0: u8 = 20;
pub const STORAGE_VALUE_0: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
pub const STORAGE_INDEX_1: u8 = 30;
pub const STORAGE_VALUE_1: Word = [Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)];

pub fn storage_item_0() -> SlotItem {
    SlotItem {
        index: STORAGE_INDEX_0,
        slot: StorageSlot::new_value(STORAGE_VALUE_0),
    }
}

pub fn storage_item_1() -> SlotItem {
    SlotItem {
        index: STORAGE_INDEX_1,
        slot: StorageSlot::new_value(STORAGE_VALUE_1),
    }
}

fn mock_account_vault() -> AssetVault {
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
    AssetVault::new(&[fungible_asset, fungible_asset_1, fungible_asset_2, non_fungible_asset])
        .unwrap()
}

pub fn mock_account_storage() -> AccountStorage {
    // create account storage
    AccountStorage::new(vec![storage_item_0(), storage_item_1()]).unwrap()
}

// Constants that define the indexes of the account procedures of interest
pub const ACCOUNT_PROCEDURE_INCR_NONCE_PROC_IDX: usize = 2;
pub const ACCOUNT_PROCEDURE_SET_ITEM_PROC_IDX: usize = 3;
pub const ACCOUNT_PROCEDURE_SET_CODE_PROC_IDX: usize = 4;

// ACCOUNT ASSEMBLY CODE
// ================================================================================================

pub const DEFAULT_ACCOUNT_CODE: &str = "
    use.miden::contracts::wallets::basic->basic_wallet
    use.miden::contracts::auth::basic->basic_eoa

    export.basic_wallet::receive_asset
    export.basic_wallet::send_asset
    export.basic_eoa::auth_tx_rpo_falcon512
";

pub const DEFAULT_AUTH_SCRIPT: &str = "
    use.miden::contracts::auth::basic->auth_tx

    begin
        call.auth_tx::auth_tx_rpo_falcon512
    end
";

pub fn mock_account_code(assembler: &Assembler) -> AccountCode {
    let account_code = "\
            use.miden::account
            use.miden::tx
            use.miden::contracts::wallets::basic->wallet

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

// MOCK ACCOUNT
// ================================================================================================

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

pub fn mock_new_account(assembler: &Assembler) -> Account {
    let (acct_id, _account_seed) =
        generate_account_seed(AccountSeedType::RegularAccountUpdatableCodeOffChain);
    let account_storage = mock_account_storage();
    let account_code = mock_account_code(assembler);
    Account::new(acct_id, AssetVault::default(), account_storage, account_code, Felt::ZERO)
}

pub fn mock_account(account_id: u64, nonce: Felt, account_code: AccountCode) -> Account {
    let account_storage = mock_account_storage();
    let account_vault = mock_account_vault();
    let account_id = AccountId::try_from(account_id).unwrap();
    Account::new(account_id, account_vault, account_storage, account_code, nonce)
}

// MOCK FAUCET
// ================================================================================================

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
    let account_storage = AccountStorage::new(vec![SlotItem {
        index: FAUCET_STORAGE_DATA_SLOT,
        slot: StorageSlot::new_value([ZERO, ZERO, ZERO, initial_balance]),
    }])
    .unwrap();
    let account_id = AccountId::try_from(account_id).unwrap();
    let account_code = mock_account_code(assembler);
    Account::new(account_id, AssetVault::default(), account_storage, account_code, nonce)
}

pub fn mock_non_fungible_faucet(
    account_id: u64,
    nonce: Felt,
    empty_reserved_slot: bool,
    assembler: &Assembler,
) -> Account {
    let entires = match empty_reserved_slot {
        true => vec![],
        false => vec![(
            Word::from(non_fungible_asset_2(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN)).into(),
            non_fungible_asset_2(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN).into(),
        )],
    };

    // construct nft tree
    let nft_tree = Smt::with_entries(entires).unwrap();

    // TODO: add nft tree data to account storage?

    let account_storage = AccountStorage::new(vec![SlotItem {
        index: FAUCET_STORAGE_DATA_SLOT,
        slot: StorageSlot::new_map(*nft_tree.root()),
    }])
    .unwrap();
    let account_id = AccountId::try_from(account_id).unwrap();
    let account_code = mock_account_code(assembler);
    Account::new(account_id, AssetVault::default(), account_storage, account_code, nonce)
}

// ACCOUNT SEED GENERATION
// ================================================================================================

pub enum AccountSeedType {
    FungibleFaucetInvalidInitialBalance,
    FungibleFaucetValidInitialBalance,
    NonFungibleFaucetInvalidReservedSlot,
    NonFungibleFaucetValidReservedSlot,
    RegularAccountUpdatableCodeOnChain,
    RegularAccountUpdatableCodeOffChain,
}

/// Returns the account id and seed for the specified account type.
pub fn generate_account_seed(account_seed_type: AccountSeedType) -> (AccountId, Word) {
    let assembler = TransactionKernel::assembler();
    let init_seed: [u8; 32] = Default::default();

    let (account, account_type) = match account_seed_type {
        AccountSeedType::FungibleFaucetInvalidInitialBalance => (
            mock_fungible_faucet(
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
                ZERO,
                false,
                &assembler,
            ),
            AccountType::FungibleFaucet,
        ),
        AccountSeedType::FungibleFaucetValidInitialBalance => (
            mock_fungible_faucet(
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
                ZERO,
                true,
                &assembler,
            ),
            AccountType::FungibleFaucet,
        ),
        AccountSeedType::NonFungibleFaucetInvalidReservedSlot => (
            mock_non_fungible_faucet(
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
                ZERO,
                false,
                &assembler,
            ),
            AccountType::NonFungibleFaucet,
        ),
        AccountSeedType::NonFungibleFaucetValidReservedSlot => (
            mock_non_fungible_faucet(
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
                ZERO,
                true,
                &assembler,
            ),
            AccountType::NonFungibleFaucet,
        ),
        AccountSeedType::RegularAccountUpdatableCodeOnChain => (
            mock_account(
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
                Felt::ONE,
                mock_account_code(&assembler),
            ),
            AccountType::RegularAccountUpdatableCode,
        ),
        AccountSeedType::RegularAccountUpdatableCodeOffChain => (
            mock_account(
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
                Felt::ONE,
                mock_account_code(&assembler),
            ),
            AccountType::RegularAccountUpdatableCode,
        ),
    };

    let seed = get_account_seed_single(
        init_seed,
        account_type,
        true,
        account.code().root(),
        account.storage().root(),
    )
    .unwrap();

    let account_id = AccountId::new(seed, account.code().root(), account.storage().root()).unwrap();

    (account_id, seed)
}
