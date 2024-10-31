use miden_objects::{
    accounts::{AccountComponent, StorageSlot},
    crypto::dsa::rpo_falcon512::PublicKey,
};

use crate::accounts::components::rpo_falcon_512_library;

pub struct RpoFalcon512 {
    public_key: PublicKey,
}

impl RpoFalcon512 {
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
