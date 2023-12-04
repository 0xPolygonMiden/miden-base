use super::{
    accounts::{Account, AccountId},
    notes::{Note, NoteEnvelope, RecordedNote},
    utils::collections::Vec,
    AdviceInputs, AdviceInputsBuilder, BlockHeader, ChainMmr, Digest, Felt, Hasher,
    PreparedTransactionError, StarkField, ToAdviceInputs, TransactionWitnessError, Word, WORD_SIZE,
    ZERO,
};
use vm_core::{Program, StackInputs, StackOutputs};

mod account_stub;
mod consumed_notes;
mod created_notes;
mod event;
mod executed_tx;
mod prepared_tx;
mod proven_tx;
mod script;
mod tx_result;
mod tx_witness;
#[cfg(not(feature = "testing"))]
mod utils;

pub use account_stub::FinalAccountStub;
pub use consumed_notes::{ConsumedNoteInfo, ConsumedNotes};
pub use created_notes::CreatedNotes;
pub use event::Event;
pub use executed_tx::ExecutedTransaction;
pub use prepared_tx::PreparedTransaction;
pub use proven_tx::ProvenTransaction;
pub use script::TransactionScript;
pub use tx_result::TransactionResult;
pub use tx_witness::TransactionWitness;

#[cfg(feature = "testing")]
pub mod utils;
