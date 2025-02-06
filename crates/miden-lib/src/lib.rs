#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::String,
    sync::Arc,
    vec::Vec,
};

use miden_objects::{
    account::{AccountDelta, AccountHeader},
    assembly::{mast::MastForest, Library},
    utils::{serde::Deserializable, sync::LazyLock},
    Hasher,
};
use transaction::AccountBeforeIncrementNonceHandler;
use utils::sync::RwLock;
use vm_processor::{AdviceProvider, Digest, EventHandler, Felt, HostLibrary, Word};

mod auth;
pub use auth::AuthScheme;

pub mod account;
pub mod errors;
pub mod note;
pub mod transaction;

mod account_delta_tracker;
pub use account_delta_tracker::AccountDeltaTracker;
// RE-EXPORTS
// ================================================================================================
pub use miden_objects::utils;
pub use miden_stdlib::{StdLibrary, FalconSigToStackEventHandler};

// CONSTANTS
// ================================================================================================

const MIDEN_LIB_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/miden.masl"));

// TODO(plafer): Properly move this to miden-lib and remove the other version

#[derive(Debug, thiserror::Error)]
pub enum AuthenticationError {
    #[error("signature rejected: {0}")]
    RejectedSignature(String),
    #[error("unknown public key: {0}")]
    UnknownPublicKey(String),
    /// Custom error variant for implementors of the
    /// [`TransactionAuthenticatior`](crate::auth::TransactionAuthenticator) trait.
    #[error("{error_msg}")]
    Other {
        error_msg: Box<str>,
        // thiserror will return this when calling Error::source on DataStoreError.
        source: Option<Box<dyn core::error::Error + Send + Sync + 'static>>,
    },
}
/// Defines an authenticator for transactions.
///
/// The main purpose of the authenticator is to generate signatures for a given message against
/// a key managed by the authenticator. That is, the authenticator maintains a set of public-
/// private key pairs, and can be requested to generate signatures against any of the managed keys.
///
/// The public keys are defined by [Digest]'s which are the hashes of the actual public keys.
pub trait TransactionAuthenticator: Send + Sync {
    /// Retrieves a signature for a specific message as a list of [Felt].
    ///
    /// The request is initiated by the VM as a consequence of the SigToStack advice
    /// injector.
    ///
    /// - `pub_key`: The public key used for signature generation.
    /// - `message`: The message to sign, usually a commitment to the transaction data.
    /// - `account_delta`: An informational parameter describing the changes made to the account up
    ///   to the point of calling `get_signature()`. This allows the authenticator to review any
    ///   alterations to the account prior to signing. It should not be directly used in the
    ///   signature computation.
    fn get_signature(
        &self,
        pub_key: Word,
        message: Word,
        account_delta: &AccountDelta,
    ) -> Result<Vec<Felt>, AuthenticationError>;
}

// TODO(plafer): Maybe this can live in `miden-tx`?
pub struct MidenFalconSigner {
    /// Account state changes accumulated during transaction execution.
    account_delta: Arc<RwLock<AccountDeltaTracker>>,

    // TODO(plafer): Remove Option?
    /// Serves signature generation requests from the transaction runtime for signatures which are
    /// not present in the `generated_signatures` field.
    authenticator: Option<Arc<dyn TransactionAuthenticator>>,

    /// Contains previously generated signatures (as a message |-> signature map) required for
    /// transaction execution.
    ///
    /// If a required signature is not present in this map, the host will attempt to generate the
    /// signature using the transaction authenticator.
    generated_signatures: BTreeMap<Digest, Vec<Felt>>,
}

