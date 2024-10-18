use alloc::vec::Vec;
use core::fmt::Display;

use assembly::Assembler;
use miden_crypto::{dsa::rpo_falcon512::PublicKey, merkle::MerkleError};
use rand::Rng;
use vm_core::FieldElement;

use super::{
    account_code::DEFAULT_ACCOUNT_CODE,
    account_id::AccountIdBuilder,
    constants::{self, FUNGIBLE_ASSET_AMOUNT, NON_FUNGIBLE_ASSET_DATA},
    storage::AccountStorageBuilder,
};
use crate::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        },
        Account, AccountCode, AccountId, AccountStorage, AccountStorageMode, AccountType,
        StorageMap, StorageSlot,
    },
    assets::{Asset, AssetVault, FungibleAsset, NonFungibleAsset, TokenSymbol},
    AccountError, AssetVaultError, Felt, Word, ZERO,
};

/// Builder for an `Account`, the builder allows for a fluent API to construct an account. Each
/// account needs a unique builder.
#[derive(Clone)]
pub struct AccountBuilder<T> {
    assets: Vec<Asset>,
    storage_builder: AccountStorageBuilder,
    code: Option<AccountCode>,
    nonce: Felt,
    account_id_builder: AccountIdBuilder<T>,
}

impl<T: Rng> AccountBuilder<T> {
    pub fn new(rng: T) -> Self {
        Self {
            assets: vec![],
            storage_builder: AccountStorageBuilder::new(),
            code: None,
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

    pub fn add_storage_slot(mut self, slot: StorageSlot) -> Self {
        self.storage_builder.add_slot(slot);
        self
    }

    pub fn add_storage_slots<I: IntoIterator<Item = StorageSlot>>(mut self, slots: I) -> Self {
        self.storage_builder.add_slots(slots);
        self
    }

    pub fn code(mut self, account_code: AccountCode) -> Self {
        self.code = Some(account_code.clone());

        self
    }

    /// Compiles [DEFAULT_ACCOUNT_CODE] into [AccountCode] and sets it.
    pub fn default_code(self, assembler: Assembler, is_faucet: bool) -> Self {
        let default_account_code = AccountCode::compile(DEFAULT_ACCOUNT_CODE, assembler, is_faucet)
            .expect("Default account code should compile.");

        self.code(default_account_code)
    }

    pub fn nonce(mut self, nonce: Felt) -> Self {
        self.nonce = nonce;
        self
    }

    pub fn account_type(mut self, account_type: AccountType) -> Self {
        self.account_id_builder.account_type(account_type);
        self
    }

    pub fn storage_mode(mut self, storage_mode: AccountStorageMode) -> Self {
        self.account_id_builder.storage_mode(storage_mode);
        self
    }

    /// Configures storage slots for a wallet account with authentication.
    pub fn with_wallet_storage(mut self, public_key: PublicKey) -> Self {
        self.storage_builder.with_wallet_storage(public_key);
        self
    }

    /// Configures storage slots for a faucet account with authentication and metadata.
    pub fn with_faucet_storage(
        mut self,
        public_key: PublicKey,
        token_symbol: TokenSymbol,
        max_supply: u64,
        total_issuance: Option<u64>,
    ) -> Self {
        self.storage_builder.with_faucet_storage(
            public_key,
            token_symbol,
            max_supply,
            total_issuance,
        );
        self
    }

    pub fn build(mut self) -> Result<(Account, Word), AccountBuilderError> {
        let vault = AssetVault::new(&self.assets).map_err(AccountBuilderError::AssetVaultError)?;
        let storage = self.storage_builder.build();
        let account_code = self.code.ok_or(AccountBuilderError::AccountCodeNotSet)?;

        self.account_id_builder.code(account_code.clone());
        self.account_id_builder.storage_commitment(storage.commitment());
        let (account_id, seed) = self.account_id_builder.build()?;

        let account = Account::from_parts(account_id, vault, storage, account_code, self.nonce);
        Ok((account, seed))
    }

    /// Build an account using the provided `seed`.
    pub fn build_with_seed(mut self, seed: Word) -> Result<Account, AccountBuilderError> {
        let vault = AssetVault::new(&self.assets).map_err(AccountBuilderError::AssetVaultError)?;
        let storage = self.storage_builder.build();
        let account_code = self.code.ok_or(AccountBuilderError::AccountCodeNotSet)?;

        let account_id = self.account_id_builder.with_seed(seed)?;
        self.account_id_builder.storage_commitment(storage.commitment());

        Ok(Account::from_parts(account_id, vault, storage, account_code, self.nonce))
    }
}

#[derive(Debug)]
pub enum AccountBuilderError {
    AccountError(AccountError),
    AccountCodeNotSet,
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

impl Account {
    /// Creates a non-new mock account with a defined number of assets and storage
    pub fn mock(account_id: u64, nonce: Felt, assembler: Assembler) -> Self {
        let account_storage = AccountStorage::mock();

        let account_vault = if nonce == Felt::ZERO {
            AssetVault::default()
        } else {
            AssetVault::mock()
        };

        let account_code = AccountCode::mock_account_code(assembler, false);

        let account_id = AccountId::try_from(account_id).unwrap();
        Account::from_parts(account_id, account_vault, account_storage, account_code, nonce)
    }

    pub fn mock_fungible_faucet(
        account_id: u64,
        nonce: Felt,
        initial_balance: Felt,
        assembler: Assembler,
    ) -> Self {
        let account_storage =
            AccountStorage::new(vec![StorageSlot::Value([ZERO, ZERO, ZERO, initial_balance])])
                .unwrap();
        let account_id = AccountId::try_from(account_id).unwrap();
        let account_code = AccountCode::mock_account_code(assembler, true);
        Account::from_parts(account_id, AssetVault::default(), account_storage, account_code, nonce)
    }

    pub fn mock_non_fungible_faucet(
        account_id: u64,
        nonce: Felt,
        empty_reserved_slot: bool,
        assembler: Assembler,
    ) -> Self {
        let entries = match empty_reserved_slot {
            true => vec![],
            false => {
                let asset = NonFungibleAsset::mock(
                    ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
                    &constants::NON_FUNGIBLE_ASSET_DATA_2,
                );
                vec![(Word::from(asset).into(), asset.into())]
            },
        };
        // construct nft tree
        let nft_storage_map = StorageMap::with_entries(entries).unwrap();

        let account_storage = AccountStorage::new(vec![StorageSlot::Map(nft_storage_map)]).unwrap();
        let account_id = AccountId::try_from(account_id).unwrap();
        let account_code = AccountCode::mock_account_code(assembler, true);
        Account::from_parts(account_id, AssetVault::default(), account_storage, account_code, nonce)
    }
}

impl AssetVault {
    /// Creates an [AssetVault] with 4 default assets.
    ///
    /// The ids of the assets added to the vault are defined by the following constants:
    ///
    /// - ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN
    /// - ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1
    /// - ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2
    /// - ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN
    pub fn mock() -> Self {
        let faucet_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
        let fungible_asset =
            Asset::Fungible(FungibleAsset::new(faucet_id, FUNGIBLE_ASSET_AMOUNT).unwrap());

        let faucet_id_1: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1.try_into().unwrap();
        let fungible_asset_1 =
            Asset::Fungible(FungibleAsset::new(faucet_id_1, FUNGIBLE_ASSET_AMOUNT).unwrap());

        let faucet_id_2: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().unwrap();
        let fungible_asset_2 =
            Asset::Fungible(FungibleAsset::new(faucet_id_2, FUNGIBLE_ASSET_AMOUNT).unwrap());

        let non_fungible_asset = NonFungibleAsset::mock(
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            &NON_FUNGIBLE_ASSET_DATA,
        );
        AssetVault::new(&[fungible_asset, fungible_asset_1, fungible_asset_2, non_fungible_asset])
            .unwrap()
    }
}
