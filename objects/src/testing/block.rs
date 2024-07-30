use alloc::{collections::BTreeMap, vec::Vec};
use core::fmt;

use miden_crypto::merkle::{Mmr, PartialMmr, SimpleSmt, Smt};
use vm_core::{utils::Serializable, Felt, Word, ZERO};
use vm_processor::Digest;
#[cfg(not(target_family = "wasm"))]
use winter_rand_utils as rand;

use crate::{
    accounts::{delta::AccountUpdateDetails, Account},
    block::{Block, BlockAccountUpdate, BlockNoteIndex, BlockNoteTree, NoteBatch},
    notes::{Note, NoteId, NoteInclusionProof, Nullifier},
    transaction::{
        ChainMmr, ExecutedTransaction, InputNote, InputNotes, OutputNote, ToInputNoteCommitments,
        TransactionId, TransactionInputs,
    },
    BlockHeader, ACCOUNT_TREE_DEPTH,
};

/// Initial timestamp value
const TIMESTAMP_START: u32 = 1693348223;
/// Timestamp of timestamp on each new block
const TIMESTAMP_STEP: u32 = 10;

#[derive(Default, Debug, Clone)]
pub struct PendingObjects {
    /// Account updates for the block.
    updated_accounts: Vec<BlockAccountUpdate>,

    /// Note batches created in transactions in the block.
    output_note_batches: Vec<NoteBatch>,

    /// Nullifiers produced in transactions in the block.
    created_nullifiers: Vec<Nullifier>,

    /// Transaction IDs added to the block.
    transaction_ids: Vec<TransactionId>,
}

impl PendingObjects {
    pub fn new() -> PendingObjects {
        PendingObjects {
            updated_accounts: vec![],
            output_note_batches: vec![],
            created_nullifiers: vec![],
            transaction_ids: vec![],
        }
    }

    /// Creates a [BlockNoteTree] tree from the `notes`.
    ///
    /// The root of the tree is a commitment to all notes created in the block. The commitment
    /// is not for all fields of the [Note] struct, but only for note metadata + core fields of
    /// a note (i.e., vault, inputs, script, and serial number).
    pub fn build_notes_tree(&self) -> BlockNoteTree {
        let entries =
            self.output_note_batches.iter().enumerate().flat_map(|(batch_index, batch)| {
                batch.iter().enumerate().map(move |(note_index, note)| {
                    (
                        BlockNoteIndex::new(batch_index, note_index),
                        note.id().into(),
                        *note.metadata(),
                    )
                })
            });

        BlockNoteTree::with_entries(entries).unwrap()
    }
}

/// Structure chain data, used to build necessary openings and to construct [BlockHeader]s.
#[derive(Debug, Clone)]
pub struct MockChain {
    /// An append-only structure used to represent the history of blocks produced for this chain.
    chain: Mmr,

    /// History of produced blocks.
    blocks: Vec<Block>,

    /// Tree containing the latest `Nullifier`'s tree.
    nullifiers: Smt,

    /// Tree containing the latest hash of each account.
    accounts: SimpleSmt<ACCOUNT_TREE_DEPTH>,

    /// Objects that have not yet been finalized.
    ///
    /// These will become available once the block is sealed.
    ///
    /// Note:
    /// - The [Note]s in this container do not have the `proof` set.
    pending_objects: PendingObjects,

    /// NoteID |-> InputNote mapping to simplify transaction inputs retrieval
    available_notes: BTreeMap<NoteId, InputNote>,

    removed_notes: Vec<NoteId>,
}

#[derive(Debug)]
pub enum MockError {
    DuplicatedNullifier,
    DuplicatedNote,
}

impl fmt::Display for MockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MockError {}

impl Default for MockChain {
    fn default() -> Self {
        Self::new()
    }
}

impl MockChain {
    // CONSTRUCTORS
    // ----------------------------------------------------------------------------------------

    pub fn new() -> Self {
        Self {
            chain: Mmr::default(),
            blocks: vec![],
            nullifiers: Smt::default(),
            accounts: SimpleSmt::<ACCOUNT_TREE_DEPTH>::new().expect("depth too big for SimpleSmt"),
            pending_objects: PendingObjects::new(),
            available_notes: BTreeMap::new(),
            removed_notes: vec![],
        }
    }

    pub fn add_executed_transaction(&mut self, transaction: ExecutedTransaction) {
        let mut account = transaction.initial_account().clone();
        account.apply_delta(transaction.account_delta()).unwrap();

        // disregard private accounts, so it's easier to retrieve data
        let account_update_details = match account.is_new() {
            true => AccountUpdateDetails::New(account.clone()),
            false => AccountUpdateDetails::Delta(transaction.account_delta().clone()),
        };

        let block_account_update = BlockAccountUpdate::new(
            transaction.account_id(),
            account.hash(),
            account_update_details,
            vec![transaction.id()],
        );
        self.pending_objects.updated_accounts.push(block_account_update);

        for note in transaction.input_notes().iter() {
            // TODO: check that nullifiers are not duplicate
            self.pending_objects.created_nullifiers.push(note.nullifier());
            self.removed_notes.push(note.id());
        }

        // TODO: check that notes are not duplicate
        let output_notes: Vec<OutputNote> = transaction.output_notes().iter().cloned().collect();
        self.pending_objects.output_note_batches.push(output_notes);
    }

