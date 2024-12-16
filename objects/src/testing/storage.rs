use alloc::{collections::BTreeMap, string::String, vec::Vec};

use assembly::Assembler;
use miden_crypto::EMPTY_WORD;
use vm_core::{Felt, FieldElement, Word, ZERO};
use vm_processor::Digest;

use super::{constants::FUNGIBLE_FAUCET_INITIAL_BALANCE, prepare_word};
use crate::{
    accounts::{
        Account, AccountId, AccountIdVersion, AccountStorage, AccountStorageDelta,
        AccountStorageMode, AccountType, StorageMap, StorageMapDelta, StorageSlot,
    },
    notes::NoteAssets,
    testing::account_id::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
    },
    AccountDeltaError, BlockHeader,
};

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

pub struct SlotWithIndex {
    pub slot: StorageSlot,
    pub index: u8,
}

// CONSTANTS
// ================================================================================================

pub const FAUCET_STORAGE_DATA_SLOT: u8 = 0;

pub const STORAGE_INDEX_0: u8 = 0;
pub const STORAGE_INDEX_1: u8 = 1;
pub const STORAGE_INDEX_2: u8 = 2;

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
        AccountStorage::new(Self::mock_storage_slots()).unwrap()
    }

    pub fn mock_storage_slots() -> Vec<StorageSlot> {
        vec![Self::mock_item_0().slot, Self::mock_item_1().slot, Self::mock_item_2().slot]
    }

    pub fn mock_item_0() -> SlotWithIndex {
        SlotWithIndex {
            slot: StorageSlot::Value(STORAGE_VALUE_0),
            index: STORAGE_INDEX_0,
        }
    }

    pub fn mock_item_1() -> SlotWithIndex {
        SlotWithIndex {
            slot: StorageSlot::Value(STORAGE_VALUE_1),
            index: STORAGE_INDEX_1,
        }
    }

    pub fn mock_item_2() -> SlotWithIndex {
        SlotWithIndex {
            slot: StorageSlot::Map(Self::mock_map()),
            index: STORAGE_INDEX_2,
        }
    }

    pub fn mock_map() -> StorageMap {
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
///
/// TODO: Not all variants are needed anymore, remove unneeded parts.
pub fn generate_account_seed(
    account_seed_type: AccountSeedType,
    anchor_block_header: &BlockHeader,
    assembler: Assembler,
) -> (Account, AccountId, Word) {
    let init_seed: [u8; 32] = Default::default();

    let (account, account_type) = match account_seed_type {
        AccountSeedType::FungibleFaucetInvalidInitialBalance => (
            Account::mock_fungible_faucet(
                ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
                ZERO,
                Felt::new(FUNGIBLE_FAUCET_INITIAL_BALANCE),
                assembler,
            ),
            AccountType::FungibleFaucet,
        ),
        AccountSeedType::FungibleFaucetValidInitialBalance => (
            Account::mock_fungible_faucet(
                ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
                ZERO,
                ZERO,
                assembler,
            ),
            AccountType::FungibleFaucet,
        ),
        AccountSeedType::NonFungibleFaucetInvalidReservedSlot => (
            Account::mock_non_fungible_faucet(
                ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
                ZERO,
                false,
                assembler,
            ),
            AccountType::NonFungibleFaucet,
        ),
        AccountSeedType::NonFungibleFaucetValidReservedSlot => (
            Account::mock_non_fungible_faucet(
                ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
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

    let seed = AccountId::compute_account_seed(
        init_seed,
        account_type,
        AccountStorageMode::Public,
        AccountIdVersion::VERSION_0,
        account.code().commitment(),
        account.storage().commitment(),
        anchor_block_header.hash(),
    )
    .unwrap();

    let account_id = AccountId::new(
        seed,
        anchor_block_header.block_epoch(),
        account.code().commitment(),
        account.storage().commitment(),
        anchor_block_header.hash(),
    )
    .unwrap();

    // Overwrite old ID with generated ID.
    let (_, vault, storage, code, nonce) = account.into_parts();
    let account = Account::from_parts(account_id, vault, storage, code, nonce);

    (account, account_id, seed)
}

// UTILITIES
// --------------------------------------------------------------------------------------------

/// Returns a list of strings, one for each note asset.
pub fn prepare_assets(note_assets: &NoteAssets) -> Vec<String> {
    let mut assets = Vec::new();
    for &asset in note_assets.iter() {
        let asset_word: Word = asset.into();
        let asset_str = prepare_word(&asset_word);
        assets.push(asset_str);
    }
    assets
}
