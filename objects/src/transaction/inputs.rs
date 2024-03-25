use alloc::{collections::BTreeSet, string::ToString, vec::Vec};
use core::fmt::Debug;

use super::{BlockHeader, ChainMmr, Digest, Felt, Hasher, Word};
use crate::{
    accounts::{validate_account_seed, Account},
    notes::{Note, NoteId, NoteInclusionProof, NoteOrigin, Nullifier},
    utils::serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    TransactionInputError, MAX_INPUT_NOTES_PER_TX,
};

// TRANSACTION INPUTS
// ================================================================================================

/// Contains the data required to execute a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionInputs {
    account: Account,
    account_seed: Option<Word>,
    block_header: BlockHeader,
    block_chain: ChainMmr,
    input_notes: InputNotes,
}

impl TransactionInputs {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns new [TransactionInputs] instantiated with the specified parameters.
    ///
    /// # Errors
    /// Returns an error if:
    /// - For a new account, account seed is not provided or the provided seed is invalid.
    /// - For an existing account, account seed was provided.
    pub fn new(
        account: Account,
        account_seed: Option<Word>,
        block_header: BlockHeader,
        block_chain: ChainMmr,
        input_notes: InputNotes,
    ) -> Result<Self, TransactionInputError> {
        match (account.is_new(), account_seed) {
            (true, Some(seed)) => validate_account_seed(&account, seed)
                .map_err(TransactionInputError::InvalidAccountSeed),
            (true, None) => Err(TransactionInputError::AccountSeedNotProvidedForNewAccount),
            (false, Some(_)) => Err(TransactionInputError::AccountSeedProvidedForExistingAccount),
            (false, None) => Ok(()),
        }?;

        // make sure block_chain and block_header are consistent

        let block_num = block_header.block_num();
        if block_chain.chain_length() != block_header.block_num() as usize {
            return Err(TransactionInputError::InconsistentChainLength {
                expected: block_header.block_num(),
                actual: block_chain.chain_length() as u32,
            });
        }

        if block_chain.peaks().hash_peaks() != block_header.chain_root() {
            return Err(TransactionInputError::InconsistentChainRoot {
                expected: block_header.chain_root(),
                actual: block_chain.peaks().hash_peaks(),
            });
        }

        // make sure that block_chain has authentication paths for all input notes; for input notes
        // which were created in the current block we skip this check because their authentication
        // paths are derived implicitly
        for note in input_notes.iter() {
            let note_block_num = note.origin().block_num;

            let block_header = if note_block_num == block_num {
                &block_header
            } else {
                match block_chain.get_block(note_block_num) {
                    Some(block_header) => block_header,
                    None => Err(TransactionInputError::InputNoteBlockNotInChainMmr(note.id()))?,
                }
            };

            // this check may have non-negligible performance impact as we need to verify inclusion
            // proofs for all notes; TODO: consider enabling this via a feature flag
            if !note.is_in_block(block_header) {
                return Err(TransactionInputError::InputNoteNotInBlock(note.id(), note_block_num));
            }
        }

        Ok(Self {
            account,
            account_seed,
            block_header,
            block_chain,
            input_notes,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns account against which the transaction is to be executed.
    pub fn account(&self) -> &Account {
        &self.account
    }

    /// For newly-created accounts, returns the account seed; for existing accounts, returns None.
    pub fn account_seed(&self) -> Option<Word> {
        self.account_seed
    }

    /// Returns block header for the block referenced by the transaction.
    pub fn block_header(&self) -> &BlockHeader {
        &self.block_header
    }

    /// Returns chain MMR containing authentication paths for all notes consumed by the
    /// transaction.
    pub fn block_chain(&self) -> &ChainMmr {
        &self.block_chain
    }

    /// Returns the notes to be consumed in the transaction.
    pub fn input_notes(&self) -> &InputNotes {
        &self.input_notes
    }

    // CONVERSIONS
    // --------------------------------------------------------------------------------------------

    /// Consumes these transaction inputs and returns their underlying components.
    pub fn into_parts(self) -> (Account, Option<Word>, BlockHeader, ChainMmr, InputNotes) {
        (
            self.account,
            self.account_seed,
            self.block_header,
            self.block_chain,
            self.input_notes,
        )
    }
}

// TO NULLIFIER TRAIT
// ================================================================================================

/// Defines how a note object can be reduced to a nullifier.
///
/// This trait is implemented on both [InputNote] and [Nullifier] so that we can treat them
/// generically as [InputNotes].
pub trait ToNullifier:
    Debug + Clone + PartialEq + Eq + Serializable + Deserializable + Sized
{
    fn nullifier(&self) -> Nullifier;
}

impl ToNullifier for InputNote {
    fn nullifier(&self) -> Nullifier {
        self.note.nullifier()
    }
}

impl ToNullifier for Nullifier {
    fn nullifier(&self) -> Nullifier {
        *self
    }
}

impl From<InputNotes> for InputNotes<Nullifier> {
    fn from(value: InputNotes) -> Self {
        Self {
            notes: value.notes.iter().map(|note| note.nullifier()).collect(),
            commitment: build_input_notes_commitment(&value.notes),
        }
    }
}

impl From<&InputNotes> for InputNotes<Nullifier> {
    fn from(value: &InputNotes) -> Self {
        Self {
            notes: value.notes.iter().map(|note| note.nullifier()).collect(),
            commitment: build_input_notes_commitment(&value.notes),
        }
    }
}

// INPUT NOTES
// ================================================================================================

/// Contains a list of input notes for a transaction. The list can be empty if the transaction does
/// not consume any notes.
///
/// For the purposes of this struct, anything that can be reduced to a [Nullifier] can be an input
/// note. However, [ToNullifier] trait is currently implemented only for [InputNote] and [Nullifier],
/// and so these are the only two allowed input note types.
#[derive(Debug, Clone)]
pub struct InputNotes<T: ToNullifier = InputNote> {
    notes: Vec<T>,
    commitment: Digest,
}

impl<T: ToNullifier> InputNotes<T> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns new [InputNotes] instantiated from the provided vector of notes.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The total number of notes is greater than 1024.
    /// - The vector of notes contains duplicates.
    pub fn new(notes: Vec<T>) -> Result<Self, TransactionInputError> {
        if notes.len() > MAX_INPUT_NOTES_PER_TX {
            return Err(TransactionInputError::TooManyInputNotes {
                max: MAX_INPUT_NOTES_PER_TX,
                actual: notes.len(),
            });
        }

        let mut seen_notes = BTreeSet::new();
        for note in notes.iter() {
            if !seen_notes.insert(note.nullifier().inner()) {
                return Err(TransactionInputError::DuplicateInputNote(note.nullifier().inner()));
            }
        }

        let commitment = build_input_notes_commitment(&notes);

        Ok(Self { notes, commitment })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to these input notes.
    pub fn commitment(&self) -> Digest {
        self.commitment
    }

    /// Returns total number of input notes.
    pub fn num_notes(&self) -> usize {
        self.notes.len()
    }

    /// Returns true if this [InputNotes] does not contain any notes.
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    /// Returns a reference to the note located at the specified index.
    pub fn get_note(&self, idx: usize) -> &T {
        &self.notes[idx]
    }

    // ITERATORS
    // --------------------------------------------------------------------------------------------

    /// Returns an iterator over notes in this [InputNotes].
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.notes.iter()
    }

    // CONVERSIONS
    // --------------------------------------------------------------------------------------------

    /// Converts self into a vector of input notes.
    pub fn into_vec(self) -> Vec<T> {
        self.notes
    }
}

impl<T: ToNullifier> IntoIterator for InputNotes<T> {
    type Item = T;
    type IntoIter = alloc::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.notes.into_iter()
    }
}

impl<T: ToNullifier> PartialEq for InputNotes<T> {
    fn eq(&self, other: &Self) -> bool {
        self.notes == other.notes
    }
}

impl<T: ToNullifier> Eq for InputNotes<T> {}

impl<T: ToNullifier> Default for InputNotes<T> {
    fn default() -> Self {
        Self {
            notes: Vec::new(),
            commitment: build_input_notes_commitment::<T>(&[]),
        }
    }
}

// SERIALIZATION
// ------------------------------------------------------------------------------------------------

impl<T: ToNullifier> Serializable for InputNotes<T> {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        // assert is OK here because we enforce max number of notes in the constructor
        assert!(self.notes.len() <= u16::MAX.into());
        target.write_u16(self.notes.len() as u16);
        target.write_many(&self.notes);
    }
}

