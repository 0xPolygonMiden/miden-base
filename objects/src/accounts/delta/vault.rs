use alloc::{
    collections::{btree_map::Entry, BTreeMap},
    string::ToString,
    vec::Vec,
};

use miden_crypto::Word;

use super::{
    AccountDeltaError, ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
};
use crate::{
    accounts::{AccountId, AccountType},
    assets::{Asset, FungibleAsset, NonFungibleAsset},
    Digest,
};
// ACCOUNT VAULT DELTA
// ================================================================================================

/// A binary tree map of fungible asset balance changes in the account vault.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FungibleAssetDelta(BTreeMap<AccountId, i64>);

impl FungibleAssetDelta {
    /// Validates and creates a new fungible asset delta.
    ///
    /// # Errors
    /// Returns an error if the delta does not pass the validation.
    pub fn new(map: BTreeMap<AccountId, i64>) -> Result<Self, AccountDeltaError> {
        let delta = Self(map);
        delta.validate()?;

        Ok(delta)
    }

    /// Adds a new fungible asset to the delta.
    ///
    /// # Errors
    /// Returns an error if the delta would overflow.
    pub fn add(&mut self, asset: FungibleAsset) -> Result<(), AccountDeltaError> {
        let amount: i64 = asset.amount().try_into().expect("Amount it too high");
        self.add_delta(asset.faucet_id(), amount)
    }

    /// Removes a fungible asset from the delta.
    ///
    /// # Errors
    /// Returns an error if the delta would overflow.
    pub fn remove(&mut self, asset: FungibleAsset) -> Result<(), AccountDeltaError> {
        let amount: i64 = asset.amount().try_into().expect("Amount it too high");
        self.add_delta(asset.faucet_id(), -amount)
    }

    /// Returns true if this vault delta contains no updates.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator over the (key, value) pairs of the map.
    pub fn iter(&self) -> impl Iterator<Item = (&AccountId, &i64)> {
        self.0.iter()
    }

    /// Merges another delta into this one, overwriting any existing values.
    ///
    /// The result is validated as part of the merge.
    ///
    /// # Errors
    /// Returns an error if the result did not pass validation.
    pub fn merge(&mut self, other: Self) -> Result<(), AccountDeltaError> {
        // Merge fungible assets.
        //
        // Track fungible asset amounts - positive and negative. `i64` is not lossy while
        // fungibles are restricted to 2^63-1. Overflow is still possible but we check for that.

        for (&faucet_id, &amount) in other.0.iter() {
            self.add_delta(faucet_id, amount)?;
        }

        self.validate()
    }

    // HELPER FUNCTIONS
    // ---------------------------------------------------------------------------------------------

    /// Updates the provided map with the provided key and amount. If the final amount is 0,
    /// the entry is removed.
    ///
    /// # Errors
    /// Returns an error if the delta would overflow.
    fn add_delta(&mut self, faucet_id: AccountId, delta: i64) -> Result<(), AccountDeltaError> {
        match self.0.entry(faucet_id) {
            Entry::Vacant(entry) => {
                entry.insert(delta);
            },
            Entry::Occupied(mut entry) => {
                let old = *entry.get();
                let new = old.checked_add(delta).ok_or(
                    AccountDeltaError::FungibleAssetDeltaOverflow {
                        faucet_id,
                        this: old,
                        other: delta,
                    },
                )?;

                if new == 0 {
                    entry.remove();
                } else {
                    *entry.get_mut() = new;
                }
            },
        }

        Ok(())
    }

    /// Checks whether this vault delta is valid.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The number updated assets is greater than [u16::MAX].
    /// - One or more fungible assets' faucet IDs are invalid.
    fn validate(&self) -> Result<(), AccountDeltaError> {
        if self.0.len() > u16::MAX as usize {
            return Err(AccountDeltaError::TooManyFungibleAssets {
                actual: self.0.len(),
                max: u16::MAX as usize,
            });
        }

        for faucet_id in self.0.keys() {
            if !matches!(faucet_id.account_type(), AccountType::FungibleFaucet) {
                return Err(AccountDeltaError::NotAFungibleFaucetId(*faucet_id));
            }
        }

        Ok(())
    }
}

impl Serializable for FungibleAssetDelta {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_u16(self.0.len().try_into().expect("too many fungible asset updates"));
        // TODO: We save `i64` as `u64` since winter utils only support unsigned integers for now.
        //   We should update this code (and deserialization as well) once it support signed
        //   integers.
        target.write_many(self.0.iter().map(|(&faucet_id, &delta)| (faucet_id, delta as u64)));
    }
}

impl Deserializable for FungibleAssetDelta {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let num_fungible_assets = source.read_u16()? as usize;
        // TODO: We save `i64` as `u64` since winter utils only support unsigned integers for now.
        //   We should update this code (and serialization as well) once it support signed integers.
        let map = source
            .read_many::<(AccountId, u64)>(num_fungible_assets)?
            .into_iter()
            .map(|(account_id, delta_as_u64)| (account_id, delta_as_u64 as i64))
            .collect();

