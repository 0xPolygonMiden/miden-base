use miden_objects::{
    accounts::{AccountComponent, StorageSlot},
    crypto::dsa::rpo_falcon512::PublicKey,
};

use crate::accounts::components::rpo_falcon_512_library;

/// An [`AccountComponent`] implementing the RpoFalcon512 signature scheme for authentication of
/// transactions.
///
/// Its exported procedures are:
/// - `auth_tx_rpo_falcon512`, which can be used to verify a signature provided via the advice stack
///   to authenticate a transaction.
///
/// This component supports all account types.
pub struct RpoFalcon512 {
    public_key: PublicKey,
}

impl RpoFalcon512 {
    /// Creates a new [`RpoFalcon512`] component with the given `public_key`.
    pub fn new(public_key: PublicKey) -> Self {
        Self { public_key }
    }
}

impl From<RpoFalcon512> for AccountComponent {
    fn from(falcon: RpoFalcon512) -> Self {
        AccountComponent::new(
            rpo_falcon_512_library(),
            vec![StorageSlot::Value(falcon.public_key.into())],
        )
        .with_supports_all_types()
    }
}