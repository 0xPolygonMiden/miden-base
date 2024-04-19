use alloc::vec::Vec;

use miden_objects::{
    accounts::Account,
    transaction::{
        ChainMmr, ExecutedTransaction, InputNotes, PreparedTransaction, TransactionArgs,
        TransactionInputs, TransactionScript, TransactionWitness,
    },
    vm::{AdviceInputs, StackInputs},
    Felt, Word, ZERO,
};

use super::TransactionKernel;

// TRANSACTION KERNEL INPUTS
// ================================================================================================

/// Defines how inputs required to execute a transaction kernel can be extracted from self.
pub trait ToTransactionKernelInputs {
    /// Returns stack and advice inputs required to execute the transaction kernel.
    fn get_kernel_inputs(&self) -> (StackInputs, AdviceInputs);
}

impl ToTransactionKernelInputs for PreparedTransaction {
    fn get_kernel_inputs(&self) -> (StackInputs, AdviceInputs) {
        let account = self.account();
        let stack_inputs = TransactionKernel::build_input_stack(
            account.id(),
            account.proof_init_hash(),
            self.input_notes().commitment(),
            self.block_header().hash(),
        );

        let mut advice_inputs = AdviceInputs::default();
        extend_advice_inputs(self.tx_inputs(), self.tx_args(), &mut advice_inputs);

        (stack_inputs, advice_inputs)
    }
}

impl ToTransactionKernelInputs for ExecutedTransaction {
    fn get_kernel_inputs(&self) -> (StackInputs, AdviceInputs) {
        let account = self.initial_account();
        let stack_inputs = TransactionKernel::build_input_stack(
            account.id(),
            account.proof_init_hash(),
            self.input_notes().commitment(),
            self.block_header().hash(),
        );

        let mut advice_inputs = self.advice_witness().clone();
        extend_advice_inputs(self.tx_inputs(), self.tx_args(), &mut advice_inputs);

        (stack_inputs, advice_inputs)
    }
}

impl ToTransactionKernelInputs for TransactionWitness {
    fn get_kernel_inputs(&self) -> (StackInputs, AdviceInputs) {
        let account = self.account();

        let stack_inputs = TransactionKernel::build_input_stack(
            account.id(),
            account.proof_init_hash(),
            self.input_notes().commitment(),
            self.block_header().hash(),
        );

        let mut advice_inputs = self.advice_witness().clone();
        extend_advice_inputs(self.tx_inputs(), self.tx_args(), &mut advice_inputs);

        (stack_inputs, advice_inputs)
    }
}

// ADVICE INPUTS
// ================================================================================================

/// Extends the provided advice inputs with the data required for executing a transaction with the
/// specified inputs.
///
/// This includes the initial account, an optional account seed (required for new accounts), and
/// the input note data, including core note data + authentication paths all the way to the root
/// of one of chain MMR peaks.
fn extend_advice_inputs(
    tx_inputs: &TransactionInputs,
    tx_args: &TransactionArgs,
    advice_inputs: &mut AdviceInputs,
) {
    // build the advice stack
    build_advice_stack(tx_inputs, tx_args.tx_script(), advice_inputs);

    // build the advice map and Merkle store for relevant components
    add_chain_mmr_to_advice_inputs(tx_inputs.block_chain(), advice_inputs);
    add_account_to_advice_inputs(tx_inputs.account(), tx_inputs.account_seed(), advice_inputs);
    add_input_notes_to_advice_inputs(tx_inputs.input_notes(), tx_args, advice_inputs);
    advice_inputs.extend_map(tx_args.advice_map().clone());
}

// ADVICE STACK BUILDER
// ------------------------------------------------------------------------------------------------

/// Builds the advice stack for the provided transaction inputs.
///
/// The advice stack is arranged as follows:
///  elements[0..3]    = hash of previous block
///  elements[4..7]    = chain MMR hash
///  elements[8..11]   = account root
///  elements[12..15]  = nullifier root
///  elements[16..19]  = batch root
///  elements[20..23]  = proof hash
///  elements[24..27]  = [block_num, version, timestamp, ZERO]
///  elements[28..31]  = [ZERO; 4]
///  elements[32..35]  = notes root
///  elements[36..39]  = [account ID, ZERO, ZERO, account nonce]
///  elements[40..43]  = account vault root
///  elements[44..47]  = account storage root
///  elements[48..51]  = account code root
///  elements[52]      = number of input notes
///  elements[53..57]  = account seed, if one was provided; otherwise [ZERO; 4]
fn build_advice_stack(
    tx_inputs: &TransactionInputs,
    tx_script: Option<&TransactionScript>,
    inputs: &mut AdviceInputs,
) {
    // push block header info into the stack
    let header = tx_inputs.block_header();
    inputs.extend_stack(header.prev_hash());
    inputs.extend_stack(header.chain_root());
    inputs.extend_stack(header.account_root());
    inputs.extend_stack(header.nullifier_root());
    inputs.extend_stack(header.batch_root());
    inputs.extend_stack(header.proof_hash());
    inputs.extend_stack([header.block_num().into(), header.version(), header.timestamp(), ZERO]);
    inputs.extend_stack([ZERO; 4]);
    inputs.extend_stack(header.note_root());

    // push core account items onto the stack
    let account = tx_inputs.account();
    inputs.extend_stack([account.id().into(), ZERO, ZERO, account.nonce()]);
    inputs.extend_stack(account.vault().commitment());
    inputs.extend_stack(account.storage().root());
    inputs.extend_stack(account.code().root());

    // push the number of input notes onto the stack
    inputs.extend_stack([Felt::from(tx_inputs.input_notes().num_notes() as u32)]);

    // push tx_script root onto the stack
    if let Some(tx_script) = tx_script {
        // insert the transaction script hash into the advice stack
        inputs.extend_stack(*tx_script.hash());
    } else {
        // if no transaction script is provided, extend the advice stack with an empty transaction
        // script root
        inputs.extend_stack(Word::default());
    }
}

