use miden_lib::transaction::memory::FAUCET_STORAGE_DATA_SLOT;
use miden_objects::{
    accounts::{
        get_account_seed_single, Account, AccountCode, AccountId, AccountStorage,
        AccountStorageType, AccountType, SlotItem, StorageMap, StorageSlot,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
    },
    assembly::{Assembler, ModuleAst},
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::{hash::rpo::RpoDigest, merkle::Smt},
    Felt, FieldElement, Word, ZERO,
};

use crate::{
    constants::{
        non_fungible_asset, non_fungible_asset_2, FUNGIBLE_ASSET_AMOUNT,
        FUNGIBLE_FAUCET_INITIAL_BALANCE,
    },
    TransactionKernel,
};

// ACCOUNT STORAGE
// ================================================================================================

pub const STORAGE_INDEX_0: u8 = 20;
pub const STORAGE_VALUE_0: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
pub const STORAGE_INDEX_1: u8 = 30;
pub const STORAGE_VALUE_1: Word = [Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)];

pub const STORAGE_INDEX_2: u8 = 40;
pub const STORAGE_LEAVES_2: [(RpoDigest, Word); 2] = [
    (
        RpoDigest::new([Felt::new(101), Felt::new(102), Felt::new(103), Felt::new(104)]),
        [Felt::new(1_u64), Felt::new(2_u64), Felt::new(3_u64), Felt::new(4_u64)],
    ),
    (
        RpoDigest::new([Felt::new(105), Felt::new(106), Felt::new(107), Felt::new(108)]),
        [Felt::new(5_u64), Felt::new(6_u64), Felt::new(7_u64), Felt::new(8_u64)],
    ),
];

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

pub fn storage_map_2() -> StorageMap {
    StorageMap::with_entries(STORAGE_LEAVES_2).unwrap()
}

pub fn storage_item_2() -> SlotItem {
    SlotItem {
        index: STORAGE_INDEX_2,
        slot: StorageSlot::new_map(Word::from(storage_map_2().root())),
    }
}

/// Creates an [AssetVault] with 4 assets.
///
/// The ids of the assets added to the vault are defined by the following constants:
///
/// - ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN
/// - ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1
/// - ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2
/// - ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN
///
fn mock_account_vault() -> AssetVault {
    let faucet_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let fungible_asset =
        Asset::Fungible(FungibleAsset::new(faucet_id, FUNGIBLE_ASSET_AMOUNT).unwrap());

    let faucet_id_1: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1.try_into().unwrap();
    let fungible_asset_1 =
        Asset::Fungible(FungibleAsset::new(faucet_id_1, FUNGIBLE_ASSET_AMOUNT).unwrap());

    let faucet_id_2: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().unwrap();
    let fungible_asset_2 =
        Asset::Fungible(FungibleAsset::new(faucet_id_2, FUNGIBLE_ASSET_AMOUNT).unwrap());

    let non_fungible_asset = non_fungible_asset(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);
    AssetVault::new(&[fungible_asset, fungible_asset_1, fungible_asset_2, non_fungible_asset])
        .unwrap()
}

pub fn mock_account_storage() -> AccountStorage {
    // create account storage
    AccountStorage::new(
        vec![storage_item_0(), storage_item_1(), storage_item_2()],
        vec![storage_map_2()],
    )
    .unwrap()
}

