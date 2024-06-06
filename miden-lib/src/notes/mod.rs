use alloc::vec::Vec;

use miden_objects::{
    accounts::AccountId,
    assets::Asset,
    crypto::rand::FeltRng,
    notes::{
        Note, NoteAssets, NoteDetails, NoteExecutionHint, NoteInputs, NoteMetadata, NoteRecipient,
        NoteTag, NoteType,
    },
    NoteError, Word, ZERO,
};

use self::utils::build_note_script;

pub mod utils;

// STANDARDIZED SCRIPTS
// ================================================================================================

/// Generates a P2ID note - pay to id note.
///
/// This script enables the transfer of assets from the `sender` account to the `target` account
/// by specifying the target's account ID.
///
/// The passed-in `rng` is used to generate a serial number for the note. The returned note's tag
/// is set to the target's account ID.
///
/// # Errors
/// Returns an error if deserialization or compilation of the `P2ID` script fails.
pub fn create_p2id_note<R: FeltRng>(
    sender: AccountId,
    target: AccountId,
    assets: Vec<Asset>,
    note_type: NoteType,
    rng: &mut R,
) -> Result<Note, NoteError> {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2ID.masb"));
    let note_script = build_note_script(bytes)?;

    let inputs = NoteInputs::new(vec![target.into()])?;
    let tag = NoteTag::from_account_id(target, NoteExecutionHint::Local)?;
    let serial_num = rng.draw_word();
    let aux = ZERO;

    let metadata = NoteMetadata::new(sender, note_type, tag, aux)?;
    let vault = NoteAssets::new(assets)?;
    let recipient = NoteRecipient::new(serial_num, note_script, inputs);
    Ok(Note::new(vault, metadata, recipient))
}

/// Generates a P2IDR note - pay to id with recall after a certain block height.
///
/// This script enables the transfer of assets from the sender `sender` account to the `target`
/// account by specifying the target's account ID. Additionally it adds the possibility for the
/// sender to reclaiming the assets if the note has not been consumed by the target within the
/// specified timeframe.
///
/// The passed-in `rng` is used to generate a serial number for the note. The returned note's tag
/// is set to the target's account ID.
///
/// # Errors
/// Returns an error if deserialization or compilation of the `P2IDR` script fails.
pub fn create_p2idr_note<R: FeltRng>(
    sender: AccountId,
    target: AccountId,
    assets: Vec<Asset>,
    note_type: NoteType,
    recall_height: u32,
    rng: &mut R,
) -> Result<Note, NoteError> {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2IDR.masb"));
    let note_script = build_note_script(bytes)?;

    let inputs = NoteInputs::new(vec![target.into(), recall_height.into()])?;
    let tag = NoteTag::from_account_id(target, NoteExecutionHint::Local)?;
    let serial_num = rng.draw_word();
    let aux = ZERO;

    let vault = NoteAssets::new(assets)?;
    let metadata = NoteMetadata::new(sender, note_type, tag, aux)?;
    let recipient = NoteRecipient::new(serial_num, note_script, inputs);
    Ok(Note::new(vault, metadata, recipient))
}

/// Generates a SWAP note - swap of assets between two accounts - and returns the note as well as
/// [NoteDetails] for the payback note.
///
/// This script enables a swap of 2 assets between the `sender` account and any other account that
/// is willing to consume the note. The consumer will receive the `offered_asset` and will create a
/// new P2ID note with `sender` as target, containing the `requested_asset`.
///
/// # Errors
/// Returns an error if deserialization or compilation of the `SWAP` script fails.
pub fn create_swap_note<R: FeltRng>(
    sender: AccountId,
    offered_asset: Asset,
    requested_asset: Asset,
    note_type: NoteType,
    rng: &mut R,
) -> Result<(Note, NoteDetails), NoteError> {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/SWAP.masb"));
    let note_script = build_note_script(bytes)?;

    let payback_serial_num = rng.draw_word();
    let payback_recipient = utils::build_p2id_recipient(sender, payback_serial_num)?;

    let payback_recipient_word: Word = payback_recipient.digest().into();
    let requested_asset_word: Word = requested_asset.into();
    let payback_tag = NoteTag::from_account_id(sender, NoteExecutionHint::Local)?;

    let inputs = NoteInputs::new(vec![
        payback_recipient_word[0],
        payback_recipient_word[1],
        payback_recipient_word[2],
        payback_recipient_word[3],
        requested_asset_word[0],
        requested_asset_word[1],
        requested_asset_word[2],
        requested_asset_word[3],
        payback_tag.inner().into(),
    ])?;

    // build the tag for the SWAP use case
    let tag = build_swap_tag(note_type, &offered_asset, &requested_asset)?;
    let serial_num = rng.draw_word();
    let aux = ZERO;

    // build the outgoing note
    let metadata = NoteMetadata::new(sender, note_type, tag, aux)?;
    let assets = NoteAssets::new(vec![offered_asset])?;
    let recipient = NoteRecipient::new(serial_num, note_script, inputs);
    let note = Note::new(assets, metadata, recipient);

    // build the payback note details
    let payback_assets = NoteAssets::new(vec![requested_asset])?;
    let payback_note = NoteDetails::new(payback_assets, payback_recipient);

    Ok((note, payback_note))
}

// HELPER FUNCTIONS
// ================================================================================================

/// Returns a note tag for a swap note with the specified parameters.
///
/// Use case ID for the returned tag is set to 0.
///
/// Tag payload is constructed by taking asset tags (8 bits of faucet ID) and concatenating them
/// together as offered_asset_tag + requested_asset tag.
///
/// Network execution hint for the returned tag is set to `Local`.
fn build_swap_tag(
    note_type: NoteType,
    offered_asset: &Asset,
    requested_asset: &Asset,
) -> Result<NoteTag, NoteError> {
    const SWAP_USE_CASE_ID: u16 = 0;

    // get bits 4..12 from faucet IDs of both assets, these bits will form the tag payload; the
    // reason we skip the 4 most significant bits is that these encode metadata of underlying
    // faucets and are likely to be the same for many different faucets.

    let offered_asset_id: u64 = offered_asset.faucet_id().into();
    let offered_asset_tag = (offered_asset_id >> 52) as u8;

    let requested_asset_id: u64 = requested_asset.faucet_id().into();
    let requested_asset_tag = (requested_asset_id >> 52) as u8;

    let payload = ((offered_asset_tag as u16) << 8) | (requested_asset_tag as u16);

    let execution = NoteExecutionHint::Local;
    match note_type {
        NoteType::Public => NoteTag::for_public_use_case(SWAP_USE_CASE_ID, payload, execution),
        _ => NoteTag::for_local_use_case(SWAP_USE_CASE_ID, payload),
    }
}
