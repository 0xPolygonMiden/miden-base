use miden_objects::{
    accounts::AccountId,
    notes::{NoteInputs, NoteRecipient, NoteScript},
    utils::Deserializable,
    vm::Program,
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
    let program =
        Program::read_from_bytes(bytes).map_err(NoteError::NoteScriptDeserializationError)?;
    let note_script = NoteScript::new(program);
    let note_inputs = NoteInputs::new(vec![target.into()])?;

    Ok(NoteRecipient::new(serial_num, note_script, note_inputs))
}
