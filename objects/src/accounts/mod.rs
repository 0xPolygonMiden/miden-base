use super::{
    assembly::{Assembler, AssemblyContext, ModuleAst},
    assets::{Asset, FungibleAsset, NonFungibleAsset},
    crypto::{
        merkle::{StoreNode, TieredSmt},
        utils::collections::TryApplyDiff,
    },
    utils::{collections::Vec, string::ToString},
    AccountError, AdviceInputsBuilder, Digest, Felt, FieldElement, Hasher, StarkField,
    ToAdviceInputs, Word, ZERO,
};

mod account_id;
pub use account_id::{compute_digest, digest_pow, validate_account_seed, AccountId, AccountType};

mod code;
pub use code::AccountCode;

pub mod delta;
pub use delta::{AccountDelta, AccountStorageDelta, AccountVaultDelta};

mod seed;
pub use seed::get_account_seed;

mod storage;
pub use storage::{AccountStorage, StorageItem};

mod stub;
pub use stub::AccountStub;

mod vault;
pub use vault::AccountVault;

// TESTING CONSTANTS
// ================================================================================================

#[cfg(any(feature = "testing", test))]
pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN: u64 = 0b0110011011u64 << 54;

#[cfg(any(feature = "testing", test))]
pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN: u64 = 0b0001101110 << 54;

#[cfg(any(feature = "testing", test))]
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1010011100 << 54;

#[cfg(any(feature = "testing", test))]
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN: u64 = 0b1101100110 << 54;

#[cfg(any(feature = "testing", test))]
pub const ACCOUNT_ID_INSUFFICIENT_ONES: u64 = 0b1100000110 << 54;

// ACCOUNT
// ================================================================================================

/// An account which can store assets and define rules for manipulating them.
///
/// An account consists of the following components:
/// - Account ID, which uniquely identifies the account and also defines basic properties of the
///   account.
/// - Account vault, which stores assets owned by the account.
/// - Account storage, which is a key-value map (both keys and values are words) used to store
///   arbitrary user-defined data.
/// - Account code, which is a set of Miden VM programs defining the public interface of the
///   account.
/// - Account nonce, a value which is incremented whenever account state is updated.
///
/// Out of the the above components account ID is always immutable (once defined it can never be
/// changed). Other components may be mutated throughout the lifetime of the account. However,
/// account state can be changed only by invoking one of account interface methods.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Account {
    id: AccountId,
    vault: AccountVault,
    storage: AccountStorage,
    #[cfg_attr(feature = "serde", serde(with = "serialization"))]
    code: AccountCode,
    nonce: Felt,
}

