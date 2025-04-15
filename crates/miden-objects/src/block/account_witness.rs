use alloc::string::ToString;

use miden_crypto::merkle::{LeafIndex, MerklePath, SMT_DEPTH, SmtLeaf, SmtProof, SmtProofError};

use crate::{
    AccountTreeError, Digest, Word,
    account::AccountId,
    block::AccountTree,
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
};

// ACCOUNT WITNESS
// ================================================================================================

/// A wrapper around an [`SmtProof`] that proves the inclusion of an account ID at a certain state
/// (i.e. [`Account::commitment`](crate::account::Account::commitment)) in the
/// [`AccountTree`](crate::block::AccountTree).
///
/// # Guarantees
///
/// This type guarantees that:
/// - its MerklePath is of depth [`SMT_DEPTH`].
/// - converting this type into an [`SmtProof`] results in a leaf with zero or one entries, i.e. the
///   account ID prefix is unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountWitness {
    /// The account ID that this witness proves inclusion for.
    id: AccountId,
    /// The state commitment of the account ID.
    commitment: Digest,
    /// The merkle path of the account witness.
    path: MerklePath,
}

impl AccountWitness {
    /// Constructs a new [`AccountWitness`] from the provided parts.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the merkle path's depth is not [`AccountTree::DEPTH`].
    pub fn new(
        account_id: AccountId,
        commitment: Digest,
        path: MerklePath,
    ) -> Result<Self, AccountTreeError> {
        if path.len() != SMT_DEPTH as usize {
            return Err(AccountTreeError::WitnessMerklePathDepthDoesNotMatchAccountTreeDepth(
                path.len(),
            ));
        }

        Ok(Self::new_unchecked(account_id, commitment, path))
    }

    /// Constructs a new [`AccountWitness`] from the provided proof without validating that it has
    /// zero or one entries.
    ///
    /// # Warning
    ///
    /// This does not validate any of the guarantees of this type.
    pub fn new_unchecked(account_id: AccountId, commitment: Digest, path: MerklePath) -> Self {
        Self { id: account_id, commitment, path }
    }

    /// Returns the underlying [`AccountId`] that this witness proves inclusion for.
    pub fn id(&self) -> AccountId {
        self.id
    }

    /// Returns the state commitment of the account witness.
    pub fn state_commitment(&self) -> Digest {
        self.commitment
    }

    /// Returns the [`MerklePath`] of the account witness.
    pub fn path(&self) -> &MerklePath {
        &self.path
    }

    /// Returns the [`SmtLeaf`] of the account witness.
    pub fn leaf(&self) -> SmtLeaf {
        if self.commitment == Digest::default() {
            let leaf_idx = LeafIndex::from(AccountTree::account_id_to_key(self.id));
            SmtLeaf::new_empty(leaf_idx)
        } else {
            let key = AccountTree::account_id_to_key(self.id);
            SmtLeaf::new_single(key, Word::from(self.commitment))
        }
    }

    /// Consumes self and returns the inner proof.
    pub fn into_proof(self) -> SmtProof {
        let leaf = self.leaf();
        SmtProof::new(self.path, leaf)
            .expect("merkle path depth should be the SMT depth by construction")
    }
}

impl From<AccountWitness> for SmtProof {
    fn from(witness: AccountWitness) -> Self {
        witness.into_proof()
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountWitness {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.id.write_into(target);
        self.commitment.write_into(target);
        self.path.write_into(target);
    }
}

impl Deserializable for AccountWitness {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let id = AccountId::read_from(source)?;
        let commitment = Digest::read_from(source)?;
        let path = MerklePath::read_from(source)?;

        if path.len() != SMT_DEPTH as usize {
            return Err(DeserializationError::InvalidValue(
                SmtProofError::InvalidMerklePathLength(path.len()).to_string(),
            ));
        }

        Ok(Self { id, commitment, path })
    }
}
