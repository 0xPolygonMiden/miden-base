pub use alloc::{string::ToString, vec::Vec};

#[cfg(feature = "std")]
use std::println;

use miden_objects::notes::NoteId;

// CONSTANTS
// ================================================================================================

/// Number of cycles needed to create an empty span whithout changing the stack state.
const SPAN_CREATION_SHIFT: u32 = 2;

// TRANSACTION PROGRESS
// ================================================================================================

/// Contains the information about the number of cycles for each of the transaction execution
/// stages.
#[derive(Default)]
pub struct TransactionProgress {
    prologue: CycleInterval,
    notes_processing: CycleInterval,
    note_execution: Vec<(Option<NoteId>, CycleInterval)>,
    tx_script_processing: CycleInterval,
    epilogue: CycleInterval,
}

impl TransactionProgress {
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

    pub fn start_note_execution(&mut self, cycle: u32) {
        self.note_execution.push((None, CycleInterval::new(cycle)));
    }

    pub fn end_note_execution(&mut self, cycle: u32, note_id: Option<NoteId>) {
        if let Some((id, interval)) = self.note_execution.last_mut() {
            *id = note_id;
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

    // DATA PRINT
    // --------------------------------------------------------------------------------------------

    /// Prints out the lengths of cycle intervals for each execution stage.
    #[cfg(feature = "std")]
    pub fn print_stages(&self) {
        println!(
            "Number of cycles it takes to execule:\n- Prologue: {},\n- Notes processing: {},",
            self.prologue
                .get_interval_len()
                .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                .unwrap_or("invalid interval".to_string()),
            self.notes_processing
                .get_interval_len()
                .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                .unwrap_or("invalid interval".to_string())
        );

        for (note_id, interval) in self.note_execution.iter() {
            println!(
                "--- Note {}: {}",
                note_id.map(|id| id.to_hex()).unwrap_or("id_unavailable".to_string()),
                interval
                    .get_interval_len()
                    .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                    .unwrap_or("invalid interval".to_string())
            )
        }

        println!(
            "- Transaction script processing: {},\n- Epilogue: {}",
            self.tx_script_processing
                .get_interval_len()
                .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                .unwrap_or("invalid interval".to_string()),
            self.epilogue
                .get_interval_len()
                .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                .unwrap_or("invalid interval".to_string())
        );
    }
}

/// Stores the cycles corresponding to the start and the end of an interval.
#[derive(Default)]
struct CycleInterval {
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
    pub fn get_interval_len(&self) -> Option<u32> {
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
