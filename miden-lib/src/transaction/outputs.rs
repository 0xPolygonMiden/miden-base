use miden_objects::{
    accounts::{AccountId, AccountStub},
    assets::Asset,
    notes::{NoteAssets, NoteId, NoteMetadata},
    transaction::OutputNote,
    utils::collections::*,
    AccountError, Digest, NoteError, Word, WORD_SIZE,
};

use super::memory::{
    ACCT_CODE_ROOT_OFFSET, ACCT_DATA_MEM_SIZE, ACCT_ID_AND_NONCE_OFFSET, ACCT_ID_IDX,
    ACCT_NONCE_IDX, ACCT_STORAGE_ROOT_OFFSET, ACCT_VAULT_ROOT_OFFSET, CREATED_NOTE_ASSETS_OFFSET,
    CREATED_NOTE_ASSET_HASH_OFFSET, CREATED_NOTE_CORE_DATA_SIZE, CREATED_NOTE_ID_OFFSET,
    CREATED_NOTE_METADATA_OFFSET, CREATED_NOTE_NUM_ASSETS_OFFSET, CREATED_NOTE_RECIPIENT_OFFSET,
};

// STACK OUTPUTS
// ================================================================================================

/// The index of the word at which the transaction script root is stored on the output stack.
pub const TX_SCRIPT_ROOT_WORD_IDX: usize = 0;

/// The index of the word at which the final account nonce is stored on the output stack.
pub const OUTPUT_NOTES_COMMITMENT_WORD_IDX: usize = 1;

/// The index of the word at which the final account hash is stored on the output stack.
pub const FINAL_ACCOUNT_HASH_WORD_IDX: usize = 2;

// ACCOUNT STUB EXTRACTOR
// ================================================================================================

/// Parses the stub account data returned by the VM into individual account component commitments.
/// Returns a tuple of account ID, vault root, storage root, code root, and nonce.
pub fn parse_final_account_stub(elements: &[Word]) -> Result<AccountStub, AccountError> {
    if elements.len() != ACCT_DATA_MEM_SIZE {
        return Err(AccountError::StubDataIncorrectLength(elements.len(), ACCT_DATA_MEM_SIZE));
    }

    let id = AccountId::try_from(elements[ACCT_ID_AND_NONCE_OFFSET as usize][ACCT_ID_IDX])?;
    let nonce = elements[ACCT_ID_AND_NONCE_OFFSET as usize][ACCT_NONCE_IDX];
    let vault_root = elements[ACCT_VAULT_ROOT_OFFSET as usize].into();
    let storage_root = elements[ACCT_STORAGE_ROOT_OFFSET as usize].into();
    let code_root = elements[ACCT_CODE_ROOT_OFFSET as usize].into();

    Ok(AccountStub::new(id, nonce, vault_root, storage_root, code_root))
}

// NOTES EXTRACTOR
// ================================================================================================

pub fn notes_try_from_elements(elements: &[Word]) -> Result<OutputNote, NoteError> {
    if elements.len() < CREATED_NOTE_CORE_DATA_SIZE {
        return Err(NoteError::InvalidStubDataLen(elements.len()));
    }

    let note_id: NoteId = elements[CREATED_NOTE_ID_OFFSET as usize].into();
    let metadata: NoteMetadata = elements[CREATED_NOTE_METADATA_OFFSET as usize].try_into()?;
    let recipient = elements[CREATED_NOTE_RECIPIENT_OFFSET as usize].into();
    let num_assets = elements[CREATED_NOTE_NUM_ASSETS_OFFSET as usize][0];
    let asset_hash: Digest = elements[CREATED_NOTE_ASSET_HASH_OFFSET as usize].into();

    if elements.len()
        < (CREATED_NOTE_ASSETS_OFFSET as usize + num_assets.as_int() as usize) * WORD_SIZE
    {
        return Err(NoteError::InvalidStubDataLen(elements.len()));
    }

    let assets = elements[CREATED_NOTE_ASSETS_OFFSET as usize
        ..(CREATED_NOTE_ASSETS_OFFSET as usize + num_assets.as_int() as usize)]
        .iter()
        .map(|word| (*word).try_into())
        .collect::<Result<Vec<Asset>, _>>()
        .map_err(NoteError::InvalidAssetData)?;

    let assets = NoteAssets::new(&assets)?;
    if assets.commitment() != asset_hash {
        return Err(NoteError::InconsistentStubAssetHash(asset_hash, assets.commitment()));
    }

    let stub = OutputNote::new(recipient, assets, metadata);
    if stub.id() != note_id {
        return Err(NoteError::InconsistentStubId(stub.id(), note_id));
    }

    Ok(stub)
}
