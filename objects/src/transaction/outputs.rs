use alloc::{collections::BTreeSet, string::ToString, vec::Vec};
use core::fmt::Debug;

use crate::{
    accounts::AccountStub,
    notes::{Note, NoteAssets, NoteEnvelope, NoteId, NoteMetadata},
    utils::serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest, Felt, Hasher, TransactionOutputError, Word, MAX_OUTPUT_NOTES_PER_TX,
};

// TRANSACTION OUTPUTS
// ================================================================================================

/// Describes the result of executing a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionOutputs {
    pub account: AccountStub,
    pub output_notes: OutputNotes,
}

// TO ENVELOPE TRAIT
// ================================================================================================

/// Defines how a note object can be reduced to a note envelope (i.e., (ID, metadata) tuple).
///
/// This trait is implemented on both [OutputNote] and [NoteEnvelope] so that we can treat them
/// generically as [OutputNotes].
pub trait ToEnvelope:
    Debug + Clone + PartialEq + Eq + Serializable + Deserializable + Sized
{
    fn id(&self) -> NoteId;
    fn metadata(&self) -> NoteMetadata;

    fn to_envelope(&self) -> NoteEnvelope {
        NoteEnvelope::new(self.id(), self.metadata())
    }
}

impl ToEnvelope for OutputNote {
    fn id(&self) -> NoteId {
        self.id()
    }

    fn metadata(&self) -> NoteMetadata {
        *self.metadata()
    }
}

impl ToEnvelope for NoteEnvelope {
    fn id(&self) -> NoteId {
        self.note_id()
    }

    fn metadata(&self) -> NoteMetadata {
        *self.metadata()
    }
}

impl From<OutputNotes> for OutputNotes<NoteEnvelope> {
    fn from(notes: OutputNotes) -> Self {
        Self {
            notes: notes.notes.iter().map(|note| note.envelope).collect(),
            commitment: build_output_notes_commitment(&notes.notes),
        }
    }
}

// OUTPUT NOTES
// ================================================================================================

/// Contains a list of output notes of a transaction. The list can be empty if the transaction does
/// not produce any notes.
///
/// For the purposes of this struct, anything that can be reduced to a note envelope can be an
/// output note. However, [ToEnvelope] trait is currently implemented only for [OutputNote] and
/// [NoteEnvelope], and so these are the only two allowed output note types.
#[derive(Debug, Clone)]
pub struct OutputNotes<T: ToEnvelope = OutputNote> {
    notes: Vec<T>,
    commitment: Digest,
}

impl<T: ToEnvelope> OutputNotes<T> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns new [OutputNotes] instantiated from the provide vector of notes.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The total number of notes is greater than 1024.
    /// - The vector of notes contains duplicates.
    pub fn new(notes: Vec<T>) -> Result<Self, TransactionOutputError> {
        if notes.len() > MAX_OUTPUT_NOTES_PER_TX {
            return Err(TransactionOutputError::TooManyOutputNotes {
                max: MAX_OUTPUT_NOTES_PER_TX,
                actual: notes.len(),
            });
        }

        let mut seen_notes = BTreeSet::new();
        for note in notes.iter() {
            if !seen_notes.insert(note.id()) {
                return Err(TransactionOutputError::DuplicateOutputNote(note.id()));
            }
        }

        let commitment = build_output_notes_commitment(&notes);

        Ok(Self { notes, commitment })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the commitment to the output notes.
    ///
    /// The commitment is computed as a sequential hash of (hash, metadata) tuples for the notes
    /// created in a transaction.
    pub fn commitment(&self) -> Digest {
        self.commitment
    }
    /// Returns total number of output notes.
    pub fn num_notes(&self) -> usize {
        self.notes.len()
    }

    /// Returns true if this [OutputNotes] does not contain any notes.
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    /// Returns a reference to the note located at the specified index.
    pub fn get_note(&self, idx: usize) -> &T {
        &self.notes[idx]
    }

    // ITERATORS
    // --------------------------------------------------------------------------------------------

    /// Returns an iterator over notes in this [OutputNotes].
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.notes.iter()
    }
}

