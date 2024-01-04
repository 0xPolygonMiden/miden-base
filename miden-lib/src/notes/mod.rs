use miden_objects::{
    accounts::AccountId,
    assembly::ProgramAst,
    assets::Asset,
    notes::{Note, NoteScript},
    utils::{collections::Vec, vec},
    Digest, Felt, Hasher, NoteError, Word, ZERO,
};

use super::transaction::TransactionKernel;

// STANDARDIZED SCRIPTS
// ================================================================================================

pub enum Script {
    P2ID { target: AccountId },
    P2IDR { target: AccountId, recall_height: u32 },
    SWAP { asset: Asset, serial_num: Word },
}

/// Users can create notes with a standard script. Atm we provide three standard scripts:
/// 1. P2ID - pay to id.
/// 2. P2IDR - pay to id with recall after a certain block height.
/// 3. SWAP - swap of assets between two accounts.
pub fn create_note(
    script: Script,
    assets: Vec<Asset>,
    sender: AccountId,
    tag: Option<Felt>,
    serial_num: Word,
) -> Result<Note, NoteError> {
    let note_assembler = TransactionKernel::assembler();

    // Include the binary version of the scripts into the source file at compile time
    let p2id_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2ID.masb"));
    let p2idr_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2IDR.masb"));
    let swap_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/SWAP.masb"));

    let (note_script_ast, inputs): (ProgramAst, Vec<Felt>) = match script {
        Script::P2ID { target } => (
            ProgramAst::from_bytes(p2id_bytes).map_err(NoteError::NoteDeserializationError)?,
            vec![target.into(), ZERO, ZERO, ZERO],
        ),
        Script::P2IDR { target, recall_height } => (
            ProgramAst::from_bytes(p2idr_bytes).map_err(NoteError::NoteDeserializationError)?,
            vec![target.into(), recall_height.into(), ZERO, ZERO],
        ),
        Script::SWAP { asset, serial_num } => {
            let recipient = build_p2id_recipient(sender, serial_num)?;
            let asset_word: Word = asset.into();
            (
                ProgramAst::from_bytes(swap_bytes).map_err(NoteError::NoteDeserializationError)?,
                vec![
                    recipient[0],
                    recipient[1],
                    recipient[2],
                    recipient[3],
                    asset_word[0],
                    asset_word[1],
                    asset_word[2],
                    asset_word[3],
                    sender.into(),
                    ZERO,
                    ZERO,
                    ZERO,
                ],
            )
        },
    };

    let (note_script, _) = NoteScript::new(note_script_ast, &note_assembler)?;

    Note::new(note_script.clone(), &inputs, &assets, serial_num, sender, tag.unwrap_or(ZERO))
}

/// Utility function generating RECIPIENT for the P2ID note script created by the SWAP script
fn build_p2id_recipient(target: AccountId, serial_num: Word) -> Result<Digest, NoteError> {
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
