use vm_core::{Program, StackInputs, StackOutputs};

use super::{
    accounts::{Account, AccountId},
    notes::{Note, NoteEnvelope, Nullifier},
    utils::collections::Vec,
    AdviceInputs, AdviceInputsBuilder, BlockHeader, Digest, Felt, Hasher, PreparedTransactionError,
    StarkField, ToAdviceInputs, TransactionWitnessError, Word, WORD_SIZE, ZERO,
};

mod account_stub;
mod chain_mmr;
mod created_notes;
mod event;
mod executed_tx;
mod inputs;
mod prepared_tx;
mod proven_tx;
mod script;
mod transaction_id;
mod tx_result;
mod tx_witness;
#[cfg(not(feature = "testing"))]
mod utils;

pub use account_stub::FinalAccountStub;
pub use chain_mmr::ChainMmr;
pub use created_notes::CreatedNotes;
pub use event::Event;
pub use executed_tx::ExecutedTransaction;
pub use inputs::{InputNote, InputNotes, TransactionInputs};
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

const MAX_NOTES_PER_TRANSACTION: usize = 1024;
