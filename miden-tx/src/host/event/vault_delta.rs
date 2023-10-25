use miden_objects::{
    assets::Asset,
    utils::collections::{btree_map::Entry, BTreeMap},
    Digest,
};
use vm_processor::{ExecutionError, HostResponse, ProcessState};

#[derive(Default, Debug)]
pub struct VaultDeltaHandler {
    fungible_assets: BTreeMap<u64, i128>,
    non_fungible_assets: BTreeMap<Digest, i8>,
}

impl VaultDeltaHandler {
    pub fn add_asset<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<HostResponse, ExecutionError> {
        let asset: Asset = process.get_stack_word(0).try_into().unwrap();
        match asset {
            Asset::Fungible(asset) => {
                update_asset_delta(
                    &mut self.fungible_assets,
                    asset.faucet_id().into(),
                    asset.amount() as i128,
                );
            }
            Asset::NonFungible(asset) => {
                update_asset_delta(&mut self.non_fungible_assets, asset.vault_key().into(), 1)
            }
        };
        Ok(HostResponse::None)
    }

    pub fn remove_asset<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<HostResponse, ExecutionError> {
        let asset: Asset = process.get_stack_word(0).try_into().unwrap();
        match asset {
            Asset::Fungible(asset) => {
                update_asset_delta(
                    &mut self.fungible_assets,
                    asset.faucet_id().into(),
                    -(asset.amount() as i128),
                );
            }
            Asset::NonFungible(asset) => {
                update_asset_delta(&mut self.non_fungible_assets, asset.vault_key().into(), -1)
            }
        };
        Ok(HostResponse::None)
    }
}

// HELPERS
// ================================================================================================
fn update_asset_delta<K, V>(delta_map: &mut BTreeMap<K, V>, key: K, amount: V)
where
    V: core::ops::Neg,
    V: core::cmp::PartialEq<<V as core::ops::Neg>::Output>,
    V: core::ops::AddAssign,
    V: Copy,
    K: Ord,
{
    match delta_map.entry(key) {
        Entry::Occupied(mut entry) => {
            if entry.get() == &-amount {
                entry.remove();
            } else {
                *entry.get_mut() += amount;
            }
        }
        Entry::Vacant(entry) => {
            entry.insert(amount);
        }
    }
}
