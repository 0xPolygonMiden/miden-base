use super::{crypto::merkle::Mmr, AdviceInputsBuilder, ToAdviceInputs};

// TODO: Consider using a PartialMmr that only contains the Mmr nodes that are relevant to the
// transaction being processed.

/// A struct that represents the chain Mmr accumulator.
/// This wraps the [Mmr] object and provides a simple interface to access / modify it.
/// We use a custom type here as the traits we implement on this type could be context specific.
///
/// The Mmr allows for efficient authentication of consumed notes during transaction execution.
/// Authentication is achieved by providing an inclusion proof for the consumed notes in the
/// transaction against the chain Mmr root associated with the latest block known at the time
/// of transaction execution.
#[derive(Clone, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
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
        let peaks = self.0.peaks(self.0.forest()).unwrap();

        peaks.to_advice_inputs(target);
    }
}
