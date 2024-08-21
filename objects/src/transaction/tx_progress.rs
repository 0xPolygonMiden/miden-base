pub use alloc::vec::Vec;
use serde::Serialize;

use super::NoteId;

// TRANSACTION PROGRESS
// ================================================================================================

/// Contains the information about the number of cycles for each of the transaction execution
/// stages.
#[derive(Clone, Default, Debug)]
pub struct TransactionProgress {
    prologue: CycleInterval,
    notes_processing: CycleInterval,
    note_execution: Vec<(NoteId, CycleInterval)>,
    tx_script_processing: CycleInterval,
    epilogue: CycleInterval,
}

impl TransactionProgress {
    // STATE ACCESSORS
    // --------------------------------------------------------------------------------------------

    pub fn prologue(&self) -> &CycleInterval {
        &self.prologue
    }

    pub fn notes_processing(&self) -> &CycleInterval {
        &self.notes_processing
    }

    pub fn note_execution(&self) -> &Vec<(NoteId, CycleInterval)> {
        &self.note_execution
    }

    pub fn tx_script_processing(&self) -> &CycleInterval {
        &self.tx_script_processing
    }

    pub fn epilogue(&self) -> &CycleInterval {
        &self.epilogue
    }

    // STATE MUTATORS
    // --------------------------------------------------------------------------------------------

    pub fn start_prologue(&mut self, cycle: usize) {
        self.prologue.set_start(cycle);
    }

    pub fn end_prologue(&mut self, cycle: usize) {
        self.prologue.set_end(cycle);
    }

    pub fn start_notes_processing(&mut self, cycle: usize) {
        self.notes_processing.set_start(cycle);
    }

    pub fn end_notes_processing(&mut self, cycle: usize) {
        self.notes_processing.set_end(cycle);
    }

    pub fn start_note_execution(&mut self, cycle: usize, note_id: NoteId) {
        self.note_execution.push((note_id, CycleInterval::new(cycle)));
    }

    pub fn end_note_execution(&mut self, cycle: usize) {
        if let Some((_, interval)) = self.note_execution.last_mut() {
            interval.set_end(cycle)
        }
    }

    pub fn start_tx_script_processing(&mut self, cycle: usize) {
        self.tx_script_processing.set_start(cycle);
    }

    pub fn end_tx_script_processing(&mut self, cycle: usize) {
        self.tx_script_processing.set_end(cycle);
    }

    pub fn start_epilogue(&mut self, cycle: usize) {
        self.epilogue.set_start(cycle);
    }

    pub fn end_epilogue(&mut self, cycle: usize) {
        self.epilogue.set_end(cycle);
    }
}

/// Stores the cycles corresponding to the start and the end of an interval.
#[derive(Clone, Default, Debug)]
pub struct CycleInterval {
    start: Option<usize>,
    end: Option<usize>,
}

impl CycleInterval {
    pub fn new(start: usize) -> Self {
        Self { start: Some(start), end: None }
    }

    pub fn set_start(&mut self, s: usize) {
        self.start = Some(s);
    }

    pub fn set_end(&mut self, e: usize) {
        self.end = Some(e);
    }

    /// Calculate the length of the interval
    pub fn len(&self) -> usize {
        if let Some(start) = self.start {
            if let Some(end) = self.end {
                if end >= start {
                    return end - start;
                }
            }
        }
        0
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TransactionMeasurements {
    pub prologue: usize,
    pub notes_processing: usize,
    pub note_execution: Vec<(NoteId, usize)>,
    pub tx_script_processing: usize,
    pub epilogue: usize,
}

impl From<TransactionProgress> for TransactionMeasurements {
    fn from(tx_progress: TransactionProgress) -> Self {
        let prologue = tx_progress.prologue().len();

        let notes_processing = tx_progress.notes_processing().len();

        let mut note_execution = Vec::new();
        tx_progress.note_execution().iter().for_each(|(note_id, interval)| {
            note_execution.push((*note_id, interval.len()));
        });

        let tx_script_processing = tx_progress.tx_script_processing().len();

        let epilogue = tx_progress.epilogue().len();

        Self {
            prologue,
            notes_processing,
            note_execution,
            tx_script_processing,
            epilogue,
        }
    }
}
