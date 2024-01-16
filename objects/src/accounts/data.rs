use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
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

/// AccountData is a representation of the Account struct meant to be used
/// for Account serialisation and deserialisation for transport of Account data
#[derive(Debug, PartialEq, Eq)]
pub struct AccountData {
    pub account: Account,
    pub account_seed: Option<Word>,
    pub auth: AuthData,
}

impl AccountData {
    pub fn new(account: Account, account_seed: Option<Word>, auth: AuthData) -> Self {
        Self { account, account_seed, auth }
    }

    /// Serialises and writes binary AccountData to specified file
    pub fn write(&self, filepath: &Path) -> Result<(), io::Error> {
        fs::write(filepath, self.to_bytes())?;

        Ok(())
    }

    /// Reads from file and tries to deserialise an AccountData
    pub fn read(filepath: &Path) -> Result<Self, io::Error> {
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

        let auth_scheme = match auth {
            AuthData::RpoFalcon512Seed(_) => "RpoFalcon512",
        };

        let auth_seed = match auth {
            AuthData::RpoFalcon512Seed(seed) => seed,
        };

        account.write_into(target);
        match account_seed {
            None => target.write_u8(0),
            Some(seed) => {
                target.write_u8(1);
                seed.write_into(target)
            },
        };
        let auth_scheme_len = auth_scheme.as_bytes().len();
        target.write_u8(auth_scheme_len as u8);
        auth_scheme.as_bytes().write_into(target);
        auth_seed.write_into(target);
    }
}

impl Deserializable for AccountData {
    fn read_from<R: ByteReader>(
        source: &mut R,
    ) -> std::prelude::v1::Result<Self, DeserializationError> {
        let account = Account::read_from(source)?;

        let account_seed = {
            let option_flag = source.read_u8()?;
            match option_flag {
                0 => None,
                1 => Some(Word::read_from(source)?),
                _ => {
                    return Err(DeserializationError::InvalidValue(
                        "Invalid option flag".to_string(),
                    ))
                },
            }
        };

        let auth_scheme_len = source.read_u8()?;
        let auth_scheme_value = source.read_vec(auth_scheme_len as usize)?;
        let auth_scheme = match std::str::from_utf8(&auth_scheme_value) {
            Ok(str) => str,
            Err(e) => {
                return Err(DeserializationError::InvalidValue(format!(
                    "Invalid auth_scheme value {}",
                    e
                )))
            },
        };

        let auth_seed = <[u8; 40]>::read_from(source)?;

        let auth = match auth_scheme {
            "RpoFalcon512" => AuthData::RpoFalcon512Seed(auth_seed),
            _ => return Err(DeserializationError::InvalidValue("Invalid auth_scheme".to_string())),
        };

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
#[derive(Debug, PartialEq, Eq)]
pub enum AuthData {
    RpoFalcon512Seed([u8; 40]),
}

#[cfg(test)]
mod tests {
    use assembly::{ast::ModuleAst, Assembler};
    use miden_crypto::utils::{Deserializable, Serializable};
    use storage::AccountStorage;
    use tempfile::tempdir;

    use crate::{
        accounts::{
            storage, Account, AccountCode, AccountId, Felt, Word,
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        },
        assets::AssetVault,
    };

    use super::{AccountData, AuthData};

    fn create_account_data() -> AccountData {
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
        let module = ModuleAst::parse(source).unwrap();
        let assembler = Assembler::default();
        let code = AccountCode::new(module, &assembler).unwrap();

        // create account and auth
        let vault = AssetVault::new(&[]).unwrap();
        let storage = AccountStorage::new(vec![]).unwrap();
        let nonce = Felt::new(0);
        let account = Account::new(id, vault, storage, code, nonce);
        let account_seed = Some(Word::default());
        let auth_seed = [0u8; 40];
        let auth = AuthData::RpoFalcon512Seed(auth_seed);

        // create AccountData
        AccountData::new(account, account_seed, auth)
    }

    #[test]
    fn account_data_correctly_serialises_and_deserialises() {
        // create AccountData
        let account_data = create_account_data();

        // serialize and deserialize the code; make sure deserialized version matches the original
        let bytes = account_data.to_bytes();
        let account_data_2 = AccountData::read_from_bytes(&bytes).unwrap();
        assert_eq!(account_data, account_data_2);
    }

    #[test]
    fn account_data_is_correctly_writen_and_read_to_and_from_file() {
        // setup temp directory
        let dir = tempdir().unwrap();
        let filepath = dir.path().join("account_data.mac");

        // create AccountData
        let account_data = create_account_data();

        // write AccountData to file
        account_data.write(filepath.as_path()).unwrap();

        // read AccountData from file
        let account_data_2 = AccountData::read(filepath.as_path()).unwrap();

        // make sure deserialized version matches the original
        assert_eq!(account_data, account_data_2)
    }
}
