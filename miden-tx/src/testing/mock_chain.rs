use alloc::{collections::BTreeMap, vec::Vec};
use core::fmt;

use miden_lib::{notes::create_p2id_note, transaction::TransactionKernel};
use miden_objects::{
    accounts::{
        delta::AccountUpdateDetails, Account, AccountDelta, AccountId, AccountType, AuthSecretKey,
        StorageSlot,
    },
    assets::{Asset, FungibleAsset, TokenSymbol},
    block::{compute_tx_hash, Block, BlockAccountUpdate, BlockNoteIndex, BlockNoteTree, NoteBatch},
    crypto::merkle::{Mmr, MmrError, PartialMmr, Smt},
    notes::{Note, NoteId, NoteInclusionProof, NoteType, Nullifier},
    testing::account::AccountBuilder,
    transaction::{
        ChainMmr, ExecutedTransaction, InputNote, InputNotes, OutputNote, ToInputNoteCommitments,
        TransactionId, TransactionInputs,
    },
    AccountError, BlockHeader, FieldElement, NoteError, ACCOUNT_TREE_DEPTH,
};
use rand::{rngs::StdRng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use vm_processor::{
    crypto::{RpoRandomCoin, SimpleSmt},
    Digest, Felt, Word, ZERO,
};

use super::TransactionContextBuilder;
use crate::auth::BasicAuthenticator;

// CONSTANTS
// ================================================================================================

/// Initial timestamp value
const TIMESTAMP_START: u32 = 1693348223;
/// Timestamp of timestamp on each new block
const TIMESTAMP_STEP: u32 = 10;

pub type MockAuthenticator = BasicAuthenticator<StdRng>;

// MOCK FUNGIBLE FAUCET
// ================================================================================================

/// Represents a fungible faucet that exists on the MockChain.
pub struct MockFungibleFaucet(Account);

impl MockFungibleFaucet {
    pub fn account(&self) -> &Account {
        &self.0
    }

    pub fn mint(&self, amount: u64) -> Asset {
        FungibleAsset::new(self.0.id(), amount).unwrap().into()
    }
}

// MOCK ACCOUNT
// ================================================================================================

/// Represents a mock account that exists on the MockChain.
/// It optionally includes the seed, and an authenticator that can be used for generating
/// valid transaction contexts.
#[derive(Clone, Debug)]
struct MockAccount {
    account: Account,
    seed: Option<Word>,
    authenticator: Option<BasicAuthenticator<StdRng>>,
}

impl MockAccount {
    pub fn new(
        account: Account,
        seed: Option<Word>,
        authenticator: Option<BasicAuthenticator<StdRng>>,
    ) -> Self {
        MockAccount { account, seed, authenticator }
    }

    #[allow(dead_code)]
    pub fn apply_delta(&mut self, delta: &AccountDelta) -> Result<(), AccountError> {
        self.account.apply_delta(delta)
    }

    pub fn account(&self) -> &Account {
        &self.account
    }

    pub fn seed(&self) -> Option<&Word> {
        self.seed.as_ref()
    }

    pub fn authenticator(&self) -> &Option<BasicAuthenticator<StdRng>> {
        &self.authenticator
    }
}

// PENDING OBJECTS
// ================================================================================================

/// Aggregates all entities that were added to the blockchain in the last block (not yet finalized)
#[derive(Default, Debug, Clone)]
struct PendingObjects {
    /// Account updates for the block.
    updated_accounts: Vec<BlockAccountUpdate>,

    /// Note batches created in transactions in the block.
    output_note_batches: Vec<NoteBatch>,

    /// Nullifiers produced in transactions in the block.
    created_nullifiers: Vec<Nullifier>,

    /// Transaction IDs added to the block.
    included_transactions: Vec<(TransactionId, AccountId)>,
}

impl PendingObjects {
    pub fn new() -> PendingObjects {
        PendingObjects {
            updated_accounts: vec![],
            output_note_batches: vec![],
            created_nullifiers: vec![],
            included_transactions: vec![],
        }
    }

    /// Creates a [BlockNoteTree] tree from the `notes`.
    ///
    /// The root of the tree is a commitment to all notes created in the block. The commitment
    /// is not for all fields of the [Note] struct, but only for note metadata + core fields of
    /// a note (i.e., vault, inputs, script, and serial number).
    pub fn build_note_tree(&self) -> BlockNoteTree {
        let entries =
            self.output_note_batches.iter().enumerate().flat_map(|(batch_index, batch)| {
                batch.iter().enumerate().map(move |(note_index, note)| {
                    (
                        BlockNoteIndex::new(batch_index, note_index).unwrap(),
                        note.id(),
                        *note.metadata(),
                    )
                })
            });

        BlockNoteTree::with_entries(entries).unwrap()
    }
}

#[derive(Debug)]
pub enum MockError {
    DuplicatedNullifier,
    DuplicatedNote,
}

// AUTH
// ================================================================================================

/// Specifies which authentication mechanism is desired for accounts
pub enum Auth {
    /// Creates a [SecretKey](miden_objects::crypto::dsa::rpo_falcon512::SecretKey) for the
    /// account and creates a [BasicAuthenticator] that gets used for authenticating the account
    BasicAuth,

    /// Does not create any authentication mechanism for the account.
    NoAuth,
}

impl fmt::Display for MockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MockError {}

// MOCK CHAIN
// ================================================================================================

/// Structure chain data, used to build necessary openings and to construct [BlockHeader].
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

    /// AccountId |-> Account mapping to simplify transaction creation
    available_accounts: BTreeMap<AccountId, MockAccount>,

    removed_notes: Vec<NoteId>,
}

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
            available_accounts: BTreeMap::new(),
            removed_notes: vec![],
        }
    }

    pub fn add_executed_transaction(&mut self, transaction: ExecutedTransaction) {
        let mut account = transaction.initial_account().clone();
        account.apply_delta(transaction.account_delta()).unwrap();

        // disregard private accounts, so it's easier to retrieve data
        let account_update_details = AccountUpdateDetails::New(account.clone());

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
        self.pending_objects
            .included_transactions
            .push((transaction.id(), transaction.account_id()));
    }

    /// Add a public [Note] to the pending objects.
    /// A block has to be created to finalize the new entity.
    pub fn add_note(&mut self, note: Note) {
        self.pending_objects.output_note_batches.push(vec![OutputNote::Full(note)]);
    }

    /// Add a P2ID [Note] to the pending objects and returns it.
    /// A block has to be created to finalize the new entity.
    pub fn add_p2id_note(
        &mut self,
        sender_account_id: AccountId,
        target_account_id: AccountId,
        asset: &[Asset],
        note_type: NoteType,
    ) -> Result<Note, NoteError> {
        let mut rng = RpoRandomCoin::new(Word::default());

        let note = create_p2id_note(
            sender_account_id,
            target_account_id,
            asset.to_vec(),
            note_type,
            Default::default(),
            &mut rng,
        )?;

        self.add_note(note.clone());

        Ok(note)
    }

    /// Mark a [Note] as consumed by inserting its nullifier into the block.
    /// A block has to be created to finalize the new entity.
    pub fn add_nullifier(&mut self, nullifier: Nullifier) {
        self.pending_objects.created_nullifiers.push(nullifier);
    }

    // OTHER IMPLEMENTATIONS
    // ================================================================================================

    pub fn add_new_wallet(&mut self, auth_method: Auth, assets: Vec<Asset>) -> Account {
        let account_builder = AccountBuilder::new(ChaCha20Rng::from_entropy())
            .default_code(TransactionKernel::testing_assembler())
            .nonce(Felt::ZERO)
            .add_assets(assets);
        self.add_from_account_builder(auth_method, account_builder)
    }

    pub fn add_existing_wallet(&mut self, auth_method: Auth, assets: Vec<Asset>) -> Account {
        let account_builder = AccountBuilder::new(ChaCha20Rng::from_entropy())
            .default_code(TransactionKernel::testing_assembler())
            .nonce(Felt::ONE)
            .add_assets(assets);
        self.add_from_account_builder(auth_method, account_builder)
    }

    pub fn add_new_faucet(
        &mut self,
        auth_method: Auth,
        token_symbol: &str,
        max_supply: u64,
    ) -> MockFungibleFaucet {
        let metadata: [Felt; 4] = [
            max_supply.try_into().unwrap(),
            Felt::new(10),
            TokenSymbol::new(token_symbol).unwrap().into(),
            ZERO,
        ];

        let faucet_metadata = StorageSlot::Value(metadata);

        let account_builder = AccountBuilder::new(ChaCha20Rng::from_entropy())
            .default_code(TransactionKernel::testing_assembler())
            .nonce(Felt::ZERO)
            .account_type(AccountType::FungibleFaucet)
            .add_storage_slot(faucet_metadata);

        let account = self.add_from_account_builder(auth_method, account_builder);

        MockFungibleFaucet(account)
    }

    pub fn add_existing_faucet(
        &mut self,
        auth_method: Auth,
        token_symbol: &str,
        max_supply: u64,
    ) -> MockFungibleFaucet {
        let metadata: [Felt; 4] = [
            max_supply.try_into().unwrap(),
            Felt::new(10),
            TokenSymbol::new(token_symbol).unwrap().into(),
            ZERO,
        ];

        let faucet_metadata = StorageSlot::Value(metadata);

        let account_builder = AccountBuilder::new(ChaCha20Rng::from_entropy())
            .default_code(TransactionKernel::testing_assembler())
            .nonce(Felt::ONE)
            .account_type(AccountType::FungibleFaucet)
            .add_storage_slot(faucet_metadata);
        MockFungibleFaucet(self.add_from_account_builder(auth_method, account_builder))
    }

    /// Add a new [Account] from an [AccountBuilder] to the list of pending objects.
    /// A block has to be created to finalize the new entity.
    pub fn add_from_account_builder(
        &mut self,
        auth_method: Auth,
        account_builder: AccountBuilder<ChaCha20Rng>,
    ) -> Account {
        let (account, seed, authenticator) = match auth_method {
            Auth::BasicAuth => {
                let mut rng = StdRng::from_entropy();

                let (acc, seed, auth) = account_builder.build_with_auth(&mut rng).unwrap();

                let authenticator = BasicAuthenticator::<StdRng>::new(&[(
                    auth.public_key().into(),
                    AuthSecretKey::RpoFalcon512(auth),
                )]);

                (acc, seed, Some(authenticator))
            },
            Auth::NoAuth => {
                let (account, seed) = account_builder.build().unwrap();
                (account, seed, None)
            },
        };

        let seed = account.is_new().then_some(seed);
        self.available_accounts
            .insert(account.id(), MockAccount::new(account.clone(), seed, authenticator));
        self.add_account(account.clone());

        account
    }

    /// Add a new [Account] to the list of pending objects.
    /// A block has to be created to finalize the new entity.
    pub fn add_account(&mut self, account: Account) {
        self.pending_objects.updated_accounts.push(BlockAccountUpdate::new(
            account.id(),
            account.hash(),
            AccountUpdateDetails::New(account),
            vec![],
        ));
    }

    pub fn build_tx_context(&self, account_id: AccountId) -> TransactionContextBuilder {
        let mock_account = self.available_accounts.get(&account_id).unwrap();

        TransactionContextBuilder::new(mock_account.account().clone())
            .authenticator(mock_account.authenticator().clone())
            .account_seed(mock_account.seed().cloned())
            .mock_chain(self.clone())
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
            let note_block_num = input_note.location().unwrap().block_num();
            let block_header = self.blocks.get(note_block_num as usize).unwrap().header();
            block_headers_map.insert(note_block_num, block_header);
            input_notes.push(input_note);
        }

        let block_headers: Vec<BlockHeader> = block_headers_map.values().cloned().collect();
        let mmr = mmr_to_chain_mmr(&self.chain, &block_headers).unwrap();

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
    // =========================================================================================

    /// Creates the next block.
    ///
    /// This will also make all the objects currently pending available for use.
    /// If `block_num` is `Some(number)`, `number` will be used as the new block's number
    pub fn seal_block(&mut self, block_num: Option<u32>) -> Block {
        let next_block_num = self.blocks.last().map_or(0, |b| b.header().block_num() + 1);
        let block_num: u32 = if let Some(input_block_num) = block_num {
            if input_block_num < next_block_num {
                panic!("Input block number should be higher than the last block number");
            }
            input_block_num
        } else {
            next_block_num
        };

        for update in self.pending_objects.updated_accounts.iter() {
            self.accounts.insert(update.account_id().into(), *update.new_state_hash());

            if let Some(mock_account) = self.available_accounts.get(&update.account_id()) {
                let account = match update.details() {
                    AccountUpdateDetails::New(acc) => acc.clone(),
                    _ => panic!("The mockchain should have full account details"),
                };
                self.available_accounts.insert(
                    update.account_id(),
                    MockAccount::new(
                        account,
                        mock_account.seed,
                        mock_account.authenticator.clone(),
                    ),
                );
            }
        }

        // TODO:
        // - resetting the nullifier tree once defined at the protocol level.
        // - inserting only nullifier from transactions included in the batches, once the batch
        // kernel has been implemented.
        for nullifier in self.pending_objects.created_nullifiers.iter() {
            self.nullifiers.insert(nullifier.inner(), [block_num.into(), ZERO, ZERO, ZERO]);
        }
        let notes_tree = self.pending_objects.build_note_tree();

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
        let tx_hash =
            compute_tx_hash(self.pending_objects.included_transactions.clone().into_iter());

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
                        let block_note_index =
                            BlockNoteIndex::new(batch_index, note_index).unwrap();
                        let note_path = notes_tree.get_note_path(block_note_index);
                        let note_inclusion_proof = NoteInclusionProof::new(
                            block.header().block_num(),
                            block_note_index.leaf_index_value(),
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
    // =========================================================================================

    /// Get the latest [ChainMmr].
    pub fn chain(&self) -> ChainMmr {
        let block_headers: Vec<BlockHeader> = self.blocks.iter().map(|b| b.header()).collect();
        mmr_to_chain_mmr(&self.chain, &block_headers).unwrap()
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

// MOCK CHAIN BUILDER
// ================================================================================================

#[derive(Default)]
pub struct MockChainBuilder {
    accounts: Vec<Account>,
    notes: Vec<Note>,
    starting_block_num: u32,
}

impl MockChainBuilder {
    pub fn accounts(mut self, accounts: Vec<Account>) -> Self {
        self.accounts = accounts;
        self
    }

    pub fn notes(mut self, notes: Vec<Note>) -> Self {
        self.notes = notes;
        self
    }

    pub fn starting_block_num(mut self, block_num: u32) -> Self {
        self.starting_block_num = block_num;
        self
    }

    /// Returns a [MockChain] with a single block
    pub fn build(self) -> MockChain {
        let mut chain = MockChain::new();
        for account in self.accounts {
            chain.add_account(account);
        }

        for note in self.notes {
            chain.add_note(note);
        }

        chain.seal_block(Some(self.starting_block_num));
        chain
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Converts the MMR into partial MMR by copying all leaves from MMR to partial MMR.
fn mmr_to_chain_mmr(mmr: &Mmr, blocks: &[BlockHeader]) -> Result<ChainMmr, MmrError> {
    let target_forest = mmr.forest() - 1;
    let mut partial_mmr = PartialMmr::from_peaks(mmr.peaks(target_forest)?);

    for i in 0..target_forest {
        let node = mmr.get(i)?;
        let path = mmr.open(i, target_forest)?.merkle_path;
        partial_mmr.track(i, node, &path)?;
    }

    Ok(ChainMmr::new(partial_mmr, blocks.to_vec()).unwrap())
}
