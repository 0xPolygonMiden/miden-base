#[cfg(feature = "std")]
use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
    vec::Vec,
};

use miden_crypto::utils::SliceReader;

use super::{
    super::utils::serde::{
        ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
    },
    Account, AuthSecretKey, Word,
};

// ACCOUNT DATA
// ================================================================================================

/// Account data contains a complete description of an account, including the [Account] struct as
/// well as account seed and account authentication info.
///
/// The intent of this struct is to provide an easy way to serialize and deserialize all
/// account-related data as a single unit (e.g., to/from files).
#[derive(Debug, Clone)]
pub struct AccountData {
    pub account: Account,
    pub account_seed: Option<Word>,
    pub auth_secret_key: AuthSecretKey,
}

impl AccountData {
    pub fn new(account: Account, account_seed: Option<Word>, auth: AuthSecretKey) -> Self {
        Self {
            account,
            account_seed,
            auth_secret_key: auth,
        }
    }

    #[cfg(feature = "std")]
    /// Serialises and writes binary AccountData to specified file
    pub fn write(&self, filepath: impl AsRef<Path>) -> io::Result<()> {
        fs::write(filepath, self.to_bytes())
    }

    #[cfg(feature = "std")]
    /// Reads from file and tries to deserialise an AccountData
    pub fn read(filepath: impl AsRef<Path>) -> io::Result<Self> {
        let mut file = File::open(filepath)?;
        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer)?;
        let mut reader = SliceReader::new(&buffer);

        Ok(AccountData::read_from(&mut reader).map_err(|_| io::ErrorKind::InvalidData)?)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountData {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let AccountData {
            account,
            account_seed,
            auth_secret_key: auth,
        } = self;

        account.write_into(target);
        account_seed.write_into(target);
        auth.write_into(target);
    }
}

impl Deserializable for AccountData {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let account = Account::read_from(source)?;
        let account_seed = <Option<Word>>::read_from(source)?;
        let auth_secret_key = AuthSecretKey::read_from(source)?;

        Ok(Self::new(account, account_seed, auth_secret_key))
    }

    fn read_from_bytes(bytes: &[u8]) -> Result<Self, DeserializationError> {
        Self::read_from(&mut SliceReader::new(bytes))
    }
}

// AUTH DATA
// ================================================================================================

/// AuthData is a representation of the AuthScheme struct meant to be used
/// for Account serialisation and deserialisation for transport of Account data
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AuthData {
    RpoFalcon512Seed([u8; 32]),
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AuthData {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            AuthData::RpoFalcon512Seed(seed) => {
                0_u8.write_into(target);
                seed.write_into(target);
            },
        }
    }
}

impl Deserializable for AuthData {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let scheme = u8::read_from(source)?;
        match scheme {
            0 => {
                let seed = <[u8; 32]>::read_from(source)?;
                Ok(AuthData::RpoFalcon512Seed(seed))
            },
            value => Err(DeserializationError::InvalidValue(format!("Invalid value: {}", value))),
        }
    }

    fn read_from_bytes(bytes: &[u8]) -> Result<Self, DeserializationError> {
        Self::read_from(&mut SliceReader::new(bytes))
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use miden_crypto::{
        dsa::rpo_falcon512::SecretKey,
        utils::{Deserializable, Serializable},
    };
    use storage::AccountStorage;
    #[cfg(feature = "std")]
    use tempfile::tempdir;

    use super::{AccountData};
    use crate::{
        accounts::{
            account_id::testing::ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
            code::testing::make_account_code, storage, Account, AccountId, AuthSecretKey, Felt,
            Word,
        },
        assets::AssetVault,
    };

    fn build_account_data() -> AccountData {
        let id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();
        let code = make_account_code();

        // create account and auth
        let vault = AssetVault::new(&[]).unwrap();
        let storage = AccountStorage::new(vec![], vec![]).unwrap();
        let nonce = Felt::new(0);
        let account = Account::new(id, vault, storage, code, nonce);
        let account_seed = Some(Word::default());
        let auth_secret_key = AuthSecretKey::RpoFalcon512(SecretKey::new());

        AccountData::new(account, account_seed, auth_secret_key)
    }

    #[test]
    fn test_serde() {
        let account_data = build_account_data();
        let serialized = account_data.to_bytes();
        let deserialized = AccountData::read_from_bytes(&serialized).unwrap();
        assert_eq!(deserialized.account, account_data.account);
        assert_eq!(deserialized.account_seed, account_data.account_seed);
        assert_eq!(
            deserialized.auth_secret_key.to_bytes(),
            account_data.auth_secret_key.to_bytes()
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_serde_file() {
        let dir = tempdir().unwrap();
        let filepath = dir.path().join("account_data.mac");

        let account_data = build_account_data();
        account_data.write(filepath.as_path()).unwrap();
        let deserialized = AccountData::read(filepath.as_path()).unwrap();

        assert_eq!(deserialized.account, account_data.account);
        assert_eq!(deserialized.account_seed, account_data.account_seed);
        assert_eq!(
            deserialized.auth_secret_key.to_bytes(),
            account_data.auth_secret_key.to_bytes()
        );
    }
}
