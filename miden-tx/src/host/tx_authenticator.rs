use alloc::{collections::BTreeMap, string::ToString, vec::Vec};
use core::cell::RefCell;

use miden_objects::{
    accounts::AccountDelta,
    crypto::dsa::rpo_falcon512::{self, Polynomial},
};
use rand::Rng;
use vm_processor::{Digest, Felt, Word};

use crate::error::AuthenticationError;

// TRANSACTION AUTHENTICATOR
// ================================================================================================

/// Defines an authenticator for transactions.
///
/// The main purpose of the authenticator is to generate signatures for a given message against
/// a key managed by the authenticator. That is, the authenticator maintains a set of public-
/// private key pairs, and can be requested to generate signatures against any of the managed keys.
///
/// The public keys are defined by [Digest]'s which are the hashes of the actual public keys.
pub trait TransactionAuthenticator {
    /// Retrieves a signataure for a specific message as a list of [Felt].
    /// The request is initiaed by the VM as a consequence of the SigToStack advice
    /// injector.
    ///
    /// - `pub_key`: The public key used for signature generation.
    /// - `message`: The message to sign, usually a commitment to the transaction data.
    /// - `account_delta`: An informational parameter describing the changes made to
    ///   the account up to the point of calling `get_signature()`. This allows the
    ///   authenticator to review any alterations to the account prior to signing.
    ///   It should not be directly used in the signature computation.
    fn get_signature(
        &self,
        pub_key: Word,
        message: Word,
        account_delta: &AccountDelta,
    ) -> Result<Vec<Felt>, AuthenticationError>;
}

// BASIC AUTHENTICATOR
// ================================================================================================

#[derive(Clone, Debug)]
pub enum SecretKey {
    RpoFalcon512(rpo_falcon512::SecretKey),
}

#[derive(Clone, Debug)]
/// Represents a signer for [KeySecret] keys
pub struct BasicAuthenticator<R> {
    /// pub_key |-> secret_key mapping
    keys: BTreeMap<Digest, SecretKey>,
    rng: RefCell<R>,
}

impl<R: Rng> BasicAuthenticator<R> {
    #[cfg(feature = "std")]
    pub fn new(keys: &[(Word, SecretKey)]) -> BasicAuthenticator<rand::rngs::StdRng> {
        use rand::{rngs::StdRng, SeedableRng};

        let rng = StdRng::from_entropy();
        BasicAuthenticator::<StdRng>::new_with_rng(keys, rng)
    }

    pub fn new_with_rng(keys: &[(Word, SecretKey)], rng: R) -> Self {
        let mut key_map = BTreeMap::new();
        for (word, secret_key) in keys {
            key_map.insert(word.into(), secret_key.clone());
        }

        BasicAuthenticator { keys: key_map, rng: RefCell::new(rng) }
    }
}

impl<R: Rng> TransactionAuthenticator for BasicAuthenticator<R> {
    /// Gets as input a [Word] containing a secret key, and a [Word] representing a message and
    /// outputs a vector of values to be pushed onto the advice stack.
    /// The values are the ones required for a Falcon signature verification inside the VM and they are:
    ///
    /// 1. The nonce represented as 8 field elements.
    /// 2. The expanded public key represented as the coefficients of a polynomial of degree < 512.
    /// 3. The signature represented as the coefficients of a polynomial of degree < 512.
    /// 4. The product of the above two polynomials in the ring of polynomials with coefficients
    /// in the Miden field.
    ///
    /// # Errors
    /// Will return an error if either:
    /// - The secret key is malformed due to either incorrect length or failed decoding.
    /// - The signature generation failed.
    fn get_signature(
        &self,
        pub_key: Word,
        message: Word,
        account_delta: &AccountDelta,
    ) -> Result<Vec<Felt>, AuthenticationError> {
        let _ = account_delta;
        let mut rng = self.rng.borrow_mut();

        match self.keys.get(&pub_key.into()) {
            Some(key) => match key {
                SecretKey::RpoFalcon512(falcon_key) => {
                    get_falcon_signature(falcon_key, message, &mut *rng)
                },
            },
            None => Err(AuthenticationError::UnknownKey(format!(
                "Public key {} is not contained in the authenticator's keys",
                Digest::from(pub_key)
            ))),
        }
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Retrieves a falcon signature over a message
fn get_falcon_signature<R: Rng>(
    key: &rpo_falcon512::SecretKey,
    message: Word,
    rng: &mut R,
) -> Result<Vec<Felt>, AuthenticationError> {
    // Generate the signature
    let sig = key.sign_with_rng(message, rng);
    // The signature is composed of a nonce and a polynomial s2
    // The nonce is represented as 8 field elements.
    let nonce = sig.nonce();
    // We convert the signature to a polynomial
    let s2 = sig.sig_poly();
    // We also need in the VM the expanded key corresponding to the public key the was provided
    // via the operand stack
    let h = key.compute_pub_key_poly().0;
    // Lastly, for the probabilistic product routine that is part of the verification procedure,
    // we need to compute the product of the expanded key and the signature polynomial in
    // the ring of polynomials with coefficients in the Miden field.
    let pi = Polynomial::mul_modulo_p(&h, s2);
    // We now push the nonce, the expanded key, the signature polynomial, and the product of the
    // expanded key and the signature polynomial to the advice stack.
    let mut result: Vec<Felt> = nonce.to_elements().to_vec();

    result.extend(h.coefficients.iter().map(|a| Felt::from(a.value() as u32)));
    result.extend(s2.coefficients.iter().map(|a| Felt::from(a.value() as u32)));
    result.extend(pi.iter().map(|a| Felt::new(*a)));
    result.reverse();
    Ok(result)
}

impl TransactionAuthenticator for () {
    fn get_signature(
        &self,
        _pub_key: Word,
        _message: Word,
        _account_delta: &AccountDelta,
    ) -> Result<Vec<Felt>, AuthenticationError> {
        Err(AuthenticationError::RejectedSignature(
            "Default authenticator cannot provide signatures".to_string(),
        ))
    }
}