    /// Add a public [Note] to the pending objects.
    /// A block has to be created to finalize the new entity.
    pub fn add_note(&mut self, note: Note) {
        self.pending_objects.output_note_batches.push(vec![OutputNote::Full(note)]);
    }

    /// Mark a [Note] as consumed by inserting its nullifier into the block.
    /// A block has to be created to finalize the new entity.
    pub fn add_nullifier(&mut self, nullifier: Nullifier) {
        self.pending_objects.created_nullifiers.push(nullifier);
    }

    /// Add a new [Account] to the list of pending objects.
    /// A block has to be created to finalize the new entity.
    pub fn add_account(&mut self, account: Account, _seed: Option<Word>) {
        self.pending_objects.updated_accounts.push(BlockAccountUpdate::new(
            account.id(),
            account.hash(),
            AccountUpdateDetails::New(account),
            vec![],
        ));
    }

    pub fn get_transaction_inputs(
        &self,
        account: Account,
        account_seed: Option<Word>,
        notes: &[NoteId],
    ) -> TransactionInputs {
        let block_header = self.blocks.last().unwrap().header();

        let mut input_notes = vec![];
        let mut block_headers_map: BTreeMap<u32, BlockHeader> = BTreeMap::new();
        for note in notes {
            let input_note = self.available_notes.get(note).unwrap().clone();
            block_headers_map.insert(
                input_note.location().unwrap().block_num,
                self.blocks
                    .get(input_note.location().unwrap().block_num as usize)
                    .unwrap()
                    .header(),
            );
            input_notes.push(input_note);
        }

        let block_headers: Vec<BlockHeader> = block_headers_map.values().cloned().collect();
        let mmr = mmr_to_chain_mmr(&self.chain, &block_headers);

        TransactionInputs::new(
            account,
            account_seed,
            block_header,
            mmr,
            InputNotes::new(input_notes).unwrap(),
        )
        .unwrap()
    }

    // MODIFIERS
    // ----------------------------------------------------------------------------------------

    /// Creates the next block.
    ///
    /// This will also make all the objects currently pending available for use.
    pub fn seal_block(&mut self) -> Block {
        let block_num: u32 = self.blocks.len().try_into().expect("usize to u32 failed");

        for update in self.pending_objects.updated_accounts.iter() {
            self.accounts.insert(update.account_id().into(), *update.new_state_hash());
        }

        // TODO:
        // - resetting the nullifier tree once defined at the protocol level.
        // - inserting only nullifier from transactions included in the batches, once the batch
        // kernel has been implemented.
        for nullifier in self.pending_objects.created_nullifiers.iter() {
            self.nullifiers.insert(nullifier.inner(), [block_num.into(), ZERO, ZERO, ZERO]);
        }
        let notes_tree = self.pending_objects.build_notes_tree();

        let version = 0;
        let previous = self.blocks.last();
        let peaks = self.chain.peaks(self.chain.forest()).unwrap();
        let chain_root: Digest = peaks.hash_peaks();
        let account_root = self.accounts.root();
        let prev_hash = previous.map_or(Digest::default(), |block| block.hash());
        let nullifier_root = self.nullifiers.root();
        let note_root = notes_tree.root();
        let timestamp =
            previous.map_or(TIMESTAMP_START, |block| block.header().timestamp() + TIMESTAMP_STEP);
        // TODO: Implement proper tx_hash once https://github.com/0xPolygonMiden/miden-base/pull/740 is merged
        let tx_hash = crate::Hasher::hash(&self.pending_objects.transaction_ids.to_bytes());

        // TODO: Set `proof_hash` to the correct value once the kernel is available.
        let proof_hash = Digest::default();

        let header = BlockHeader::new(
            version,
            prev_hash,
            block_num,
            chain_root,
            account_root,
            nullifier_root,
            note_root,
            tx_hash,
            proof_hash,
            timestamp,
        );

        let block = Block::new(
            header,
            self.pending_objects.updated_accounts.clone(),
            self.pending_objects.output_note_batches.clone(),
            self.pending_objects.created_nullifiers.clone(),
        )
        .unwrap();

        for (batch_index, note_batch) in self.pending_objects.output_note_batches.iter().enumerate()
        {
            for (note_index, note) in note_batch.iter().enumerate() {
                // All note details should be OutputNote::Full at this point
                match note {
                    OutputNote::Full(note) => {
                        let block_note_index = BlockNoteIndex::new(batch_index, note_index);
                        let note_path = notes_tree.get_note_path(block_note_index).unwrap();
                        let note_inclusion_proof = NoteInclusionProof::new(
                            block.header().block_num(),
                            block.header().sub_hash(),
                            block.header().note_root(),
                            block_note_index.to_absolute_index(),
                            note_path,
                        )
                        .unwrap();

                        self.available_notes.insert(
                            note.id(),
                            InputNote::authenticated(note.clone(), note_inclusion_proof),
                        );
                    },
                    _ => continue,
                }
            }
        }

        for removed_note in self.removed_notes.iter() {
            self.available_notes.remove(removed_note);
        }

        self.blocks.push(block.clone());
        self.chain.add(header.hash());
        self.reset_pending();

        block
    }

