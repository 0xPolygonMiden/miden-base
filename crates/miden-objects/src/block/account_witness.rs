use alloc::string::ToString;

use miden_crypto::merkle::{SmtLeaf, SmtProof};
use vm_core::{
    EMPTY_WORD, Felt,
    utils::{ByteReader, ByteWriter, Deserializable, Serializable},
};
use vm_processor::DeserializationError;

use crate::{
    AccountTreeError, Digest,
    account::{AccountId, AccountIdPrefix},
    block::AccountTree,
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
/// - its SmtLeaf contains zero or one entries, i.e. that the account ID prefix is unique.
/// - the leaf index is a valid account ID prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountWitness {
    /// The suffix of the account ID for which this witness is for. Storing just the suffix of the
    /// account ID is sufficient.
    ///
    /// Even though we only ever store zero or one entry, this is needed to differentiate two
    /// cases:
    ///
    /// - The leaf contains exactly the account ID which this proof is for. If so, the commitment
    ///   in the leaf is for that account ID.
    /// - The leaf contains another account ID whose prefix matches, but not its suffix. For
    ///   example, if a new account is attempted to be created in the chain while an account whose
    ///   prefix matches already exists, the witness that is fetched for this account will
    ///   (correctly) contain the leaf of the existing account. In that case,
    ///   `Self::state_commitment` must return the empty digest instead.
    id_suffix: Felt,
    /// The underlying proof of the witness.
    proof: SmtProof,
}

impl AccountWitness {
    /// Constructs a new [`AccountWitness`] from the provided proof.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the guarantees of the type are not met. See the type-level docs
    /// for details.
    pub fn new(account_id: AccountId, proof: SmtProof) -> Result<Self, AccountTreeError> {
        Self::new_inner(account_id.suffix(), proof)
    }

    /// Constructs a new [`AccountWitness`] from the provided proof.
    ///
    /// Note that we do not check whether the suffix exists in the leaf, because the proof could be
    /// for an empty leaf - which is valid - but then the suffix wouldn't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the guarantees of the type are not met. See the type-level docs
    /// for details.
    fn new_inner(id_suffix: Felt, proof: SmtProof) -> Result<Self, AccountTreeError> {
        let id_prefix = AccountIdPrefix::try_from(proof.leaf().index().value())
            .map_err(AccountTreeError::InvalidAccountIdPrefix)?;

        if proof.leaf().num_entries() >= 2 {
            return Err(AccountTreeError::DuplicateIdPrefix { duplicate_prefix: id_prefix });
        }

        Ok(Self { id_suffix, proof })
    }

    /// Constructs a new [`AccountWitness`] from the provided proof without validating that it has
    /// zero or one entries.
    ///
    /// # Warning
    ///
    /// This does not validate any of the guarantees of this type.
    pub(super) fn new_unchecked(account_id: AccountId, proof: SmtProof) -> Self {
        Self { id_suffix: account_id.suffix(), proof }
    }

    /// Returns the inner proof for the account tree of this witness.
    pub fn as_proof(&self) -> &SmtProof {
        &self.proof
    }

    /// Returns the underlying [`AccountIdPrefix`] that this witness prove inclusion for.
    pub fn id_prefix(&self) -> AccountIdPrefix {
        // SAFETY: By construction the account witness guarantees it tracks a valid account ID
        // prefix so we can safely convert the leaf idx to that prefix.
        AccountTree::key_to_account_id_prefix(self.proof.leaf().index())
    }

    /// Returns the state commitment of the account witness.
    pub fn state_commitment(&self) -> Digest {
        // SAFETY: By construction, this type contains only proofs with zero or one entry, so
        // the leaf is either of variant Empty or Single.
        match self.proof.leaf() {
            SmtLeaf::Empty(_) => Digest::default(),
            SmtLeaf::Single((key, commitment)) => {
                // See the docs of the `id_suffix` field for details on why this distinction is
                // necessary.
                if key[AccountTree::KEY_SUFFIX_IDX] == self.id_suffix {
                    Digest::from(commitment)
                } else {
                    Digest::from(EMPTY_WORD)
                }
            },
            SmtLeaf::Multiple(_) => {
                unreachable!("account witness is guaranteed to contain zero or one entries")
            },
        }
    }

    /// Consumes self and returns the inner proof.
    pub fn into_proof(self) -> SmtProof {
        self.proof
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountWitness {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.id_suffix.write_into(target);
        self.proof.write_into(target);
    }
}

impl Deserializable for AccountWitness {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let id_suffix = Felt::read_from(source)?;
        let proof = SmtProof::read_from(source)?;

        // Note: This potentially swallows the source error.
        Self::new_inner(id_suffix, proof)
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}
