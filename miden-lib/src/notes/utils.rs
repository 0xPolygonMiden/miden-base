use assembly::ast::ProgramAst;
use miden_objects::{
    accounts::AccountId, notes::NoteScript, Digest, Hasher, NoteError, Word, ZERO,
};

use crate::transaction::TransactionKernel;

/// Creates the note_script from inputs
pub fn build_note_script(bytes: &[u8]) -> Result<NoteScript, NoteError> {
    let note_assembler = TransactionKernel::assembler();

    let script_ast = ProgramAst::from_bytes(bytes).map_err(NoteError::NoteDeserializationError)?;
    let (note_script, _) = NoteScript::new(script_ast, &note_assembler)?;

    Ok(note_script)
}

/// Creates the RECIPIENT for the P2ID note script created by the SWAP script
pub fn build_p2id_recipient(target: AccountId, serial_num: Word) -> Result<Digest, NoteError> {
    // TODO: add lazy_static initialization or compile-time optimization instead of re-generating
    // the script hash every time we call the SWAP script
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2ID.masb"));
    let note_script = build_note_script(bytes)?;

    let script_hash = note_script.hash();

    let serial_num_hash = Hasher::merge(&[serial_num.into(), Digest::default()]);

    let merge_script = Hasher::merge(&[serial_num_hash, script_hash]);

    Ok(Hasher::merge(&[
        merge_script,
        Hasher::hash_elements(&[target.into(), ZERO, ZERO, ZERO]),
    ]))
}
