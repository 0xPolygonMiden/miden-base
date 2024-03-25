use alloc::vec::Vec;

use super::{
    ByteReader, ByteWriter, Deserializable, DeserializationError, Digest, Felt, Hasher, NoteError,
    Serializable, WORD_SIZE, ZERO,
};
use crate::MAX_INPUTS_PER_NOTE;

// NOTE INPUTS
// ================================================================================================

/// An container for note inputs.
///
/// A note can be associated with up to 128 input values. Each value is represented by a single
/// field element. Thus, note input values can contain up to ~1 KB of data.
///
/// All inputs associated with a note can be reduced to a single commitment which is computed by
/// first padding the inputs with ZEROs to the next multiple of 8, and then by computing a
/// sequential hash of the resulting elements.
#[derive(Clone, Debug)]
pub struct NoteInputs {
    values: Vec<Felt>,
    hash: Digest,
}

impl NoteInputs {
    /// Maximum number of input values associated with a single note.
    const MAX_INPUTS_PER_NOTE: usize = MAX_INPUTS_PER_NOTE;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns [NoteInputs] instantiated from the provided values.
    ///
    /// # Errors
    /// Returns an error if the number of provided inputs is greater than 128.
    pub fn new(values: Vec<Felt>) -> Result<Self, NoteError> {
        if values.len() > Self::MAX_INPUTS_PER_NOTE {
            return Err(NoteError::too_many_inputs(values.len()));
        }

        let hash = {
            let padded_values = pad_inputs(&values);
            Hasher::hash_elements(&padded_values)
        };

        Ok(Self { values, hash })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to these inputs.
    pub fn commitment(&self) -> Digest {
        self.hash
    }

    /// Returns the number of input values.
    ///
    /// The returned value is guaranteed to be smaller than or equal to 128.
    pub fn num_values(&self) -> u8 {
        self.values.len() as u8
    }

    /// Returns a reference to the input values.
    pub fn values(&self) -> &[Felt] {
        &self.values
    }

    /// Returns a vector of input values padded with ZEROs to the next multiple of 8.
    pub fn to_padded_values(&self) -> Vec<Felt> {
        pad_inputs(&self.values)
    }

    /// Returns a vector of input values.
    pub fn to_vec(&self) -> Vec<Felt> {
        self.values.to_vec()
    }
}

impl PartialEq for NoteInputs {
    fn eq(&self, other: &Self) -> bool {
        let NoteInputs { values: inputs, hash: _ } = self;
        inputs == &other.values
    }
}

impl Eq for NoteInputs {}

// HELPER FUNCTIONS
// ================================================================================================

/// Returns a vector with built from the provided inputs and padded to the next multiple of 8.
fn pad_inputs(inputs: &[Felt]) -> Vec<Felt> {
    const BLOCK_SIZE: usize = WORD_SIZE * 2;

    let padded_len = inputs.len().next_multiple_of(BLOCK_SIZE);
    let mut padded_inputs = Vec::with_capacity(padded_len);
    padded_inputs.extend(inputs.iter());
    padded_inputs.resize(padded_len, ZERO);

    padded_inputs
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteInputs {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let NoteInputs { values, hash: _hash } = self;
        target.write_u8(values.len().try_into().expect("inputs len is not a u8 value"));
        target.write_many(values);
    }
}

impl Deserializable for NoteInputs {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let num_values = source.read_u8()? as usize;
        let values = source.read_many::<Felt>(num_values)?;
        Self::new(values).map_err(|v| DeserializationError::InvalidValue(format!("{v}")))
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use miden_crypto::utils::Deserializable;

    use super::{Felt, NoteInputs, Serializable};

    #[test]
    fn test_input_ordering() {
        // inputs are provided in reverse stack order
        let inputs = vec![Felt::new(1), Felt::new(2), Felt::new(3)];
        // we expect the inputs to be padded to length 16 and to remain in reverse stack order.
        let expected_ordering = vec![Felt::new(1), Felt::new(2), Felt::new(3)];

        let note_inputs = NoteInputs::new(inputs).expect("note created should succeed");
        assert_eq!(&expected_ordering, &note_inputs.values);
    }

    #[test]
    fn test_input_serialization() {
        let inputs = vec![Felt::new(1), Felt::new(2), Felt::new(3)];
        let note_inputs = NoteInputs::new(inputs).unwrap();

        let bytes = note_inputs.to_bytes();
        let parsed_note_inputs = NoteInputs::read_from_bytes(&bytes).unwrap();
        assert_eq!(note_inputs, parsed_note_inputs);
    }
}
