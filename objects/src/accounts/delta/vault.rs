use alloc::{collections::BTreeMap, string::ToString, vec::Vec};

use super::{
    AccountDeltaError, Asset, ByteReader, ByteWriter, Deserializable, DeserializationError,
    Serializable,
};
use crate::{
    accounts::AccountId,
    assets::{FungibleAsset, NonFungibleAsset},
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
    ///
    /// Inputs and the result are validated as part of the merge.
    pub fn merge(self, other: Self) -> Result<Self, AccountDeltaError> {
        self.validate()?;
        other.validate()?;

        // Merge fungible and non-fungible assets separately. The former can be summed while the
        // latter is more of a boolean affair.
        //
        // Track fungible asset amounts - positive and negative. i64 is not lossy because fungible's
        // are restricted to 2^63-1. Overflow is still possible but we check for that.
        let mut fungibles = BTreeMap::<AccountId, i64>::new();
        let mut non_fungibles = BTreeMap::<NonFungibleAsset, bool>::new();

        let added = self.added_assets.into_iter().chain(other.added_assets);
        let removed = self.removed_assets.into_iter().chain(other.removed_assets);

        let assets = added.map(|asset| (asset, true)).chain(removed.map(|asset| (asset, false)));

        for (asset, is_added) in assets {
            match asset {
                Asset::Fungible(fungible) => {
                    // Ensure overflow is not possible here.
                    const _: () = assert!(FungibleAsset::MAX_AMOUNT <= i64::MIN.unsigned_abs());
                    const _: () = assert!(FungibleAsset::MAX_AMOUNT <= i64::MAX.unsigned_abs());
                    let amount = i64::try_from(fungible.amount()).unwrap();

                    let entry = fungibles.entry(fungible.faucet_id()).or_default();
                    *entry = if is_added {
                        entry.checked_add(amount)
                    } else {
                        entry.checked_sub(amount)
                    }
                    .ok_or_else(|| {
                        AccountDeltaError::AssetAmountTooBig(
                            entry.unsigned_abs() + amount.unsigned_abs(),
                        )
                    })?;
                },
                Asset::NonFungible(non_fungible) => {
                    let previous = non_fungibles.insert(non_fungible, is_added);
                    if let Some(previous) = previous {
                        if previous == is_added {
                            // Asset cannot be added nor removed twice.
                            return Err(AccountDeltaError::DuplicateVaultUpdate(asset));
                        } else {
                            // Otherwise they cancel out.
                            non_fungibles.remove(&non_fungible);
                        }
                    }
                },
            }
        }

        let mut added = Vec::new();
        let mut removed = Vec::new();

        for (faucet_id, amount) in fungibles {
            let is_positive = amount.is_positive();
            let amount: u64 = amount.abs().try_into().expect("i64::abs() always fits in u64");

            if amount == 0 {
                continue;
            }

            // We know that the faucet ID is valid since this comes from an existing asset, so the
            // only possible error case is the amount overflowing.
            let asset = FungibleAsset::new(faucet_id, amount)
                .map_err(|_| AccountDeltaError::AssetAmountTooBig(amount))?;

            if is_positive {
                added.push(Asset::Fungible(asset));
            } else {
                removed.push(Asset::Fungible(asset));
            }
        }

        for (non_fungible, is_added) in non_fungibles {
            let asset = Asset::NonFungible(non_fungible);
            if is_added {
                added.push(asset);
            } else {
                removed.push(asset);
            }
        }

        let delta = Self::from_iterators(added, removed);
        delta.validate()?;

        Ok(delta)
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

    #[rstest::rstest]
    #[case::pos_pos(50, 50, Some(100))]
    #[case::neg_neg(-50, -50, Some(-100))]
    #[case::empty_pos(0, 50, Some(50))]
    #[case::empty_neg(0, -50, Some(-50))]
    #[case::nullify_pos_neg(100, -100, Some(0))]
    #[case::nullify_neg_pos(-100, 100, Some(0))]
    #[case::overflow(FungibleAsset::MAX_AMOUNT as i64, FungibleAsset::MAX_AMOUNT as i64, None)]
    #[case::underflow(-(FungibleAsset::MAX_AMOUNT as i64), -(FungibleAsset::MAX_AMOUNT as i64), None)]
    #[test]
    fn merge_fungible_aggregation(#[case] x: i64, #[case] y: i64, #[case] expected: Option<i64>) {
        /// Creates an [AccountVaultDelta] with a single [FungibleAsset] delta. This delta will
        /// be added if `amount > 0`, removed if `amount < 0` or entirely missing if `amount == 0`.
        fn create_delta_with_fungible(account_id: AccountId, amount: i64) -> AccountVaultDelta {
            let asset = FungibleAsset::new(account_id, amount.unsigned_abs()).unwrap().into();
            match amount {
                0 => AccountVaultDelta::empty(),
                x if x.is_positive() => AccountVaultDelta {
                    added_assets: vec![asset],
                    ..Default::default()
                },
                _ => AccountVaultDelta {
                    removed_assets: vec![asset],
                    ..Default::default()
                },
            }
        }

        let account_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap();

        let delta_x = create_delta_with_fungible(account_id, x);
        let delta_y = create_delta_with_fungible(account_id, y);

        let result = delta_x.merge(delta_y);

        // None is used to indicate an error is expected.
        if let Some(expected) = expected {
            let expected = create_delta_with_fungible(account_id, expected);
            assert_eq!(result.unwrap(), expected);
        } else {
            assert!(result.is_err());
        }
    }

    #[rstest::rstest]
    #[case::empty_removed(None, Some(false), Ok(Some(false)))]
    #[case::empty_added(None, Some(true), Ok(Some(true)))]
    #[case::add_remove(Some(true), Some(false), Ok(None))]
    #[case::remove_add(Some(false), Some(true), Ok(None))]
    #[case::double_add(Some(true), Some(true), Err(()))]
    #[case::double_remove(Some(false), Some(false), Err(()))]
    #[test]
    fn merge_non_fungible_aggregation(
        #[case] x: Option<bool>,
        #[case] y: Option<bool>,
        #[case] expected: Result<Option<bool>, ()>,
    ) {
        /// Creates an [AccountVaultDelta] with an optional [NonFungibleAsset] delta. This delta
        /// will be added if `Some(true)`, removed for `Some(false)` and missing for `None`.
        fn create_delta_with_non_fungible(
            account_id: AccountId,
            added: Option<bool>,
        ) -> AccountVaultDelta {
            let asset: Asset = NonFungibleAsset::new(
                &NonFungibleAssetDetails::new(account_id, vec![1, 2, 3]).unwrap(),
            )
            .unwrap()
            .into();

            match added {
                Some(true) => AccountVaultDelta {
                    added_assets: vec![asset],
                    ..Default::default()
                },
                Some(false) => AccountVaultDelta {
                    removed_assets: vec![asset],
                    ..Default::default()
                },
                None => AccountVaultDelta::empty(),
            }
        }

        let account_id = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap();

        let delta_x = create_delta_with_non_fungible(account_id, x);
        let delta_y = create_delta_with_non_fungible(account_id, y);

        let result = delta_x.merge(delta_y);

        if let Ok(expected) = expected {
            let expected = create_delta_with_non_fungible(account_id, expected);
            assert_eq!(result.unwrap(), expected);
        } else {
            assert!(result.is_err());
        }
    }
}
