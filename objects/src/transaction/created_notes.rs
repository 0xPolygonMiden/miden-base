use super::{
    BTreeMap, Digest, Felt, Hasher, MerkleStore, NoteStub, StackOutputs, TransactionResultError,
    TryFromVmResult, Vec, Word, WORD_SIZE,
};
use miden_core::utils::group_slice_elements;
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

impl TryFromVmResult for CreatedNotes {
    type Error = TransactionResultError;

    fn try_from_vm_result(
        stack_outputs: &StackOutputs,
        _advice_stack: &[Felt],
        advice_map: &BTreeMap<[u8; 32], Vec<Felt>>,
        _merkle_store: &MerkleStore,
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

        let created_notes_data = group_slice_elements::<Felt, WORD_SIZE>(
            advice_map
                .get(&created_notes_commitment.as_bytes())
                .ok_or(TransactionResultError::CreatedNoteDataNotFound)?,
        );

        let mut created_notes = Vec::new();
        let mut created_note_ptr = 0;
        while created_note_ptr < created_notes_data.len() {
            let note_stub: NoteStub = created_notes_data[created_note_ptr..]
                .try_into()
                .map_err(TransactionResultError::CreatedNoteDataInvalid)?;
            created_notes.push(note_stub);
            created_note_ptr += NOTE_MEM_SIZE as usize;
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
