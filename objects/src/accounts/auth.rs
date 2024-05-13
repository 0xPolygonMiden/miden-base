// AUTH SECRET KEY
// ================================================================================================

use alloc::string::ToString;

use miden_crypto::dsa::rpo_falcon512::{self, SecretKey};
use vm_core::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

/// Types of secret keys used for signing messages
#[derive(Clone, Debug)]
#[repr(u8)]
pub enum AuthSecretKey {
    RpoFalcon512(rpo_falcon512::SecretKey) = 0,
}

impl AuthSecretKey {
    /// Identifier for the type of authentication key
    pub fn key_id(&self) -> u8 {
        match self {
            AuthSecretKey::RpoFalcon512(_) => 0u8,
        }
    }
}

impl Serializable for AuthSecretKey {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_u8(self.key_id());
        match self {
            AuthSecretKey::RpoFalcon512(secret_key) => {
                secret_key.write_into(target);
            },
        }
    }
}

impl Deserializable for AuthSecretKey {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let auth_key_id: u8 = source.read_u8()?;
        match auth_key_id {
            // RpoFalcon512
            0u8 => {
                let secret_key = SecretKey::read_from(source)?;
                Ok(AuthSecretKey::RpoFalcon512(secret_key))
            },
            val => Err(DeserializationError::InvalidValue(val.to_string())),
        }
    }
}