impl<T: ToNullifier> Deserializable for InputNotes<T> {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let num_notes = source.read_u16()?;
        let notes = source.read_many::<T>(num_notes.into())?;
        Self::new(notes).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// HELPER FUNCTIONS
// ------------------------------------------------------------------------------------------------

/// Returns the commitment to the input notes represented by the specified nullifiers.
///
/// For a non-empty list of notes, this is a sequential hash of all (nullifier, ZERO) pairs for
/// the notes consumed in the transaction. For an empty list, [ZERO; 4] is returned.
pub fn build_input_notes_commitment<T: ToNullifier>(notes: &[T]) -> Digest {
    if notes.is_empty() {
        return Digest::default();
    }

    let mut elements: Vec<Felt> = Vec::new();
    for note in notes {
        elements.extend_from_slice(note.nullifier().as_elements());
        elements.extend_from_slice(&Word::default());
    }
    Hasher::hash_elements(&elements)
}

// INPUT NOTE
// ================================================================================================

/// An input note for a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct InputNote {
    note: Note,
    proof: NoteInclusionProof,
}

impl InputNote {
    /// Returns a new instance of an [InputNote] with the specified note and proof.
    pub fn new(note: Note, proof: NoteInclusionProof) -> Self {
        Self { note, proof }
    }

    /// Returns the ID of the note.
    pub fn id(&self) -> NoteId {
        self.note.id()
    }

    /// Returns a reference to the underlying note.
    pub fn note(&self) -> &Note {
        &self.note
    }

    /// Returns a reference to the inclusion proof of the note.
    pub fn proof(&self) -> &NoteInclusionProof {
        &self.proof
    }

    /// Returns a reference to the origin of the note.
    pub fn origin(&self) -> &NoteOrigin {
        self.proof.origin()
    }

    /// Returns true if this note belongs to the note tree of the specified block.
    fn is_in_block(&self, block_header: &BlockHeader) -> bool {
        let note_index = self.origin().node_index.value();
        let note_hash = self.note.authentication_hash();
        self.proof.note_path().verify(note_index, note_hash, &block_header.note_root())
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
