use alloc::string::String;

use miden_lib::transaction::memory::{
    CREATED_NOTE_ASSETS_OFFSET, CREATED_NOTE_METADATA_OFFSET, CREATED_NOTE_NUM_ASSETS_OFFSET,
    CREATED_NOTE_RECIPIENT_OFFSET, CREATED_NOTE_SECTION_OFFSET, NOTE_MEM_SIZE,
    NUM_CREATED_NOTES_PTR,
};
use miden_objects::{
    notes::Note,
    testing::{prepare_word, storage::prepare_assets},
    vm::StackInputs,
    Felt, Hasher, Word, ONE, ZERO,
};
use vm_processor::{ContextId, Host, MemAdviceProvider, Process, ProcessState};

mod test_account;
mod test_asset;
mod test_asset_vault;
mod test_epilogue;
mod test_faucet;
mod test_note;
mod test_prologue;
mod test_tx;

// HELPER FUNCTIONS
// ================================================================================================

pub fn read_root_mem_value<H: Host>(process: &Process<H>, addr: u32) -> Word {
    process.get_mem_value(ContextId::root(), addr).unwrap()
}

/// Given an slice of [Note]s, generates a procudere to initialize the VM's memory with the note's
/// data.
pub fn output_notes_data_procedure(notes: &[Note]) -> String {
    let mut code = String::new();

    code.push_str(
        "
        proc.create_mock_notes
            # remove padding from prologue
            dropw dropw dropw dropw
        ",
    );

    for (i, note) in (0u32..).zip(notes) {
        let metadata = prepare_word(&note.metadata().into());
        let recipient = prepare_word(&note.recipient().digest());
        let assets = prepare_assets(note.assets());
        let num_assets = note.assets().num_assets();
        let note_offset = NOTE_MEM_SIZE * i;

        // TODO: loop over the assets to initialize the memory
        assert_eq!(num_assets, 1, "Code currently only handles a single asset");

        code.push_str(&format!(
            "
                # populate note {i}
                push.{metadata}
                push.{CREATED_NOTE_SECTION_OFFSET}.{CREATED_NOTE_METADATA_OFFSET}.{note_offset} add add mem_storew dropw

                push.{recipient}
                push.{CREATED_NOTE_SECTION_OFFSET}.{CREATED_NOTE_RECIPIENT_OFFSET}.{note_offset} add add mem_storew dropw

                push.{num_assets}
                push.{CREATED_NOTE_SECTION_OFFSET}.{CREATED_NOTE_NUM_ASSETS_OFFSET}.{note_offset} add add mem_store

                push.{}
                push.{CREATED_NOTE_SECTION_OFFSET}.{CREATED_NOTE_ASSETS_OFFSET}.{note_offset} add add mem_storew dropw
            ",
            assets[0],
        ));
    }

    code.push_str(&format!(
        "
            # set num created notes
            push.{num_notes}.{NUM_CREATED_NOTES_PTR} mem_store
        end
        ",
        num_notes = notes.len(),
    ));

    code
}
