use core::cell::OnceCell;

use super::{
    Account, AdviceInputsBuilder, BlockHeader, ChainMmr, Digest, Felt, Hasher, Note, Nullifier,
    ToAdviceInputs, Word, MAX_INPUT_NOTES_PER_TRANSACTION,
};
use crate::{
    notes::{NoteInclusionProof, NoteOrigin},
    utils::{
        collections::{self, BTreeSet, Vec},
        serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
        string::ToString,
    },
    TransactionInputsError,
};

// TRANSACTION INPUTS
// ================================================================================================

/// Contains the data required to execute a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionInputs {
    pub account: Account,
    pub account_seed: Option<Word>,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub input_notes: InputNotes,
}

// INPUT NOTES
// ================================================================================================

/// Contains a list of input notes for a transaction.
///
/// The list can be empty if the transaction does not consume any notes.
#[derive(Debug, Clone)]
pub struct InputNotes {
    notes: Vec<InputNote>,
    commitment: OnceCell<Digest>,
}

impl InputNotes {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns new [InputNotes] instantiated from the provided vector of notes.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The total number of notes is greater than 1024.
    /// - The vector of notes contains duplicates.
    pub fn new(notes: Vec<InputNote>) -> Result<Self, TransactionInputsError> {
        if notes.len() > MAX_INPUT_NOTES_PER_TRANSACTION {
            return Err(TransactionInputsError::TooManyInputNotes {
                max: MAX_INPUT_NOTES_PER_TRANSACTION,
                actual: notes.len(),
            });
        }

        let mut seen_notes = BTreeSet::new();
        for note in notes.iter() {
            if !seen_notes.insert(note.note().hash()) {
                return Err(TransactionInputsError::DuplicateInputNote(note.note().hash()));
            }
        }

        Ok(Self { notes, commitment: OnceCell::new() })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to these input notes.
    pub fn commitment(&self) -> Digest {
        *self.commitment.get_or_init(|| build_input_notes_commitment(self.nullifiers()))
    }

    /// Returns total number of input notes.
    pub fn num_notes(&self) -> usize {
        self.notes.len()
    }

    /// Returns true if this [InputNotes] does not contain any notes.
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    /// Returns a reference to the [InputNote] located at the specified index.
    pub fn get_note(&self, idx: usize) -> &InputNote {
        &self.notes[idx]
    }

    // ITERATORS
    // --------------------------------------------------------------------------------------------

    /// Returns an iterator over notes in this [InputNotes].
    pub fn iter(&self) -> impl Iterator<Item = &InputNote> {
        self.notes.iter()
    }

    /// Returns an iterator over nullifiers of all notes in this [InputNotes].
    pub fn nullifiers(&self) -> impl Iterator<Item = Nullifier> + '_ {
        self.notes.iter().map(|note| note.note().nullifier())
    }
}

impl IntoIterator for InputNotes {
    type Item = InputNote;
    type IntoIter = collections::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.notes.into_iter()
    }
}

impl PartialEq for InputNotes {
    fn eq(&self, other: &Self) -> bool {
        self.notes == other.notes
    }
}

impl Eq for InputNotes {}

// ADVICE INPUTS
// --------------------------------------------------------------------------------------------

