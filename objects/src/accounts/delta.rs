use super::{AccountStorage, Felt};
use crate::{
    assembly::ModuleAst,
    assets::Asset,
    crypto::merkle::{MerkleStoreDelta, MerkleTreeDelta},
    utils::collections::Vec,
};

// ACCOUNT DELTA
// ================================================================================================

/// [AccountDelta] stores the differences between the initial and final account states.
///
/// The differences are represented as follows:
/// - code: an Option<ModuleAst> that contains the updated code of the account.
/// - nonce: if the nonce of the account has changed, the new nonce is stored here.
/// - storage: an [AccountStorageDelta] that contains the changes to the account storage.
/// - vault: an [MerkleTreeDelta] object that contains the changes to the account vault assets tree.
#[derive(Debug, Clone)]
pub struct AccountDelta {
    pub code: Option<ModuleAst>,
    pub nonce: Option<Felt>,
    pub storage: AccountStorageDelta,
    pub vault: AccountVaultDelta,
}

// ACCOUNT STORAGE DELTA
// ================================================================================================

/// [AccountStorageDelta] stores the differences between the initial and final account storage
/// states.
///
/// The differences are represented as follows:
/// - slots_delta: a `MerkleTreeDelta` that represents the changes to the account storage slots.
/// - store_delta: a `MerkleStoreDelta` that represents the changes to the account storage store.
#[derive(Debug, Clone)]
pub struct AccountStorageDelta {
    pub slots_delta: MerkleTreeDelta,
    pub store_delta: MerkleStoreDelta,
}

impl Default for AccountStorageDelta {
    fn default() -> Self {
        Self {
            slots_delta: MerkleTreeDelta::new(AccountStorage::STORAGE_TREE_DEPTH),
            store_delta: MerkleStoreDelta::default(),
        }
    }
}

// ACCOUNT VAULT DELTA
// ================================================================================================

/// [AccountVaultDelta] stores the difference between the initial and final account vault states.
///
/// The difference is represented as follows:
/// - added_assets: a vector of assets that were added to the account vault.
/// - removed_assets: a vector of assets that were removed from the account vault.
#[derive(Clone, Debug, Default)]
pub struct AccountVaultDelta {
    pub added_assets: Vec<Asset>,
    pub removed_assets: Vec<Asset>,
}
