use super::{notes::Note, Account, AccountId, Digest, Felt, Hasher, StarkField, Vec, Word};
use miden_core::{StackInputs, StackOutputs};
use miden_processor::AdviceInputs;

mod compiled_tx;
mod consumed_note;
mod created_note;
mod executed_tx;
mod inputs;
mod proven_tx;
pub(self) mod utils;

pub use compiled_tx::CompiledTransaction;
pub use consumed_note::ConsumedNoteInfo;
pub use created_note::CreatedNoteInfo;
pub use executed_tx::ExecutedTransaction;
pub use inputs::TransactionInputs;
pub use proven_tx::ProvenTransaction;
