use crate::{
    accounts::{Account, AccountDelta, AccountId},
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccountUpdateDetails {
    /// Account is private (no on-chain state change).
    Private,

    /// The whole state is needed for new accounts.
    New(Account),

    /// For existing accounts, only the delta is needed.
    Delta(AccountDelta),
}

impl AccountUpdateDetails {
    /// Returns `true` if the account update details are for private account.
    pub fn is_private(&self) -> bool {
        matches!(self, Self::Private)
    }
}

/// Account update data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountUpdate {
    /// Account ID.
    account_id: AccountId,

    /// The hash of the account after the transaction was executed.
    new_state_hash: Digest,

    /// Optional account state changes used for on-chain accounts. This data is used to update an
    /// on-chain account's state after a local transaction execution. For private accounts, this
    /// is [AccountUpdateDetails::Private].
    details: AccountUpdateDetails,
}

impl AccountUpdate {
    /// Creates a new [AccountUpdate].
    pub const fn new(
        account_id: AccountId,
        new_state_hash: Digest,
        details: AccountUpdateDetails,
    ) -> Self {
        Self { account_id, new_state_hash, details }
    }

    /// Returns the account ID.
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns the final account state hash.
    pub fn new_state_hash(&self) -> Digest {
        self.new_state_hash
    }

    /// Returns the account update details.
    pub fn details(&self) -> &AccountUpdateDetails {
        &self.details
    }

    /// Returns `true` if the account update details are for private account.
    pub fn is_private(&self) -> bool {
        self.details.is_private()
    }
}

/// Describes the changes made to the account state resulting from a transaction execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountUpdateInfo {
    /// The hash of the account before the transaction was executed.
    ///
    /// Set to `Digest::default()` for new accounts.
    init_state_hash: Digest,

    /// Account update information.
    update: AccountUpdate,
}

impl AccountUpdateInfo {
    /// Returns a new [AccountUpdateInfo] instantiated from the specified components.
    pub const fn new(
        account_id: AccountId,
        init_state_hash: Digest,
        new_state_hash: Digest,
        details: AccountUpdateDetails,
    ) -> Self {
        Self {
            init_state_hash,
            update: AccountUpdate::new(account_id, new_state_hash, details),
        }
    }

    /// Returns the ID of the updated account.
    pub fn account_id(&self) -> AccountId {
        self.update.account_id()
    }

    /// Returns the initial account state hash.
    pub fn init_state_hash(&self) -> Digest {
        self.init_state_hash
    }

    /// Returns the hash of the account after the transaction was executed.
    pub fn final_state_hash(&self) -> Digest {
        self.update.new_state_hash()
    }

    /// Returns the account update details.
    pub fn details(&self) -> &AccountUpdateDetails {
        self.update.details()
    }

    /// Returns `true` if the account update details are for private account.
    pub fn is_private(&self) -> bool {
        self.update.is_private()
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountUpdateDetails {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            AccountUpdateDetails::Private => {
                0_u8.write_into(target);
            },
            AccountUpdateDetails::New(account) => {
                1_u8.write_into(target);
                account.write_into(target);
            },
            AccountUpdateDetails::Delta(delta) => {
                2_u8.write_into(target);
                delta.write_into(target);
            },
        }
    }
}

impl Deserializable for AccountUpdateDetails {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        match u8::read_from(source)? {
            0 => Ok(Self::Private),
            1 => Ok(Self::New(Account::read_from(source)?)),
            2 => Ok(Self::Delta(AccountDelta::read_from(source)?)),
            v => Err(DeserializationError::InvalidValue(format!(
                "Unknown variant {v} for AccountDetails"
            ))),
        }
    }
}

impl Serializable for AccountUpdate {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.account_id.write_into(target);
        self.new_state_hash.write_into(target);
        self.details.write_into(target);
    }
}

impl Deserializable for AccountUpdate {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        Ok(Self {
            account_id: AccountId::read_from(source)?,
            new_state_hash: Digest::read_from(source)?,
            details: AccountUpdateDetails::read_from(source)?,
        })
    }
}

impl Serializable for AccountUpdateInfo {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.init_state_hash.write_into(target);
        self.update.write_into(target);
    }
}

impl Deserializable for AccountUpdateInfo {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        Ok(Self {
            init_state_hash: Digest::read_from(source)?,
            update: AccountUpdate::read_from(source)?,
        })
    }
}
