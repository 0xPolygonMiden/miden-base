use super::{AccountError, AccountId, AccountType, Asset, Digest, Vec};
use core::default::Default;

// ACCOUNT VAULT
// ================================================================================================

/// An asset container for an account.
///
/// An account vault can contain an unlimited number of assets. The assets are stored in a Sparse
/// Merkle tree as follows:
/// - For fungible assets, the index of a node is defined by the issuing faucet ID, and the value
///   of the node is the asset itself. Thus, for any fungible asset there will be only one node
///   in the tree.
/// - For non-fungible assets, the index is defined by the asset itself, and the asset is also
///   the value of the node.
///
/// An account vault can be reduced to a single hash which is the root of the Sparse Merkle tree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountVault {
    // TODO: add backing sparse Merkle tree
    assets: Vec<Asset>,
}

impl AccountVault {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new account vault initialized with the provided assets.
    ///
    /// TODO: return error if there are duplicates in the provided asset list.
    pub fn new(assets: &[Asset]) -> Self {
        Self {
            // TODO: put assets into a Sparse Merkle trees
            assets: assets.to_vec(),
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to this vault.
    pub fn root(&self) -> Digest {
        Digest::default()
    }

    /// Returns true if the specified non-fungible asset is stored in this vault.
    pub fn has_non_fungible_asset(&self, asset: Asset) -> Result<bool, AccountError> {
        if asset.is_fungible() {
            return Err(AccountError::not_a_non_fungible_asset(asset));
        }
        todo!()
    }

    /// Returns the balance of the asset issued by the specified faucet. If the vault does not
    /// contain such an asset, 0 is returned.
    ///
    /// # Errors
    /// Returns an error if the specified ID is not an ID of a fungible asset faucet.
    pub fn get_balance(&self, faucet_id: AccountId) -> Result<u64, AccountError> {
        if !matches!(faucet_id.account_type(), AccountType::FungibleFaucet) {
            return Err(AccountError::not_a_fungible_faucet_id(faucet_id));
        }
        todo!()
    }

    /// Returns a list of assets stored in this vault.
    pub fn assets(&self) -> &[Asset] {
        &self.assets
    }
}
