use alloc::{
    collections::{btree_map::Entry, BTreeMap},
    vec::Vec,
};

use super::{
    AccountDeltaError, ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
};
use crate::{accounts::AccountId, Digest};
// ACCOUNT VAULT DELTA
// ================================================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NonFungibleDeltaAction {
    Add,
    Remove,
}

/// [AccountVaultDelta] stores the difference between the initial and final account vault states.
///
/// The difference is represented as follows:
/// - fungible: a binary tree map of fungible asset balance changes in the account vault.
/// - non_fungible: a binary tree map of non-fungible assets that were added to or removed from the
///   account vault.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccountVaultDelta {
    fungible: BTreeMap<AccountId, i64>,
    non_fungible: BTreeMap<Digest, NonFungibleDeltaAction>,
}

impl AccountVaultDelta {
    /// Creates an empty [AccountVaultDelta].
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates an [AccountVaultDelta] with the given fungible and non-fungible asset deltas.
    pub const fn new(
        fungible: BTreeMap<AccountId, i64>,
        non_fungible: BTreeMap<Digest, NonFungibleDeltaAction>,
    ) -> Self {
        Self { fungible, non_fungible }
    }

    /// Creates an [AccountVaultDelta] from the given iterators.
    #[cfg(feature = "testing")]
    pub fn from_iters(
        added_assets: impl IntoIterator<Item = crate::assets::Asset>,
        removed_assets: impl IntoIterator<Item = crate::assets::Asset>,
    ) -> Self {
        use miden_crypto::Word;

        use crate::assets::Asset;

        let mut fungible = BTreeMap::new();
        let mut non_fungible = BTreeMap::new();

        for asset in added_assets {
            match asset {
                Asset::Fungible(asset) => {
                    fungible.insert(asset.faucet_id(), asset.amount().try_into().unwrap());
                },
                Asset::NonFungible(asset) => {
                    non_fungible.insert(Word::from(asset).into(), NonFungibleDeltaAction::Add);
                },
            }
        }

        for asset in removed_assets {
            match asset {
                Asset::Fungible(asset) => {
                    fungible.insert(asset.faucet_id(), -i64::try_from(asset.amount()).unwrap());
                },
                Asset::NonFungible(asset) => {
                    non_fungible.insert(Word::from(asset).into(), NonFungibleDeltaAction::Remove);
                },
            }
        }

        Self { fungible, non_fungible }
    }

    /// Returns a reference to the fungible asset delta.
    pub fn fungible(&self) -> &BTreeMap<AccountId, i64> {
        &self.fungible
    }

    /// Returns a reference to the non-fungible asset delta.
    pub fn non_fungible(&self) -> &BTreeMap<Digest, NonFungibleDeltaAction> {
        &self.non_fungible
    }

    /// Returns true if this vault delta contains no updates.
    pub fn is_empty(&self) -> bool {
        self.fungible.is_empty() && self.non_fungible.is_empty()
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn added_assets(&self) -> impl Iterator<Item = crate::assets::Asset> + '_ {
        use crate::assets::{Asset, FungibleAsset, NonFungibleAsset};
        self.fungible
            .iter()
            .filter(|&(_, &value)| value >= 0)
            .map(|(&faucet_id, &diff)| {
                Asset::Fungible(FungibleAsset::new(faucet_id, diff.unsigned_abs()).unwrap())
            })
            .chain(
                self.non_fungible
                    .iter()
                    .filter(|&(_, &action)| action == NonFungibleDeltaAction::Add)
                    .map(|(&key, _)| {
                        Asset::NonFungible(unsafe { NonFungibleAsset::new_unchecked(key.into()) })
                    }),
            )
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn removed_assets(&self) -> impl Iterator<Item = crate::assets::Asset> + '_ {
        use crate::assets::{Asset, FungibleAsset, NonFungibleAsset};
        self.fungible
            .iter()
            .filter(|&(_, &value)| value < 0)
            .map(|(&faucet_id, &diff)| {
                Asset::Fungible(FungibleAsset::new(faucet_id, diff.unsigned_abs()).unwrap())
            })
            .chain(
                self.non_fungible
                    .iter()
                    .filter(|&(_, &action)| action == NonFungibleDeltaAction::Remove)
                    .map(|(&key, _)| {
                        Asset::NonFungible(unsafe { NonFungibleAsset::new_unchecked(key.into()) })
                    }),
            )
    }

    /// Merges another delta into this one, overwriting any existing values.
    ///
    /// Inputs and the result are validated as part of the merge.
    pub fn merge(&mut self, other: Self) -> Result<(), AccountDeltaError> {
        self.validate()?;
        other.validate()?;

        // Merge fungible and non-fungible assets. The former are summed while the latter can cancel
        // each other out.
        //
        // Track fungible asset amounts - positive and negative. `i64` is not lossy while
        // fungibles are restricted to 2^63-1. Overflow is still possible but we check for that.

        for (&key, &value) in other.fungible.iter() {
            match self.fungible.entry(key) {
                Entry::Vacant(entry) => {
                    entry.insert(value);
                },
                Entry::Occupied(mut entry) => {
                    let old = *entry.get();
                    let new = old.checked_add(value).ok_or(
                        AccountDeltaError::FungibleAssetDeltaOverflow {
                            faucet_id: key,
                            this: old,
                            other: value,
                        },
                    )?;

                    if new == 0 {
                        entry.remove();
                    } else {
                        *entry.get_mut() = new;
                    }
                },
            }
        }

        for (&key, &action) in other.non_fungible.iter() {
            match self.non_fungible.entry(key) {
                Entry::Vacant(entry) => {
                    entry.insert(action);
                },
                Entry::Occupied(entry) => {
                    let previous = *entry.get();
                    if previous == action {
                        // Asset cannot be added nor removed twice.
                        return Err(AccountDeltaError::DuplicateNonFungibleVaultUpdate(key));
                    }
                    // Otherwise they cancel out.
                    entry.remove();
                },
            }
        }

        self.validate()
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
        if self.fungible.len() > u16::MAX as usize {
            return Err(AccountDeltaError::TooManyFungibleAssets {
                actual: self.fungible.len(),
                max: u16::MAX as usize,
            });
        } else if self.non_fungible.len() > u16::MAX as usize {
            return Err(AccountDeltaError::TooManyNonFungibleAssets {
                actual: self.non_fungible.len(),
                max: u16::MAX as usize,
            });
        }

        Ok(())
    }
}

