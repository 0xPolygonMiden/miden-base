use alloc::{collections::BTreeMap, vec::Vec};

use miden_objects::{
    accounts::{
        AccountDelta, AccountId, AccountStorageDelta, AccountStub, AccountVaultDelta,
        StorageMapDelta,
    },
    assets::{Asset, FungibleAsset, NonFungibleAsset},
    Digest, Felt, Word, EMPTY_WORD, ZERO,
};

// ACCOUNT DELTA TRACKER
// ================================================================================================

/// Keeps track of changes made to the account during transaction execution.
///
/// Currently, this tracks:
/// - Changes to the account storage, slots and maps.
/// - Changes to the account vault.
/// - Changes to the account nonce.
///
/// TODO: implement tracking of:
/// - all account storage changes.
/// - account code changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountDeltaTracker {
    storage: AccountStorageDeltaTracker,
    vault: AccountVaultDeltaTracker,
    init_nonce: Felt,
    nonce_delta: Felt,
}

impl AccountDeltaTracker {
    /// Returns a new [AccountDeltaTracker] instantiated for the specified account.
    pub fn new(account: &AccountStub) -> Self {
        Self {
            storage: AccountStorageDeltaTracker::default(),
            vault: AccountVaultDeltaTracker::default(),
            init_nonce: account.nonce(),
            nonce_delta: ZERO,
        }
    }

    /// Consumes `self` and returns the resulting [AccountDelta].
    pub fn into_delta(self) -> AccountDelta {
        let storage_delta = self.storage.into_delta();
        let vault_delta = self.vault.into_delta();
        let nonce_delta = if self.nonce_delta == ZERO {
            None
        } else {
            Some(self.init_nonce + self.nonce_delta)
        };

        AccountDelta::new(storage_delta, vault_delta, nonce_delta).expect("invalid account delta")
    }

    /// Tracks nonce delta.
    pub fn increment_nonce(&mut self, value: Felt) {
        self.nonce_delta += value;
    }

    /// Get the vault tracker
    pub fn vault_tracker(&mut self) -> &mut AccountVaultDeltaTracker {
        &mut self.vault
    }

    /// Get the storage tracker
    pub fn storage_tracker(&mut self) -> &mut AccountStorageDeltaTracker {
        &mut self.storage
    }
}

// ACCOUNT STORAGE DELTA TRACKER
// ================================================================================================

/// The account storage delta tracker is responsible for tracking changes to the storage of the
/// account the transaction is being executed against.
///
/// The delta tracker is composed of:
/// - A map which records the latest states for the updated storage slots.
/// - A map which records the latest states for the updates storage maps
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct AccountStorageDeltaTracker {
    slot_updates: BTreeMap<u8, Word>,
    maps_updates: BTreeMap<u8, Vec<(Word, Word)>>,
}

impl AccountStorageDeltaTracker {
    /// Consumes `self` and returns the [AccountStorageDelta] that represents the changes to
    /// the account's storage.
    pub fn into_delta(self) -> AccountStorageDelta {
        let mut cleared_items = Vec::new();
        let mut updated_items = Vec::new();
        let mut updated_maps: Vec<(u8, StorageMapDelta)> = Vec::new();

        for (idx, value) in self.slot_updates {
            if value == EMPTY_WORD {
                cleared_items.push(idx);
            } else {
                updated_items.push((idx, value));
            }
        }

        for (idx, map_deltas) in self.maps_updates {
            let mut updated_leafs = Vec::new();
            let mut cleared_leafs = Vec::new();

            for map_delta in map_deltas {
                if map_delta.1 == EMPTY_WORD {
                    cleared_leafs.push(map_delta.0);
                } else {
                    updated_leafs.push(map_delta);
                }
            }
            let storage_map_delta = StorageMapDelta::from(cleared_leafs, updated_leafs);
            updated_maps.push((idx, storage_map_delta));
        }

        AccountStorageDelta {
            cleared_items,
            updated_items,
            updated_maps,
        }
    }

    /// Tracks a slot change
    pub fn slot_update(&mut self, slot_index: u8, new_slot_value: [Felt; 4]) {
        self.slot_updates.insert(slot_index, new_slot_value);
    }

    /// Tracks a slot change
    pub fn maps_update(&mut self, slot_index: u8, key: [Felt; 4], new_value: [Felt; 4]) {
        self.maps_updates.entry(slot_index).or_default().push((key, new_value));
    }
}

// ACCOUNT VAULT DELTA TRACKER
// ================================================================================================

/// The account vault delta tracker is responsible for tracking changes to the vault of the account
/// the transaction is being executed against.
///
/// The delta tracker is composed of two maps:
/// - Fungible asset map: tracks changes to the vault's fungible assets, where the key is the
///   faucet ID of the asset, and the value is the amount of the asset being added or removed from
///   the vault (positive value for added assets, negative value for removed assets).
/// - Non-fungible asset map: tracks changes to the vault's non-fungible assets, where the key is
///   the non-fungible asset, and the value is either 1 or -1 depending on whether the asset is
///   being added or removed from the vault.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct AccountVaultDeltaTracker {
    fungible_assets: BTreeMap<AccountId, i128>,
    non_fungible_assets: BTreeMap<Digest, i8>,
}

impl AccountVaultDeltaTracker {
    // STATE MUTATORS
    // --------------------------------------------------------------------------------------------

    pub fn add_asset(&mut self, asset: Asset) {
        match asset {
            Asset::Fungible(asset) => {
                update_asset_delta(
                    &mut self.fungible_assets,
                    asset.faucet_id(),
                    asset.amount() as i128,
                );
            },
            Asset::NonFungible(asset) => {
                update_asset_delta(&mut self.non_fungible_assets, asset.vault_key().into(), 1)
            },
        }
    }

    /// Track asset removal.
    pub fn remove_asset(&mut self, asset: Asset) {
        match asset {
            Asset::Fungible(asset) => {
                update_asset_delta(
                    &mut self.fungible_assets,
                    asset.faucet_id(),
                    -(asset.amount() as i128),
                );
            },
            Asset::NonFungible(asset) => {
                update_asset_delta(&mut self.non_fungible_assets, asset.vault_key().into(), -1)
            },
        }
    }

    // CONVERSIONS
    // --------------------------------------------------------------------------------------------

    /// Consumes `self` and returns the [AccountVaultDelta] that represents the changes to the
    /// account's vault.
    pub fn into_delta(self) -> AccountVaultDelta {
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
                },
                -1 => {
                    removed_assets.push(Asset::NonFungible(unsafe {
                        NonFungibleAsset::new_unchecked(*non_fungible_asset)
                    }));
                },
                _ => unreachable!("non-fungible asset amount must be 1 or -1"),
            }
        }

        AccountVaultDelta { added_assets, removed_assets }
    }
}

// HELPER FUNCTIONS
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
    use alloc::collections::btree_map::Entry;

    match delta_map.entry(key) {
        Entry::Occupied(mut entry) => {
            if entry.get() == &-amount {
                entry.remove();
            } else {
                *entry.get_mut() += amount;
            }
        },
        Entry::Vacant(entry) => {
            entry.insert(amount);
        },
    }
}
