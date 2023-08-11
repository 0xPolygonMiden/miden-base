use super::{
    AccountError, AccountId, AccountType, AdviceInputsBuilder, ApplyDiff, Asset, Digest,
    FungibleAsset, NonFungibleAsset, StoreNode, TieredSmt, ToAdviceInputs, Vec, EMPTY_WORD, ZERO,
};
use crypto::merkle::MerkleTreeDelta;

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
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AccountVault {
    asset_tree: TieredSmt,
}

impl AccountVault {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new account vault initialized with the provided assets.
    pub fn new(assets: &[Asset]) -> Result<Self, AccountError> {
        Ok(Self {
            asset_tree: TieredSmt::with_leaves(
                assets.iter().map(|asset| (asset.vault_key().into(), (*asset).into())),
            )
            .map_err(AccountError::DuplicateAsset)?,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to this vault.
    pub fn commitment(&self) -> Digest {
        self.asset_tree.root()
    }

    /// Returns true if the specified non-fungible asset is stored in this vault.
    pub fn has_non_fungible_asset(&self, asset: Asset) -> Result<bool, AccountError> {
        if asset.is_fungible() {
            return Err(AccountError::not_a_non_fungible_asset(asset));
        }

        // check if the asset is stored in the vault
        match self.asset_tree.get_value(asset.vault_key().into()) {
            asset if asset == EMPTY_WORD => Ok(false),
            _ => Ok(true),
        }
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

        // if the tree value is [0, 0, 0, 0], the asset is not stored in the vault
        match self.asset_tree.get_value([ZERO, ZERO, ZERO, faucet_id.into()].into()) {
            asset if asset == EMPTY_WORD => Ok(0),
            asset => Ok(FungibleAsset::try_from(asset)
                .expect("tree only contains valid assets")
                .amount()),
        }
    }

    /// Returns an iterator over the assets stored in the vault.
    pub fn assets(&self) -> impl Iterator<Item = Asset> + '_ {
        // TODO: We will update [TieredSmt] to expose `.values()` which will simplify this logic.
        self.asset_tree
            .bottom_leaves()
            .flat_map(|(_, values)| {
                values
                    .iter()
                    .map(|value| Asset::try_from(value.1).expect("tree only contains valid assets"))
                    .collect::<Vec<_>>()
            })
            .chain(self.asset_tree.upper_leaves().map(|(_, _, value)| {
                Asset::try_from(value).expect("tree only contains valid assets")
            }))
    }

    // PUBLIC MODIFIERS
    // --------------------------------------------------------------------------------------------

    // ADD ASSET
    // --------------------------------------------------------------------------------------------
    /// Add the specified asset to the vault.
    ///
    /// # Errors
    /// - If the total value of two fungible assets is greater than or equal to 2^63.
    /// - If the vault already contains the same non-fungible asset.
    pub fn add_asset(&mut self, asset: Asset) -> Result<Asset, AccountError> {
        Ok(match asset {
            Asset::Fungible(asset) => Asset::Fungible(self.add_fungible_asset(asset)?),
            Asset::NonFungible(asset) => Asset::NonFungible(self.add_non_fungible_asset(asset)?),
        })
    }

    /// Add the specified fungible asset to the vault.  If the vault already contains an asset
    /// issued by the same faucet, the amounts are added together.
    ///
    /// # Errors
    /// - If the total value of assets is greater than or equal to 2^63.
    fn add_fungible_asset(&mut self, asset: FungibleAsset) -> Result<FungibleAsset, AccountError> {
        // fetch current asset value from the tree and add the new asset to it.
        let new: FungibleAsset = match self.asset_tree.get_value(asset.vault_key().into()) {
            current if current == EMPTY_WORD => asset,
            current => {
                let current: FungibleAsset =
                    current.try_into().expect("tree only contains valid assets");
                current.add(asset).map_err(AccountError::AddFungibleAssetBalanceError)?
            }
        };
        self.asset_tree.insert(new.vault_key().into(), new.into());

        // return the new asset
        Ok(new)
    }

    /// Add the specified non-fungible asset to the vault.
    ///
    /// # Errors
    /// - If the vault already contains the same non-fungible asset.
    fn add_non_fungible_asset(
        &mut self,
        asset: NonFungibleAsset,
    ) -> Result<NonFungibleAsset, AccountError> {
        // add non-fungible asset to the vault
        let old = self.asset_tree.insert(asset.vault_key().into(), asset.into());

        // if the asset already exists, return an error
        if old != EMPTY_WORD {
            return Err(AccountError::DuplicateNonFungibleAsset(asset));
        }

        Ok(asset)
    }

    // REMOVE ASSET
    // --------------------------------------------------------------------------------------------
    /// Remove the specified asset from the vault.
    ///
    /// # Errors
    /// - The fungible asset is not found in the vault.
    /// - The amount of the fungible asset in the vault is less than the amount to be removed.
    /// - The non-fungible asset is not found in the vault.
    pub fn remove_asset(&mut self, asset: Asset) -> Result<Asset, AccountError> {
        Ok(match asset {
            Asset::Fungible(asset) => Asset::Fungible(self.remove_fungible_asset(asset)?),
            Asset::NonFungible(asset) => Asset::NonFungible(self.remove_non_fungible_asset(asset)?),
        })
    }

    /// Remove the specified fungible asset from the vault.
    ///
    /// # Errors
    /// - The asset is not found in the vault.
    /// - The amount of the asset in the vault is less than the amount to be removed.
    fn remove_fungible_asset(
        &mut self,
        asset: FungibleAsset,
    ) -> Result<FungibleAsset, AccountError> {
        // fetch the asset from the vault.
        let mut current: FungibleAsset = match self.asset_tree.get_value(asset.vault_key().into()) {
            current if current == EMPTY_WORD => {
                return Err(AccountError::FungibleAssetNotFound(asset))
            }
            current => current.try_into().expect("tree only contains valid assets"),
        };

        // subtract the amount of the asset to be removed from the current amount.
        current
            .sub(asset.amount())
            .map_err(AccountError::SubtractFungibleAssetBalanceError)?;

        // if the amount of the asset is zero, remove the asset from the vault.
        let new = match current.amount() {
            0 => {
                // TODO: This logic will not result in the correct result - we need to update it as
                // [TieredSmt] doesn't handle deletions correctly at the minute.
                // return ZERO value to insert into the vault
                EMPTY_WORD
            }
            _ => current.into(),
        };
        self.asset_tree.insert(asset.vault_key().into(), new);

        // return the asset that was removed.
        Ok(asset)
    }

    /// Remove the specified non-fungible asset from the vault.
    ///
    /// # Errors
    /// - The non-fungible asset is not found in the vault.
    fn remove_non_fungible_asset(
        &mut self,
        asset: NonFungibleAsset,
    ) -> Result<NonFungibleAsset, AccountError> {
        // remove the asset from the vault.
        let old = self.asset_tree.insert(asset.vault_key().into(), EMPTY_WORD);

        // TODO: This logic will not result in the correct result - we need to update it as
        // [TieredSmt] doesn't handle deletions correctly at the minute.
        // return an error if the asset did not exist in the vault.
        if old == EMPTY_WORD {
            return Err(AccountError::NonFungibleAssetNotFound(asset));
        }

        // return the asset that was removed.
        Ok(asset)
    }
}

impl ToAdviceInputs for AccountVault {
    fn to_advice_inputs<T: AdviceInputsBuilder>(&self, target: &mut T) {
        // extend the merkle store with account vault data
        target.add_merkle_nodes(self.asset_tree.inner_nodes());

        // populate advice map with tiered merkle tree leaf nodes
        self.asset_tree.upper_leaves().for_each(|(node, key, value)| {
            target.insert_into_map(*node, (*key).into_iter().chain(value).collect());
        })
    }
}

// DIFF
// ================================================================================================
impl ApplyDiff<Digest, StoreNode> for AccountVault {
    type DiffType = MerkleTreeDelta;

    // TODO: Must find a way to apply this diff
    fn apply(&mut self, _diff: MerkleTreeDelta) {}
}
