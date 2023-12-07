use miden_crypto::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

use super::{Digest, Felt, Hasher, NoteError, Vec, ZERO};

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
    /// The inputs are padded with ZERO such that they are always of length 16.
    ///
    /// # Errors
    /// Returns an error if the number of provided inputs is greater than 16.
    pub fn new(inputs: &[Felt]) -> Result<Self, NoteError> {
        if inputs.len() > Self::NOTE_NUM_INPUTS {
            return Err(NoteError::too_many_inputs(inputs.len()));
        }

        // pad inputs with ZERO to be constant size (16 elements)
        let padded_inputs: [Felt; Self::NOTE_NUM_INPUTS] = inputs
            .iter()
            .cloned()
            .chain(core::iter::repeat(ZERO))
            .take(Self::NOTE_NUM_INPUTS)
            .collect::<Vec<_>>()
            .try_into()
            .expect("padded are of the correct length");

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

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteInputs {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.inputs.write_into(target);
    }
}

impl Deserializable for NoteInputs {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let inputs = <[Felt; 16]>::read_from(source)?;
        Self::new(&inputs).map_err(|v| DeserializationError::InvalidValue(format!("{v}")))
    }
}

// TESTS
// ================================================================================================

#[test]
fn test_input_ordering() {
    use super::Vec;

    // inputs are provided in reverse stack order
    let inputs = Vec::from([Felt::new(1), Felt::new(2), Felt::new(3)]);
    // we expect the inputs to be padded to length 16 and to remain in reverse stack order.
    let expected_ordering = Vec::from([
        Felt::new(1),
        Felt::new(2),
        Felt::new(3),
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
    ]);

    let note_inputs = NoteInputs::new(&inputs).expect("note created should succeed");
    assert_eq!(&expected_ordering, note_inputs.inputs());
}
