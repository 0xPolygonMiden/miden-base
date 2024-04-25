pub use alloc::vec::Vec;

use miden_objects::notes::NoteId;

// TRANSACTION PROGRESS
// ================================================================================================

/// Contains the information about the number of cycles for each of the transaction execution
/// stages.
#[derive(Clone, Default)]
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

    pub fn start_prologue(&mut self, cycle: u32) {
        self.prologue.set_start(cycle);
    }

    pub fn end_prologue(&mut self, cycle: u32) {
        self.prologue.set_end(cycle);
    }

    pub fn start_notes_processing(&mut self, cycle: u32) {
        self.notes_processing.set_start(cycle);
    }

    pub fn end_notes_processing(&mut self, cycle: u32) {
        self.notes_processing.set_end(cycle);
    }

    pub fn start_note_execution(&mut self, cycle: u32, note_id: NoteId) {
        self.note_execution.push((note_id, CycleInterval::new(cycle)));
    }

    pub fn end_note_execution(&mut self, cycle: u32) {
        if let Some((_, interval)) = self.note_execution.last_mut() {
            interval.set_end(cycle)
        }
    }

    pub fn start_tx_script_processing(&mut self, cycle: u32) {
        self.tx_script_processing.set_start(cycle);
    }

    pub fn end_tx_script_processing(&mut self, cycle: u32) {
        self.tx_script_processing.set_end(cycle);
    }

    pub fn start_epilogue(&mut self, cycle: u32) {
        self.epilogue.set_start(cycle);
    }

    pub fn end_epilogue(&mut self, cycle: u32) {
        self.epilogue.set_end(cycle);
    }
}

/// Stores the cycles corresponding to the start and the end of an interval.
#[derive(Clone, Default)]
pub struct CycleInterval {
    start: Option<u32>,
    end: Option<u32>,
}

impl CycleInterval {
    pub fn new(start: u32) -> Self {
        Self { start: Some(start), end: None }
    }

    pub fn set_start(&mut self, s: u32) {
        self.start = Some(s);
    }

    pub fn set_end(&mut self, e: u32) {
        self.end = Some(e);
    }

    /// Calculate the length of the interval
    pub fn len(&self) -> Option<u32> {
        if let Some(start) = self.start {
            if let Some(end) = self.end {
                if end >= start {
                    return Some(end - start);
                }
            }
        }
        None
    }
}
