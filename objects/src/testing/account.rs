use assembly::Assembler;
use vm_core::FieldElement;

use super::constants::{self, FUNGIBLE_ASSET_AMOUNT, NON_FUNGIBLE_ASSET_DATA};
use crate::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        },
        Account, AccountCode, AccountId, AccountStorage, StorageMap, StorageSlot,
    },
    assets::{Asset, AssetVault, FungibleAsset, NonFungibleAsset},
    Felt, Word, ZERO,
};

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
