use alloc::{collections::BTreeMap, string::String, vec::Vec};

use assembly::Assembler;
use miden_crypto::EMPTY_WORD;
use vm_core::{Felt, FieldElement, Word, ZERO};
use vm_processor::Digest;

use super::{constants::FUNGIBLE_FAUCET_INITIAL_BALANCE, prepare_word};
use crate::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
        },
        get_account_seed_single, Account, AccountCode, AccountDelta, AccountId, AccountStorage,
        AccountStorageDelta, AccountStorageMode, AccountType, AccountVaultDelta, StorageMap,
        StorageMapDelta, StorageSlot,
    },
    assets::{Asset, AssetVault, FungibleAsset},
    notes::NoteAssets,
    AccountDeltaError,
};

#[derive(Default, Debug, Clone)]
pub struct AccountStorageBuilder {
    slots: Vec<StorageSlot>,
}

/// Builder for an `AccountStorage`, the builder can be configured and used multiple times.
impl AccountStorageBuilder {
    pub fn new() -> Self {
        Self {
            slots: vec![
                AccountStorage::mock_item_0().0,
                AccountStorage::mock_item_0().0,
                AccountStorage::mock_item_1().0,
                AccountStorage::mock_item_1().0,
                AccountStorage::mock_item_2().0,
            ],
        }
    }

    pub fn add_slot(&mut self, slot: StorageSlot) -> &mut Self {
        self.slots.push(slot);
        self
    }

    pub fn add_slots<I: IntoIterator<Item = StorageSlot>>(&mut self, slots: I) -> &mut Self {
        for slot in slots.into_iter() {
            self.add_slot(slot);
        }
        self
    }

    pub fn build(&self) -> AccountStorage {
        AccountStorage::new(self.slots.clone()).unwrap()
    }
}

// ACCOUNT STORAGE DELTA BUILDER
// ================================================================================================

#[derive(Clone, Debug, Default)]
pub struct AccountStorageDeltaBuilder {
    values: BTreeMap<u8, Word>,
    maps: BTreeMap<u8, StorageMapDelta>,
}

impl AccountStorageDeltaBuilder {
    // MODIFIERS
    // -------------------------------------------------------------------------------------------

    pub fn add_cleared_items(mut self, items: impl IntoIterator<Item = u8>) -> Self {
        self.values.extend(items.into_iter().map(|slot| (slot, EMPTY_WORD)));
        self
    }

    pub fn add_updated_values(mut self, items: impl IntoIterator<Item = (u8, Word)>) -> Self {
        self.values.extend(items);
        self
    }

    pub fn add_updated_maps(
        mut self,
        items: impl IntoIterator<Item = (u8, StorageMapDelta)>,
    ) -> Self {
        self.maps.extend(items);
        self
    }

    // BUILDERS
    // -------------------------------------------------------------------------------------------

    pub fn build(self) -> Result<AccountStorageDelta, AccountDeltaError> {
        AccountStorageDelta::new(self.values, self.maps)
    }
}

// ACCOUNT STORAGE UTILS
// ================================================================================================

pub const FAUCET_STORAGE_DATA_SLOT: u8 = 0;

pub const STORAGE_VALUE_0: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
pub const STORAGE_VALUE_1: Word = [Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)];
pub const STORAGE_LEAVES_2: [(Digest, Word); 2] = [
    (
        Digest::new([Felt::new(101), Felt::new(102), Felt::new(103), Felt::new(104)]),
        [Felt::new(1_u64), Felt::new(2_u64), Felt::new(3_u64), Felt::new(4_u64)],
    ),
    (
        Digest::new([Felt::new(105), Felt::new(106), Felt::new(107), Felt::new(108)]),
        [Felt::new(5_u64), Felt::new(6_u64), Felt::new(7_u64), Felt::new(8_u64)],
    ),
];

impl AccountStorage {
    /// Create account storage:
    pub fn mock() -> Self {
        AccountStorage::new(vec![
            Self::mock_item_0().0,
            Self::mock_item_1().0,
            Self::mock_item_2().0,
        ])
        .unwrap()
    }

    /// Creates Slot with [STORAGE_VALUE_0]
    pub fn mock_item_0() -> (StorageSlot, u8) {
        (StorageSlot::Value(STORAGE_VALUE_0), 0)
    }