// CHAIN MMR INJECTOR
// ------------------------------------------------------------------------------------------------

/// Inserts the chain MMR data into the provided advice inputs.
///
/// Inserts the following items into the Merkle store:
/// - Inner nodes of all authentication paths contained in the chain MMR.
///
/// Inserts the following entries into the advice map:
/// - peaks_hash |-> MMR peaks info
///
/// where MMR peaks info has the following layout:
///  elements[0]       = number of leaves in the MMR
///  elements[1..4]    = padding ([Felt::ZERO; 3])
///  elements[4..]     = MMR peak roots
fn add_chain_mmr_to_advice_inputs(mmr: &ChainMmr, inputs: &mut AdviceInputs) {
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
/// - The Merkle nodes associated with the account code procedures tree.
/// - If present, the Merkle nodes associated with the account storage maps.
///
/// Inserts the following entries into the advice map:
/// - The storage types commitment |-> storage slot types vector.
/// - The account procedure root |-> procedure index, for each account procedure.
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
    if !(account.storage().maps().is_empty()) {
        for map in account.storage().maps() {
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

    // extend the merkle store with account code tree
    inputs.extend_merkle_store(code.procedure_tree().inner_nodes());

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
/// For each note the authentication path is populated into the Merkle store, the note inputs
/// and assets are populated in the advice map.
///
/// A combined note data vector is also constructed that holds core data for all notes. This
/// combined vector is added to the advice map against the input notes commitment. For each note
/// the following data items are added to the vector:
///   out[0..4]    = serial_num
///   out[4..8]    = script_root
///   out[8..12]   = inputs_hash
///   out[12..16]  = assets_hash
///   out[16..20]  = metadata
///   out[20..24]  = note_args
///   out[24]      = num_inputs
///   out[25]      = num_assets
///   out[26..30]  = asset_1
///   out[30..34]  = asset_2
///   ...
///   out[34 + num_assets * 4..] = Word::default() (this is conditional padding only applied
///                                                 if the number of assets is odd)
///   out[-10]      = origin.block_number
///   out[-9..-5]   = origin.SUB_HASH
///   out[-5..-1]   = origin.NOTE_ROOT
///   out[-1]       = origin.node_index
///
/// Inserts the following items into the Merkle store:
/// - The Merkle nodes associated with the note's authentication path.
///
/// Inserts the following entries into the advice map:
/// - inputs_hash |-> inputs
/// - asset_hash |-> assets
/// - notes_hash |-> combined note data
fn add_input_notes_to_advice_inputs(
    notes: &InputNotes,
    tx_args: &TransactionArgs,
    inputs: &mut AdviceInputs,
) {
    // if there are no input notes, nothing is added to the advice inputs
    if notes.is_empty() {
        return;
    }

    let mut note_data = Vec::new();
    for input_note in notes.iter() {
        let note = input_note.note();
        let assets = note.assets();
        let proof = input_note.proof();
        let recipient = note.recipient();
        let note_arg = tx_args.get_note_args(note.id()).unwrap_or(&[ZERO; 4]);

        // insert note inputs and assets into the advice map
        inputs
            .extend_map([(recipient.inputs().commitment(), recipient.inputs().to_padded_values())]);
        inputs.extend_map([(assets.commitment(), assets.to_padded_assets())]);

        // insert note authentication path nodes into the Merkle store
        inputs.extend_merkle_store(
            proof
                .note_path()
                .inner_nodes(proof.origin().node_index.value(), note.authentication_hash())
                .unwrap(),
        );

        // add the note elements to the combined vector of note data
        note_data.extend(recipient.serial_num());
        note_data.extend(*recipient.script().hash());
        note_data.extend(*recipient.inputs().commitment());
        note_data.extend(*assets.commitment());

        note_data.extend(Word::from(note.metadata()));
        note_data.extend(Word::from(*note_arg));

        note_data.push(recipient.inputs().num_values().into());

        note_data.push((assets.num_assets() as u32).into());
        note_data.extend(assets.to_padded_assets());

        note_data.push(proof.origin().block_num.into());
        note_data.extend(*proof.sub_hash());
        note_data.extend(*proof.note_root());
        note_data.push(
            proof
                .origin()
                .node_index
                .value()
                .try_into()
                .expect("value is greater than or equal to the field modulus"),
        );
    }

    // insert the combined note data into the advice map
    inputs.extend_map([(notes.commitment(), note_data)]);
}
