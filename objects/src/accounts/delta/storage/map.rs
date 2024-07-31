use alloc::{collections::BTreeMap, vec::Vec};
use vm_core::{
    utils::{ByteReader, ByteWriter, Deserializable, Serializable},
    Felt, Word,
};
use vm_processor::{DeserializationError, Digest};

use crate::AccountDeltaError;

/// [StorageMapDelta] stores the differences between two states of account storage maps.
///
/// The differences are represented as follows:
/// - leave updates: represented by `cleared_leaves` and `updated_leaves` field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorageMapDelta {
    pub cleared_leaves: Vec<Word>,
    pub updated_leaves: Vec<(Word, Word)>,
}

impl StorageMapDelta {
    /// Creates a new [StorageMapDelta] from the provided iteartors.
    fn from_iters<A, B>(cleared_leaves: A, updated_leaves: B) -> Self
    where
        A: IntoIterator<Item = Word>,
        B: IntoIterator<Item = (Word, Word)>,
    {
        Self {
            cleared_leaves: Vec::from_iter(cleared_leaves),
            updated_leaves: Vec::from_iter(updated_leaves),
        }
    }

    /// Creates a new storage map delta from the provided cleared and updated leaves.
    pub fn from(cleared_leaves: Vec<Word>, updated_leaves: Vec<(Word, Word)>) -> Self {
        Self::from_iters(cleared_leaves, updated_leaves)
    }

    /// Checks whether this storage map delta is valid.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Any of the cleared or updated leaves is referenced more than once (e.g., updated twice).
    pub fn validate(&self) -> Result<(), AccountDeltaError> {
        // we add all keys to a single vector and sort them to check for duplicates
        // we don't use a hash set because we want to use no-std compatible code
        let mut all_keys: Vec<Vec<u64>> = self
            .cleared_leaves
            .iter()
            .chain(self.updated_leaves.iter().map(|(key, _)| key))
            .map(|x| x.iter().map(|x| x.as_int()).collect::<Vec<_>>())
            .collect();

        all_keys.sort_unstable();

        if let Some(key) = all_keys.windows(2).find(|els| els[0] == els[1]) {
            let mut iter = key[0].iter().map(|&x| Felt::new(x));
            // we know that the key is 4 elements long
            let digest = Word::from([
                iter.next().unwrap(),
                iter.next().unwrap(),
                iter.next().unwrap(),
                iter.next().unwrap(),
            ]);
            return Err(AccountDeltaError::DuplicateStorageMapLeaf { key: digest.into() });
        }

        Ok(())
    }

    /// Returns true if storage map delta contains no updates.
    pub fn is_empty(&self) -> bool {
        self.cleared_leaves.is_empty() && self.updated_leaves.is_empty()
    }

    /// Merge `other` into this delta, giving precedence to `other`.
    pub fn merge(self, other: Self) -> Self {
        // Aggregate the changes into a map such that `other` overwrites self.
        let leaves = self.cleared_leaves.into_iter().map(|k| (k, None));
        let leaves = leaves
            .chain(self.updated_leaves.into_iter().map(|(k, v)| (k, Some(v))))
            .chain(other.cleared_leaves.into_iter().map(|k| (k, None)))
            .chain(other.updated_leaves.into_iter().map(|(k, v)| (k, Some(v))))
            .map(|(k, v)| (Digest::from(k), v.map(Digest::from)))
            .collect::<BTreeMap<_, _>>();

        let mut cleared = Vec::new();
        let mut updated = Vec::new();

        for (key, value) in leaves {
            match value {
                Some(value) => updated.push((key.into(), value.into())),
                None => cleared.push(key.into()),
            }
        }

        Self::from(cleared, updated)
    }
}

impl Serializable for StorageMapDelta {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.cleared_leaves.write_into(target);
        self.updated_leaves.write_into(target);
    }
}

impl Deserializable for StorageMapDelta {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let cleared_leaves = Vec::<_>::read_from(source)?;
        let updated_leaves = Vec::<_>::read_from(source)?;
        Ok(Self { cleared_leaves, updated_leaves })
    }
}
