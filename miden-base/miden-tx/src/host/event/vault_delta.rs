use miden_objects::{
    accounts::{AccountId, AccountVaultDelta},
    assets::{Asset, FungibleAsset, NonFungibleAsset},
    utils::collections::{btree_map::Entry, BTreeMap},
    Digest,
};
use vm_processor::{ContextId, ExecutionError, HostResponse, ProcessState};

/// The [AccountVaultDeltaHandler] is responsible for tracking changes to the vault of the account
/// the transaction is being executed against.
///
/// It is composed of two maps:
/// - [AccountVaultDeltaHandler::fungible_assets] - tracks changes to the vault's fungible assets,
/// where the key is the faucet ID of the asset, and the value is the amount of the asset being
/// added or removed from the vault.
/// - [AccountVaultDeltaHandler::non_fungible_assets] - tracks changes to the vault's non-fungible
/// assets, where the key is the non-fungible asset, and the value is either 1 or -1 depending
/// on whether the asset is being added or removed from the vault.
#[derive(Default, Debug)]
pub struct AccountVaultDeltaHandler {
    fungible_assets: BTreeMap<u64, i128>,
    non_fungible_assets: BTreeMap<Digest, i8>,
}

impl AccountVaultDeltaHandler {
    // MODIFIERS
    // --------------------------------------------------------------------------------------------

    /// Extracts the asset that is being added to the account's vault from the process state and
    /// updates the appropriate [AccountVaultDeltaHandler::fungible_assets] or
    /// [AccountVaultDeltaHandler::non_fungible_assets] map.
    pub fn add_asset<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<HostResponse, ExecutionError> {
        if process.ctx() != ContextId::root() {
            return Err(ExecutionError::EventError(
                "AddAssetToAccountVault event can only be emitted from the root context".into(),
            ));
        }

        let asset: Asset = process.get_stack_word(0).try_into().map_err(|err| {
            ExecutionError::EventError(format!(
                "Failed to apply account vault delta - asset is malformed - {err}"
            ))
        })?;

        match asset {
            Asset::Fungible(asset) => {
                update_asset_delta(
                    &mut self.fungible_assets,
                    asset.faucet_id().into(),
                    asset.amount() as i128,
                );
            }
            Asset::NonFungible(asset) => {
                update_asset_delta(&mut self.non_fungible_assets, asset.vault_key().into(), 1)
            }
        };

        Ok(HostResponse::None)
    }

    /// Extracts the asset that is being removed from the account's vault from the process state
    /// and updates the appropriate [AccountVaultDeltaHandler::fungible_assets] or
    /// [AccountVaultDeltaHandler::non_fungible_assets] map.
    pub fn remove_asset<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<HostResponse, ExecutionError> {
        if process.ctx() != ContextId::root() {
            return Err(ExecutionError::EventError(
                "RemoveAssetFromAccountVault event can only be emitted from the root context"
                    .into(),
            ));
        }

        let asset: Asset = process.get_stack_word(0).try_into().map_err(|err| {
            ExecutionError::EventError(format!(
                "Failed to apply account vault delta - asset is malformed - {err}"
            ))
        })?;

        match asset {
            Asset::Fungible(asset) => {
                update_asset_delta(
                    &mut self.fungible_assets,
                    asset.faucet_id().into(),
                    -(asset.amount() as i128),
                );
            }
            Asset::NonFungible(asset) => {
                update_asset_delta(&mut self.non_fungible_assets, asset.vault_key().into(), -1)
            }
        };

        Ok(HostResponse::None)
    }

    // CONSUMERS
    // --------------------------------------------------------------------------------------------

    /// Consumes the [AccountVaultDeltaHandler] and returns the [AccountVaultDelta] that represents the
    /// changes to the account's vault.
    pub fn finalize(self) -> AccountVaultDelta {
        let mut added_assets = Vec::new();
        let mut removed_assets = Vec::new();

        // process fungible assets
        for (faucet_id, amount) in self.fungible_assets {
            if amount > 0 {
                added_assets.push(Asset::Fungible(
                    FungibleAsset::new(
                        AccountId::new_unchecked(faucet_id.into()),
                        amount.unsigned_abs() as u64,
                    )
                    .expect("fungible asset is well formed"),
                ));
            } else {
                removed_assets.push(Asset::Fungible(
                    FungibleAsset::new(
                        AccountId::new_unchecked(faucet_id.into()),
                        amount.unsigned_abs() as u64,
                    )
                    .expect("fungible asset is well formed"),
                ));
            }
        }

        // process non-fungible assets
        for (non_fungible_asset, amount) in self.non_fungible_assets {
            match amount {
                1 => {
                    added_assets.push(Asset::NonFungible(unsafe {
                        NonFungibleAsset::new_unchecked(*non_fungible_asset)
                    }));
                }
                -1 => {
                    removed_assets.push(Asset::NonFungible(unsafe {
                        NonFungibleAsset::new_unchecked(*non_fungible_asset)
                    }));
                }
                _ => unreachable!("non-fungible asset amount must be 1 or -1"),
            }
        }

        AccountVaultDelta {
            added_assets,
            removed_assets,
        }
    }
}

// HELPERS
// ================================================================================================
/// Updates the provided map with the provided key and amount. If the final amount is 0, the entry
/// is removed from the map.
fn update_asset_delta<K, V>(delta_map: &mut BTreeMap<K, V>, key: K, amount: V)
where
    V: core::ops::Neg,
    V: core::cmp::PartialEq<<V as core::ops::Neg>::Output>,
    V: core::ops::AddAssign,
    V: Copy,
    K: Ord,
{
    match delta_map.entry(key) {
        Entry::Occupied(mut entry) => {
            if entry.get() == &-amount {
                entry.remove();
            } else {
                *entry.get_mut() += amount;
            }
        }
        Entry::Vacant(entry) => {
            entry.insert(amount);
        }
    }
}