        Self::new(map).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

/// A binary tree map of non-fungible asset changes (addition and removal) in the account vault.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NonFungibleAssetDelta(BTreeMap<Digest, NonFungibleDeltaAction>);

impl NonFungibleAssetDelta {
    /// Validates and creates a new non-fungible asset delta.
    ///
    /// # Errors
    /// Returns an error if the delta does not pass the validation.
    pub fn new(map: BTreeMap<Digest, NonFungibleDeltaAction>) -> Result<Self, AccountDeltaError> {
        let delta = Self(map);
        delta.validate()?;

        Ok(delta)
    }

    /// Adds a new non-fungible asset to the delta.
    ///
    /// # Errors
    /// Returns an error if the delta already contains the asset addition.
    pub fn add(&mut self, asset: NonFungibleAsset) -> Result<(), AccountDeltaError> {
        self.apply_action(Word::from(asset).into(), NonFungibleDeltaAction::Add)
    }

    /// Removes a non-fungible asset from the delta.
    ///
    /// # Errors
    /// Returns an error if the delta already contains the asset removal.
    pub fn remove(&mut self, asset: NonFungibleAsset) -> Result<(), AccountDeltaError> {
        self.apply_action(Word::from(asset).into(), NonFungibleDeltaAction::Remove)
    }

    /// Returns true if this vault delta contains no updates.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator over the (key, value) pairs of the map.
    pub fn iter(&self) -> impl Iterator<Item = (&Digest, &NonFungibleDeltaAction)> {
        self.0.iter()
    }

    /// Merges another delta into this one, overwriting any existing values.
    ///
    /// The result is validated as part of the merge.
    ///
    /// # Errors
    /// Returns an error if the resulted delta did not pass the validation.
    pub fn merge(&mut self, other: Self) -> Result<(), AccountDeltaError> {
        // Merge non-fungible assets. Each non-fungible asset can cancel others out.
        for (&key, &action) in other.0.iter() {
            self.apply_action(key, action)?;
        }

        self.validate()
    }

    // HELPER FUNCTIONS
    // ---------------------------------------------------------------------------------------------

    /// Updates the provided map with the provided key and action.
    /// If the action is the opposite to the previous one, the entry is removed.
    ///
    /// # Errors
    /// Returns an error if the delta already contains the provided key and action.
    fn apply_action(
        &mut self,
        key: Digest,
        action: NonFungibleDeltaAction,
    ) -> Result<(), AccountDeltaError> {
        match self.0.entry(key) {
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

        Ok(())
    }

    /// Returns an iterator over all keys that have the provided action.
    fn filter_by_action(
        &self,
        action: NonFungibleDeltaAction,
    ) -> impl Iterator<Item = Digest> + '_ {
        self.0
            .iter()
            .filter(move |&(_, cur_action)| cur_action == &action)
            .map(|(key, _)| *key)
    }

    /// Checks whether this vault delta is valid.
    ///
    /// # Errors
    /// Returns an error if the number of updates is greater than [u16::MAX].
    fn validate(&self) -> Result<(), AccountDeltaError> {
        if self.0.len() > u16::MAX as usize {
            return Err(AccountDeltaError::TooManyNonFungibleAssets {
                actual: self.0.len(),
                max: u16::MAX as usize,
            });
        }

        Ok(())
    }
}

impl Serializable for NonFungibleAssetDelta {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let added: Vec<_> = self.filter_by_action(NonFungibleDeltaAction::Add).collect();
        let removed: Vec<_> = self.filter_by_action(NonFungibleDeltaAction::Remove).collect();

        target.write_usize(added.len());
        target.write_many(added.iter());

        target.write_usize(removed.len());
        target.write_many(removed.iter());
    }
}

impl Deserializable for NonFungibleAssetDelta {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let num_added = source.read_usize()?;
        let added = source.read_many::<Digest>(num_added)?;

        let num_removed = source.read_usize()?;
        let removed = source.read_many::<Digest>(num_removed)?;

        let map = added
            .into_iter()
            .map(|key| (key, NonFungibleDeltaAction::Add))
            .chain(removed.into_iter().map(|key| (key, NonFungibleDeltaAction::Remove)))
            .collect();

        Self::new(map).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

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
    fungible: FungibleAssetDelta,
    non_fungible: NonFungibleAssetDelta,
}

impl AccountVaultDelta {
    /// Validates and creates an [AccountVaultDelta] with the given fungible and non-fungible asset
    /// deltas.
    ///
    /// # Errors
    /// Returns an error if the delta does not pass the validation.
    pub fn new(
        fungible: FungibleAssetDelta,
        non_fungible: NonFungibleAssetDelta,
    ) -> Result<Self, AccountDeltaError> {
        let delta = Self { fungible, non_fungible };

        delta.validate()?;

        Ok(delta)
    }

