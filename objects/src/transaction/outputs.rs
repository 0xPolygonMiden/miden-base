use core::cell::OnceCell;

use super::MAX_OUTPUT_NOTES_PER_TRANSACTION;
use crate::{
    accounts::AccountStub,
    notes::{Note, NoteEnvelope, NoteMetadata, NoteVault},
    utils::collections::{self, BTreeSet, Vec},
    Digest, Felt, Hasher, StarkField, TransactionResultError, Word,
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

/// Contains a list of output notes of a transaction.
#[derive(Debug, Clone)]
pub struct OutputNotes {
    notes: Vec<OutputNote>,
    commitment: OnceCell<Digest>,
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
    pub fn new(notes: Vec<OutputNote>) -> Result<Self, TransactionResultError> {
        if notes.len() > MAX_OUTPUT_NOTES_PER_TRANSACTION {
            return Err(TransactionResultError::TooManyOutputNotes {
                max: MAX_OUTPUT_NOTES_PER_TRANSACTION,
                actual: notes.len(),
            });
        }

        let mut seen_notes = BTreeSet::new();
        for note in notes.iter() {
            if !seen_notes.insert(note.hash()) {
                return Err(TransactionResultError::DuplicateOutputNote(note.hash()));
            }
        }

        Ok(Self { notes, commitment: OnceCell::new() })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the commitment to the output notes.
    ///
    /// The commitment is computed as a sequential hash of (hash, metadata) tuples for the notes
    /// created in a transaction.
    pub fn commitment(&self) -> Digest {
        *self.commitment.get_or_init(|| build_input_notes_commitment(&self.notes))
    }
    /// Returns total number of output notes.
    pub fn num_notes(&self) -> usize {
        self.notes.len()
    }

    /// Returns true if this [OutputNotes] does not contain any notes.
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    /// Returns a reference to the [OutputNote] located at the specified index.
    pub fn get_note(&self, idx: usize) -> &OutputNote {
        &self.notes[idx]
    }

    // ITERATORS
    // --------------------------------------------------------------------------------------------

    /// Returns an iterator over notes in this [OutputNote].
    pub fn iter(&self) -> impl Iterator<Item = &OutputNote> {
        self.notes.iter()
    }

    /// Returns an iterator over envelopes of all notes in this [OutputNotes].
    pub fn envelopes(&self) -> impl Iterator<Item = NoteEnvelope> + '_ {
        self.notes.iter().map(|note| note.envelope)
    }
}

impl IntoIterator for OutputNotes {
    type Item = OutputNote;
    type IntoIter = collections::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.notes.into_iter()
    }
}

impl PartialEq for OutputNotes {
    fn eq(&self, other: &Self) -> bool {
        self.notes == other.notes
    }
}

impl Eq for OutputNotes {}

// HELPER FUNCTIONS
// ------------------------------------------------------------------------------------------------

/// Build a commitment to output notes.
///
/// The commitment is computed as a sequential hash of (hash, metadata) tuples for the notes
/// created in a transaction.
fn build_input_notes_commitment(notes: &[OutputNote]) -> Digest {
    let mut elements: Vec<Felt> = Vec::with_capacity(notes.len() * 8);
    for note in notes.iter() {
        elements.extend_from_slice(note.hash().as_elements());
        elements.extend_from_slice(&Word::from(note.metadata()));
    }

    Hasher::hash_elements(&elements)
}

// OUTPUT NOTE
// ================================================================================================

/// An note create during a transaction.
///
/// When a note is produced in a transaction, the note's recipient, vault and metadata must be
/// known. However, other information about the note may or may not be know to the note's producer.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct OutputNote {
    envelope: NoteEnvelope,
    recipient: Digest,
    vault: NoteVault,
}

impl OutputNote {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new [OutputNote] instantiated from the provided parameters.
    pub fn new(recipient: Digest, vault: NoteVault, metadata: NoteMetadata) -> Self {
        // assert is OK here because we'll eventually remove `num_assets` from the metadata
        assert_eq!(vault.num_assets() as u64, metadata.num_assets().as_int());

        let hash = Hasher::merge(&[recipient, vault.hash()]);
        Self {
            envelope: NoteEnvelope::new(hash, metadata),
            recipient,
            vault,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the recipient of the note.
    pub fn recipient(&self) -> &Digest {
        &self.recipient
    }

    /// Returns a reference to the asset vault of this note.
    pub fn vault(&self) -> &NoteVault {
        &self.vault
    }

    /// Returns the metadata associated with this note.
    pub fn metadata(&self) -> &NoteMetadata {
        self.envelope.metadata()
    }

    /// Returns the hash of this note stub.
    pub fn hash(&self) -> Digest {
        self.envelope.note_hash()
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
        Self::new(recipient, note.vault().clone(), *note.metadata())
    }
}
