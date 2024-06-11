use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::Display;

use assembly::Assembler;
use miden_crypto::merkle::MerkleError;
use rand::Rng;

use super::{
    account_code::{mock_account_code, DEFAULT_ACCOUNT_CODE},
    account_id::{str_to_account_code, AccountIdBuilder},
    assets::non_fungible_asset,
    constants::FUNGIBLE_ASSET_AMOUNT,
    storage::{
        generate_account_seed, storage_map_2, AccountSeedType, AccountStorageBuilder,
        STORAGE_INDEX_0, STORAGE_INDEX_1, STORAGE_INDEX_2, STORAGE_VALUE_0, STORAGE_VALUE_1,
    },
};
use crate::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        },
        Account, AccountCode, AccountId, AccountStorage, AccountStorageType, AccountType, SlotItem,
    },
    assets::{Asset, AssetVault, FungibleAsset},
    AccountError, AssetVaultError, Felt, Word, ZERO,
};

/// Builder for an `Account`, the builder allows for a fluent API to construct an account. Each
/// account needs a unique builder.
#[derive(Debug, Clone)]
pub struct AccountBuilder<T> {
    assets: Vec<Asset>,
    storage_builder: AccountStorageBuilder,
    code: String,
    nonce: Felt,
    account_id_builder: AccountIdBuilder<T>,
}

impl<T: Rng> AccountBuilder<T> {
    pub fn new(rng: T) -> Self {
        Self {
            assets: vec![],
            storage_builder: AccountStorageBuilder::new(),
            code: DEFAULT_ACCOUNT_CODE.to_string(),
            nonce: ZERO,
            account_id_builder: AccountIdBuilder::new(rng),
        }
    }

    pub fn add_asset(mut self, asset: Asset) -> Self {
        self.assets.push(asset);
        self
    }

    pub fn add_assets<I: IntoIterator<Item = Asset>>(mut self, assets: I) -> Self {
        for asset in assets.into_iter() {
            self.assets.push(asset);
        }
        self
    }

    pub fn add_storage_item(mut self, item: SlotItem) -> Self {
        self.storage_builder.add_item(item);
        self
    }

    pub fn add_storage_items<I: IntoIterator<Item = SlotItem>>(mut self, items: I) -> Self {
        self.storage_builder.add_items(items);
        self
    }

    pub fn code<C: AsRef<str>>(mut self, code: C) -> Self {
        self.code = code.as_ref().to_string();
        self
    }

    pub fn nonce(mut self, nonce: Felt) -> Self {
        self.nonce = nonce;
        self
    }

    pub fn account_type(mut self, account_type: AccountType) -> Self {
        self.account_id_builder.account_type(account_type);
        self
    }

    pub fn storage_type(mut self, storage_type: AccountStorageType) -> Self {
        self.account_id_builder.storage_type(storage_type);
        self
    }

    pub fn build(mut self, assembler: &Assembler) -> Result<Account, AccountBuilderError> {
        let vault = AssetVault::new(&self.assets).map_err(AccountBuilderError::AssetVaultError)?;
        let storage = self.storage_builder.build();
        self.account_id_builder.code(&self.code);
        self.account_id_builder.storage_root(storage.root());
        let account_id = self.account_id_builder.build(assembler)?;
        let account_code = str_to_account_code(&self.code, assembler)
            .map_err(AccountBuilderError::AccountError)?;
        Ok(Account::from_parts(account_id, vault, storage, account_code, self.nonce))
    }

    /// Build an account using the provided `seed`.
    pub fn with_seed(
        mut self,
        seed: Word,
        assembler: &Assembler,
    ) -> Result<Account, AccountBuilderError> {
        let vault = AssetVault::new(&self.assets).map_err(AccountBuilderError::AssetVaultError)?;
        let storage = self.storage_builder.build();
        self.account_id_builder.code(&self.code);
        self.account_id_builder.storage_root(storage.root());
        let account_id = self.account_id_builder.with_seed(seed, assembler)?;
        let account_code = str_to_account_code(&self.code, assembler)
            .map_err(AccountBuilderError::AccountError)?;
        Ok(Account::from_parts(account_id, vault, storage, account_code, self.nonce))
    }

    /// Build an account using the provided `seed` and `storage`.
    ///
    /// The storage items added to this builder will added on top of `storage`.
    pub fn with_seed_and_storage(
        mut self,
        seed: Word,
        mut storage: AccountStorage,
        assembler: &Assembler,
    ) -> Result<Account, AccountBuilderError> {
        let vault = AssetVault::new(&self.assets).map_err(AccountBuilderError::AssetVaultError)?;
        let inner_storage = self.storage_builder.build();

        for (key, value) in inner_storage.slots().leaves() {
            if key != AccountStorage::SLOT_LAYOUT_COMMITMENT_INDEX.into() {
                // don't copy the reserved key
                storage.set_item(key as u8, *value).map_err(AccountBuilderError::AccountError)?;
            }
        }

        self.account_id_builder.code(&self.code);
        self.account_id_builder.storage_root(storage.root());
        let account_id = self.account_id_builder.with_seed(seed, assembler)?;
        let account_code = str_to_account_code(&self.code, assembler)
            .map_err(AccountBuilderError::AccountError)?;
        Ok(Account::from_parts(account_id, vault, storage, account_code, self.nonce))
    }
}

#[derive(Debug)]
pub enum AccountBuilderError {
    AccountError(AccountError),
    AssetVaultError(AssetVaultError),
    MerkleError(MerkleError),

    /// When the created [AccountId] doesn't match the builder's configured [AccountType].
    SeedAndAccountTypeMismatch,

    /// When the created [AccountId] doesn't match the builder's `on_chain` config.
    SeedAndOnChainMismatch,
}

impl Display for AccountBuilderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AccountBuilderError {}

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
        generate_account_seed(AccountSeedType::RegularAccountUpdatableCodeOffChain, assembler);
    let account_storage = mock_account_storage();
    let account_code = mock_account_code(assembler);
    Account::from_parts(acct_id, AssetVault::default(), account_storage, account_code, ZERO)
}

pub fn mock_account(account_id: u64, nonce: Felt, account_code: AccountCode) -> Account {
    let account_storage = mock_account_storage();
    let account_vault = mock_account_vault();
    let account_id = AccountId::try_from(account_id).unwrap();
    Account::from_parts(account_id, account_vault, account_storage, account_code, nonce)
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
pub fn mock_account_vault() -> AssetVault {
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
    let mut maps = BTreeMap::new();
    maps.insert(STORAGE_INDEX_2, storage_map_2());
    AccountStorage::new(
        vec![
            SlotItem::new_value(STORAGE_INDEX_0, 0, STORAGE_VALUE_0),
            SlotItem::new_value(STORAGE_INDEX_1, 0, STORAGE_VALUE_1),
            SlotItem::new_map(STORAGE_INDEX_2, 0, storage_map_2().root().into()),
        ],
        maps,
    )
    .unwrap()
}
