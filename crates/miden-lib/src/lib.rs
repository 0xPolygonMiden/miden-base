#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use miden_objects::{
    account::{AccountDelta, AccountHeader},
    assembly::{mast::MastForest, Library},
    utils::{serde::Deserializable, sync::LazyLock},
    Hasher,
};
use transaction::{
    AccountBeforeIncrementNonceHandler, AccountProcedureIndexMap, AccountPushProcedureIndexHandler,
    AccountStorageAfterSetItemHandler, AccountStorageAfterSetMapItemHandler,
    AccountVaultAfterAddAssetHandler, AccountVaultAfterRemoveAssetHandler, NoteAfterCreatedHandler,
    NoteBeforeAddAssetHandler, OutputNoteBuilder, TransactionEvent,
};
use utils::sync::RwLock;
use vm_processor::{
    AdviceProvider, Digest, EventHandler, Felt, HostLibrary, NoopEventHandler, Word,
};

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
pub use miden_stdlib::{FalconSigToStackEventHandler, StdLibrary};

// CONSTANTS
// ================================================================================================

const MIDEN_LIB_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/miden.masl"));

#[derive(Debug, thiserror::Error)]
pub enum AuthenticationError {
    #[error("signature rejected: {0}")]
    RejectedSignature(String),
    #[error("unknown public key: {0}")]
    UnknownPublicKey(String),
    /// Custom error variant for implementors of the
    /// [`TransactionAuthenticatior`](crate::TransactionAuthenticator) trait.
    #[error("{error_msg}")]
    Other {
        error_msg: Box<str>,
        // thiserror will return this when calling Error::source on DataStoreError.
        source: Option<Box<dyn core::error::Error + Send + Sync + 'static>>,
    },
}

impl AuthenticationError {
    /// Creates a custom error using the [`AuthenticationError::Other`] variant from an error
    /// message.
    pub fn other(message: impl Into<String>) -> Self {
        let message: String = message.into();
        Self::Other { error_msg: message.into(), source: None }
    }

    /// Creates a custom error using the [`AuthenticationError::Other`] variant from an error
    /// message and a source error.
    pub fn other_with_source(
        message: impl Into<String>,
        source: impl core::error::Error + Send + Sync + 'static,
    ) -> Self {
        let message: String = message.into();
        Self::Other {
            error_msg: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

/// Defines an authenticator for transactions.
///
/// The main purpose of the authenticator is to generate signatures for a given message against
/// a key managed by the authenticator. That is, the authenticator maintains a set of public-
/// private key pairs, and can be requested to generate signatures against any of the managed keys.
///
/// The public keys are defined by [Digest]'s which are the hashes of the actual public keys.
pub trait TransactionAuthenticator {
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

impl TransactionAuthenticator for () {
    fn get_signature(
        &self,
        _pub_key: Word,
        _message: Word,
        _account_delta: &AccountDelta,
    ) -> Result<Vec<Felt>, AuthenticationError> {
        Err(AuthenticationError::RejectedSignature(
            "default authenticator cannot provide signatures".to_string(),
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FalconSigError {
    #[error("Failed to generate signature: {0}")]
    FailedSignatureGeneration(&'static str),
}

pub struct MidenFalconSignerInputs {
    /// Account state changes accumulated during transaction execution.
    pub account_delta: Arc<RwLock<AccountDeltaTracker>>,

    /// Serves signature generation requests from the transaction runtime for signatures which are
    /// not present in the `generated_signatures` field.
    pub authenticator: Option<Arc<dyn TransactionAuthenticator>>,
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

impl miden_stdlib::FalconSigner<MidenFalconSignerInputs> for MidenFalconSigner {
    fn sign_message<A>(
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
                        "No authenticator provided to Falcon signer",
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

    fn new(inputs: MidenFalconSignerInputs) -> Self {
        Self::new(inputs.account_delta, inputs.authenticator)
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
    pub account: AccountHeader,
    pub account_delta: Arc<RwLock<AccountDeltaTracker>>,
    pub account_code_commitments: BTreeSet<Digest>,
    pub account_proc_index_map: AccountProcedureIndexMap,
    pub output_notes: Arc<RwLock<BTreeMap<usize, OutputNoteBuilder>>>,
}

impl HostLibrary<EventHandlersInputs> for MidenLib {
    fn get_event_handlers<A>(&self, inputs: EventHandlersInputs) -> Vec<Box<dyn EventHandler<A>>>
    where
        A: AdviceProvider + 'static,
    {
        let EventHandlersInputs {
            account,
            account_delta,
            mut account_code_commitments,
            account_proc_index_map: acct_procedure_index_map,
            output_notes,
        } = inputs;
        // create account delta tracker and code commitments
        account_code_commitments.insert(account.code_commitment());

        // create all events
        let account_vault_after_add_asset =
            AccountVaultAfterAddAssetHandler { account_delta: account_delta.clone() };

        let account_vault_after_remove_asset =
            AccountVaultAfterRemoveAssetHandler { account_delta: account_delta.clone() };

        let account_storage_after_set_item =
            AccountStorageAfterSetItemHandler { account_delta: account_delta.clone() };

        let account_storage_after_set_map_item =
            AccountStorageAfterSetMapItemHandler { account_delta: account_delta.clone() };

        let account_before_increment_nonce =
            AccountBeforeIncrementNonceHandler { account_delta: account_delta.clone() };

        let account_push_procedure_index =
            AccountPushProcedureIndexHandler { acct_procedure_index_map };

        let note_after_created = NoteAfterCreatedHandler { output_notes: output_notes.clone() };

        let note_before_add_asset =
            NoteBeforeAddAssetHandler { output_notes: output_notes.clone() };

        vec![
            Box::new(account_vault_after_add_asset),
            Box::new(account_vault_after_remove_asset),
            Box::new(account_storage_after_set_item),
            Box::new(account_storage_after_set_map_item),
            Box::new(account_before_increment_nonce),
            Box::new(account_push_procedure_index),
            Box::new(note_after_created),
            Box::new(note_before_add_asset),
            // No-op event handlers
            Box::new(NoopEventHandler::new(TransactionEvent::AccountVaultBeforeAddAsset as u32)),
            Box::new(NoopEventHandler::new(TransactionEvent::AccountVaultBeforeRemoveAsset as u32)),
            Box::new(NoopEventHandler::new(TransactionEvent::AccountStorageBeforeSetItem as u32)),
            Box::new(NoopEventHandler::new(
                TransactionEvent::AccountStorageBeforeSetMapItem as u32,
            )),
            Box::new(NoopEventHandler::new(TransactionEvent::AccountAfterIncrementNonce as u32)),
            Box::new(NoopEventHandler::new(TransactionEvent::NoteBeforeCreated as u32)),
            Box::new(NoopEventHandler::new(TransactionEvent::NoteAfterAddAsset as u32)),
        ]
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

#[cfg(test)]
mod error_assertions {
    use super::*;

    /// Asserts at compile time that the passed error has Send + Sync + 'static bounds.
    fn _assert_error_is_send_sync_static<E: core::error::Error + Send + Sync + 'static>(_: E) {}

    fn _assert_authentication_error_bounds(err: AuthenticationError) {
        _assert_error_is_send_sync_static(err);
    }
}
