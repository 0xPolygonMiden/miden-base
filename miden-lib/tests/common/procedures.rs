use super::{
    memory::{
        CREATED_NOTE_METADATA_OFFSET, CREATED_NOTE_NUM_ASSETS_OFFSET,
        CREATED_NOTE_RECIPIENT_OFFSET, CREATED_NOTE_SECTION_OFFSET, NOTE_MEM_SIZE,
        NUM_CREATED_NOTES_PTR,
    },
    Note, NoteVault, StarkField, Word,
};

pub fn created_notes_data_procedure(notes: &[Note]) -> String {
    let note_0_metadata = prepare_word(&notes[0].metadata());
    let note_0_recipient = prepare_word(&notes[0].recipient());
    let note_0_num_assets = notes[0].vault().num_assets();
    let note_0_assets = prepare_assets(notes[0].vault());

    let note_1_metadata = prepare_word(&notes[1].metadata());
    let note_1_recipient = prepare_word(&notes[1].recipient());
    let note_1_num_assets = notes[1].vault().num_assets();
    let note_1_assets = prepare_assets(notes[1].vault());

    let note_2_metadata = prepare_word(&notes[2].metadata());
    let note_2_recipient = prepare_word(&notes[2].recipient());
    let note_2_num_assets = notes[2].vault().num_assets();
    let note_2_assets = prepare_assets(notes[2].vault());

    const NOTE_1_OFFSET: u64 = NOTE_MEM_SIZE;
    const NOTE_2_OFFSET: u64 = NOTE_MEM_SIZE * 2;

    format!(
        "
    proc.create_mock_notes
        # populate note 0
        push.{note_0_metadata}
        push.{CREATED_NOTE_SECTION_OFFSET}.{CREATED_NOTE_METADATA_OFFSET} add mem_storew dropw

        push.{note_0_recipient}
        push.{CREATED_NOTE_SECTION_OFFSET}.{CREATED_NOTE_RECIPIENT_OFFSET} add mem_storew dropw

        push.{note_0_num_assets}
        push.{CREATED_NOTE_SECTION_OFFSET}.{CREATED_NOTE_NUM_ASSETS_OFFSET} add mem_store

        push.{}
        push.{CREATED_NOTE_SECTION_OFFSET}.5 add mem_storew dropw

        push.{}
        push.{CREATED_NOTE_SECTION_OFFSET}.6 add mem_storew dropw

        # populate note 1
        push.{note_1_metadata}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_1_OFFSET}.{CREATED_NOTE_METADATA_OFFSET} add add mem_storew dropw

        push.{note_1_recipient}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_1_OFFSET}.{CREATED_NOTE_RECIPIENT_OFFSET} add add mem_storew dropw

        push.{note_1_num_assets}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_1_OFFSET}.{CREATED_NOTE_NUM_ASSETS_OFFSET} add add mem_store

        push.{}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_1_OFFSET}.5 add add mem_storew dropw

        push.{}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_1_OFFSET}.6 add add mem_storew dropw

        push.{}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_1_OFFSET}.7 add add mem_storew dropw

        # populate note 2
        push.{note_2_metadata}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_2_OFFSET}.{CREATED_NOTE_METADATA_OFFSET} add add mem_storew dropw

        push.{note_2_recipient}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_2_OFFSET}.{CREATED_NOTE_RECIPIENT_OFFSET} add add mem_storew dropw

        push.{note_2_num_assets}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_2_OFFSET}.{CREATED_NOTE_NUM_ASSETS_OFFSET} add add mem_store

        push.{}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_2_OFFSET}.5 add add mem_storew dropw

        push.{}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_2_OFFSET}.6 add add mem_storew dropw

        push.{}
        push.{CREATED_NOTE_SECTION_OFFSET}.{NOTE_2_OFFSET}.7 add add mem_storew dropw

        # set num created notes
        push.{}.{NUM_CREATED_NOTES_PTR} mem_store
    end
    ",
        note_0_assets[0],
        note_0_assets[1],
        note_1_assets[0],
        note_1_assets[1],
        note_1_assets[2],
        note_2_assets[0],
        note_2_assets[1],
        note_2_assets[2],
        notes.len()
    )
}

fn prepare_word(word: &Word) -> String {
    word.iter().map(|x| x.as_int().to_string()).collect::<Vec<_>>().join(".")
}

fn prepare_assets(vault: &NoteVault) -> Vec<String> {
    let mut assets = Vec::new();
    for &asset in vault.iter() {
        let asset_word: Word = asset.into();
        let asset_str = prepare_word(&asset_word);
        assets.push(asset_str);
    }
    assets
}
