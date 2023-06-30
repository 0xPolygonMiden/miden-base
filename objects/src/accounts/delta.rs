use super::{Digest, Felt};
use crypto::{merkle::StoreNode, utils::collections::KvMapDiff};

// ACCOUNT DELTA
// ================================================================================================

/// [AccountDelta] stores the differences between the initial and final account states.
///
/// The differences are represented as follows:
/// - code_delta: an optional tuple that contains the updated root of the account code and a
///               [KvMapDiff] that contains the changes to the account code procedure tree.
/// - nonce_delta: if the nonce of the account has changed, the new nonce is stored here.
/// - storage_delta: an optional tuple that contains the updated root of the account storage and a
///                  [KvMapDiff] that contains the changes to the account storage slot tree and store.
/// - vault_delta: an optional tuple that contains the updated root of the account vault and a
///                [KvMapDiff] that contains the changes to the account vault asset tree.
#[derive(Debug, Clone)]
pub struct AccountDelta {
    // TODO: Change `code` to a more appropriate type that encodes the changes in the account code.
    //       see https://github.com/0xPolygonMiden/miden-base/issues/158#issuecomment-1609121727
    pub code_delta: Option<(Digest, KvMapDiff<Digest, StoreNode>)>,
    pub nonce_delta: Option<Felt>,
    pub storage_delta: Option<(Digest, KvMapDiff<Digest, StoreNode>)>,
    pub vault_delta: Option<(Digest, KvMapDiff<Digest, StoreNode>)>,
}