    /// Returns a reference to the fungible asset delta.
    pub fn fungible(&self) -> &FungibleAssetDelta {
        &self.fungible
    }

    /// Returns a reference to the non-fungible asset delta.
    pub fn non_fungible(&self) -> &NonFungibleAssetDelta {
        &self.non_fungible
    }

    /// Returns true if this vault delta contains no updates.
    pub fn is_empty(&self) -> bool {
        self.fungible.is_empty() && self.non_fungible.is_empty()
    }

    /// Tracks asset addition.
    pub fn add_asset(&mut self, asset: Asset) -> Result<(), AccountDeltaError> {
        match asset {
            Asset::Fungible(asset) => self.fungible.add(asset),
            Asset::NonFungible(asset) => self.non_fungible.add(asset),
        }
    }

    /// Tracks asset removal.
    pub fn remove_asset(&mut self, asset: Asset) -> Result<(), AccountDeltaError> {
        match asset {
            Asset::Fungible(asset) => self.fungible.remove(asset),
            Asset::NonFungible(asset) => self.non_fungible.remove(asset),
        }
    }

    /// Merges another delta into this one, overwriting any existing values.
    ///
    /// The result is validated as part of the merge.
    ///
    /// # Errors
    /// Returns an error if the resulted delta does not pass the validation.
    pub fn merge(&mut self, other: Self) -> Result<(), AccountDeltaError> {
        self.fungible.merge(other.fungible)?;
        self.non_fungible.merge(other.non_fungible)
    }

    // HELPER FUNCTIONS
    // ---------------------------------------------------------------------------------------------

    /// Checks whether this vault delta is valid.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The number of updated fungible or non-fungible assets is greater than [u16::MAX].
    /// - One or more fungible assets' faucet IDs are invalid.
    fn validate(&self) -> Result<(), AccountDeltaError> {
        self.fungible.validate()?;
        self.non_fungible.validate()
    }
}

#[cfg(any(feature = "testing", test))]
impl AccountVaultDelta {
    /// Creates an [AccountVaultDelta] from the given iterators.
    pub fn from_iters(
        added_assets: impl IntoIterator<Item = crate::assets::Asset>,
        removed_assets: impl IntoIterator<Item = crate::assets::Asset>,
    ) -> Self {
        use crate::assets::Asset;

        let mut fungible = FungibleAssetDelta::default();
        let mut non_fungible = NonFungibleAssetDelta::default();

        for asset in added_assets {
            match asset {
                Asset::Fungible(asset) => {
                    fungible.add(asset).unwrap();
                },
                Asset::NonFungible(asset) => {
                    non_fungible.add(asset).unwrap();
                },
            }
        }

        for asset in removed_assets {
            match asset {
                Asset::Fungible(asset) => {
                    fungible.remove(asset).unwrap();
                },
                Asset::NonFungible(asset) => {
                    non_fungible.remove(asset).unwrap();
                },
            }
        }

        Self { fungible, non_fungible }
    }

    /// Returns an iterator over the added assets in this delta.
    pub fn added_assets(&self) -> impl Iterator<Item = crate::assets::Asset> + '_ {
        use crate::assets::{Asset, FungibleAsset, NonFungibleAsset};
        self.fungible
            .0
            .iter()
            .filter(|&(_, &value)| value >= 0)
            .map(|(&faucet_id, &diff)| {
                Asset::Fungible(FungibleAsset::new(faucet_id, diff.unsigned_abs()).unwrap())
            })
            .chain(self.non_fungible.filter_by_action(NonFungibleDeltaAction::Add).map(|key| {
                Asset::NonFungible(unsafe { NonFungibleAsset::new_unchecked(key.into()) })
            }))
    }

    /// Returns an iterator over the removed assets in this delta.
    pub fn removed_assets(&self) -> impl Iterator<Item = crate::assets::Asset> + '_ {
        use crate::assets::{Asset, FungibleAsset, NonFungibleAsset};
        self.fungible
            .0
            .iter()
            .filter(|&(_, &value)| value < 0)
            .map(|(&faucet_id, &diff)| {
                Asset::Fungible(FungibleAsset::new(faucet_id, diff.unsigned_abs()).unwrap())
            })
            .chain(self.non_fungible.filter_by_action(NonFungibleDeltaAction::Remove).map(|key| {
                Asset::NonFungible(unsafe { NonFungibleAsset::new_unchecked(key.into()) })
            }))
    }
}

impl Serializable for AccountVaultDelta {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write(&self.fungible);
        target.write(&self.non_fungible);
    }
}

impl Deserializable for AccountVaultDelta {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let fungible = source.read()?;
        let non_fungible = source.read()?;

        Self::new(fungible, non_fungible)
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))
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

        assert!(AccountVaultDelta::default().is_empty());
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
                0 => AccountVaultDelta::default(),
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
                None => AccountVaultDelta::default(),
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
