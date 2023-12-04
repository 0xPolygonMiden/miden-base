use miden_crypto::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

use super::{
    utils::generate_consumed_notes_commitment, AdviceInputsBuilder, Digest, Felt, RecordedNote,
    ToAdviceInputs, Vec, Word,
};

// CONSUMED NOTES
// ================================================================================================

/// An object that holds a list of notes that were consumed by a transaction.
///
/// This objects primary use case is to enable all consumed notes to be populated into the advice
/// provider at once via the [ToAdviceInputs] trait.
#[derive(Debug, Clone)]
pub struct ConsumedNotes {
    notes: Vec<RecordedNote>,
    commitment: Digest,
}

impl ConsumedNotes {
    /// Creates a new [ConsumedNotes] object.
    pub fn new(notes: Vec<RecordedNote>) -> Self {
        assert!(notes.len() <= u16::MAX.into());
        let commitment = generate_consumed_notes_commitment(&notes);
        Self { notes, commitment }
    }

    /// Returns the consumed notes.
    pub fn notes(&self) -> &[RecordedNote] {
        &self.notes
    }

    /// Returns a commitment to the consumed notes.
    pub fn commitment(&self) -> Digest {
        self.commitment
    }
}

impl ToAdviceInputs for ConsumedNotes {
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

            note_data.push(proof.origin().block_num);
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

        target.insert_into_map(*self.commitment, note_data);
    }
}

impl From<ConsumedNotes> for Vec<ConsumedNoteInfo> {
    fn from(consumed_notes: ConsumedNotes) -> Self {
        consumed_notes.notes.into_iter().map(|note| note.into()).collect::<Vec<_>>()
    }
}

impl From<Vec<RecordedNote>> for ConsumedNotes {
    fn from(recorded_notes: Vec<RecordedNote>) -> Self {
        Self::new(recorded_notes)
    }
}

impl FromIterator<RecordedNote> for ConsumedNotes {
    fn from_iter<T: IntoIterator<Item = RecordedNote>>(iter: T) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

// CONSUMED NOTE INFO
// ================================================================================================

/// Holds information about a note that was consumed by a transaction.
/// Contains:
/// - nullifier: nullifier of the note that was consumed
/// - script_root: script root of the note that was consumed
#[derive(Clone, Copy, Debug)]
pub struct ConsumedNoteInfo {
    nullifier: Digest,
    script_root: Digest,
}

impl ConsumedNoteInfo {
    /// Creates a new ConsumedNoteInfo object.
    pub fn new(nullifier: Digest, script_root: Digest) -> Self {
        Self {
            nullifier,
            script_root,
        }
    }

    /// Returns the nullifier of the note that was consumed.
    pub fn nullifier(&self) -> Digest {
        self.nullifier
    }

    /// Returns the script root of the note that was consumed.
    pub fn script_root(&self) -> Digest {
        self.script_root
    }
}

impl From<ConsumedNoteInfo> for [Felt; 8] {
    fn from(note_info: ConsumedNoteInfo) -> Self {
        (&note_info).into()
    }
}

impl From<ConsumedNoteInfo> for [Word; 2] {
    fn from(note_info: ConsumedNoteInfo) -> Self {
        (&note_info).into()
    }
}

impl From<ConsumedNoteInfo> for [u8; 64] {
    fn from(note_info: ConsumedNoteInfo) -> Self {
        (&note_info).into()
    }
}

impl From<RecordedNote> for ConsumedNoteInfo {
    fn from(recorded_note: RecordedNote) -> Self {
        (&recorded_note).into()
    }
}

impl From<&ConsumedNoteInfo> for [Felt; 8] {
    fn from(note_info: &ConsumedNoteInfo) -> Self {
        let mut elements: [Felt; 8] = Default::default();
        elements[..4].copy_from_slice(note_info.nullifier.as_elements());
        elements[4..].copy_from_slice(note_info.script_root.as_elements());
        elements
    }
}

impl From<&ConsumedNoteInfo> for [Word; 2] {
    fn from(note_info: &ConsumedNoteInfo) -> Self {
        let mut elements: [Word; 2] = Default::default();
        elements[0].copy_from_slice(note_info.nullifier.as_elements());
        elements[1].copy_from_slice(note_info.script_root.as_elements());
        elements
    }
}

impl From<&ConsumedNoteInfo> for [u8; 64] {
    fn from(note_info: &ConsumedNoteInfo) -> Self {
        let mut elements: [u8; 64] = [0; 64];
        elements[..32].copy_from_slice(&note_info.nullifier.as_bytes());
        elements[32..].copy_from_slice(&note_info.script_root.as_bytes());
        elements
    }
}

impl From<&RecordedNote> for ConsumedNoteInfo {
    fn from(recorded_note: &RecordedNote) -> Self {
        Self::new(recorded_note.note().nullifier(), recorded_note.note().script().hash())
    }
}

// SERIALIZATION
// ================================================================================================
//

impl Serializable for ConsumedNotes {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        assert!(self.notes.len() <= u16::MAX.into());
        target.write_u16(self.notes.len() as u16);
        self.notes.write_into(target);
    }
}

impl Deserializable for ConsumedNotes {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let count = source.read_u16()?;
        let notes = RecordedNote::read_batch_from(source, count.into())?;

        Ok(Self::new(notes))
    }
}

impl Serializable for ConsumedNoteInfo {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_bytes(&self.nullifier.to_bytes());
        target.write_bytes(&self.script_root.to_bytes());
    }
}

impl Deserializable for ConsumedNoteInfo {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let nullifier = Digest::read_from(source)?;
        let script_root = Digest::read_from(source)?;

        Ok(Self {
            nullifier,
            script_root,
        })
    }
}
