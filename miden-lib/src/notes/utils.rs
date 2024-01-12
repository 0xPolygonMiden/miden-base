use assembly::ast::ProgramAst;
use miden_objects::{
    accounts::AccountId, notes::NoteScript, Digest, Hasher, NoteError, Word, ZERO,
};

use crate::transaction::TransactionKernel;

/// Utility function generating RECIPIENT for the P2ID note script created by the SWAP script
pub fn build_p2id_recipient(target: AccountId, serial_num: Word) -> Result<Digest, NoteError> {
    // TODO: add lazy_static initialization or compile-time optimization instead of re-generating
    // the script hash every time we call the SWAP script
    let assembler = TransactionKernel::assembler();

    let p2id_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2ID.masb"));

    let note_script_ast =
        ProgramAst::from_bytes(p2id_bytes).map_err(NoteError::NoteDeserializationError)?;

    let (note_script, _) = NoteScript::new(note_script_ast, &assembler)?;

    let script_hash = note_script.hash();

    let serial_num_hash = Hasher::merge(&[serial_num.into(), Digest::default()]);

    let merge_script = Hasher::merge(&[serial_num_hash, script_hash]);

    Ok(Hasher::merge(&[
        merge_script,
        Hasher::hash_elements(&[target.into(), ZERO, ZERO, ZERO]),
    ]))
}
