use crate::notes::{Note, NoteEnvelope, NoteStub};
use core::iter::FromIterator;
use crypto::{
    hash::rpo::{Rpo256 as Hasher, RpoDigest as Digest},
    utils::collections::Vec,
    Felt, Word,
};

// CREATED NOTES
// ================================================================================================
/// [CreatedNotes] represents the notes created by a transaction.
///
/// [CreatedNotes] is composed of:
/// - notes: a vector of [NoteStub] objects representing the notes created by the transaction.
/// - commitment: a commitment to the created notes.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatedNotes {
    notes: Vec<NoteStub>,
    commitment: Digest,
}

impl CreatedNotes {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates a new [CreatedNotes] object from the provided vector of [NoteStub]s.
    pub fn new(notes: Vec<NoteStub>) -> Self {
        let commitment = generate_created_notes_stub_commitment(&notes);
        Self { notes, commitment }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------
    /// Returns a reference to the vector of [NoteStub]s.
    pub fn notes(&self) -> &[NoteStub] {
        &self.notes
    }

    /// Returns the commitment to the created notes.
    pub fn commitment(&self) -> Digest {
        self.commitment
    }
}

/// Returns the created notes commitment.
/// This is a sequential hash of all (hash, metadata) pairs for the notes created in the transaction.
pub fn generate_created_notes_stub_commitment(notes: &[NoteStub]) -> Digest {
    let mut elements: Vec<Felt> = Vec::with_capacity(notes.len() * 8);
    for note in notes.iter() {
        elements.extend_from_slice(note.hash().as_elements());
        elements.extend_from_slice(&Word::from(note.metadata()));
    }

    Hasher::hash_elements(&elements)
}

impl From<CreatedNotes> for Vec<NoteEnvelope> {
    fn from(created_notes: CreatedNotes) -> Self {
        (&created_notes).into()
    }
}

impl From<&CreatedNotes> for Vec<NoteEnvelope> {
    fn from(created_notes: &CreatedNotes) -> Self {
        created_notes.notes.iter().map(|note| note.into()).collect::<Vec<_>>()
    }
}

impl From<Vec<Note>> for CreatedNotes {
    fn from(notes: Vec<Note>) -> Self {
        Self::new(notes.into_iter().map(|note| note.into()).collect())
    }
}

impl From<Vec<&Note>> for CreatedNotes {
    fn from(notes: Vec<&Note>) -> Self {
        Self::new(notes.iter().map(|note| (*note).into()).collect())
    }
}

impl FromIterator<Note> for CreatedNotes {
    fn from_iter<T: IntoIterator<Item = Note>>(iter: T) -> Self {
        Self::new(iter.into_iter().map(|v| v.into()).collect())
    }
}
