use super::{
    Account, AccountId, AdviceInputs, BlockHeader, Digest, Felt, Hasher, Mmr, Note, StackInputs,
    StackOutputs, Word,
};
use assembly::utils::IntoBytes;
use hashbrown::HashMap;

/// Returns the advice inputs required when executing a transaction.
/// This includes the initial account, the number of consumed notes, the core consumed note data,
/// and the consumed note inputs.
///
/// Advice Tape: [acct_id, ZERO, ZERO, nonce, AVR, ASR, ACR,
///               num_cn,
///               CN1_SN, CN1_SR, CN1_IR, CN1_VR,
///               cn1_na,
///               CN1_A1, CN1_A2, ...
///               CN2_SN,CN2_SR, CN2_IR, CN2_VR,
///               cn2_na,
///               CN2_A1, CN2_A2, ...
///               ...
///               CN1_I3, CN1_I2, CN1_I1, CN1_I0,
///               CN2_I3, CN2_I2, CN2_I1, CN2_I0,
///               ...]
/// Advice Map: {CHAIN_ROOT, [num_leaves, PEAK_0, ..., PEAK_N]}
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
/// - CN1_I3..0 are the script inputs of consumed note 1.
/// - CN2_I3..0 are the script inputs of consumed note 2.
/// - CHAIN_ROOT is the root of the block chain MMR from the last known block.
/// - num_leaves is the number of leaves in the block chain MMR from the last known block.
/// - PEAK_0 is the first peak in the block chain MMR from the last known block.
/// - PEAK_N is the n'th peak in the block chain MMR from the last known block.
pub fn generate_advice_provider_inputs(
    account: &Account,
    block_header: &BlockHeader,
    block_chain: &Mmr,
    notes: &[Note],
) -> AdviceInputs {
    let mut advice_map: HashMap<[u8; 32], Vec<Felt>> = HashMap::new();
    let mut advice_stack: Vec<Felt> = Vec::new();

    // insert block data
    let block_data = Vec::<Felt>::from(block_header);
    advice_stack.extend(block_data);

    // insert block chain mmr
    let chain_accumulator = block_chain.accumulator();
    advice_map.insert(chain_accumulator.hash_peaks().into_bytes(), (&chain_accumulator).into());

    // insert account data
    let account: [Felt; 16] = account.into();
    advice_stack.extend(account);

    // insert consumed notes data to advice stack
    advice_stack.push(Felt::new(notes.len() as u64));
    let note_data: Vec<Felt> = notes.iter().flat_map(<Vec<Felt>>::from).collect();
    advice_stack.extend(note_data);

    // insert consumed note assets data to advice map
    for note in notes.iter() {
        advice_map.insert(note.vault().hash().into_bytes(), note.vault().to_padded_assets());
    }

    // insert consumed notes inputs
    let note_inputs: Vec<Felt> =
        notes.iter().flat_map(|note| note.inputs().inputs().to_vec()).collect();
    advice_stack.extend(note_inputs);

    AdviceInputs::default().with_stack(advice_stack).with_map(advice_map)
}

/// Returns the consumed notes commitment.
/// This is a sequential hash of all (nullifier, script_root) pairs for the notes consumed in the
/// transaction.
pub fn generate_consumed_notes_commitment(notes: &[Note]) -> Digest {
    let mut elements: Vec<Felt> = Vec::with_capacity(notes.len() * 8);
    for note in notes.iter() {
        elements.extend_from_slice(note.nullifier().as_elements());
        elements.extend_from_slice(note.script().hash().as_elements());
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
    account_hash: &Digest,
    notes: &[Note],
    block_header: &BlockHeader,
) -> StackInputs {
    let mut inputs: Vec<Felt> = Vec::with_capacity(13);
    inputs.extend_from_slice(generate_consumed_notes_commitment(notes).as_elements());
    inputs.extend_from_slice(account_hash.as_elements());
    inputs.push(**account_id);
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
    StackOutputs::from_elements(outputs, Default::default())
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
