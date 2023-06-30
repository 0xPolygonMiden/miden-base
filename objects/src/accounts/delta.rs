use super::{Digest, Felt, Word};
use assembly::ast::ModuleAst;
use crypto::{merkle::StoreNode, utils::collections::KvMapDiff};

// ACCOUNT DELTA
// ================================================================================================

/// [AccountDelta] stores the differences between the initial and final account states.
///
/// The differences are represented as follows:
/// - code: an Option<ModuleAst> that contains the updated code of the account.
/// - nonce: if the nonce of the account has changed, the new nonce is stored here.
/// - storage: an [AccountStorageDelta] that contains the changes to the account storage.
/// - vault: an tuple that contains the updated root of the account vault and a [KvMapDiff] that
///          contains the changes to the account vault asset tree.
#[derive(Debug, Clone)]
pub struct AccountDelta {
    pub code: Option<ModuleAst>,
    pub nonce: Option<Felt>,
    pub storage: AccountStorageDelta,
    pub vault: (Digest, KvMapDiff<Digest, StoreNode>),
}

// ACCOUNT STORAGE DELTA
// ================================================================================================

/// [AccountStorageDelta] stores the differences between the initial and final account storage
/// states.
///
/// The differences are represented as follows:
/// - cleared_slots: indexes of storage slots where values were set to [ZERO; 4].
/// - updated_slots: index-value pairs of slots where values were set to non [ZERO; 4] values.
/// - store_delta:   changes that have been applied to the account store represented as a [KvMapDiff].
/// TODO: Is this additional structure warranted? We could model the whole account storage delta as
///       [KvMapDiff].
#[derive(Default, Debug, Clone)]
pub struct AccountStorageDelta {
    pub cleared_slots: Vec<u8>,
    pub updated_slots: Vec<(u8, Word)>,
    pub store_delta: KvMapDiff<Digest, StoreNode>,
}
