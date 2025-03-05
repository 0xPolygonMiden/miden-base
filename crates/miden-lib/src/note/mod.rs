use alloc::vec::Vec;

use miden_objects::{
    account::AccountId,
    assembly::{ProcedureName, QualifiedProcedureName},
    asset::Asset,
    block::BlockNumber,
    crypto::rand::FeltRng,
    note::{
        Note, NoteAssets, NoteDetails, NoteExecutionHint, NoteExecutionMode, NoteInputs,
        NoteMetadata, NoteRecipient, NoteScript, NoteTag, NoteType,
    },
    Digest, Felt, NoteError, Word,
};
use scripts::{p2id, p2id_commitment, p2idr, p2idr_commitment, swap, swap_commitment};
use utils::build_swap_tag;

use crate::account::{
    components::basic_wallet_library,
    interface::{component_proc_digests, AccountComponentInterface, AccountInterface},
};

pub mod scripts;
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
    aux: Felt,
    rng: &mut R,
) -> Result<Note, NoteError> {
    let serial_num = rng.draw_word();
    let recipient = utils::build_p2id_recipient(target, serial_num)?;

    let tag = NoteTag::from_account_id(target, NoteExecutionMode::Local)?;

    let metadata = NoteMetadata::new(sender, note_type, tag, NoteExecutionHint::always(), aux)?;
    let vault = NoteAssets::new(assets)?;

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
    aux: Felt,
    recall_height: BlockNumber,
    rng: &mut R,
) -> Result<Note, NoteError> {
    let note_script = scripts::p2idr();

    let inputs =
        NoteInputs::new(vec![target.suffix(), target.prefix().as_felt(), recall_height.into()])?;
    let tag = NoteTag::from_account_id(target, NoteExecutionMode::Local)?;
    let serial_num = rng.draw_word();

    let vault = NoteAssets::new(assets)?;
    let metadata = NoteMetadata::new(sender, note_type, tag, NoteExecutionHint::always(), aux)?;
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
    aux: Felt,
    rng: &mut R,
) -> Result<(Note, NoteDetails), NoteError> {
    let note_script = scripts::swap();

    let payback_serial_num = rng.draw_word();
    let payback_recipient = utils::build_p2id_recipient(sender, payback_serial_num)?;

    let payback_recipient_word: Word = payback_recipient.digest().into();
    let requested_asset_word: Word = requested_asset.into();
    let payback_tag = NoteTag::from_account_id(sender, NoteExecutionMode::Local)?;

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
        NoteExecutionHint::always().into(),
    ])?;

    // build the tag for the SWAP use case
    let tag = build_swap_tag(note_type, &offered_asset, &requested_asset)?;
    let serial_num = rng.draw_word();

    // build the outgoing note
    let metadata = NoteMetadata::new(sender, note_type, tag, NoteExecutionHint::always(), aux)?;
    let assets = NoteAssets::new(vec![offered_asset])?;
    let recipient = NoteRecipient::new(serial_num, note_script, inputs);
    let note = Note::new(assets, metadata, recipient);

    // build the payback note details
    let payback_assets = NoteAssets::new(vec![requested_asset])?;
    let payback_note = NoteDetails::new(payback_assets, payback_recipient);

    Ok((note, payback_note))
}

// WELL KNOWN NOTES
// ================================================================================================

/// The enum holding the types of basic well-known notes provided by the `miden-lib`.
pub enum WellKnownNote {
    P2ID,
    P2IDR,
    SWAP,
}

impl WellKnownNote {
    /// Returns a [WellKnownNote] instance based on the note script of the provided [Note]. Returns
    /// `None` if the provided note is not a basic well-known note.
    pub fn from_note(note: &Note) -> Option<Self> {
        let note_script_commitment = note.script().hash();

        if note_script_commitment == p2id_commitment() {
            return Some(Self::P2ID);
        }
        if note_script_commitment == p2idr_commitment() {
            return Some(Self::P2IDR);
        }
        if note_script_commitment == swap_commitment() {
            return Some(Self::SWAP);
        }

        None
    }

    /// Returns the note script of the current [WellKnownNote] instance.
    pub fn script(&self) -> NoteScript {
        match self {
            Self::P2ID => p2id(),
            Self::P2IDR => p2idr(),
            Self::SWAP => swap(),
        }
    }

    /// Returns the script commitment of the current [WellKnownNote] instance.
    pub fn script_root(&self) -> Digest {
        match self {
            Self::P2ID => p2id_commitment(),
            Self::P2IDR => p2idr_commitment(),
            Self::SWAP => swap_commitment(),
        }
    }

    /// Returns a boolean value indicating whether this [WellKnownNote] is compatible with the
    /// provided [AccountInterface].
    pub fn is_compatible_with(&self, account_interface: &AccountInterface) -> bool {
        if account_interface.interfaces().contains(&AccountComponentInterface::BasicWallet) {
            return true;
        }

        let interface_proc_digests = component_proc_digests(account_interface.interfaces());
        match self {
            Self::P2ID | &Self::P2IDR => {
                // Get the hash of the "receive_asset" procedure and check that this procedure is
                // presented in the provided account interfaces. P2ID and P2IDR notes requires only
                // this procedure to be consumed by the account.
                let receive_asset_proc_name = QualifiedProcedureName::new(
                    Default::default(),
                    ProcedureName::new("receive_asset").unwrap(),
                );
                let node_id = basic_wallet_library().get_export_node_id(&receive_asset_proc_name);
                let receive_asset_digest = basic_wallet_library().mast_forest()[node_id].digest();

                interface_proc_digests.contains(&receive_asset_digest)
            },
            Self::SWAP => {
                // Make sure that all procedures from the basic wallet library are presented in the
                // provided account interfaces. SWAP note requires the whole basic wallet interface
                // to be consumed by the account.
                basic_wallet_library()
                    .mast_forest()
                    .procedure_digests()
                    .all(|proc_digest| interface_proc_digests.contains(&proc_digest))
            },
        }
    }
}
