use alloc::{collections::BTreeMap, string::ToString, vec::Vec};

use super::{
    AccountDeltaError, Asset, ByteReader, ByteWriter, Deserializable, DeserializationError,
    Serializable,
};

// ACCOUNT VAULT DELTA
// ================================================================================================

/// [AccountVaultDelta] stores the difference between the initial and final account vault states.
///
/// The difference is represented as follows:
/// - added_assets: a vector of assets that were added to the account vault.
/// - removed_assets: a vector of assets that were removed from the account vault.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccountVaultDelta {
    pub added_assets: Vec<Asset>,
    pub removed_assets: Vec<Asset>,
}

impl AccountVaultDelta {
    /// Creates an empty [AccountVaultDelta].
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates an [AccountVaultDelta] from the given iterators.
    pub fn from_iterators<A, B>(added_assets: A, removed_assets: B) -> Self
    where
        A: IntoIterator<Item = Asset>,
        B: IntoIterator<Item = Asset>,
    {
        Self {
            added_assets: Vec::from_iter(added_assets),
            removed_assets: Vec::from_iter(removed_assets),
        }
    }

    /// Merges another delta into this one, overwriting any existing values.
    pub fn merge(self, other: Self) -> Self {
        // Collec the assets into a hashmap where true indicates added, and false removed.
        // This lets us simplify the addition/removal dance we would otherwise have to go through.
        //
        // Ordering matters. Since later items will overwrite earlier ones we need to begin
        // with self, then other.
        let assets = self
            .added_assets
            .into_iter()
            .map(|asset| (asset, true))
            .chain(self.removed_assets.into_iter().map(|asset| (asset, true)))
            .chain(other.added_assets.into_iter().map(|asset| (asset, false)))
            .chain(other.removed_assets.into_iter().map(|asset| (asset, false)))
            .collect::<BTreeMap<_, _>>();

        let added = assets.iter().filter_map(|(asset, was_added)| was_added.then_some(*asset));
        let removed = assets.iter().filter_map(|(asset, was_added)| (!was_added).then_some(*asset));

        Self::from_iterators(added, removed)
    }

    /// Checks whether this vault delta is valid.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The number added assets is greater than [u16::MAX].
    /// - The number of removed assets is greater than [u16::MAX].
    /// - The same asset was added more than once, removed more than once, or both added and
    ///   removed.
    pub fn validate(&self) -> Result<(), AccountDeltaError> {
        if self.added_assets.len() > u16::MAX as usize {
            return Err(AccountDeltaError::TooManyAddedAsset {
                actual: self.added_assets.len(),
                max: u16::MAX as usize,
            });
        } else if self.removed_assets.len() > u16::MAX as usize {
            return Err(AccountDeltaError::TooManyRemovedAssets {
                actual: self.removed_assets.len(),
                max: u16::MAX as usize,
            });
        }

        // make sure all added assets are unique
        for (pos, asset) in self.added_assets.iter().enumerate() {
            if self.added_assets[..pos].iter().any(|a| a.is_same(asset)) {
                return Err(AccountDeltaError::DuplicateVaultUpdate(*asset));
            }
        }

        // make sure all removed assets are the same
        for (pos, asset) in self.removed_assets.iter().enumerate() {
            if self.removed_assets[..pos].iter().any(|a| a.is_same(asset)) {
                return Err(AccountDeltaError::DuplicateVaultUpdate(*asset));
            }

            if self.added_assets.iter().any(|a| a.is_same(asset)) {
                return Err(AccountDeltaError::DuplicateVaultUpdate(*asset));
            }
        }

        Ok(())
    }

    /// Returns true if this vault delta contains no updates.
    pub fn is_empty(&self) -> bool {
        self.added_assets.is_empty() && self.removed_assets.is_empty()
    }
}

impl Serializable for AccountVaultDelta {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        assert!(self.added_assets.len() <= u16::MAX as usize, "too many added assets");
        target.write_u16(self.added_assets.len() as u16);
        target.write_many(self.added_assets.iter());

        assert!(self.removed_assets.len() <= u16::MAX as usize, "too many removed assets");
        target.write_u16(self.removed_assets.len() as u16);
        target.write_many(self.removed_assets.iter());
    }
}

impl Deserializable for AccountVaultDelta {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        // deserialize and validate added assets
        let num_added_assets = source.read_u16()? as usize;
        let mut added_assets: Vec<Asset> = Vec::with_capacity(num_added_assets);
        for _ in 0..num_added_assets {
            let asset = Asset::read_from(source)?;
            if added_assets.iter().any(|a| a.is_same(&asset)) {
                return Err(DeserializationError::InvalidValue(
                    "asset added more than once".to_string(),
                ));
            }

            added_assets.push(asset);
        }