impl Account {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates and returns a new account initialized with the specified ID, vault, storage, code,
    /// and nonce.
    pub fn new(
        id: AccountId,
        vault: AccountVault,
        storage: AccountStorage,
        code: AccountCode,
        nonce: Felt,
    ) -> Self {
        Self {
            id,
            vault,
            storage,
            code,
            nonce,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns hash of this account.
    ///
    /// Hash of an account is computed as hash(id, nonce, vault_root, storage_root, code_root).
    /// Computing the account hash requires 2 permutations of the hash function.
    pub fn hash(&self) -> Digest {
        hash_account(
            self.id,
            self.nonce,
            self.vault.commitment(),
            self.storage.root(),
            self.code.root(),
        )
    }

    /// Returns unique identifier of this account.
    pub fn id(&self) -> AccountId {
        self.id
    }

    /// Returns the account type
    pub fn account_type(&self) -> AccountType {
        self.id.account_type()
    }

    /// Returns a reference to the vault of this account.
    pub fn vault(&self) -> &AccountVault {
        &self.vault
    }

    #[cfg(test)]
    /// Returns a mutable reference to the vault of this account.
    pub fn vault_mut(&mut self) -> &mut AccountVault {
        &mut self.vault
    }

    /// Returns a reference to the storage of this account.
    pub fn storage(&self) -> &AccountStorage {
        &self.storage
    }

    /// Returns a reference to the code of this account.
    pub fn code(&self) -> &AccountCode {
        &self.code
    }

    /// Returns nonce for this account.
    pub fn nonce(&self) -> Felt {
        self.nonce
    }

    /// Returns nonce for this account.
    pub fn set_nonce(&mut self, nonce: Felt) {
        self.nonce = nonce;
    }

    /// Returns true if this account can issue assets.
    pub fn is_faucet(&self) -> bool {
        self.id.is_faucet()
    }

    /// Returns true if this is a regular account.
    pub fn is_regular_account(&self) -> bool {
        self.id.is_regular_account()
    }

    /// Returns true if this account is on-chain.
    pub fn is_on_chain(&self) -> bool {
        self.id.is_on_chain()
    }

    /// Returns true if the account is new (i.e. it has not been initialized yet).
    pub fn is_new(&self) -> bool {
        self.nonce == ZERO
    }
}

impl ToAdviceInputs for Account {
    /// Pushes an array of field elements representing this account onto the advice stack.
    /// The array (elements) is in the following format:
    ///    elements[0]       = account id
    ///    elements[2..3]    = padding ([Felt::ZERO; 2])
    ///    elements[3]       = account nonce
    ///    elements[4..8]    = account vault root
    ///    elements[8..12]   = storage root
    ///    elements[12..16]  = code root
    fn to_advice_inputs<T: AdviceInputsBuilder>(&self, target: &mut T) {
        // push core items onto the stack
        target.push_onto_stack(&[self.id.into(), ZERO, ZERO, self.nonce]);
        target.push_onto_stack(self.vault.commitment().as_elements());
        target.push_onto_stack(&*self.storage.root());
        target.push_onto_stack(self.code.root().as_elements());

        // extend the merkle store with the storage items
        target.add_merkle_nodes(self.storage.slots().inner_nodes());
        target.add_merkle_nodes(self.storage.store().inner_nodes());

        // extend the merkle store with account code tree
        target.add_merkle_nodes(self.code.procedure_tree().inner_nodes());

        // extend advice map with (account proc root -> method tree index)
        for (idx, leaf) in self.code.procedure_tree().leaves() {
            target.insert_into_map(*leaf, vec![idx.into()]);
        }

        // extend the advice provider with [AccountVault] inputs
        self.vault.to_advice_inputs(target);
    }
}

// SERIALIZATION
// ================================================================================================

#[cfg(feature = "serde")]
mod serialization {
    use super::AccountCode;
    use crate::utils::serde::{Deserializable, Serializable};

    pub fn serialize<S>(code: &AccountCode, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = code.to_bytes();
        serializer.serialize_bytes(&bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<AccountCode, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = <Vec<u8> as serde::Deserialize>::deserialize(deserializer)?;

        AccountCode::read_from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

// HELPERS
// ================================================================================================

/// Returns hash of an account with the specified ID, nonce, vault root, storage root, and code root.
///
/// Hash of an account is computed as hash(id, nonce, vault_root, storage_root, code_root).
/// Computing the account hash requires 2 permutations of the hash function.
pub fn hash_account(
    id: AccountId,
    nonce: Felt,
    vault_root: Digest,
    storage_root: Digest,
    code_root: Digest,
) -> Digest {
    let mut elements = [ZERO; 16];
    elements[0] = id.into();
    elements[3] = nonce;
    elements[4..8].copy_from_slice(&*vault_root);
    elements[8..12].copy_from_slice(&*storage_root);
    elements[12..].copy_from_slice(&*code_root);
    Hasher::hash_elements(&elements)
}

// DIFF IMPLEMENTATION
// ================================================================================================

impl TryApplyDiff<Digest, StoreNode> for Account {
    type DiffType = AccountDelta;
    type Error = AccountError;

    fn try_apply(&mut self, diff: Self::DiffType) -> Result<(), Self::Error> {
        let AccountDelta {
            code: _code,
            nonce,
            storage,
            vault,
        } = diff;

        self.storage.try_apply(storage)?;
        self.vault.try_apply(vault)?;

        if let Some(nonce) = nonce {
            if nonce.as_int() <= self.nonce.as_int() {
                return Err(AccountError::NonceMustBeMonotonicallyIncreasing(
                    nonce.as_int(),
                    self.nonce.as_int(),
                ));
            }
            self.nonce = nonce;
        }

        Ok(())
    }
}
