use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};

use miden_lib::{utils::sync::RwLock, AuthenticationError, TransactionAuthenticator};
use miden_objects::account::{AccountDelta, AuthSecretKey};
use rand::Rng;
use vm_processor::{Digest, Felt, Word};

use super::signatures::get_falcon_signature;

// BASIC AUTHENTICATOR
// ================================================================================================

#[derive(Clone, Debug)]
/// Represents a signer for [AuthSecretKey] keys.
pub struct BasicAuthenticator<R> {
    /// pub_key |-> secret_key mapping
    keys: BTreeMap<Digest, AuthSecretKey>,
    rng: Arc<RwLock<R>>,
}

impl<R: Rng> BasicAuthenticator<R> {
    #[cfg(feature = "std")]
    pub fn new(keys: &[(Word, AuthSecretKey)]) -> BasicAuthenticator<rand::rngs::StdRng> {
        use rand::{rngs::StdRng, SeedableRng};

        let rng = StdRng::from_entropy();
        BasicAuthenticator::<StdRng>::new_with_rng(keys, rng)
    }

    pub fn new_with_rng(keys: &[(Word, AuthSecretKey)], rng: R) -> Self {
        let mut key_map = BTreeMap::new();
        for (word, secret_key) in keys {
            key_map.insert(word.into(), secret_key.clone());
        }

        BasicAuthenticator {
            keys: key_map,
            rng: Arc::new(RwLock::new(rng)),
        }
    }
}

impl<R: Rng> TransactionAuthenticator for BasicAuthenticator<R> {
    /// Gets a signature over a message, given a public key.
    /// The key should be included in the `keys` map and should be a variant of [AuthSecretKey].
    ///
    /// Supported signature schemes:
    /// - RpoFalcon512
    ///
    /// # Errors
    /// If the public key is not contained in the `keys` map,
    /// [`AuthenticationError::UnknownPublicKey`] is returned.
    fn get_signature(
        &self,
        pub_key: Word,
        message: Word,
        account_delta: &AccountDelta,
    ) -> Result<Vec<Felt>, AuthenticationError> {
        let _ = account_delta;
        let mut rng = self.rng.write();

        match self.keys.get(&pub_key.into()) {
            Some(key) => match key {
                AuthSecretKey::RpoFalcon512(falcon_key) => {
                    get_falcon_signature(falcon_key, message, &mut *rng)
                },
            },
            None => Err(AuthenticationError::UnknownPublicKey(format!(
                "public key {} is not contained in the authenticator's keys",
                Digest::from(pub_key)
            ))),
        }
    }
}

#[cfg(test)]
mod test {
    use miden_lib::utils::{Deserializable, Serializable};
    use miden_objects::{account::AuthSecretKey, crypto::dsa::rpo_falcon512::SecretKey};

    #[test]
    fn serialize_auth_key() {
        let secret_key = SecretKey::new();
        let auth_key = AuthSecretKey::RpoFalcon512(secret_key.clone());
        let serialized = auth_key.to_bytes();
        let deserialized = AuthSecretKey::read_from_bytes(&serialized).unwrap();

        match deserialized {
            AuthSecretKey::RpoFalcon512(key) => assert_eq!(secret_key.to_bytes(), key.to_bytes()),
        }
    }
}