        // deserialize and validate removed assets
        let num_removed_assets = source.read_u16()? as usize;
        let mut removed_assets: Vec<Asset> = Vec::with_capacity(num_removed_assets);
        for _ in 0..num_removed_assets {
            let asset = Asset::read_from(source)?;

            if removed_assets.iter().any(|a| a.is_same(&asset)) {
                return Err(DeserializationError::InvalidValue(
                    "asset added more than once".to_string(),
                ));
            }

            if added_assets.iter().any(|a| a.is_same(&asset)) {
                return Err(DeserializationError::InvalidValue(
                    "asset both added and removed".to_string(),
                ));
            }
            removed_assets.push(asset);
        }

        Ok(Self { added_assets, removed_assets })
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::{AccountVaultDelta, Asset, Deserializable, Serializable};
    use crate::{
        accounts::{
            account_id::testing::{
                ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
                ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            },
            AccountId,
        },
        assets::{FungibleAsset, NonFungibleAsset, NonFungibleAssetDetails},
        testing::storage::build_assets,
    };

    #[test]
    fn account_vault_delta_validation() {
        let ffid1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap();
        let ffid2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
        let nffid1 = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap();
        let nffid2 = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();

        let asset1: Asset = FungibleAsset::new(ffid1, 10).unwrap().into();
        let asset2: Asset = FungibleAsset::new(ffid1, 30).unwrap().into();
        let asset3: Asset = FungibleAsset::new(ffid2, 20).unwrap().into();

        let asset4: Asset =
            NonFungibleAsset::new(&NonFungibleAssetDetails::new(nffid1, vec![1, 2, 3]).unwrap())
                .unwrap()
                .into();
        let asset5: Asset =
            NonFungibleAsset::new(&NonFungibleAssetDetails::new(nffid1, vec![4, 5, 6]).unwrap())
                .unwrap()
                .into();
        let asset6: Asset =
            NonFungibleAsset::new(&NonFungibleAssetDetails::new(nffid2, vec![7, 8, 9]).unwrap())
                .unwrap()
                .into();

        assert_eq!(asset5, Asset::read_from_bytes(&asset5.to_bytes()).unwrap());

        // control case
        let delta = AccountVaultDelta {
            added_assets: vec![asset1, asset4, asset5],
            removed_assets: vec![asset3, asset6],
        };
        assert!(delta.validate().is_ok());

        let bytes = delta.to_bytes();
        assert_eq!(AccountVaultDelta::read_from_bytes(&bytes), Ok(delta));

        // duplicate asset in added assets
        let delta = AccountVaultDelta {
            added_assets: vec![asset1, asset4, asset5, asset2],
            removed_assets: vec![],
        };
        assert!(delta.validate().is_err());

        let bytes = delta.to_bytes();
        assert!(AccountVaultDelta::read_from_bytes(&bytes).is_err());

        // duplicate asset in removed assets
        let delta = AccountVaultDelta {
            added_assets: vec![],
            removed_assets: vec![asset1, asset4, asset5, asset2],
        };
        assert!(delta.validate().is_err());

        let bytes = delta.to_bytes();
        assert!(AccountVaultDelta::read_from_bytes(&bytes).is_err());

        // duplicate asset across added and removed assets
        let delta = AccountVaultDelta {
            added_assets: vec![asset1, asset3],
            removed_assets: vec![asset4, asset5, asset2],
        };
        assert!(delta.validate().is_err());

        let bytes = delta.to_bytes();
        assert!(AccountVaultDelta::read_from_bytes(&bytes).is_err());
    }

    #[test]
    fn test_serde_account_vault() {
        let (asset_0, asset_1) = build_assets();
        let delta = AccountVaultDelta::from_iterators([asset_0], [asset_1]);

        let serialized = delta.to_bytes();
        let deserialized = AccountVaultDelta::read_from_bytes(&serialized).unwrap();
        assert_eq!(deserialized, delta);
    }

    #[test]
    fn test_is_empty_account_vault() {
        let faucet = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
        let asset: Asset = FungibleAsset::new(faucet, 123).unwrap().into();

        assert!(AccountVaultDelta::empty().is_empty());
        assert!(!AccountVaultDelta::from_iterators([asset], []).is_empty());
        assert!(!AccountVaultDelta::from_iterators([], [asset]).is_empty());
    }
}
