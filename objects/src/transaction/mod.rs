use super::{
    notes::{Note, NoteMetadata},
    Account, AccountId, AdviceInputsBuilder, BlockHeader, ChainMmr, Digest, Felt, Hasher,
    StarkField, ToAdviceInputs, TransactionWitnessError, Vec, Word, WORD_SIZE,
};
use miden_core::{Program, StackInputs, StackOutputs};
use miden_processor::AdviceInputs;

mod consumed_notes;
mod created_note;
mod executed_tx;
mod prepared_tx;
mod proven_tx;
mod tx_witness;
mod utils;

pub use consumed_notes::{ConsumedNoteInfo, ConsumedNotes};
pub use created_note::CreatedNoteInfo;
pub use executed_tx::ExecutedTransaction;
pub use prepared_tx::PreparedTransaction;
pub use proven_tx::ProvenTransaction;
pub use tx_witness::TransactionWitness;
