use miden_crypto::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

use super::{Asset, Digest, Felt, Hasher, NoteError, Vec, Word, WORD_SIZE, ZERO};

// NOTE VAULT
// ================================================================================================
/// An asset container for a note.
///
/// A note vault can contain up to 255 assets. The entire vault can be reduced to a single hash
/// which is computed by sequentially hashing the list of the vault's assets.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteVault {
    assets: Vec<Asset>,
    hash: Digest,
}

impl NoteVault {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------
    /// The maximum number of assets which can be carried by a single note.
    pub const MAX_NUM_ASSETS: usize = 255;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns an note asset vault constructed from the provided list of assets.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The asset list is empty.
    /// - The list contains more than 255 assets.
    /// - There are duplicate assets in the list.
    pub fn new(assets: &[Asset]) -> Result<Self, NoteError> {
        if assets.is_empty() {
            return Err(NoteError::EmptyAssetList);
        } else if assets.len() > Self::MAX_NUM_ASSETS {
            return Err(NoteError::too_many_assets(assets.len()));
        }

        // If we have an odd number of assets we pad the vector with 4 zero elements. This is to
        // ensure the number of elements is a multiple of 8 - the size of the hasher rate.
        let word_capacity = if assets.len() % 2 == 0 {
            assets.len()
        } else {
            assets.len() + 1
        };
        let mut asset_elements = Vec::with_capacity(word_capacity * WORD_SIZE);

        for (i, asset) in assets.iter().enumerate() {
            // for all assets except the last one, check if the asset is the same as any other
            // asset in the list, and if so return an error
            if i < assets.len() - 1 && assets[i + 1..].iter().any(|a| a.is_same(asset)) {
                return Err(match asset {
                    Asset::Fungible(a) => NoteError::duplicate_fungible_asset(a.faucet_id()),
                    Asset::NonFungible(a) => NoteError::duplicate_non_fungible_asset(*a),
                });
            }
            // convert the asset into field elements and add them to the list elements
            let asset_word: Word = (*asset).into();
            asset_elements.extend_from_slice(&asset_word);
        }

        // If we have an odd number of assets we pad the vector with 4 zero elements. This is to
        // ensure the number of elements is a multiple of 8 - the size of the hasher rate. This
        // simplifies hashing inside of the virtual machine when ingesting assets from the vault.
        if assets.len() % 2 == 1 {
            asset_elements.extend_from_slice(&Word::default());
        }

        Ok(Self {
            assets: assets.to_vec(),
            hash: Hasher::hash_elements(&asset_elements),
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to this vault.
    pub fn hash(&self) -> Digest {
        self.hash
    }

    /// Returns the number of assets in this vault.
    pub fn num_assets(&self) -> usize {
        self.assets.len()
    }

    /// Returns an iterator over the assets of this vault.
    pub fn iter(&self) -> core::slice::Iter<Asset> {
        self.assets.iter()
    }

    pub fn to_padded_assets(&self) -> Vec<Felt> {
        // if we have an odd number of assets with pad with a single word.
        let padded_len = if self.assets.len() % 2 == 0 {
            self.assets.len() * WORD_SIZE
        } else {
            (self.assets.len() + 1) * WORD_SIZE
        };

        // allocate a vector to hold the padded assets
        let mut padded_assets = Vec::with_capacity(padded_len * WORD_SIZE);

        // populate the vector with the assets
        padded_assets.extend(self.assets.iter().flat_map(|asset| <[Felt; 4]>::from(*asset)));

        // pad with an empty word if we have an odd number of assets
        padded_assets.resize(padded_len, ZERO);

        padded_assets
    }
}

impl TryFrom<&[Word]> for NoteVault {
    type Error = NoteError;

    fn try_from(value: &[Word]) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(NoteError::EmptyAssetList);
        } else if value.len() > Self::MAX_NUM_ASSETS {
            return Err(NoteError::too_many_assets(value.len()));
        }

        let assets = value
            .iter()
            .map(|word| (*word).try_into())
            .collect::<Result<Vec<Asset>, _>>()
            .map_err(NoteError::InvalidVaultAssetData)?;

        Self::new(&assets)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteVault {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        debug_assert!(self.assets.len() <= NoteVault::MAX_NUM_ASSETS);
        target.write_u8((self.assets.len() - 1) as u8);
        self.assets.write_into(target);
    }
}

impl Deserializable for NoteVault {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let count = source.read_u8()? + 1;
        let assets = Asset::read_batch_from(source, count.into())?;

        Self::new(&assets).map_err(|e| DeserializationError::InvalidValue(format!("{e:?}")))
    }
}
