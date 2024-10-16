use alloc::vec::Vec;

use miden_objects::{
    accounts::Account,
    transaction::{ChainMmr, InputNote, TransactionArgs, TransactionInputs, TransactionScript},
    vm::AdviceInputs,
    Felt, FieldElement, Word, EMPTY_WORD, ZERO,
};

// ADVICE INPUTS
// ================================================================================================

/// Extends the provided advice inputs with the data required for executing a transaction with the
/// specified inputs.
///
/// This includes the initial account, an optional account seed (required for new accounts), and
/// the input note data, including core note data + authentication paths all the way to the root
/// of one of chain MMR peaks.
pub(super) fn extend_advice_inputs(
    tx_inputs: &TransactionInputs,
    tx_args: &TransactionArgs,
    advice_inputs: &mut AdviceInputs,
) {
    build_advice_stack(tx_inputs, tx_args.tx_script(), advice_inputs);

    // build the advice map and Merkle store for relevant components
    add_chain_mmr_to_advice_inputs(tx_inputs.block_chain(), advice_inputs);
    add_account_to_advice_inputs(tx_inputs.account(), tx_inputs.account_seed(), advice_inputs);
    add_input_notes_to_advice_inputs(tx_inputs, tx_args, advice_inputs);
    advice_inputs.extend(tx_args.advice_inputs().clone());
}

// ADVICE STACK BUILDER
// ------------------------------------------------------------------------------------------------

/// Extend the advice stack with the transaction inputs.
///
/// The following data is pushed to the advice stack:
///
/// [
///     PREVIOUS_BLOCK_HASH,
///     CHAIN_MMR_HASH,
///     ACCOUNT_ROOT,
///     NULLIFIER_ROOT,
///     TX_HASH,
///     PROOF_HASH,
///     [block_num, version, timestamp, 0],
///     ZERO,
///     NOTE_ROOT,
///     [account_id, 0, 0, account_nonce],
///     ACCOUNT_VAULT_ROOT,
///     ACCOUNT_STORAGE_ROOT,
///     ACCOUNT_CODE_COMMITMENT,
///     number_of_input_notes,
///     TX_SCRIPT_ROOT,
/// ]
fn build_advice_stack(
    tx_inputs: &TransactionInputs,
    tx_script: Option<&TransactionScript>,
    inputs: &mut AdviceInputs,
) {
    // push block header info into the stack
    // Note: keep in sync with the process_block_data kernel procedure
    let header = tx_inputs.block_header();
    inputs.extend_stack(header.prev_hash());
    inputs.extend_stack(header.chain_root());
    inputs.extend_stack(header.account_root());
    inputs.extend_stack(header.nullifier_root());
    inputs.extend_stack(header.tx_hash());
    inputs.extend_stack(header.proof_hash());
    inputs.extend_stack([
        header.block_num().into(),
        header.version().into(),
        header.timestamp().into(),
        ZERO,
    ]);
    inputs.extend_stack(EMPTY_WORD);
    inputs.extend_stack(header.note_root());

    // push core account items onto the stack
    // Note: keep in sync with the process_account_data kernel procedure
    let account = tx_inputs.account();
    inputs.extend_stack([account.id().into(), ZERO, ZERO, account.nonce()]);
    inputs.extend_stack(account.vault().commitment());
    inputs.extend_stack(account.storage().root());
    inputs.extend_stack(account.code().commitment());

    // push the number of input notes onto the stack
    inputs.extend_stack([Felt::from(tx_inputs.input_notes().num_notes() as u32)]);

    // push tx_script root onto the stack
    inputs.extend_stack(tx_script.map_or(Word::default(), |script| *script.hash()));
}

// CHAIN MMR INJECTOR
// ------------------------------------------------------------------------------------------------

/// Inserts the chain MMR data into the provided advice inputs.
///
/// Inserts the following items into the Merkle store:
/// - Inner nodes of all authentication paths contained in the chain MMR.
///
/// Inserts the following data to the advice map:
///
/// > {MMR_ROOT: [[num_blocks, 0, 0, 0], PEAK_1, ..., PEAK_N]}
///
/// Where:
/// - MMR_ROOT, is the sequential hash of the padded MMR peaks
/// - num_blocks, is the number of blocks in the MMR.
/// - PEAK_1 .. PEAK_N, are the MMR peaks.
fn add_chain_mmr_to_advice_inputs(mmr: &ChainMmr, inputs: &mut AdviceInputs) {
    // NOTE: keep this code in sync with the `process_chain_data` kernel procedure

    // add authentication paths from the MMR to the Merkle store
    inputs.extend_merkle_store(mmr.inner_nodes());

    // insert MMR peaks info into the advice map
    let peaks = mmr.peaks();
    let mut elements = vec![Felt::new(peaks.num_leaves() as u64), ZERO, ZERO, ZERO];
    elements.extend(peaks.flatten_and_pad_peaks());
    inputs.extend_map([(peaks.hash_peaks(), elements)]);
}

// ACCOUNT DATA INJECTOR
// ------------------------------------------------------------------------------------------------

