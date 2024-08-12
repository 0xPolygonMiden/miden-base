use miden_objects::{
    accounts::AccountId,
    notes::{NoteInputs, NoteRecipient, NoteScript},
    NoteError, Word,
};

/// Creates a [NoteRecipient] for the P2ID note.
///
/// Notes created with this recipient will be P2ID notes consumable by the specified target
/// account.
pub fn build_p2id_recipient(
    target: AccountId,
    serial_num: Word,
) -> Result<NoteRecipient, NoteError> {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2ID.masb"));
    let note_script = NoteScript::from_bytes(bytes)?;
    let note_inputs = NoteInputs::new(vec![target.into()])?;

    Ok(NoteRecipient::new(serial_num, note_script, note_inputs))
}
