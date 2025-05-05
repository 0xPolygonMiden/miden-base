use alloc::vec::Vec;

use vm_core::{
    Felt, ZERO,
    utils::{Deserializable, Serializable},
};

use super::{Account, AccountCode, AccountId, PartialStorage};
use crate::asset::PartialVault;

/// A partial representation of an account.
///
/// A partial account is used as inputs to the transaction kernel and contains only the essential
/// data needed for verification and transaction processing without requiring the full account
/// state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialAccount {
    /// The ID for the partial account
    id: AccountId,
    /// The current transaction nonce of the account
    nonce: Felt,
    /// Account code
    account_code: AccountCode,
    /// Partial representation of the account's storage, containing the storage commitment and
    /// proofs for specific storage slots that need to be accessed
    partial_storage: PartialStorage,
    /// Partial representation of the account's vault, containing the vault root and necessary
    /// proof information for asset verification
    partial_vault: PartialVault,
}

impl PartialAccount {
    /// Creates a new instance of a partial account with the specified components.
    pub fn new(
        id: AccountId,
        nonce: Felt,
        account_code: AccountCode,
        partial_storage: PartialStorage,
        partial_vault: PartialVault,
    ) -> Self {
        Self {
            id,
            nonce,
            account_code,
            partial_storage,
            partial_vault,
        }
    }

    /// Returns the account's unique identifier.
    pub fn id(&self) -> AccountId {
        self.id
    }

    /// Returns the account's current nonce value.
    pub fn nonce(&self) -> Felt {
        self.nonce
    }

    /// Returns a reference to the account code.
    pub fn code(&self) -> &AccountCode {
        &self.account_code
    }

    /// Returns a reference to the partial storage representation of the account.
    pub fn partial_storage(&self) -> &PartialStorage {
        &self.partial_storage
    }

    /// Returns a reference to the partial vault representation of the account.
    pub fn partial_vault(&self) -> &PartialVault {
        &self.partial_vault
    }

    /// Converts the account header into a vector of field elements.
    ///
    /// This is done by first converting the account header data into an array of Words as follows:
    /// ```text
    /// [
    ///     [account_id_suffix, account_id_prefix, 0, account_nonce]
    ///     [VAULT_ROOT]
    ///     [STORAGE_COMMITMENT]
    ///     [CODE_COMMITMENT]
    /// ]
    /// ```
    /// And then concatenating the resulting elements into a single vector.
    pub fn as_elements(&self) -> Vec<Felt> {
        [
            &[self.id.suffix(), self.id.prefix().as_felt(), ZERO, self.nonce],
            self.partial_vault.root().as_elements(),
            self.partial_storage.commitment().as_elements(),
            self.account_code.commitment().as_elements(),
        ]
        .concat()
    }
}

impl From<Account> for PartialAccount {
    fn from(value: Account) -> Self {
        let (id, vault, storage, code, nonce) = value.into_parts();

        PartialAccount::new(id, nonce, code, storage.into(), vault.into())
    }
}

impl Serializable for PartialAccount {
    fn write_into<W: vm_core::utils::ByteWriter>(&self, target: &mut W) {
        target.write(self.id);
        target.write(self.nonce);
        target.write(&self.account_code);
        target.write(&self.partial_storage);
        target.write(&self.partial_vault);
    }
}

impl Deserializable for PartialAccount {
    fn read_from<R: vm_core::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, vm_processor::DeserializationError> {
        let account_id = source.read()?;
        let nonce = source.read()?;
        let code_header = source.read()?;
        let partial_storage = source.read()?;
        let partial_vault = source.read()?;

        Ok(PartialAccount {
            id: account_id,
            nonce,
            account_code: code_header,
            partial_storage,
            partial_vault,
        })
    }
}
