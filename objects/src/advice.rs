use super::{AdviceInputs, Felt, Vec, Word};
use assembly::utils::IntoBytes;
use crypto::merkle::InnerNodeInfo;

/// [AdviceInputsBuilder] trait specifies the interface for building advice inputs.
/// The trait provides three methods for building advice inputs:
/// - `push_onto_stack` pushes the given values onto the advice stack.
/// - `insert_into_map` inserts the given values into the advice map.
/// - `add_merkle_nodes` adds the given merkle nodes to the advice merkle store.
pub trait AdviceInputsBuilder {
    /// Pushes the given values onto the advice stack.
    fn push_onto_stack(&mut self, values: &[Felt]);

    /// Inserts the given values into the advice map.
    fn insert_into_map(&mut self, key: Word, values: Vec<Felt>);

    /// Adds the given merkle nodes to the advice merkle store.
    fn add_merkle_nodes<I: Iterator<Item = InnerNodeInfo>>(&mut self, nodes: I);
}

/// ToAdviceInputs trait specifies the interface for converting a rust object into advice inputs.
pub trait ToAdviceInputs {
    /// Converts the rust object into advice inputs and pushes them onto the given advice inputs
    /// builder.
    fn to_advice_inputs<T: AdviceInputsBuilder>(&self, target: &mut T);
}

/// Implement the `AdviceInputsBuilder` trait on `AdviceInputs`.
impl AdviceInputsBuilder for AdviceInputs {
    fn push_onto_stack(&mut self, values: &[Felt]) {
        self.extend_stack(values.iter().copied());
    }

    fn insert_into_map(&mut self, key: Word, values: Vec<Felt>) {
        self.extend_map([(key.into_bytes(), values)]);
    }

    fn add_merkle_nodes<I: Iterator<Item = InnerNodeInfo>>(&mut self, nodes: I) {
        self.extend_merkle_store(nodes);
    }
}
