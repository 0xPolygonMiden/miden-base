use super::{AdviceInputsBuilder, Felt, Mmr, ToAdviceInputs, ZERO};

// TODO: Consider using a PartialMmr that only contains the Mmr nodes that are relevant to the
// transaction being processed.

/// A struct that represents the chain Mmr accumulator.
/// This wraps the [Mmr] object and provides a simple interface to access / modify it.
/// We use a custom type here as the traits we implement on this type could be context specific.
///
/// The Mmr allows for efficient authentication of consumed notes during transaction execution.
/// Authenticaiton is achieved by providing an inclusion proof for the consumed notes in the
/// transaction against the chain Mmr root associated with the latest block known at the time
/// of transaction exectuion.
#[derive(Default)]
pub struct ChainMmr(Mmr);

impl ChainMmr {
    /// Returns a reference to the Mmr.
    pub fn mmr(&self) -> &Mmr {
        &self.0
    }

    /// Returns a mutable reference to the Mmr.
    pub fn mmr_mut(&mut self) -> &mut Mmr {
        &mut self.0
    }
}

impl ToAdviceInputs for &ChainMmr {
    fn to_advice_inputs<T: AdviceInputsBuilder>(&self, target: &mut T) {
        // Add the Mmr nodes to the merkle store
        target.add_merkle_nodes(self.0.inner_nodes());

        // Extract Mmr accumulator
        let accumulator = self.0.accumulator();

        // create the vector of items to insert into the map
        // The vector is in the following format:
        //    elements[0]       = number of leaves in the Mmr
        //    elements[1..4]    = padding ([Felt::ZERO; 3])
        //    elements[4..]     = Mmr peak roots
        let mut elements = vec![Felt::new(accumulator.num_leaves as u64), ZERO, ZERO, ZERO];
        elements.extend(accumulator.flatten_and_pad_peaks());

        // insert the Mmr accumulator vector into the advice map against the Mmr root, which acts
        // as the key.
        target.insert_into_map(accumulator.hash_peaks(), elements);
    }
}