impl ToAdviceInputs for InputNotes {
    /// Populates the advice inputs for all consumed notes.
    ///
    /// For each note the authentication path is populated into the Merkle store, the note inputs
    /// and vault assets are populated in the advice map.  A combined note data vector is also
    /// constructed that holds core data for all notes. This combined vector is added to the advice
    /// map against the consumed notes commitment. For each note the following data items are added
    /// to the vector:
    ///     out[0..4]    = serial num
    ///     out[4..8]    = script root
    ///     out[8..12]   = input root
    ///     out[12..16]  = vault_hash
    ///     out[16..20]  = metadata
    ///     out[20..24]  = asset_1
    ///     out[24..28]  = asset_2
    ///     ...
    ///     out[20 + num_assets * 4..] = Word::default() (this is conditional padding only applied
    ///                                                   if the number of assets is odd)
    ///     out[-10]      = origin.block_number
    ///     out[-9..-5]   = origin.SUB_HASH
    ///     out[-5..-1]   = origin.NOTE_ROOT
    ///     out[-1]       = origin.node_index
    fn to_advice_inputs<T: AdviceInputsBuilder>(&self, target: &mut T) {
        let mut note_data: Vec<Felt> = Vec::new();

        note_data.push(Felt::from(self.notes.len() as u64));

        for recorded_note in &self.notes {
            let note = recorded_note.note();
            let proof = recorded_note.proof();

            note_data.extend(note.serial_num());
            note_data.extend(*note.script().hash());
            note_data.extend(*note.inputs().hash());
            note_data.extend(*note.vault().hash());
            note_data.extend(Word::from(note.metadata()));

            note_data.extend(note.vault().to_padded_assets());
            target.insert_into_map(note.vault().hash().into(), note.vault().to_padded_assets());

            note_data.push(proof.origin().block_num.into());
            note_data.extend(*proof.sub_hash());
            note_data.extend(*proof.note_root());
            note_data.push(Felt::from(proof.origin().node_index.value()));
            target.add_merkle_nodes(
                proof
                    .note_path()
                    .inner_nodes(proof.origin().node_index.value(), note.authentication_hash())
                    .unwrap(),
            );

            target.insert_into_map(note.inputs().hash().into(), note.inputs().inputs().to_vec());
        }

        target.insert_into_map(self.commitment().into(), note_data);
    }
}

// SERIALIZATION
// ------------------------------------------------------------------------------------------------

impl Serializable for InputNotes {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        // assert is OK here because we enforce max number of notes in the constructor
        assert!(self.notes.len() <= u16::MAX.into());
        target.write_u16(self.notes.len() as u16);
        self.notes.write_into(target);
    }
}

impl Deserializable for InputNotes {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let num_notes = source.read_u16()?;
        let notes = InputNote::read_batch_from(source, num_notes.into())?;
        Self::new(notes).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// HELPER FUNCTIONS
// ------------------------------------------------------------------------------------------------

/// Returns the commitment to the input notes represented by the specified nullifiers.
///
/// This is a sequential hash of all (nullifier, ZERO) pairs for the notes consumed in the
/// transaction.
pub fn build_input_notes_commitment<I: Iterator<Item = Nullifier>>(nullifiers: I) -> Digest {
    let mut elements: Vec<Felt> = Vec::new();
    for nullifier in nullifiers {
        elements.extend_from_slice(nullifier.as_elements());
        elements.extend_from_slice(&Word::default());
    }
    Hasher::hash_elements(&elements)
}

// RECORDED NOTE
// ================================================================================================

/// An input note for a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct InputNote {
    note: Note,
    proof: NoteInclusionProof,
}

impl InputNote {
    /// Returns a new instance of a [RecordedNote] with the specified note and origin.
    pub fn new(note: Note, proof: NoteInclusionProof) -> Self {
        Self { note, proof }
    }

    /// Returns a reference to the note which was recorded.
    pub fn note(&self) -> &Note {
        &self.note
    }

    /// Returns a reference to the inclusion proof of the recorded note.
    pub fn proof(&self) -> &NoteInclusionProof {
        &self.proof
    }

    /// Returns a reference to the origin of the recorded note.
    pub fn origin(&self) -> &NoteOrigin {
        self.proof.origin()
    }
}

// SERIALIZATION
// ------------------------------------------------------------------------------------------------

impl Serializable for InputNote {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.note.write_into(target);
        self.proof.write_into(target);
    }
}

impl Deserializable for InputNote {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let note = Note::read_from(source)?;
        let proof = NoteInclusionProof::read_from(source)?;

        Ok(Self { note, proof })
    }
}
