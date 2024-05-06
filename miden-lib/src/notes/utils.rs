use miden_objects::{
    accounts::AccountId,
    assembly::ProgramAst,
    notes::{NoteInputs, NoteRecipient, NoteScript},
    NoteError, Word, ZERO,
};

use crate::transaction::TransactionKernel;

/// Creates the note_script from inputs
pub fn build_note_script(bytes: &[u8]) -> Result<NoteScript, NoteError> {
    let note_assembler = TransactionKernel::assembler();

    let script_ast = ProgramAst::from_bytes(bytes).map_err(NoteError::NoteDeserializationError)?;
    let (note_script, _) = NoteScript::new(script_ast, &note_assembler)?;

    Ok(note_script)
}

/// Creates a [NoteRecipient] for the P2ID note.
/// 
/// Notes created with this recipient will be P2ID notes consumable by the specified target
/// account.
pub fn build_p2id_recipient(
    target: AccountId,
    serial_num: Word,
) -> Result<NoteRecipient, NoteError> {
    // TODO: add lazy_static initialization or compile-time optimization instead of re-generating
    // the script hash every time we call the SWAP script
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2ID.masb"));
    let note_script = build_note_script(bytes)?;
    let note_inputs = NoteInputs::new(vec![target.into(), ZERO, ZERO, ZERO])?;

    Ok(NoteRecipient::new(serial_num, note_script, note_inputs))
}