// The MAST root of the default account's interface. Use these constants to interact with the
// account's procedures.
const MASTS: [&str; 9] = [
    "0xe06a83054c72efc7e32698c4fc6037620cde834c9841afb038a5d39889e502b6",
    "0xd0260c15a64e796833eb2987d4072ac2ea824b3ce4a54a1e693bada6e82f71dd",
    "0xd765111e22479256e87a57eaf3a27479d19cc876c9a715ee6c262e0a0d47a2ac",
    "0x17b326d5403115afccc0727efa72bd929bfdc7bbf284c7c28a7aadade5d4cc9d",
    "0x8f6abddf9215c9fcb8cd02dfeb8cbfbba3130a6da3477bb918d17cfec91176ce",
    "0x73c14f65d2bab6f52eafc4397e104b3ab22a470f6b5cbc86d4aa4d3978c8b7d4",
    "0xef07641ea1aa8fe85d8f854d29bf729b92251e1433244892138fd9ca898a5a22",
    "0xff06b90f849c4b262cbfbea67042c4ea017ea0e9c558848a951d44b23370bec5",
    "0x8ef0092134469a1330e3c468f57c7f085ce611645d09cc7516c786fefc71d794",
];
pub const ACCOUNT_RECEIVE_ASSET_MAST_ROOT: &str = MASTS[0];
pub const ACCOUNT_SEND_ASSET_MAST_ROOT: &str = MASTS[1];
pub const ACCOUNT_INCR_NONCE_MAST_ROOT: &str = MASTS[2];
pub const ACCOUNT_SET_ITEM_MAST_ROOT: &str = MASTS[3];
pub const ACCOUNT_SET_MAP_ITEM_MAST_ROOT: &str = MASTS[4];
pub const ACCOUNT_SET_CODE_MAST_ROOT: &str = MASTS[5];
pub const ACCOUNT_CREATE_NOTE_MAST_ROOT: &str = MASTS[6];
pub const ACCOUNT_ACCOUNT_PROCEDURE_1_MAST_ROOT: &str = MASTS[7];
pub const ACCOUNT_ACCOUNT_PROCEDURE_2_MAST_ROOT: &str = MASTS[8];

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

            # acct proc 4
            export.set_map_item
                exec.account::set_map_item
                # => [R', V, 0, 0, 0]

                movup.8 drop movup.8 drop movup.8 drop
                # => [R', V]
            end

            # acct proc 5
            export.set_code
                padw swapw
                # => [CODE_ROOT, 0, 0, 0, 0]

                exec.account::set_code
                # => [0, 0, 0, 0]
            end

            # acct proc 6
            export.create_note
                exec.tx::create_note
                # => [ptr, 0, 0, 0, 0, 0, 0, 0, 0, 0]
            end

            # acct proc 7
            export.account_procedure_1
                push.1.2
                add
            end

            # acct proc 8
            export.account_procedure_2
                push.2.1
                sub
            end
            ";
    let account_module_ast = ModuleAst::parse(account_code).unwrap();
    let code = AccountCode::new(account_module_ast, assembler).unwrap();

    // Ensures the mast root constants match the latest version of the code.
    //
    // The constants will change if the library code changes, and need to be updated so that the
    // tests will work properly. If these asserts fail, copy the value of the code (the left
    // value), into the constants.
    //
    // Comparing all the values together, in case multiple of them change, a single test run will
    // detect it.
    let current = [
        code.procedures()[0].to_hex(),
        code.procedures()[1].to_hex(),
        code.procedures()[2].to_hex(),
        code.procedures()[3].to_hex(),
        code.procedures()[4].to_hex(),
        code.procedures()[5].to_hex(),
        code.procedures()[6].to_hex(),
        code.procedures()[7].to_hex(),
        code.procedures()[8].to_hex(),
    ];
    assert!(current == MASTS, "const MASTS: [&str; 9] = {:?};", current);

    code
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
    Account::new(acct_id, AssetVault::default(), account_storage, account_code, ZERO)
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
    let account_storage = AccountStorage::new(
        vec![SlotItem {
            index: FAUCET_STORAGE_DATA_SLOT,
            slot: StorageSlot::new_value([ZERO, ZERO, ZERO, initial_balance]),
        }],
        vec![],
    )
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

    let account_storage = AccountStorage::new(
        vec![SlotItem {
            index: FAUCET_STORAGE_DATA_SLOT,
            slot: StorageSlot::new_map(*nft_tree.root()),
        }],
        vec![],
    )
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
        AccountStorageType::OnChain,
        account.code().root(),
        account.storage().root(),
    )
    .unwrap();

    let account_id = AccountId::new(seed, account.code().root(), account.storage().root()).unwrap();

    (account_id, seed)
}
