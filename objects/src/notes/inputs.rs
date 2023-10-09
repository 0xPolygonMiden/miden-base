use super::{Digest, Felt, Hasher, NoteError, ZERO};

/// Holds the inputs which are placed onto the stack before a note's script is executed.
/// - inputs are stored in reverse stack order such that when they are pushed onto stack they are
///   in the correct order
/// - hash is computed from inputs in the order they are stored (reverse stack order)
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteInputs {
    inputs: [Felt; 16],
    hash: Digest,
}

impl NoteInputs {
    /// Number of note inputs.
    const NOTE_NUM_INPUTS: usize = 16;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns NoteInputs created from the provided inputs.
    ///
    /// The inputs (provided in reverse stack order) are padded with ZERO such that they are always
    /// of length 16.
    ///
    /// # Errors
    /// Returns an error if the number of provided inputs is greater than 16.
    pub fn new(inputs: &[Felt]) -> Result<Self, NoteError> {
        if inputs.len() > Self::NOTE_NUM_INPUTS {
            return Err(NoteError::too_many_inputs(inputs.len()));
        }

        // pad inputs with ZERO to be constant size (16 elements)
        let mut padded_inputs = [ZERO; Self::NOTE_NUM_INPUTS];

        // insert inputs in reverse stack order starting from the end of the array
        let start_index = Self::NOTE_NUM_INPUTS - inputs.len();
        padded_inputs[start_index..].copy_from_slice(inputs);

        // compute hash from padded inputs.
        let hash = Hasher::hash_elements(&padded_inputs);

        Ok(Self {
            inputs: padded_inputs,
            hash,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a reference to the inputs.
    pub fn inputs(&self) -> &[Felt] {
        &self.inputs
    }

    /// Returns a hash digest of the inputs. Computed as a linear hash of the inputs.
    pub fn hash(&self) -> Digest {
        self.hash
    }
}

#[test]
fn test_input_ordering() {
    use super::Vec;

    // inputs are provided in reverse stack order
    let inputs = Vec::from([Felt::new(1), Felt::new(2), Felt::new(3)]);
    // we expect the inputs to be padded to length 16 and to remain in reverse stack order.
    let expected_ordering = Vec::from([
        ZERO,
        ZERO,
        ZERO,
        ZERO,
        ZERO,
        ZERO,
        ZERO,
        ZERO,
        ZERO,
        ZERO,
        ZERO,
        ZERO,
        ZERO,
        Felt::new(1),
        Felt::new(2),
        Felt::new(3),
    ]);

    let note_inputs = NoteInputs::new(&inputs).expect("note created should succeed");
    assert_eq!(&expected_ordering, note_inputs.inputs());
}
