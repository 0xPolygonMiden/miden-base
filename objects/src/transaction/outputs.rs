use alloc::{collections::BTreeSet, string::ToString, vec::Vec};
use core::fmt::Debug;

use miden_crypto::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

use crate::{
    accounts::AccountStub,
    notes::{Note, NoteAssets, NoteEnvelope, NoteId, NoteMetadata},
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

// OUTPUT NOTES
// ================================================================================================

/// Contains a list of output notes of a transaction. The list can be empty if the transaction does
/// not produce any notes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputNotes {
    notes: Vec<OutputNote>,
    commitment: Digest,
}

impl OutputNotes {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns new [OutputNotes] instantiated from the provide vector of notes.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The total number of notes is greater than 1024.
    /// - The vector of notes contains duplicates.
    pub fn new(notes: Vec<OutputNote>) -> Result<Self, TransactionOutputError> {
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
    pub fn get_note(&self, idx: usize) -> &OutputNote {
        &self.notes[idx]
    }

    // ITERATORS
    // --------------------------------------------------------------------------------------------

    /// Returns an iterator over notes in this [OutputNotes].
    pub fn iter(&self) -> impl Iterator<Item = &OutputNote> {
        self.notes.iter()
    }
}

// SERIALIZATION
// ------------------------------------------------------------------------------------------------

impl Serializable for OutputNotes {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        // assert is OK here because we enforce max number of notes in the constructor
        assert!(self.notes.len() <= u16::MAX.into());
        target.write_u16(self.notes.len() as u16);
        target.write_many(&self.notes);
    }
}

impl Deserializable for OutputNotes {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let num_notes = source.read_u16()?;
        let notes = source.read_many::<OutputNote>(num_notes.into())?;
        Self::new(notes).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// HELPER FUNCTIONS
// ------------------------------------------------------------------------------------------------

/// Build a commitment to output notes.
///
/// For a non-empty list of notes, this is a sequential hash of (note_id, metadata) tuples for the
/// notes created in a transaction. For an empty list, [ZERO; 4] is returned.
fn build_output_notes_commitment(notes: &[OutputNote]) -> Digest {
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

// CONSTANTS
// ------------------------------------------------------------------------------------------------

const PUBLIC: u8 = 0;
const PRIVATE: u8 = 1;

/// The types of note outputs supported by the transaction kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputNote {
    Public(Note),
    Private(NoteEnvelope),
}

impl OutputNote {
    /// The assets contained in the note.
    pub fn assets(&self) -> Option<&NoteAssets> {
        match self {
            OutputNote::Public(note) => Some(note.assets()),
            OutputNote::Private(_) => None,
        }
    }

    /// Unique note identifier.
    ///
    /// This value is both an unique identifier and a commitment to the note.
    pub fn id(&self) -> NoteId {
        match self {
            OutputNote::Public(note) => note.id(),
            OutputNote::Private(note) => note.id(),
        }
    }

    /// Value that represents under which condition a note can be consumed.
    ///
    /// See [super::NoteRecipient] for more details.
    pub fn recipient_digest(&self) -> Option<Digest> {
        match self {
            OutputNote::Public(note) => Some(note.recipient().digest()),
            OutputNote::Private(_) => None,
        }
    }

    /// Note's metadata.
    pub fn metadata(&self) -> NoteMetadata {
        match self {
            OutputNote::Public(note) => *note.metadata(),
            OutputNote::Private(note) => *note.metadata(),
        }
    }
}

// CONVERSIONS
// ------------------------------------------------------------------------------------------------

impl From<OutputNote> for NoteEnvelope {
    fn from(value: OutputNote) -> Self {
        (&value).into()
    }
}

impl From<&OutputNote> for NoteEnvelope {
    fn from(value: &OutputNote) -> Self {
        match value {
            OutputNote::Public(note) => note.into(),
            OutputNote::Private(note) => *note,
        }
    }
}

// SERIALIZATION
// ------------------------------------------------------------------------------------------------

impl Serializable for OutputNote {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            OutputNote::Public(note) => {
                target.write(PUBLIC);
                target.write(note);
            },
            OutputNote::Private(note) => {
                target.write(PRIVATE);
                target.write(note);
            },
        }
    }
}

impl Deserializable for OutputNote {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        match source.read_u8()? {
            PUBLIC => Ok(OutputNote::Public(Note::read_from(source)?)),
            PRIVATE => Ok(OutputNote::Private(NoteEnvelope::read_from(source)?)),
            v => Err(DeserializationError::InvalidValue(format!("Invalid note type: {v}"))),
        }
    }
}