impl MidenFalconSigner {
    /// Creates a new instance of the Falcon signer.
    pub fn new(
        account_delta: Arc<RwLock<AccountDeltaTracker>>,
        authenticator: Option<Arc<dyn TransactionAuthenticator>>,
    ) -> Self {
        Self {
            account_delta,
            authenticator,
            generated_signatures: BTreeMap::new(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FalconSigError {
    #[error("Failed to generate signature: {0}")]
    FailedSignatureGeneration(&'static str),
}

impl<A> miden_stdlib::FalconSigner<A> for MidenFalconSigner {
    fn sign_message(
        &mut self,
        pub_key: Word,
        msg: Word,
        advice_provider: &A,
    ) -> Result<Vec<Felt>, Box<dyn core::error::Error + Send + Sync + 'static>>
    where
        A: AdviceProvider,
    {
        let signature_key = Hasher::merge(&[pub_key.into(), msg.into()]);

        let signature = if let Some(signature) = advice_provider.get_mapped_values(&signature_key) {
            signature.to_vec()
        } else {
            let account_delta = self.account_delta.read();
            let account_delta = account_delta.clone().into_delta();

            let signature: Vec<Felt> = match &self.authenticator {
                None => {
                    return Err(FalconSigError::FailedSignatureGeneration(
                        "No authenticator assigned to transaction host",
                    )
                    .into())
                },
                Some(authenticator) => {
                    authenticator.get_signature(pub_key, msg, &account_delta).map_err(|_| {
                        Box::new(FalconSigError::FailedSignatureGeneration(
                            "Error generating signature",
                        ))
                    })
                },
            }?;

            self.generated_signatures.insert(signature_key, signature.clone());
            signature
        };

        Ok(signature)
    }
}

// MIDEN LIBRARY
// ================================================================================================

#[derive(Clone)]
pub struct MidenLib(Library);

impl MidenLib {
    /// Returns a reference to the [`MastForest`] of the inner [`Library`].
    pub fn mast_forest(&self) -> &Arc<MastForest> {
        self.0.mast_forest()
    }
}

impl AsRef<Library> for MidenLib {
    fn as_ref(&self) -> &Library {
        &self.0
    }
}

impl From<MidenLib> for Library {
    fn from(value: MidenLib) -> Self {
        value.0
    }
}

impl Default for MidenLib {
    fn default() -> Self {
        static MIDEN_LIB: LazyLock<MidenLib> = LazyLock::new(|| {
            let contents =
                Library::read_from_bytes(MIDEN_LIB_BYTES).expect("failed to read miden lib masl!");
            MidenLib(contents)
        });
        MIDEN_LIB.clone()
    }
}

pub struct EventHandlersInputs {
    account: AccountHeader,
    account_delta: Arc<RwLock<AccountDeltaTracker>>,
    account_code_commitments: BTreeSet<Digest>,
}

impl HostLibrary<EventHandlersInputs> for MidenLib {
    fn get_event_handlers<A>(&self, inputs: EventHandlersInputs) -> Vec<Box<dyn EventHandler<A>>>
    where
        A: AdviceProvider + 'static,
    {
        let EventHandlersInputs { account, account_delta, mut account_code_commitments } = inputs;
        // create account delta tracker and code commitments
        account_code_commitments.insert(account.code_commitment());
        let account_delta = Arc::new(RwLock::new(AccountDeltaTracker::new(&account)));

        // create all events
        let account_vault_before_add_asset =
            AccountBeforeIncrementNonceHandler { account_delta: account_delta.clone() };

        // TODO(plafer): Add all events here
        vec![Box::new(account_vault_before_add_asset)]
    }

    fn get_mast_forest(&self) -> Arc<MastForest> {
        self.0.mast_forest().clone()
    }
}

// TESTS
// ================================================================================================

// NOTE: Most kernel-related tests can be found under /miden-tx/kernel_tests
#[cfg(all(test, feature = "std"))]
mod tests {
    use miden_objects::assembly::LibraryPath;

    use super::MidenLib;

    #[test]
    fn test_compile() {
        let path = "miden::account::get_id".parse::<LibraryPath>().unwrap();
        let miden = MidenLib::default();
        let exists = miden.0.module_infos().any(|module| {
            module
                .procedures()
                .any(|(_, proc)| module.path().clone().append(&proc.name).unwrap() == path)
        });

        assert!(exists);
    }
}
