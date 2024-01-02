use crate::assembler::assembler;
use crate::memory::{
    CREATED_NOTE_ASSETS_OFFSET, CREATED_NOTE_CORE_DATA_SIZE, CREATED_NOTE_HASH_OFFSET,
    CREATED_NOTE_METADATA_OFFSET, CREATED_NOTE_RECIPIENT_OFFSET, CREATED_NOTE_VAULT_HASH_OFFSET,
};
use miden_objects::crypto::rand::FeltRng;

use miden_objects::{
    accounts::AccountId,
    assembly::ProgramAst,
    assets::Asset,
    notes::{Note, NoteMetadata, NoteScript, NoteStub, NoteVault},
    utils::{collections::Vec, vec},
    Digest, Felt, Hasher, NoteError, StarkField, Word, WORD_SIZE, ZERO,
};

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
pub fn create_note<R: FeltRng>(
    script: Script,
    assets: Vec<Asset>,
    sender: AccountId,
    tag: Option<Felt>,
    mut rng: R,
) -> Result<Note, NoteError> {
    let note_assembler = assembler();

    // Include the binary version of the scripts into the source file at compile time
    let p2id_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/P2ID.masb"));
    let p2idr_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/P2IDR.masb"));
    let swap_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/SWAP.masb"));

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

    let serial_num = rng.draw_word();
    Note::new(note_script.clone(), &inputs, &assets, serial_num, sender, tag.unwrap_or(ZERO))
}

pub fn notes_try_from_elements(elements: &[Word]) -> Result<NoteStub, NoteError> {
    if elements.len() < CREATED_NOTE_CORE_DATA_SIZE {
        return Err(NoteError::InvalidStubDataLen(elements.len()));
    }

    let hash: Digest = elements[CREATED_NOTE_HASH_OFFSET as usize].into();
    let metadata: NoteMetadata = elements[CREATED_NOTE_METADATA_OFFSET as usize].try_into()?;
    let recipient = elements[CREATED_NOTE_RECIPIENT_OFFSET as usize].into();
    let vault_hash: Digest = elements[CREATED_NOTE_VAULT_HASH_OFFSET as usize].into();

    if elements.len()
        < (CREATED_NOTE_ASSETS_OFFSET as usize + metadata.num_assets().as_int() as usize)
            * WORD_SIZE
    {
        return Err(NoteError::InvalidStubDataLen(elements.len()));
    }

    let vault: NoteVault = elements[CREATED_NOTE_ASSETS_OFFSET as usize
        ..(CREATED_NOTE_ASSETS_OFFSET as usize + metadata.num_assets().as_int() as usize)]
        .try_into()?;
    if vault.hash() != vault_hash {
        return Err(NoteError::InconsistentStubVaultHash(vault_hash, vault.hash()));
    }

    let stub = NoteStub::new(recipient, vault, metadata)?;
    if stub.hash() != hash {
        return Err(NoteError::InconsistentStubHash(stub.hash(), hash));
    }

    Ok(stub)
}

/// Utility function generating RECIPIENT for the P2ID note script created by the SWAP script
fn build_p2id_recipient(target: AccountId, serial_num: Word) -> Result<Digest, NoteError> {
    // TODO: add lazy_static initialization or compile-time optimization instead of re-generating
    // the script hash every time we call the SWAP script
    let assembler = assembler();

    let p2id_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/P2ID.masb"));

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
