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
    /// The hash of the account before the transaction was executed.
    ///
    /// Set to `Digest::default()` for new accounts.
    init_hash: Digest,

    /// The hash of the account after the transaction was executed.
    final_hash: Digest,

    /// Optional account state changes used for on-chain accounts. This data is used to update an
    /// on-chain account's state after a local transaction execution. For private accounts, this
    /// is [AccountUpdateDetails::Private].
    details: AccountUpdateDetails,
}

impl AccountUpdate {
    /// Creates a new [AccountUpdate].
    pub const fn new(init_hash: Digest, final_hash: Digest, details: AccountUpdateDetails) -> Self {
        Self { init_hash, final_hash, details }
    }

    /// Returns the initial account state hash.
    pub fn init_hash(&self) -> Digest {
        self.init_hash
    }

    /// Returns the final account state hash.
    pub fn final_hash(&self) -> Digest {
        self.final_hash
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

/// Account update data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountUpdateData {
    /// Account ID.
    account_id: AccountId,

    /// The hash of the account after the transaction was executed.
    final_state_hash: Digest,

    /// Account update details.
    details: AccountUpdateDetails,
}

impl AccountUpdateData {
    /// Creates a new [AccountUpdateData].
    pub const fn new(
        account_id: AccountId,
        final_state_hash: Digest,
        details: AccountUpdateDetails,
    ) -> Self {
        Self { account_id, final_state_hash, details }
    }

    /// Returns the account ID.
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns the hash of the account after the transaction was executed.
    pub fn final_state_hash(&self) -> Digest {
        self.final_state_hash
    }

    /// Returns the account update details.
    pub fn details(&self) -> &AccountUpdateDetails {
        &self.details
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
        self.init_hash.write_into(target);
        self.final_hash.write_into(target);
        self.details.write_into(target);
    }
}

impl Deserializable for AccountUpdate {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        Ok(Self {
            init_hash: Digest::read_from(source)?,
            final_hash: Digest::read_from(source)?,
            details: AccountUpdateDetails::read_from(source)?,
        })
    }
}

impl Serializable for AccountUpdateData {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.account_id.write_into(target);
        self.final_state_hash.write_into(target);
        self.details.write_into(target);
    }
}

impl Deserializable for AccountUpdateData {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        Ok(Self {
            account_id: AccountId::read_from(source)?,
            final_state_hash: Digest::read_from(source)?,
            details: AccountUpdateDetails::read_from(source)?,
        })
    }
}
