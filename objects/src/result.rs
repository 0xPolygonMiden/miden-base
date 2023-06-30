use super::{BTreeMap, Felt, MerkleStore, StackOutputs, Vec};

/// A trait that defines the interface for extracting objects from the result of a VM execution.
pub trait TryFromVmResult: Sized {
    type Error;

    /// Tries to create an object from the provided stack outputs and advice provider components.
    fn try_from_vm_result(
        stack_outputs: &StackOutputs,
        advice_stack: &[Felt],
        advice_map: &BTreeMap<[u8; 32], Vec<Felt>>,
        merkle_store: &MerkleStore,
    ) -> Result<Self, Self::Error>;
}
