pub use alloc::vec::Vec;

use super::NoteId;

/// Stores the resulting number of cycles for each transaction execution stage obtained from the
/// `TransactionProgress` struct.
#[derive(Debug, Clone)]
pub struct TransactionMeasurements {
    pub prologue: usize,
    pub notes_processing: usize,
    pub note_execution: Vec<(NoteId, usize)>,
    pub tx_script_processing: usize,
    pub epilogue: usize,
}
