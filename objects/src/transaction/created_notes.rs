use super::{
    utils::generate_created_notes_stub_commitment, AdviceProvider, Digest, Felt, NoteMetadata,
    NoteStub, StackOutputs, StarkField, TransactionResultError, TryFromVmResult, Vec, Word,
    WORD_SIZE,
};
use miden_lib::memory::NOTE_MEM_SIZE;

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

impl<T: AdviceProvider> TryFromVmResult<T> for CreatedNotes {
    type Error = TransactionResultError;

    fn try_from_vm_result(
        stack_outputs: &StackOutputs,
        advice_provider: &T,
    ) -> Result<Self, Self::Error> {
        const CREATED_NOTES_COMMITMENT_WORD_IDX: usize = 0;

        let created_notes_commitment: Word =
            stack_outputs.stack()[CREATED_NOTES_COMMITMENT_WORD_IDX * WORD_SIZE
                ..(CREATED_NOTES_COMMITMENT_WORD_IDX + 1) * WORD_SIZE]
                .iter()
                .rev()
                .map(|x| Felt::from(*x))
                .collect::<Vec<_>>()
                .try_into()
                .expect("word size is correct");
        let created_notes_commitment: Digest = created_notes_commitment.into();

        let created_notes_data = advice_provider
            .get_mapped_values(&created_notes_commitment.as_bytes())
            .ok_or(TransactionResultError::CreatedNoteDataNotFound)?;

        let mut created_notes = Vec::new();
        let mut created_note_ptr = 0;
        while created_note_ptr < created_notes_data.len() {
            let note_stub: NoteStub = created_notes_data[created_note_ptr..]
                .try_into()
                .map_err(TransactionResultError::CreatedNoteDataInvalid)?;
            created_notes.push(note_stub);
            created_note_ptr += (NOTE_MEM_SIZE as usize) * WORD_SIZE;
        }

        let created_notes = Self::new(created_notes);
        if created_notes_commitment != created_notes.commitment() {
            return Err(TransactionResultError::CreatedNotesCommitmentInconsistent(
                created_notes_commitment,
                created_notes.commitment(),
            ));
        }

        Ok(created_notes)
    }
}

// CREATED NOTE INFO
// ================================================================================================

/// Holds information about a note that was created by a transaction.
/// Contains:
/// - note_hash: hash of the note that was created
/// - note_metadata: metadata of the note that was created. Metadata is padded with ZERO such that
///   it is four elements in size (a word). The metadata includes the following elements:
///     - sender
///     - tag
///     - ZERO
///     - ZERO
pub struct CreatedNoteInfo {
    note_hash: Digest,
    note_metadata: NoteMetadata,
}

impl CreatedNoteInfo {
    /// Creates a new CreatedNoteInfo object.
    pub fn new(note_hash: Digest, note_metadata: NoteMetadata) -> Self {
        Self {
            note_hash,
            note_metadata,
        }
    }

    /// Returns the hash of the note that was created.
    pub fn note_hash(&self) -> Digest {
        self.note_hash
    }

    /// Returns the metadata of the note that was created.
    pub fn metadata(&self) -> &NoteMetadata {
        &self.note_metadata
    }
}

impl From<CreatedNoteInfo> for [Felt; 8] {
    fn from(cni: CreatedNoteInfo) -> Self {
        let mut elements: [Felt; 8] = Default::default();
        elements[..4].copy_from_slice(cni.note_hash.as_elements());
        elements[4..].copy_from_slice(&Word::from(cni.metadata()));
        elements
    }
}

impl From<CreatedNoteInfo> for [Word; 2] {
    fn from(cni: CreatedNoteInfo) -> Self {
        let mut elements: [Word; 2] = Default::default();
        elements[0].copy_from_slice(cni.note_hash.as_elements());
        elements[1].copy_from_slice(&Word::from(cni.metadata()));
        elements
    }
}

impl From<CreatedNoteInfo> for [u8; 64] {
    fn from(cni: CreatedNoteInfo) -> Self {
        let mut elements: [u8; 64] = [0; 64];
        let note_metadata_bytes = Word::from(cni.metadata())
            .iter()
            .flat_map(|x| x.as_int().to_le_bytes())
            .collect::<Vec<u8>>();
        elements[..32].copy_from_slice(&cni.note_hash.as_bytes());
        elements[32..].copy_from_slice(&note_metadata_bytes);
        elements
    }
}

impl From<&NoteStub> for CreatedNoteInfo {
    fn from(note_stub: &NoteStub) -> Self {
        Self::new(*note_stub.hash(), *note_stub.metadata())
    }
}
