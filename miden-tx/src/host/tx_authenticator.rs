use alloc::vec::Vec;

use miden_objects::{
    accounts::AccountDelta,
    crypto::dsa::rpo_falcon512::{Polynomial, SecretKey},
};
use vm_processor::{Felt, Word};

use crate::error::AuthenticationError;

// TRANSACTION AUTHENTICATOR
// ================================================================================================

/// Represents an authenticator for transactions.
///
/// Its main use is to provide a method to create a DSA signature for a message,
/// based on an [AccountDelta].
///
/// The signature is intended to provide authentication. The implementer can verify 
/// and approve any changes before the message is signed.
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

// FALCON AUTHENTICATOR
// ================================================================================================

/// Represents a signer for Falcon signatures, based on a user's [SecretKey]
#[derive(Clone, Debug)]
pub struct FalconAuthenticator {
    secret_key: SecretKey,
}

impl FalconAuthenticator {
    pub fn new(secret_key: SecretKey) -> Self {
        FalconAuthenticator { secret_key }
    }
}

impl TransactionAuthenticator for FalconAuthenticator {
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
        if pub_key != Word::from(self.secret_key.public_key()) {
            return Err(AuthenticationError::InvalidKey(
                "Public key does not match with the key from the signature request".into(),
            ));
        }

        // Generate the signature
        let sig = self.secret_key.sign(message);

        // The signature is composed of a nonce and a polynomial s2
        // The nonce is represented as 8 field elements.
        let nonce = sig.nonce();

        // We convert the signature to a polynomial
        let s2 = sig.sig_poly();

        // We also need in the VM the expanded key corresponding to the public key the was provided
        // via the operand stack
        let h = self.secret_key.compute_pub_key_poly().0;

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
}
