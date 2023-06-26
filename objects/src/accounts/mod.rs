use super::{
    assets::{Asset, FungibleAsset, NonFungibleAsset},
    AccountError, AdviceInputsBuilder, Assembler, AssemblyContext, AssemblyContextType, Digest,
    Felt, Hasher, LibraryPath, Module, ModuleAst, StarkField, TieredSmt, ToAdviceInputs, ToString,
    Vec, Word, ZERO,
};

mod account_id;
pub use account_id::{AccountId, AccountType};

mod code;
pub use code::AccountCode;

mod storage;
pub use storage::AccountStorage;
pub use storage::StorageItem;

mod vault;
pub use vault::AccountVault;

#[cfg(test)]
mod tests;

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
#[derive(Debug, Clone)]
pub struct Account {
    id: AccountId,
    vault: AccountVault,
    storage: AccountStorage,
    code: AccountCode,
    nonce: Felt,
}

impl Account {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates and returns a new account initialized with the specified ID, storage items, and
    /// code.
    ///
    /// The vault of the account is initially empty and nonce is set to ZERO.
    ///
    /// # Errors
    /// Returns an error if compilation of the source code fails.
    pub fn new(
        id: AccountId,
        vault: AccountVault,
        storage: AccountStorage,
        code: AccountCode,
        nonce: Felt,
    ) -> Result<Self, AccountError> {
        Ok(Self {
            id,
            vault,
            storage,
            code,
            nonce,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns hash of this account.
    ///
    /// Hash of an account is computed as hash(id, nonce, vault_root, storage_root, code_root).
    /// Computing the account hash requires 2 permutations of the hash function.
    pub fn hash(&self) -> Digest {
        let mut elements = [ZERO; 16];
        elements[0] = *self.id;
        elements[3] = self.nonce;
        elements[4..8].copy_from_slice(self.vault.commitment().as_elements());
        elements[8..12].copy_from_slice(&*self.storage.root());
        elements[12..].copy_from_slice(self.code.root().as_elements());
        Hasher::hash_elements(&elements)
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
        target.push_onto_stack(&[*self.id, ZERO, ZERO, self.nonce]);
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
