pub use alloc::{
    string::{String, ToString},
    vec::Vec,
};

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

    pub fn start_note_execution(&mut self, cycle: u32, note_id: Option<NoteId>) {
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

    // DATA PRINT
    // --------------------------------------------------------------------------------------------

    pub fn to_json_string(&self) -> String {
        let mut json_string = String::new();
        json_string.push('{');

        // push lenght of the prologue cycle interval
        json_string.push_str(&format!(
            "\"prologue\": {}, ",
            self.prologue
                .len()
                .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                .unwrap_or("invalid interval".to_string())
        ));

        // push lenght of the notes processing cycle interval
        json_string.push_str(&format!(
            "\"notes_processing\": {}, ",
            self.notes_processing
                .len()
                .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                .unwrap_or("invalid interval".to_string())
        ));

        // prepare string with executed notes
        let mut notes = String::new();
        self.note_execution.iter().fold(true, |first, (note_id, interval)| {
            if !first {
                notes.push_str(", ");
            }
            notes.push_str(&format!(
                "{{{}: {}}}",
                note_id
                    .map(|id| format!("\"{}\"", id.to_hex()))
                    .unwrap_or("id_unavailable".to_string()),
                interval
                    .len()
                    .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                    .unwrap_or("invalid interval".to_string())
            ));
            false
        });

        // push lenghts of the note execution cycle intervals
        json_string.push_str(&format!("\"note_execution\": [{}], ", notes));

        // push lenght of the transaction script processing cycle interval
        json_string.push_str(&format!(
            "\"tx_script_processing\": {},",
            self.tx_script_processing
                .len()
                .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                .unwrap_or("invalid interval".to_string())
        ));

        // push lenght of the epilogue cycle interval
        json_string.push_str(&format!(
            "\"epilogue\": {}",
            self.epilogue
                .len()
                .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                .unwrap_or("invalid interval".to_string())
        ));

        json_string.push('}');

        json_string
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