impl<T: ToEnvelope> IntoIterator for OutputNotes<T> {
    type Item = T;
    type IntoIter = alloc::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.notes.into_iter()
    }
}

impl<T: ToEnvelope> PartialEq for OutputNotes<T> {
    fn eq(&self, other: &Self) -> bool {
        self.notes == other.notes
    }
}

impl<T: ToEnvelope> Eq for OutputNotes<T> {}

impl Default for OutputNotes {
    fn default() -> Self {
        OutputNotes::new(Vec::new()).expect("failed to create empty output notes")
    }
}

// SERIALIZATION
// ------------------------------------------------------------------------------------------------

impl<T: ToEnvelope> Serializable for OutputNotes<T> {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        // assert is OK here because we enforce max number of notes in the constructor
        assert!(self.notes.len() <= u16::MAX.into());
        target.write_u16(self.notes.len() as u16);
        target.write_many(&self.notes);
    }
}

impl<T: ToEnvelope> Deserializable for OutputNotes<T> {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let num_notes = source.read_u16()?;
        let notes = source.read_many::<T>(num_notes.into())?;
        Self::new(notes).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// HELPER FUNCTIONS
// ------------------------------------------------------------------------------------------------

/// Build a commitment to output notes.
///
/// For a non-empty list of notes, this is a sequential hash of (note_id, metadata) tuples for the
/// notes created in a transaction. For an empty list, [ZERO; 4] is returned.
fn build_output_notes_commitment<T: ToEnvelope>(notes: &[T]) -> Digest {
    if notes.is_empty() {
        return Digest::default();
    }

    let mut elements: Vec<Felt> = Vec::with_capacity(notes.len() * 8);
    for note in notes.iter() {
        elements.extend_from_slice(note.id().as_elements());
        elements.extend_from_slice(&Word::from(note.metadata()));
    }

    Hasher::hash_elements(&elements)
}

// OUTPUT NOTE
// ================================================================================================

/// An note create during a transaction.
///
/// When a note is produced in a transaction, the note's recipient, assets, and metadata must be
/// known. However, other information about the note may or may not be know to the note's producer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputNote {
    envelope: NoteEnvelope,
    recipient: Digest,
    assets: NoteAssets,
}

impl OutputNote {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new [OutputNote] instantiated from the provided parameters.
    pub fn new(recipient: Digest, assets: NoteAssets, metadata: NoteMetadata) -> Self {
        let note_id = NoteId::new(recipient, assets.commitment());
        Self {
            envelope: NoteEnvelope::new(note_id, metadata),
            recipient,
            assets,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the recipient of the note.
    pub fn recipient(&self) -> &Digest {
        &self.recipient
    }

    /// Returns a reference to the asset of this note.
    pub fn assets(&self) -> &NoteAssets {
        &self.assets
    }

    /// Returns the metadata associated with this note.
    pub fn metadata(&self) -> &NoteMetadata {
        self.envelope.metadata()
    }

    /// Return the unique ID of this note.
    pub fn id(&self) -> NoteId {
        self.envelope.note_id()
    }
}

impl From<OutputNote> for NoteEnvelope {
    fn from(note_stub: OutputNote) -> Self {
        note_stub.envelope
    }
}

impl From<&OutputNote> for NoteEnvelope {
    fn from(note_stub: &OutputNote) -> Self {
        note_stub.envelope
    }
}

impl From<Note> for OutputNote {
    fn from(note: Note) -> Self {
        (&note).into()
    }
}

impl From<&Note> for OutputNote {
    fn from(note: &Note) -> Self {
        let recipient = note.recipient();
        Self::new(recipient, note.assets().clone(), *note.metadata())
    }
}

// SERIALIZATION
// ------------------------------------------------------------------------------------------------

impl Serializable for OutputNote {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.recipient.write_into(target);
        self.assets.write_into(target);
        self.envelope.metadata().write_into(target);
    }
}

impl Deserializable for OutputNote {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let recipient = Digest::read_from(source)?;
        let assets = NoteAssets::read_from(source)?;
        let metadata = NoteMetadata::read_from(source)?;

        Ok(Self::new(recipient, assets, metadata))
    }
}
