use miden_lib::transaction::memory;

// TEST HELPERS
// ================================================================================================

pub fn consumed_note_data_ptr(note_idx: u32) -> memory::MemoryAddress {
    memory::CONSUMED_NOTE_DATA_SECTION_OFFSET + note_idx * memory::NOTE_MEM_SIZE
}
