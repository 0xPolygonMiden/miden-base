use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use super::{
    memory::{
        CREATED_NOTE_ASSETS_OFFSET, CREATED_NOTE_METADATA_OFFSET, CREATED_NOTE_NUM_ASSETS_OFFSET,
        CREATED_NOTE_RECIPIENT_OFFSET, CREATED_NOTE_SECTION_OFFSET, NOTE_MEM_SIZE,
        NUM_CREATED_NOTES_PTR,
    },
    NoteAssets, OutputNotes, Word,
};

pub fn output_notes_data_procedure(notes: &OutputNotes) -> String {
    let note_0_metadata = prepare_word(&notes.get_note(0).metadata().into());
    let note_0_recipient = prepare_word(notes.get_note(0).recipient());
    let note_0_assets = prepare_assets(notes.get_note(0).assets());
    let note_0_num_assets = 1;

    let note_1_metadata = prepare_word(&notes.get_note(1).metadata().into());
    let note_1_recipient = prepare_word(notes.get_note(1).recipient());
    let note_1_assets = prepare_assets(notes.get_note(1).assets());
    let note_1_num_assets = 1;

    let note_2_metadata = prepare_word(&notes.get_note(2).metadata().into());
    let note_2_recipient = prepare_word(notes.get_note(2).recipient());
    let note_2_assets = prepare_assets(notes.get_note(2).assets());
    let note_2_num_assets = 1;

    const NOTE_1_OFFSET: u32 = NOTE_MEM_SIZE;
    const NOTE_2_OFFSET: u32 = NOTE_MEM_SIZE * 2;

    format!(
        "
    proc.create_mock_notes
        # remove padding from prologue
        dropw dropw dropw dropw

        # populate note 0
        push.{note_0_metadata}
        push.{CREATED_NOTE_SECTION_OFFSET}.{CREATED_NOTE_METADATA_OFFSET} add mem_storew dropw

        push.{note_0_recipient}
        push.{CREATED_NOTE_SECTION_OFFSET}.{CREATED_NOTE_RECIPIENT_OFFSET} add mem_storew dropw

        push.{note_0_num_assets}
        push.{CREATED_NOTE_SECTION_OFFSET}.{CREATED_NOTE_NUM_ASSETS_OFFSET} add mem_store

        push.{}
        push.{CREATED_NOTE_SECTION_OFFSET}.{CREATED_NOTE_ASSETS_OFFSET} add mem_storew dropw

        # populate note 1
        push.{note_1_metadata}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_1_OFFSET}.{CREATED_NOTE_METADATA_OFFSET} add add mem_storew dropw

        push.{note_1_recipient}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_1_OFFSET}.{CREATED_NOTE_RECIPIENT_OFFSET} add add mem_storew dropw

        push.{note_1_num_assets}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_1_OFFSET}.{CREATED_NOTE_NUM_ASSETS_OFFSET} add add mem_store

        push.{}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_1_OFFSET}.{CREATED_NOTE_ASSETS_OFFSET} add add mem_storew dropw

        # populate note 2
        push.{note_2_metadata}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_2_OFFSET}.{CREATED_NOTE_METADATA_OFFSET} add add mem_storew dropw

        push.{note_2_recipient}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_2_OFFSET}.{CREATED_NOTE_RECIPIENT_OFFSET} add add mem_storew dropw

        push.{note_2_num_assets}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_2_OFFSET}.{CREATED_NOTE_NUM_ASSETS_OFFSET} add add mem_store

        push.{}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_2_OFFSET}.{CREATED_NOTE_ASSETS_OFFSET} add add mem_storew dropw

        # set num created notes
        push.{}.{NUM_CREATED_NOTES_PTR} mem_store
    end
    ",
        note_0_assets[0],
        note_1_assets[0],
        note_2_assets[0],
        notes.num_notes()
    )
}

pub fn prepare_word(word: &Word) -> String {
    word.iter().map(|x| x.as_int().to_string()).collect::<Vec<_>>().join(".")
}

fn prepare_assets(note_assets: &NoteAssets) -> Vec<String> {
    let mut assets = Vec::new();
    for &asset in note_assets.iter() {
        let asset_word: Word = asset.into();
        let asset_str = prepare_word(&asset_word);
        assets.push(asset_str);
    }
    assets
}
