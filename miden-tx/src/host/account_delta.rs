use alloc::{collections::BTreeMap, string::ToString, vec::Vec};

use miden_lib::transaction::{memory::ACCT_STORAGE_ROOT_PTR, TransactionKernelError};
use miden_objects::{
    accounts::{
        AccountDelta, AccountId, AccountStorage, AccountStorageDelta, AccountStub,
        AccountVaultDelta,
    },
    assets::{Asset, FungibleAsset, NonFungibleAsset},
    Digest, Felt, Word, EMPTY_WORD, ZERO,
};
use vm_processor::{ContextId, ProcessState};

use super::{AdviceProvider, TransactionHost};

// CONSTANTS
// ================================================================================================

const STORAGE_TREE_DEPTH: Felt = Felt::new(AccountStorage::STORAGE_TREE_DEPTH as u64);

// ACCOUNT DELTA TRACKER
// ================================================================================================

/// Keeps track of changes made to the account during transaction execution.
///
/// Currently, this tracks:
/// - Changes to the account storage slots.
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
}

// EVENT HANDLERS
// ================================================================================================

impl<A: AdviceProvider> TransactionHost<A> {
    /// Extracts the nonce increment from the process state and adds it to the nonce delta tracker.
    pub(super) fn on_account_increment_nonce<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        let value = process.get_stack_item(0);
        self.account_delta.nonce_delta += value;
        Ok(())
    }

    // ACCOUNT STORAGE UPDATE HANDLERS
    // --------------------------------------------------------------------------------------------

    /// Extracts information from the process state about the storage slot being updated and
    /// records the latest value of this storage slot.
    pub(super) fn on_account_storage_set_item<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        let storage_root = process
            .get_mem_value(ContextId::root(), ACCT_STORAGE_ROOT_PTR)
            .expect("no storage root");

        // get slot index from the stack and make sure it is valid
        let slot_index = process.get_stack_item(0);
        if slot_index.as_int() as usize >= AccountStorage::NUM_STORAGE_SLOTS {
            return Err(TransactionKernelError::InvalidStorageSlotIndex(slot_index.as_int()));
        }

        // get the value to which the slot is being updated
        let new_slot_value = [
            process.get_stack_item(4),
            process.get_stack_item(3),
            process.get_stack_item(2),
            process.get_stack_item(1),
        ];

        // try to get the current value for the slot from the advice provider
        let current_slot_value = self
            .adv_provider
            .get_tree_node(storage_root, &STORAGE_TREE_DEPTH, &slot_index)
            .map_err(|err| {
                TransactionKernelError::MissingStorageSlotValue(
                    slot_index.as_int() as u8,
                    err.to_string(),
                )
            })?;

        // update the delta tracker only if the current and new values are different
        if current_slot_value != new_slot_value {
            let slot_index = slot_index.as_int() as u8;
            self.account_delta.storage.slot_updates.insert(slot_index, new_slot_value);
        }

        Ok(())
    }

    // ACCOUNT VAULT UPDATE HANDLERS
    // --------------------------------------------------------------------------------------------

    /// Extracts the asset that is being added to the account's vault from the process state and
    /// updates the appropriate fungible or non-fungible asset map.
    pub(super) fn on_account_vault_add_asset<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        let asset: Asset = process
            .get_stack_word(0)
            .try_into()
            .map_err(TransactionKernelError::MalformedAssetOnAccountVaultUpdate)?;

        self.account_delta.vault.add_asset(asset);
        Ok(())
    }

    /// Extracts the asset that is being removed from the account's vault from the process state
    /// and updates the appropriate fungible or non-fungible asset map.
    pub(super) fn on_account_vault_remove_asset<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        let asset: Asset = process
            .get_stack_word(0)
            .try_into()
            .map_err(TransactionKernelError::MalformedAssetOnAccountVaultUpdate)?;

        self.account_delta.vault.remove_asset(asset);
        Ok(())
    }
}

// ACCOUNT STORAGE DELTA TRACKER
// ================================================================================================

/// The account storage delta tracker is responsible for tracking changes to the storage of the
/// account the transaction is being executed against.
///
/// The delta tracker is composed of:
/// - A map which records the latest states for the updated storage slots.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct AccountStorageDeltaTracker {
    slot_updates: BTreeMap<u8, Word>,
}

impl AccountStorageDeltaTracker {
    /// Consumes `self` and returns the [AccountStorageDelta] that represents the changes to
    /// the account's storage.
    pub fn into_delta(self) -> AccountStorageDelta {
        let mut cleared_items = Vec::new();
        let mut updated_items = Vec::new();

        for (idx, value) in self.slot_updates {
            if value == EMPTY_WORD {
                cleared_items.push(idx);
            } else {
                updated_items.push((idx, value));
            }
        }

        AccountStorageDelta { cleared_items, updated_items }
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
struct AccountVaultDeltaTracker {
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
