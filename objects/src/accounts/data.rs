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
    Account, Word,
};

// ACCOUNT DATA
// ================================================================================================

/// Account data contains a complete description of an account, including the [Account] struct as
/// well as account seed and account authentication info.
///
/// The intent of this struct is to provide an easy way to serialize and deserialize all
/// account-related data as a single unit (e.g., to/from files).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AccountData {
    pub account: Account,
    pub account_seed: Option<Word>,
    pub auth: AuthData,
}

impl AccountData {
    pub fn new(account: Account, account_seed: Option<Word>, auth: AuthData) -> Self {
        Self { account, account_seed, auth }
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
        let AccountData { account, account_seed, auth } = self;

        account.write_into(target);
        account_seed.write_into(target);
        auth.write_into(target);
    }
}

impl Deserializable for AccountData {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let account = Account::read_from(source)?;
        let account_seed = <Option<Word>>::read_from(source)?;
        let auth = AuthData::read_from(source)?;

        Ok(Self::new(account, account_seed, auth))
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
    use assembly::{ast::ModuleAst, Assembler};
    use miden_crypto::utils::{Deserializable, Serializable};
    use storage::AccountStorage;
    #[cfg(feature = "std")]
    use tempfile::tempdir;

    use super::{AccountData, AuthData};
    use crate::{
        accounts::{
            storage, Account, AccountCode, AccountId, Felt, Word,
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        },
        assets::AssetVault,
    };

    fn build_account_data() -> AccountData {
        // create account id
        let id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();

        // build account code
        let source = "
            export.foo
                push.1 push.2 mul
            end

            export.bar
                push.1 push.2 add
            end
        ";
        let mut module = ModuleAst::parse(source).unwrap();
        // clears are needed since imports and source locations are not serialized for account code
        module.clear_locations();
        module.clear_imports();
        let assembler = Assembler::default();
        let code = AccountCode::new(module, &assembler).unwrap();

        // create account and auth
        let vault = AssetVault::new(&[]).unwrap();
        let storage = AccountStorage::new(vec![], vec![]).unwrap();
        let nonce = Felt::new(0);
        let account = Account::new(id, vault, storage, code, nonce);
        let account_seed = Some(Word::default());
        let auth_seed = [0u8; 32];
        let auth = AuthData::RpoFalcon512Seed(auth_seed);

        // create AccountData
        AccountData::new(account, account_seed, auth)
    }

    #[test]
    fn account_data_correctly_serialises_and_deserialises() {
        // create AccountData
        let account_data = build_account_data();

        // serialize and deserialize the code; make sure deserialized version matches the original
        let bytes = account_data.to_bytes();
        let account_data_2 = AccountData::read_from_bytes(&bytes).unwrap();
        assert_eq!(account_data, account_data_2);
    }

    #[cfg(feature = "std")]
    #[test]
    fn account_data_is_correctly_writen_and_read_to_and_from_file() {
        // setup temp directory
        let dir = tempdir().unwrap();
        let filepath = dir.path().join("account_data.mac");

        // create AccountData
        let account_data = build_account_data();

        // write AccountData to file
        account_data.write(filepath.as_path()).unwrap();

        // read AccountData from file
        let account_data_2 = AccountData::read(filepath.as_path()).unwrap();

        // make sure deserialized version matches the original
        assert_eq!(account_data, account_data_2)
    }
}
