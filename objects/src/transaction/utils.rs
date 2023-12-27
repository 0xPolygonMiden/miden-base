use vm_core::utils::IntoBytes;

use super::{
    AdviceInputs, Digest, Felt, StackInputs, ToAdviceInputs, TransactionInputs, TransactionScript,
    Vec, Word, ZERO,
};

// ADVICE INPUT CONSTRUCTORS
// ================================================================================================

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
    tx_inputs: &TransactionInputs,
    tx_script: &Option<TransactionScript>,
) -> AdviceInputs {
    let mut advice_inputs = AdviceInputs::default();

    // insert block data
    (&tx_inputs.block_header).to_advice_inputs(&mut advice_inputs);

    // insert block chain mmr
    (&tx_inputs.block_chain).to_advice_inputs(&mut advice_inputs);

    // insert account data
    tx_inputs.account.to_advice_inputs(&mut advice_inputs);

    // insert consumed notes data to advice stack
    (tx_inputs.input_notes).to_advice_inputs(&mut advice_inputs);

    if let Some(tx_script) = tx_script.as_ref() {
        // populate the advice inputs with the transaction script data
        tx_script.to_advice_inputs(&mut advice_inputs)
    } else {
        // if no transaction script is provided, extend the advice stack with an empty transaction
        // script root
        advice_inputs.extend_stack(Word::default());
    }

    // insert account id seed into advice map
    if let Some(seed) = tx_inputs.account_seed {
        advice_inputs.extend_map(vec![(
            [tx_inputs.account.id().into(), ZERO, ZERO, ZERO].into_bytes(),
            seed.to_vec(),
        )]);
    }

    advice_inputs
}

// INPUT STACK CONSTRUCTOR
// ================================================================================================

/// Returns the stack inputs required when executing a transaction.
///
/// This includes the input notes commitment, the account hash, the account id, and the block hash.
///
/// Stack: [BH, acct_id, IAH, NC]
///
/// - BH is the latest known block hash at the time of transaction execution.
/// - acct_id is the account id of the account that the transaction is being executed against.
/// - IAH is the initial account hash of the account that the transaction is being executed against.
/// - NC is the nullifier commitment of the transaction. This is a sequential hash of all
///   (nullifier, ZERO) tuples for the notes consumed in the transaction.
pub fn generate_stack_inputs(tx_inputs: &TransactionInputs) -> StackInputs {
    let initial_acct_hash = if tx_inputs.account.is_new() {
        Digest::default()
    } else {
        tx_inputs.account.hash()
    };

    let mut inputs: Vec<Felt> = Vec::with_capacity(13);
    inputs.extend(tx_inputs.input_notes.commitment());
    inputs.extend_from_slice(initial_acct_hash.as_elements());
    inputs.push((tx_inputs.account.id()).into());
    inputs.extend_from_slice(tx_inputs.block_header.hash().as_elements());
    StackInputs::new(inputs)
}