impl Serializable for AccountVaultDelta {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_u16(self.fungible.len().try_into().expect("too many fungible asset updates"));
        // TODO: We save `i64` as `u64` since winter utils only support unsigned integers for now.
        //   We should update this code (and deserialization as well) once it support signed
        //   integers.
        target
            .write_many(self.fungible.iter().map(|(&faucet_id, &delta)| (faucet_id, delta as u64)));

        let added: Vec<_> = self
            .non_fungible
            .iter()
            .filter(|&(_, action)| action == &NonFungibleDeltaAction::Add)
            .map(|(key, _)| *key)
            .collect();
        let removed: Vec<_> = self
            .non_fungible
            .iter()
            .filter(|&(_, action)| action == &NonFungibleDeltaAction::Remove)
            .map(|(key, _)| *key)
            .collect();

        target.write_u16(added.len().try_into().expect("too many added non-fungible assets"));
        target.write_many(added.iter());

        target.write_u16(removed.len().try_into().expect("too many removed non-fungible assets"));
        target.write_many(removed.iter());
    }
}

impl Deserializable for AccountVaultDelta {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let num_fungible_assets = source.read_u16()? as usize;
        // TODO: We save `i64` as `u64` since winter utils only support unsigned integers for now.
        //   We should update this code (and serialization as well) once it support signed integers.
        let fungible = source
            .read_many::<(AccountId, u64)>(num_fungible_assets)?
            .into_iter()
            .map(|(account_id, delta_as_u64)| (account_id, delta_as_u64 as i64))
            .collect();

        let num_added = source.read_u16()? as usize;
        let added = source.read_many::<Digest>(num_added)?;

        let num_removed = source.read_u16()? as usize;
        let removed = source.read_many::<Digest>(num_removed)?;

        let non_fungible = added
            .into_iter()
            .map(|key| (key, NonFungibleDeltaAction::Add))
            .chain(removed.into_iter().map(|key| (key, NonFungibleDeltaAction::Remove)))
            .collect();

        Ok(Self { fungible, non_fungible })
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::{AccountVaultDelta, Deserializable, Serializable};
    use crate::{
        accounts::{
            account_id::testing::{
                ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
                ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
            },
            AccountId,
        },
        assets::{Asset, FungibleAsset, NonFungibleAsset, NonFungibleAssetDetails},
        testing::storage::build_assets,
    };

    #[test]
    fn test_serde_account_vault() {
        let (asset_0, asset_1) = build_assets();
        let delta = AccountVaultDelta::from_iters([asset_0], [asset_1]);

        let serialized = delta.to_bytes();
        let deserialized = AccountVaultDelta::read_from_bytes(&serialized).unwrap();
        assert_eq!(deserialized, delta);
    }

    #[test]
    fn test_is_empty_account_vault() {
        let faucet = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
        let asset: Asset = FungibleAsset::new(faucet, 123).unwrap().into();

        assert!(AccountVaultDelta::empty().is_empty());
        assert!(!AccountVaultDelta::from_iters([asset], []).is_empty());
        assert!(!AccountVaultDelta::from_iters([], [asset]).is_empty());
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
                x if x.is_positive() => AccountVaultDelta::from_iters([asset], []),
                _ => AccountVaultDelta::from_iters([], [asset]),
            }
        }

        let account_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap();

        let mut delta_x = create_delta_with_fungible(account_id, x);
        let delta_y = create_delta_with_fungible(account_id, y);

        let result = delta_x.merge(delta_y);

        // None is used to indicate an error is expected.
        if let Some(expected) = expected {
            let expected = create_delta_with_fungible(account_id, expected);
            assert_eq!(result.map(|_| delta_x).unwrap(), expected);
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
        /// Creates an [AccountVaultDelta] with an optional [NonFungibleAsset] delta. This delta will
        /// be added if `Some(true)`, removed for `Some(false)` and missing for `None`.
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
                Some(true) => AccountVaultDelta::from_iters([asset], []),
                Some(false) => AccountVaultDelta::from_iters([], [asset]),
                None => AccountVaultDelta::empty(),
            }
        }

        let account_id = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap();

        let mut delta_x = create_delta_with_non_fungible(account_id, x);
        let delta_y = create_delta_with_non_fungible(account_id, y);

        let result = delta_x.merge(delta_y);

        if let Ok(expected) = expected {
            let expected = create_delta_with_non_fungible(account_id, expected);
            assert_eq!(result.map(|_| delta_x).unwrap(), expected);
        } else {
            assert!(result.is_err());
        }
    }
}
