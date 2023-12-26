use vm_core::{Program, StackInputs, StackOutputs};

use super::{
    accounts::{Account, AccountId},
    notes::{Note, NoteEnvelope, Nullifier},
    utils::collections::Vec,
    AdviceInputs, AdviceInputsBuilder, BlockHeader, Digest, Felt, Hasher, PreparedTransactionError,
    StarkField, ToAdviceInputs, TransactionWitnessError, Word, WORD_SIZE, ZERO,
};

mod chain_mmr;
mod event;
mod executed_tx;
mod inputs;
mod outputs;
mod prepared_tx;
mod proven_tx;
mod script;
mod transaction_id;
mod tx_result;
mod tx_witness;
#[cfg(not(feature = "testing"))]
mod utils;

pub use chain_mmr::ChainMmr;
pub use event::Event;
pub use executed_tx::ExecutedTransaction;
pub use inputs::{InputNote, InputNotes, TransactionInputs};
pub use outputs::{OutputNote, OutputNotes, TransactionOutputs};
pub use prepared_tx::PreparedTransaction;
pub use proven_tx::ProvenTransaction;
pub use script::TransactionScript;
pub use transaction_id::TransactionId;
pub use tx_result::TransactionResult;
pub use tx_witness::TransactionWitness;

#[cfg(feature = "testing")]
pub mod utils;

// CONSTANTS
// ================================================================================================

/// Maximum number of notes consumed in a single transaction.
const MAX_INPUT_NOTES_PER_TRANSACTION: usize = 1024;

/// Maximum number of notes created in a single transaction.
const MAX_OUTPUT_NOTES_PER_TRANSACTION: usize = 1024;
