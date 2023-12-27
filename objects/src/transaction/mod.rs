use super::{
    accounts::{Account, AccountId},
    notes::{NoteEnvelope, Nullifier},
    utils::collections::Vec,
    vm::{AdviceInputs, Program, StackInputs},
    BlockHeader, Digest, Felt, Hasher, StarkField, TransactionWitnessError, Word, WORD_SIZE, ZERO,
};

mod chain_mmr;
mod event;
mod executed_tx;
mod inputs;
mod outputs;
mod prepared_tx;
mod proven_tx;
mod transaction_id;
mod tx_script;
mod tx_witness;

pub use chain_mmr::ChainMmr;
pub use event::Event;
pub use executed_tx::ExecutedTransaction;
pub use inputs::{InputNote, InputNotes, TransactionInputs};
pub use outputs::{OutputNote, OutputNotes, TransactionOutputs};
pub use prepared_tx::PreparedTransaction;
pub use proven_tx::ProvenTransaction;
pub use transaction_id::TransactionId;
pub use tx_script::TransactionScript;
pub use tx_witness::TransactionWitness;

// CONSTANTS
// ================================================================================================

/// Maximum number of notes consumed in a single transaction.
const MAX_INPUT_NOTES_PER_TRANSACTION: usize = 1024;

/// Maximum number of notes created in a single transaction.
const MAX_OUTPUT_NOTES_PER_TRANSACTION: usize = 1024;
