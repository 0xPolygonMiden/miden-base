use vm_core::utils::IntoBytes;

use super::{
    Account, AccountId, AdviceInputs, BlockHeader, ChainMmr, ConsumedNotes, Digest, Felt, Hasher,
    Note, RecordedNote, StackInputs, StackOutputs, ToAdviceInputs, TransactionScript, Vec, Word,
    ZERO,
};

/// Returns the advice inputs required when executing a transaction.
/// This includes the initial account, an optional account seed (required for new accounts), the
/// number of consumed notes, the core consumed note data, and the consumed note inputs.
///
/// Advice Tape: [acct_id, ZERO, ZERO, nonce, AVR, ASR, ACR,
///               num_cn,
///               CN1_SN, CN1_SR, CN1_IR, CN1_VR,
///               cn1_na,
///               CN1_A1, CN1_A2, ...
///               CN2_SN,CN2_SR, CN2_IR, CN2_VR,
///               cn2_na,
///               CN2_A1, CN2_A2, ...
///               ...]
/// Advice Map: {CHAIN_ROOT:             [num_leaves, PEAK_0, ..., PEAK_N],
///              CN1_IH:                 [CN1_I3, CN1_I2, CN1_I1, CN1_I0],
///              CN2_IH:                 [CN2_I3, CN2_I2, CN2_I1, CN2_I0],
///              [acct_id, 0, 0, 0]?:    [ACT_ID_SEED3, ACT_ID_SEED2, ACT_ID_SEED1, ACT_ID_SEED0],
///              ...}
/// - acct_id is the account id of the account that the transaction is being executed against.
/// - nonce is the account nonce.
/// - AVR is the account vault root.
/// - ASR is the account storage root.
/// - ACR is the account code root.
/// - num_cn is the number of consumed notes.
/// - CN1_SN is the serial number of consumed note 1.
/// - CN1_SR is the script root of consumed note 1.
/// - CN1_IR is the inputs root of consumed note 1.
/// - CN1_VR is the vault root of consumed note 1.
/// - CN1_M is the metadata of consumed note 1.
/// - CN1_A1 is the first asset of consumed note 1.
/// - CN1_A2 is the second asset of consumed note 1.
/// - CN1_IH is the inputs hash of consumed note 1.
/// - CN2_SN is the serial number of consumed note 2.
/// - CN1_I3..0 are the script inputs of consumed note 1.
/// - CN2_I3..0 are the script inputs of consumed note 2.
/// - CHAIN_ROOT is the root of the block chain MMR from the last known block.
/// - num_leaves is the number of leaves in the block chain MMR from the last known block.
/// - PEAK_0 is the first peak in the block chain MMR from the last known block.
/// - PEAK_N is the n'th peak in the block chain MMR from the last known block.
/// - ACT_ID_SEED3..0 is the account id seed.
pub fn generate_advice_provider_inputs(
    account: &Account,
    account_id_seed: Option<Word>,
    block_header: &BlockHeader,
    block_chain: &ChainMmr,
    notes: &ConsumedNotes,
    tx_script: &Option<TransactionScript>,
) -> AdviceInputs {
    let mut advice_inputs = AdviceInputs::default();

    // insert block data
    block_header.to_advice_inputs(&mut advice_inputs);

    // insert block chain mmr
    block_chain.to_advice_inputs(&mut advice_inputs);

    // insert account data
    account.to_advice_inputs(&mut advice_inputs);

    // insert consumed notes data to advice stack
    notes.to_advice_inputs(&mut advice_inputs);

    if let Some(tx_script) = tx_script.as_ref() {
        // populate the advice inputs with the transaction script data
        tx_script.to_advice_inputs(&mut advice_inputs)
    } else {
        // if no transaction script is provided, extend the advice stack with an empty transaction
        // script root
        advice_inputs.extend_stack(Word::default());
    }

    // insert account id seed into advice map
    if let Some(seed) = account_id_seed {
        advice_inputs.extend_map(vec![(
            [account.id().into(), ZERO, ZERO, ZERO].into_bytes(),
            seed.to_vec(),
        )]);
    }

    advice_inputs
}

/// Returns the consumed notes commitment.
///
/// This is a sequential hash of all (nullifier, ZERO) pairs for the notes consumed in the
/// transaction.
pub fn generate_consumed_notes_commitment(recorded_notes: &[RecordedNote]) -> Digest {
    let mut elements: Vec<Felt> = Vec::with_capacity(recorded_notes.len() * 8);
    for recorded_note in recorded_notes.iter() {
        elements.extend_from_slice(recorded_note.note().nullifier().as_elements());
        elements.extend_from_slice(&Word::default());
    }
    Hasher::hash_elements(&elements)
}

/// Returns the stack inputs required when executing a transaction.
/// This includes the consumed notes commitment, the account hash, the account id, and the block
/// reference.
///
/// Stack: [BH, acct_id, IAH, NC]
///
/// - BH is the latest known block hash at the time of transaction execution.
/// - acct_id is the account id of the account that the transaction is being executed against.
/// - IAH is the initial account hash of the account that the transaction is being executed against.
/// - NC is the nullifier commitment of the transaction. This is a sequential hash of all
///   (nullifier, script_root) pairs for the notes consumed in the transaction.
pub fn generate_stack_inputs(
    account_id: &AccountId,
    account_hash: Digest,
    consumed_notes_commitment: Digest,
    block_header: &BlockHeader,
) -> StackInputs {
    let mut inputs: Vec<Felt> = Vec::with_capacity(13);
    inputs.extend(*consumed_notes_commitment);
    inputs.extend_from_slice(account_hash.as_elements());
    inputs.push((*account_id).into());
    inputs.extend_from_slice(block_header.hash().as_elements());
    StackInputs::new(inputs)
}

/// Returns the stack outputs produced as a result of executing a transaction. This includes the
/// final account hash and created notes commitment.
///
/// Output: [CREATED_NOTES_COMMITMENT, FINAL_ACCOUNT_HASH]
///
/// - CREATED_NOTES_COMMITMENT is the commitment of the created notes
/// - FINAL_ACCOUNT_HASH is the final account hash
pub fn generate_stack_outputs(created_notes: &[Note], final_account_hash: &Digest) -> StackOutputs {
    let mut outputs: Vec<Felt> = Vec::with_capacity(8);
    outputs.extend_from_slice(generate_created_notes_commitment(created_notes).as_elements());
    outputs.extend_from_slice(final_account_hash.as_elements());
    outputs.reverse();
    StackOutputs::from_elements(outputs, Default::default()).expect("stack outputs are valid")
}

/// Returns the created notes commitment.
/// This is a sequential hash of all (hash, metadata) pairs for the notes created in the transaction.
pub fn generate_created_notes_commitment(notes: &[Note]) -> Digest {
    let mut elements: Vec<Felt> = Vec::with_capacity(notes.len() * 8);
    for note in notes.iter() {
        elements.extend_from_slice(note.hash().as_elements());
        elements.extend_from_slice(&Word::from(note.metadata()));
    }

    Hasher::hash_elements(&elements)
}