/// Inserts core account data into the provided advice inputs.
///
/// Inserts the following items into the Merkle store:
/// - The Merkle nodes associated with the storage slots tree.
/// - The Merkle nodes associated with the account vault tree.
/// - If present, the Merkle nodes associated with the account storage maps.
///
/// Inserts the following entries into the advice map:
/// - The storage types commitment |-> storage slot types vector.
/// - The account code commitment |-> procedures as elements and length.
/// - The node |-> (key, value), for all leaf nodes of the asset vault SMT.
/// - [account_id, 0, 0, 0] |-> account_seed, when account seed is provided.
/// - If present, the Merkle leaves associated with the account storage maps.
fn add_account_to_advice_inputs(
    account: &Account,
    account_seed: Option<Word>,
    inputs: &mut AdviceInputs,
) {
    // --- account storage ----------------------------------------------------
    let storage = account.storage();

    // extend the merkle store with the storage items
    inputs.extend_merkle_store(account.storage().slots().inner_nodes());

    // extend advice map with storage types commitment |-> storage types
    inputs.extend_map([(
        storage.layout_commitment(),
        storage.layout().iter().map(Felt::from).collect(),
    )]);

    // If there are storage maps, we populate the merkle store and advice map
    if !account.storage().maps().is_empty() {
        for map in account.storage().maps().values() {
            // extend the merkle store and map with the storage maps
            inputs.extend_merkle_store(map.inner_nodes());

            // populate advice map with Sparse Merkle Tree leaf nodes
            inputs.extend_map(map.leaves().map(|(_, leaf)| (leaf.hash(), leaf.to_elements())));
        }
    }

    // --- account vault ------------------------------------------------------
    let vault = account.vault();

    // extend the merkle store with account vault data
    inputs.extend_merkle_store(vault.asset_tree().inner_nodes());

    // populate advice map with Sparse Merkle Tree leaf nodes
    inputs
        .extend_map(vault.asset_tree().leaves().map(|(_, leaf)| (leaf.hash(), leaf.to_elements())));

    // --- account code -------------------------------------------------------
    let code = account.code();

    // extend the advice_map with the account code data and number of procedures
    let mut proc_elements: Vec<Felt> = vec![(code.num_procedures() as u32).into()];
    proc_elements.append(&mut code.as_elements());
    inputs.extend_map([(code.commitment(), proc_elements)]);

    // --- account seed -------------------------------------------------------
    if let Some(account_seed) = account_seed {
        inputs.extend_map(vec![(
            [account.id().into(), ZERO, ZERO, ZERO].into(),
            account_seed.to_vec(),
        )]);
    }
}

// INPUT NOTE INJECTOR
// ------------------------------------------------------------------------------------------------

/// Populates the advice inputs for all input notes.
///
/// The advice provider is populated with:
///
/// - For each note:
///     - The note's details (serial number, script root, and its input / assets hash).
///     - The note's private arguments.
///     - The note's public metadata.
///     - The note's public inputs data. Prefixed by its length and padded to an even word length.
///     - The note's asset padded. Prefixed by its length and padded to an even word length.
///     - For authenticated notes (determined by the `is_authenticated` flag):
///         - The note's authentication path against its block's note tree.
///         - The block number, sub hash, note root.
///         - The note's position in the note tree
///
/// The data above is processed by `prologue::process_input_notes_data`.
fn add_input_notes_to_advice_inputs(
    tx_inputs: &TransactionInputs,
    tx_args: &TransactionArgs,
    inputs: &mut AdviceInputs,
) {
    // if there are no input notes, nothing is added to the advice inputs
    if tx_inputs.input_notes().is_empty() {
        return;
    }

    let mut note_data = Vec::new();
    for input_note in tx_inputs.input_notes().iter() {
        let note = input_note.note();
        let assets = note.assets();
        let recipient = note.recipient();
        let note_arg = tx_args.get_note_args(note.id()).unwrap_or(&EMPTY_WORD);

        // NOTE: keep map in sync with the `note::get_inputs` API procedure
        inputs.extend_map([(
            recipient.inputs().commitment(),
            recipient.inputs().format_for_advice(),
        )]);

        inputs.extend_map([(assets.commitment(), assets.to_padded_assets())]);

        // NOTE: keep in sync with the `prologue::process_input_note_details` kernel procedure
        note_data.extend(recipient.serial_num());
        note_data.extend(*recipient.script().hash());
        note_data.extend(*recipient.inputs().commitment());
        note_data.extend(*assets.commitment());

        // NOTE: keep in sync with the `prologue::process_note_args_and_metadata` kernel procedure
        note_data.extend(Word::from(*note_arg));
        note_data.extend(Word::from(note.metadata()));

        // NOTE: keep in sync with the `prologue::process_note_assets` kernel procedure
        note_data.push((assets.num_assets() as u32).into());
        note_data.extend(assets.to_padded_assets());

        // insert note authentication path nodes into the Merkle store
        match input_note {
            InputNote::Authenticated { note, proof } => {
                let block_num = proof.location().block_num();
                let note_block_header = if block_num == tx_inputs.block_header().block_num() {
                    tx_inputs.block_header()
                } else {
                    tx_inputs
                        .block_chain()
                        .get_block(block_num)
                        .expect("block not found in chain MMR")
                };

                // NOTE: keep in sync with the `prologue::process_input_note` kernel procedure
                // Push the `is_authenticated` flag
                note_data.push(Felt::ONE);

                // NOTE: keep in sync with the `prologue::authenticate_note` kernel procedure
                inputs.extend_merkle_store(
                    proof
                        .note_path()
                        .inner_nodes(proof.location().node_index_in_block().into(), note.hash())
                        .unwrap(),
                );
                note_data.push(proof.location().block_num().into());
                note_data.extend(note_block_header.sub_hash());
                note_data.extend(note_block_header.note_root());
                note_data.push(proof.location().node_index_in_block().into());
            },
            InputNote::Unauthenticated { .. } => {
                // NOTE: keep in sync with the `prologue::process_input_note` kernel procedure
                // Push the `is_authenticated` flag
                note_data.push(Felt::ZERO);
            },
        }
    }

    // NOTE: keep map in sync with the `prologue::process_input_notes_data` kernel procedure
    inputs.extend_map([(tx_inputs.input_notes().commitment(), note_data)]);
}