    fn reset_pending(&mut self) {
        self.pending_objects = PendingObjects::new();
        self.removed_notes = vec![];
    }

    // ACCESSORS
    // ----------------------------------------------------------------------------------------

    /// Get the latest [ChainMmr].
    pub fn chain(&self) -> ChainMmr {
        let block_headers: Vec<BlockHeader> = self.blocks.iter().map(|b| b.header()).collect();
        mmr_to_chain_mmr(&self.chain, &block_headers)
    }

    /// Get a reference to [BlockHeader] with `block_number`.
    pub fn block_header(&self, block_number: usize) -> BlockHeader {
        self.blocks[block_number].header()
    }

    /// Get a reference to the nullifier tree.
    pub fn nullifiers(&self) -> &Smt {
        &self.nullifiers
    }

    pub fn available_notes(&self) -> Vec<InputNote> {
        self.available_notes.values().cloned().collect()
    }
}

impl BlockHeader {
    /// Creates a mock block. The account tree is formed from the provided `accounts`,
    /// and the chain root and note root are set to the provided `chain_root` and `note_root`
    /// values respectively.
    ///
    /// For non-WASM targets, the remaining header values are initialized randomly. For WASM
    /// targets, values are initialized to [Default::default()]
    pub fn mock(
        block_num: u32,
        chain_root: Option<Digest>,
        note_root: Option<Digest>,
        accounts: &[Account],
    ) -> Self {
        let acct_db = SimpleSmt::<ACCOUNT_TREE_DEPTH>::with_leaves(
            accounts
                .iter()
                .flat_map(|acct| {
                    if acct.is_new() {
                        None
                    } else {
                        let felt_id: Felt = acct.id().into();
                        Some((felt_id.as_int(), *acct.hash()))
                    }
                })
                .collect::<Vec<_>>(),
        )
        .expect("failed to create account db");
        let acct_root = acct_db.root();

        #[cfg(not(target_family = "wasm"))]
        let (prev_hash, chain_root, nullifier_root, note_root, tx_hash, proof_hash, timestamp) = {
            let prev_hash = rand::rand_array().into();
            let chain_root = chain_root.unwrap_or(rand::rand_array().into());
            let nullifier_root = rand::rand_array().into();
            let note_root = note_root.unwrap_or(rand::rand_array().into());
            let tx_hash = rand::rand_array().into();
            let proof_hash = rand::rand_array().into();
            let timestamp = rand::rand_value();

            (prev_hash, chain_root, nullifier_root, note_root, tx_hash, proof_hash, timestamp)
        };

        #[cfg(target_family = "wasm")]
        let (prev_hash, chain_root, nullifier_root, note_root, tx_hash, proof_hash, timestamp) =
            Default::default();

        BlockHeader::new(
            0,
            prev_hash,
            block_num,
            chain_root,
            acct_root,
            nullifier_root,
            note_root,
            tx_hash,
            proof_hash,
            timestamp,
        )
    }
}

pub struct MockChainBuilder {
    accounts: Vec<Account>,
    notes: Vec<Note>,
}

impl Default for MockChainBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MockChainBuilder {
    pub fn new() -> Self {
        Self { accounts: vec![], notes: vec![] }
    }

    pub fn accounts(mut self, accounts: Vec<Account>) -> Self {
        self.accounts = accounts;
        self
    }

    pub fn notes(mut self, notes: Vec<Note>) -> Self {
        self.notes = notes;
        self
    }

    /// Returns a [MockChain] with a single block
    pub fn build(self) -> MockChain {
        let mut chain = MockChain::new();
        for account in self.accounts {
            chain.add_account(account, None);
        }

        for note in self.notes {
            chain.add_note(note);
        }

        chain.seal_block();
        chain
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Converts the MMR into partial MMR by copying all leaves from MMR to partial MMR.
pub fn mmr_to_chain_mmr(mmr: &Mmr, blocks: &[BlockHeader]) -> ChainMmr {
    let target_forest = mmr.forest() - 1;
    let mut partial_mmr = PartialMmr::from_peaks(mmr.peaks(target_forest).unwrap());

    for i in 0..target_forest {
        let node = mmr.get(i).unwrap();
        let path = mmr.open(i, target_forest).unwrap().merkle_path;
        partial_mmr.track(i, node, &path).unwrap();
    }

    ChainMmr::new(partial_mmr, blocks.to_vec()).unwrap()
}