    /// Creates Slot with [STORAGE_VALUE_1]
    pub fn mock_item_1() -> (StorageSlot, u8) {
        (StorageSlot::Value(STORAGE_VALUE_1), 1)
    }

    /// Creates Slot with a map with [STORAGE_LEAVES_2]
    pub fn mock_item_2() -> (StorageSlot, u8) {
        (StorageSlot::Map(Self::mock_map_2()), 2)
    }

    /// Creates map with [STORAGE_LEAVES_2]
    pub fn mock_map_2() -> StorageMap {
        StorageMap::with_entries(STORAGE_LEAVES_2).unwrap()
    }
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
pub fn generate_account_seed(
    account_seed_type: AccountSeedType,
    assembler: Assembler,
) -> (AccountId, Word) {
    let init_seed: [u8; 32] = Default::default();

    let (account, account_type) = match account_seed_type {
        AccountSeedType::FungibleFaucetInvalidInitialBalance => (
            Account::mock_fungible_faucet(
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
                ZERO,
                Felt::new(FUNGIBLE_FAUCET_INITIAL_BALANCE),
                assembler,
            ),
            AccountType::FungibleFaucet,
        ),
        AccountSeedType::FungibleFaucetValidInitialBalance => (
            Account::mock_fungible_faucet(
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
                ZERO,
                ZERO,
                assembler,
            ),
            AccountType::FungibleFaucet,
        ),
        AccountSeedType::NonFungibleFaucetInvalidReservedSlot => (
            Account::mock_non_fungible_faucet(
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
                ZERO,
                false,
                assembler,
            ),
            AccountType::NonFungibleFaucet,
        ),
        AccountSeedType::NonFungibleFaucetValidReservedSlot => (
            Account::mock_non_fungible_faucet(
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
                ZERO,
                true,
                assembler,
            ),
            AccountType::NonFungibleFaucet,
        ),
        AccountSeedType::RegularAccountUpdatableCodeOnChain => (
            Account::mock(
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
                Felt::ZERO,
                assembler,
            ),
            AccountType::RegularAccountUpdatableCode,
        ),
        AccountSeedType::RegularAccountUpdatableCodeOffChain => (
            Account::mock(
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
                Felt::ZERO,
                assembler,
            ),
            AccountType::RegularAccountUpdatableCode,
        ),
    };

    let seed = get_account_seed_single(
        init_seed,
        account_type,
        AccountStorageMode::Public,
        account.code().commitment(),
        account.storage().commitment(),
    )
    .unwrap();

    let account_id =
        AccountId::new(seed, account.code().commitment(), account.storage().commitment()).unwrap();

    (account_id, seed)
}

// UTILITIES
// --------------------------------------------------------------------------------------------

pub fn build_account(assets: Vec<Asset>, nonce: Felt, slots: Vec<StorageSlot>) -> Account {
    let id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();
    let code = AccountCode::mock();

    let vault = AssetVault::new(&assets).unwrap();

    let storage = AccountStorage::new(slots).unwrap();

    Account::from_parts(id, vault, storage, code, nonce)
}

pub fn build_account_delta(
    added_assets: Vec<Asset>,
    removed_assets: Vec<Asset>,
    nonce: Felt,
    storage_delta: AccountStorageDelta,
) -> AccountDelta {
    let vault_delta = AccountVaultDelta::from_iters(added_assets, removed_assets);
    AccountDelta::new(storage_delta, vault_delta, Some(nonce)).unwrap()
}

pub fn build_assets() -> (Asset, Asset) {
    let faucet_id_0 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let asset_0: Asset = FungibleAsset::new(faucet_id_0, 123).unwrap().into();

    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
    let asset_1: Asset = FungibleAsset::new(faucet_id_1, 345).unwrap().into();

    (asset_0, asset_1)
}

pub fn prepare_assets(note_assets: &NoteAssets) -> Vec<String> {
    let mut assets = Vec::new();
    for &asset in note_assets.iter() {
        let asset_word: Word = asset.into();
        let asset_str = prepare_word(&asset_word);
        assets.push(asset_str);
    }
    assets
}
