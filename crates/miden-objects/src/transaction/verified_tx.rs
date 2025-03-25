use crate::{
    block::BlockNumber,
    note::NoteHeader,
    transaction::{
        AccountId, InputNoteCommitment, InputNotes, Nullifier, OutputNotes, TransactionId,
        TxAccountUpdate,
    },
    utils::serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
};

// VERIFIED TRANSACTION
// ================================================================================================

/// A verified transaction.
///
/// It is the result of verifying the ZK proof of a [`ProvenTransaction`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedTransaction {
    /// A unique identifier for the transaction, see [`TransactionId`] for additional details.
    id: TransactionId,

    /// Account update data.
    account_update: TxAccountUpdate,

    /// Committed details of all notes consumed by the transaction.
    input_notes: InputNotes<InputNoteCommitment>,

    /// Notes created by the transaction.
    ///
    /// For private notes, this will contain only note headers, while for public notes this will
    /// also contain full note details.
    output_notes: OutputNotes,

    /// [`BlockNumber`] of the transaction's reference block.
    ref_block_num: BlockNumber,
}

impl VerifiedTransaction {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Constructs a new [`VerifiedTransaction`] from the provided parameteres.
    pub fn new_unchecked(
        id: TransactionId,
        account_update: TxAccountUpdate,
        input_notes: InputNotes<InputNoteCommitment>,
        output_notes: OutputNotes,
        ref_block_num: BlockNumber,
    ) -> Self {
        Self {
            id,
            account_update,
            input_notes,
            output_notes,
            ref_block_num,
        }
    }

    /// Returns unique identifier of this transaction.
    pub fn id(&self) -> TransactionId {
        self.id
    }

    /// Returns ID of the account against which this transaction was executed.
    pub fn account_id(&self) -> AccountId {
        self.account_update.account_id()
    }

    /// Returns the account update details.
    pub fn account_update(&self) -> &TxAccountUpdate {
        &self.account_update
    }

    /// Returns a reference to the notes consumed by the transaction.
    pub fn input_notes(&self) -> &InputNotes<InputNoteCommitment> {
        &self.input_notes
    }

    /// Returns a reference to the notes produced by the transaction.
    pub fn output_notes(&self) -> &OutputNotes {
        &self.output_notes
    }

    /// Returns the number of the reference block the transaction was executed against.
    pub fn ref_block_num(&self) -> BlockNumber {
        self.ref_block_num
    }

    /// Returns an iterator of the headers of unauthenticated input notes in this transaction.
    pub fn unauthenticated_notes(&self) -> impl Iterator<Item = &NoteHeader> {
        self.input_notes.iter().filter_map(|note| note.header())
    }

    /// Returns an iterator over the nullifiers of all input notes in this transaction.
    ///
    /// This includes both authenticated and unauthenticated notes.
    pub fn nullifiers(&self) -> impl Iterator<Item = Nullifier> + '_ {
        self.input_notes.iter().map(InputNoteCommitment::nullifier)
    }
}

impl Serializable for VerifiedTransaction {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.account_update.write_into(target);
        self.input_notes.write_into(target);
        self.output_notes.write_into(target);
        self.ref_block_num.write_into(target);
    }
}

impl Deserializable for VerifiedTransaction {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let account_update = TxAccountUpdate::read_from(source)?;

        let input_notes = <InputNotes<InputNoteCommitment>>::read_from(source)?;
        let output_notes = OutputNotes::read_from(source)?;

        let ref_block_num = BlockNumber::read_from(source)?;

        let id = TransactionId::new(
            account_update.initial_state_commitment(),
            account_update.final_state_commitment(),
            input_notes.commitment(),
            output_notes.commitment(),
        );

        Ok(Self::new_unchecked(
            id,
            account_update,
            input_notes,
            output_notes,
            ref_block_num,
        ))
    }
}
