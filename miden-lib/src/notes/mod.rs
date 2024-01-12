use miden_objects::{
    accounts::AccountId,
    assembly::ProgramAst,
    assets::Asset,
    crypto::rand::FeltRng,
    notes::{Note, NoteScript},
    utils::{collections::Vec, vec},
    Felt, NoteError, Word, ZERO,
};

use super::transaction::TransactionKernel;

mod utils;

// STANDARDIZED SCRIPTS NOTE GENERATORS
// ================================================================================================

/// Generates a P2ID - pay to id note.
pub fn create_p2id_note<R: FeltRng>(
    sender: AccountId,
    target: AccountId,
    assets: Vec<Asset>,
    mut rng: R,
) -> Result<Note, NoteError> {
    let assembler = TransactionKernel::assembler();
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2ID.masb"));
    let script_ast = ProgramAst::from_bytes(bytes).map_err(NoteError::NoteDeserializationError)?;
    let inputs = vec![target.into(), ZERO, ZERO, ZERO];
    let (note_script, _) = NoteScript::new(script_ast, &assembler)?;
    let tag: Felt = target.into();
    let serial_num = rng.draw_word();
    Note::new(note_script.clone(), &inputs, &assets, serial_num, sender, tag)
}

/// Generates a P2IDR - pay to id with recall after a certain block height.
pub fn create_p2idr_note<R: FeltRng>(
    sender: AccountId,
    target: AccountId,
    assets: Vec<Asset>,
    recall_height: u32,
    mut rng: R,
) -> Result<Note, NoteError> {
    let assembler = TransactionKernel::assembler();
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2IDR.masb"));
    let script_ast = ProgramAst::from_bytes(bytes).map_err(NoteError::NoteDeserializationError)?;
    let inputs = vec![target.into(), recall_height.into(), ZERO, ZERO];
    let (note_script, _) = NoteScript::new(script_ast, &assembler)?;
    let tag: Felt = target.into();
    let serial_num = rng.draw_word();
    Note::new(note_script.clone(), &inputs, &assets, serial_num, sender, tag)
}

/// Generates a SWAP - swap of assets between two accounts.
pub fn create_swap_note<R: FeltRng>(
    sender: AccountId,
    offered_asset: Asset,
    requested_asset: Asset,
    mut rng: R,
) -> Result<(Note, Note), NoteError> {
    let assembler = TransactionKernel::assembler();

    let swap_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/SWAP.masb"));

    let swap_script_ast =
        ProgramAst::from_bytes(swap_bytes).map_err(NoteError::NoteDeserializationError)?;

    let requested_serial_num = rng.draw_word();

    let recipient = utils::build_p2id_recipient(sender, requested_serial_num.clone())?;

    let asset_word: Word = requested_asset.into();

    let swap_inputs = vec![
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
    ];

    let (note_script, _) = NoteScript::new(swap_script_ast, &assembler)?;

    let tag: Felt = Felt::new(0);

    let serial_num = rng.draw_word();

    let swap_note = Note::new(
        note_script.clone(),
        &swap_inputs,
        &vec![offered_asset],
        serial_num,
        sender,
        tag,
    )?;

    let p2id_note = create_p2id_note(sender, sender, vec![requested_asset], rng)?;

    Ok((swap_note, p2id_note))
}
