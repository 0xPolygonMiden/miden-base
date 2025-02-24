use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

use miden_lib::{
    account::{auth::RpoFalcon512, faucets::BasicFungibleFaucet, wallets::BasicWallet},
    note::{create_p2id_note, create_p2idr_note},
    transaction::{memory, TransactionKernel},
};
use miden_objects::{
    account::{
        delta::AccountUpdateDetails, Account, AccountBuilder, AccountComponent, AccountDelta,
        AccountId, AccountIdAnchor, AccountType, AuthSecretKey,
    },
    asset::{Asset, FungibleAsset, TokenSymbol},
    batch::{ProposedBatch, ProvenBatch},
    block::{
        AccountWitness, BlockAccountUpdate, BlockHeader, BlockInputs, BlockNoteIndex,
        BlockNoteTree, BlockNumber, NullifierWitness, OutputNoteBatch, ProposedBlock, ProvenBlock,
    },
    crypto::{
        dsa::rpo_falcon512::SecretKey,
        merkle::{LeafIndex, Mmr, Smt},
    },
    note::{Note, NoteHeader, NoteId, NoteInclusionProof, NoteType, Nullifier},
    testing::account_code::DEFAULT_AUTH_SCRIPT,
    transaction::{
        ChainMmr, ExecutedTransaction, InputNote, InputNotes, OutputNote, ProvenTransaction,
        ToInputNoteCommitments, TransactionId, TransactionInputs, TransactionScript,
    },
    AccountError, NoteError, ProposedBatchError, ProposedBlockError, ACCOUNT_TREE_DEPTH,
};
use rand::{Rng, SeedableRng};
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
const TIMESTAMP_START_SECS: u32 = 1693348223;
/// Timestamp increment on each new block
const TIMESTAMP_STEP_SECS: u32 = 10;

// AUTH
// ================================================================================================

/// Specifies which authentication mechanism is desired for accounts
#[derive(Debug, Clone, Copy)]
pub enum Auth {
    /// Creates a [SecretKey] for the account and creates a [BasicAuthenticator] that gets used for
    /// authenticating the account.
    BasicAuth,

    /// Does not create any authentication mechanism for the account.
    NoAuth,
}

impl Auth {
    /// Converts `self` into its corresponding authentication [`AccountComponent`] and a
    /// [`BasicAuthenticator`] or `None` when [`Auth::NoAuth`] is passed.
    fn build_component(&self) -> Option<(AccountComponent, BasicAuthenticator<ChaCha20Rng>)> {
        match self {
            Auth::BasicAuth => {
                let mut rng = ChaCha20Rng::from_seed(Default::default());
                let sec_key = SecretKey::with_rng(&mut rng);
                let pub_key = sec_key.public_key();

                let component = RpoFalcon512::new(pub_key).into();

                let authenticator = BasicAuthenticator::<ChaCha20Rng>::new_with_rng(
                    &[(pub_key.into(), AuthSecretKey::RpoFalcon512(sec_key))],
                    rng,
                );

                Some((component, authenticator))
            },
            Auth::NoAuth => None,
        }
    }
}

// MOCK FUNGIBLE FAUCET
// ================================================================================================

/// Represents a fungible faucet that exists on the MockChain.
pub struct MockFungibleFaucet(Account);

impl MockFungibleFaucet {
    pub fn account(&self) -> &Account {
        &self.0
    }

    pub fn id(&self) -> AccountId {
        self.0.id()
    }

    pub fn mint(&self, amount: u64) -> Asset {
        FungibleAsset::new(self.0.id(), amount).unwrap().into()
    }
}

// MOCK ACCOUNT
// ================================================================================================

/// Represents a mock account that exists on the MockChain.
/// It optionally includes the seed, and an authenticator that can be used to authenticate
/// transaction contexts.
#[derive(Clone, Debug)]
struct MockAccount {
    account: Account,
    seed: Option<Word>,
    authenticator: Option<BasicAuthenticator<ChaCha20Rng>>,
}

impl MockAccount {
    pub fn new(
        account: Account,
        seed: Option<Word>,
        authenticator: Option<BasicAuthenticator<ChaCha20Rng>>,
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

    pub fn authenticator(&self) -> &Option<BasicAuthenticator<ChaCha20Rng>> {
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
    output_note_batches: Vec<OutputNoteBatch>,

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
    pub fn build_notes_tree(&self) -> BlockNoteTree {
        let entries =
            self.output_note_batches.iter().enumerate().flat_map(|(batch_index, batch)| {
                batch.iter().map(move |(note_index, note)| {
                    (
                        BlockNoteIndex::new(batch_index, *note_index).expect(
                            "max batches in block and max notes in batches should be enforced",
                        ),
                        note.id(),
                        *note.metadata(),
                    )
                })
            });

        BlockNoteTree::with_entries(entries).unwrap()
    }
}

// MOCK CHAIN
// ================================================================================================

/// [MockChain] simulates a simplified blockchain environment for testing purposes.
/// It allows to create and manage accounts, mint assets, execute transactions, and apply state
/// updates.
///
/// This struct is designed to mock transaction workflows, asset transfers, and
/// note creation in a test setting. Once entities are set up, [TransactionContextBuilder] objects
/// can be obtained in order to execute transactions accordingly.
///
/// # Examples
///
/// ## Create mock objects and build a transaction context
/// ```no_run
/// # use miden_tx::testing::{Auth, MockChain, TransactionContextBuilder};
/// # use miden_objects::{asset::FungibleAsset, Felt, note::NoteType};
/// let mut mock_chain = MockChain::new();
/// let faucet = mock_chain.add_new_faucet(Auth::BasicAuth, "USDT", 100_000);  // Create a USDT faucet
/// let asset = faucet.mint(1000);  
/// let sender = mock_chain.add_new_wallet(Auth::BasicAuth);  
/// let target = mock_chain.add_new_wallet(Auth::BasicAuth);  
/// let note = mock_chain
///     .add_p2id_note(
///         faucet.id(),
///         target.id(),
///         &[FungibleAsset::mock(10)],
///         NoteType::Public,
///       None,
///     )
///   .unwrap();
/// mock_chain.seal_block(None);
/// let tx_context = mock_chain.build_tx_context(sender.id(), &[note.id()], &[]).build();
/// let result = tx_context.execute();
/// ```
///
/// ## Executing a Simple Transaction
///
/// NOTE: Transaction script is defaulted to either [DEFAULT_AUTH_SCRIPT] if the account includes
/// an authenticator.
///
/// ```
/// # use miden_tx::testing::{Auth, MockChain, TransactionContextBuilder};
/// # use miden_objects::{asset::FungibleAsset, Felt, transaction::TransactionScript};
/// # use miden_lib::transaction::TransactionKernel;
/// let mut mock_chain = MockChain::new();
/// let sender = mock_chain.add_existing_wallet(Auth::BasicAuth, vec![FungibleAsset::mock(256)]);  // Add a wallet with assets
/// let receiver = mock_chain.add_new_wallet(Auth::BasicAuth);  // Add a recipient wallet
///
/// let tx_context = mock_chain.build_tx_context(sender.id(), &[], &[]);
///
/// let script = "begin nop end";
/// let tx_script = TransactionScript::compile(script, vec![], TransactionKernel::testing_assembler()).unwrap();
///
/// let transaction = tx_context.tx_script(tx_script).build().execute().unwrap();
/// mock_chain.apply_executed_transaction(&transaction);  // Apply transaction
/// ```
#[derive(Debug, Clone)]
pub struct MockChain {
    /// An append-only structure used to represent the history of blocks produced for this chain.
    chain: Mmr,

    /// History of produced blocks.
    blocks: Vec<ProvenBlock>,

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

    rng: ChaCha20Rng, // RNG field
}

impl Default for MockChain {
    fn default() -> Self {
        MockChain {
            chain: Mmr::default(),
            blocks: vec![],
            nullifiers: Smt::default(),
            accounts: SimpleSmt::<ACCOUNT_TREE_DEPTH>::new().expect("depth too big for SimpleSmt"),
            pending_objects: PendingObjects::new(),
            available_notes: BTreeMap::new(),
            available_accounts: BTreeMap::new(),
            removed_notes: vec![],
            rng: ChaCha20Rng::from_seed(Default::default()), // Initialize RNG with default seed
        }
    }
}

impl MockChain {
    // CONSTRUCTORS
    // ----------------------------------------------------------------------------------------

    /// Creates a new `MockChain`.
    pub fn empty() -> Self {
        MockChain::default()
    }

    /// Creates a new `MockChain` with two blocks.
    pub fn new() -> Self {
        let mut chain = MockChain::default();
        chain.seal_block(None);
        chain
    }

    /// Creates a new `MockChain` with two blocks and accounts in the genesis block.
    pub fn with_accounts(accounts: &[Account]) -> Self {
        let mut chain = MockChain::default();
        for acc in accounts {
            chain.add_pending_account(acc.clone());
            chain.available_accounts.insert(
                acc.id(),
                MockAccount {
                    account: acc.clone(),
                    seed: None,
                    authenticator: None,
                },
            );
        }
        chain.seal_block(None);
        chain
    }

    /// Sets the seed for the internal RNG.
    pub fn set_rng_seed(&mut self, seed: [u8; 32]) {
        self.rng = ChaCha20Rng::from_seed(seed);
    }

    /// Applies the transaction, adding the entities to the mockchain.
    /// Returns the resulting state of the executing account after executing the transaction.
    pub fn apply_executed_transaction(&mut self, transaction: &ExecutedTransaction) -> Account {
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
        self.pending_objects
            .output_note_batches
            .push(output_notes.into_iter().enumerate().collect());
        self.pending_objects
            .included_transactions
            .push((transaction.id(), transaction.account_id()));

        account
    }

    /// Adds a public [Note] to the pending objects.
    /// A block has to be created to finalize the new entity.
    pub fn add_pending_note(&mut self, note: Note) {
        self.pending_objects.output_note_batches.push(vec![(0, OutputNote::Full(note))]);
    }

    /// Adds a P2ID [Note] to the pending objects and returns it.
    /// A block has to be created to finalize the new entity.
    pub fn add_p2id_note(
        &mut self,
        sender_account_id: AccountId,
        target_account_id: AccountId,
        asset: &[Asset],
        note_type: NoteType,
        reclaim_height: Option<BlockNumber>,
    ) -> Result<Note, NoteError> {
        let mut rng = RpoRandomCoin::new(Word::default());

        let note = if let Some(height) = reclaim_height {
            create_p2idr_note(
                sender_account_id,
                target_account_id,
                asset.to_vec(),
                note_type,
                Default::default(),
                height,
                &mut rng,
            )?
        } else {
            create_p2id_note(
                sender_account_id,
                target_account_id,
                asset.to_vec(),
                note_type,
                Default::default(),
                &mut rng,
            )?
        };

        self.add_pending_note(note.clone());

        Ok(note)
    }

    /// Marks a [Note] as consumed by inserting its nullifier into the block.
    /// A block has to be created to finalize the new entity.
    pub fn add_nullifier(&mut self, nullifier: Nullifier) {
        self.pending_objects.created_nullifiers.push(nullifier);
    }

    /// Proposes a new transaction batch from the provided transactions and returns it.
    ///
    /// This method does not modify the chain state.
    pub fn propose_transaction_batch<I>(
        &self,
        txs: impl IntoIterator<Item = ProvenTransaction, IntoIter = I>,
    ) -> Result<ProposedBatch, ProposedBatchError>
    where
        I: Iterator<Item = ProvenTransaction> + Clone,
    {
        let transactions: Vec<_> = txs.into_iter().map(alloc::sync::Arc::new).collect();

        let (batch_reference_block, chain_mmr) =
            self.get_batch_inputs(transactions.iter().map(|tx| tx.block_num()));

        // TODO: Get the actual proofs as part of get_batch_inputs.
        let unauthenticated_note_proofs = BTreeMap::new();

        ProposedBatch::new(
            transactions,
            batch_reference_block,
            chain_mmr,
            unauthenticated_note_proofs,
        )
    }

    /// Mock-proves a proposed transaction batch from the provided [`ProposedBatch`] and returns it.
    ///
    /// This method does not modify the chain state.
    pub fn prove_transaction_batch(&self, proposed_batch: ProposedBatch) -> ProvenBatch {
        let (
            _transactions,
            block_header,
            _chain_mmr,
            _unauthenticated_note_proofs,
            id,
            account_updates,
            input_notes,
            output_notes,
            batch_expiration_block_num,
        ) = proposed_batch.into_parts();

        ProvenBatch::new_unchecked(
            id,
            block_header.hash(),
            block_header.block_num(),
            account_updates,
            input_notes,
            output_notes,
            batch_expiration_block_num,
        )
    }

    /// Proposes a new block from the provided batches and returns it.
    ///
    /// This method does not modify the chain state.
    pub fn propose_block<I>(
        &self,
        batches: impl IntoIterator<Item = ProvenBatch, IntoIter = I>,
    ) -> Result<ProposedBlock, ProposedBlockError>
    where
        I: Iterator<Item = ProvenBatch> + Clone,
    {
        let batches: Vec<_> = batches.into_iter().collect();
        let block_inputs = self.get_block_inputs(batches.iter());
        // We can't access system time because the testing feature does not depend on std at this
        // time. So we use the minimally correct next timestamp.
        let timestamp = block_inputs.prev_block_header().timestamp() + 1;

        let proposed_block = ProposedBlock::new_at(block_inputs, batches, timestamp)?;

        Ok(proposed_block)
    }

    // OTHER IMPLEMENTATIONS
    // ----------------------------------------------------------------------------------------

    /// Adds a new wallet with the specified authentication method and assets.
    pub fn add_new_wallet(&mut self, auth_method: Auth) -> Account {
        let account_builder = AccountBuilder::new(self.rng.gen()).with_component(BasicWallet);

        self.add_from_account_builder(auth_method, account_builder, AccountState::New)
    }

    /// Adds an existing wallet (nonce == 1) with the specified authentication method and assets.
    pub fn add_existing_wallet(&mut self, auth_method: Auth, assets: Vec<Asset>) -> Account {
        let account_builder =
            Account::builder(self.rng.gen()).with_component(BasicWallet).with_assets(assets);

        self.add_from_account_builder(auth_method, account_builder, AccountState::Exists)
    }

    /// Adds a new faucet with the specified authentication method and metadata.
    pub fn add_new_faucet(
        &mut self,
        auth_method: Auth,
        token_symbol: &str,
        max_supply: u64,
    ) -> MockFungibleFaucet {
        let account_builder = AccountBuilder::new(self.rng.gen())
            .account_type(AccountType::FungibleFaucet)
            .with_component(
                BasicFungibleFaucet::new(
                    TokenSymbol::new(token_symbol).unwrap(),
                    10,
                    max_supply.try_into().unwrap(),
                )
                .unwrap(),
            );

        MockFungibleFaucet(self.add_from_account_builder(
            auth_method,
            account_builder,
            AccountState::New,
        ))
    }

    /// Adds an existing (nonce == 1) faucet with the specified authentication method and metadata.
    pub fn add_existing_faucet(
        &mut self,
        auth_method: Auth,
        token_symbol: &str,
        max_supply: u64,
        total_issuance: Option<u64>,
    ) -> MockFungibleFaucet {
        let mut account_builder = AccountBuilder::new(self.rng.gen())
            .with_component(
                BasicFungibleFaucet::new(
                    TokenSymbol::new(token_symbol).unwrap(),
                    10u8,
                    Felt::new(max_supply),
                )
                .unwrap(),
            )
            .account_type(AccountType::FungibleFaucet);

        let authenticator = match auth_method.build_component() {
            Some((auth_component, authenticator)) => {
                account_builder = account_builder.with_component(auth_component);
                Some(authenticator)
            },
            None => None,
        };
        let mut account = account_builder.build_existing().unwrap();

        // The faucet's reserved slot is initialized to an empty word by default.
        // If total_issuance is set, overwrite it.
        if let Some(issuance) = total_issuance {
            account
                .storage_mut()
                .set_item(memory::FAUCET_STORAGE_DATA_SLOT, [ZERO, ZERO, ZERO, Felt::new(issuance)])
                .unwrap();
        }

        self.available_accounts
            .insert(account.id(), MockAccount::new(account.clone(), None, authenticator));

        MockFungibleFaucet(account)
    }

    /// Adds the [`AccountComponent`] corresponding to `auth_method` to the account in the builder
    /// and builds a new or existing account depending on `account_state`.
    ///
    /// This account is added to the available accounts and are immediately available without having
    /// to seal a block.
    pub fn add_from_account_builder(
        &mut self,
        auth_method: Auth,
        mut account_builder: AccountBuilder,
        account_state: AccountState,
    ) -> Account {
        let authenticator = match auth_method.build_component() {
            Some((auth_component, authenticator)) => {
                account_builder = account_builder.with_component(auth_component);
                Some(authenticator)
            },
            None => None,
        };

        let (account, seed) = if let AccountState::New = account_state {
            let last_block = self.blocks.last().expect("one block should always exist");
            account_builder =
                account_builder.anchor(AccountIdAnchor::try_from(&last_block.header()).unwrap());

            account_builder.build().map(|(account, seed)| (account, Some(seed))).unwrap()
        } else {
            account_builder.build_existing().map(|account| (account, None)).unwrap()
        };

        self.available_accounts
            .insert(account.id(), MockAccount::new(account.clone(), seed, authenticator));
        self.accounts.insert(LeafIndex::from(account.id()), Word::from(account.hash()));

        account
    }

    /// Adds a new `Account` to the list of pending objects.
    /// A block has to be created to finalize the new entity.
    pub fn add_pending_account(&mut self, account: Account) {
        self.pending_objects.updated_accounts.push(BlockAccountUpdate::new(
            account.id(),
            account.hash(),
            AccountUpdateDetails::New(account),
            vec![],
        ));
    }

    /// Initializes a [TransactionContextBuilder].
    ///
    /// This initializes the builder with the correct [TransactionInputs] based on what is
    /// requested. The account's seed and authenticator are also introduced. Additionally, if
    /// the account is set to authenticate with [Auth::BasicAuth], the executed transaction
    /// script is defaulted to [DEFAULT_AUTH_SCRIPT].
    pub fn build_tx_context(
        &mut self,
        account_id: AccountId,
        note_ids: &[NoteId],
        unauthenticated_notes: &[Note],
    ) -> TransactionContextBuilder {
        let mock_account = self.available_accounts.get(&account_id).unwrap().clone();

        let tx_inputs = self.get_transaction_inputs(
            mock_account.account.clone(),
            mock_account.seed().cloned(),
            note_ids,
            unauthenticated_notes,
        );

        let mut tx_context_builder = TransactionContextBuilder::new(mock_account.account().clone())
            .authenticator(mock_account.authenticator().clone())
            .account_seed(mock_account.seed().cloned())
            .tx_inputs(tx_inputs);

        if mock_account.authenticator.is_some() {
            let tx_script = TransactionScript::compile(
                DEFAULT_AUTH_SCRIPT,
                vec![],
                TransactionKernel::testing_assembler_with_mock_account(),
            )
            .unwrap();
            tx_context_builder = tx_context_builder.tx_script(tx_script);
        }

        tx_context_builder
    }

    /// Returns a valid [TransactionInputs] for the specified entities.
    pub fn get_transaction_inputs(
        &self,
        account: Account,
        account_seed: Option<Word>,
        notes: &[NoteId],
        unauthenticated_notes: &[Note],
    ) -> TransactionInputs {
        let block = self.blocks.last().unwrap();

        let mut input_notes = vec![];
        let mut block_headers_map: BTreeMap<BlockNumber, BlockHeader> = BTreeMap::new();
        for note in notes {
            let input_note = self.available_notes.get(note).expect("Note not found").clone();
            let note_block_num = input_note.location().unwrap().block_num();
            if note_block_num != block.header().block_num() {
                block_headers_map.insert(
                    note_block_num,
                    self.blocks.get(note_block_num.as_usize()).unwrap().header(),
                );
            }
            input_notes.push(input_note);
        }

        // If the account is new, add the anchor block's header from which the account ID is derived
        // to the MMR.
        if account.is_new() {
            let epoch_block_num = BlockNumber::from_epoch(account.id().anchor_epoch());
            // The reference block of the transaction is added to the MMR in
            // prologue::process_chain_data so we can skip adding it to the block headers here.
            if epoch_block_num != block.header().block_num() {
                block_headers_map.insert(
                    epoch_block_num,
                    self.blocks.get(epoch_block_num.as_usize()).unwrap().header(),
                );
            }
        }

        for note in unauthenticated_notes {
            input_notes.push(InputNote::Unauthenticated { note: note.clone() })
        }

        let block_headers = block_headers_map.values().cloned();
        let mmr = ChainMmr::from_mmr(&self.chain, block_headers).unwrap();

        TransactionInputs::new(
            account,
            account_seed,
            block.header(),
            mmr,
            InputNotes::new(input_notes).unwrap(),
        )
        .unwrap()
    }

    /// Gets inputs for a transaction batch for all the reference blocks of the provided
    /// transactions.
    pub fn get_batch_inputs(
        &self,
        tx_reference_blocks: impl IntoIterator<Item = BlockNumber>,
    ) -> (BlockHeader, ChainMmr) {
        let (batch_reference_block, chain_mmr) =
            self.latest_selective_chain_mmr(tx_reference_blocks);

        (batch_reference_block, chain_mmr)
    }

    /// Gets the inputs for a block for the provided batches.
    pub fn get_block_inputs<'batch, I>(
        &self,
        batch_iter: impl IntoIterator<Item = &'batch ProvenBatch, IntoIter = I>,
    ) -> BlockInputs
    where
        I: Iterator<Item = &'batch ProvenBatch> + Clone,
    {
        let batch_iterator = batch_iter.into_iter();

        let unauthenticated_note_proofs =
            self.unauthenticated_note_proofs(batch_iterator.clone().flat_map(|batch| {
                batch.input_notes().iter().filter_map(|note| note.header().map(NoteHeader::id))
            }));

        let (block_reference_block, chain_mmr) = self.latest_selective_chain_mmr(
            batch_iterator.clone().map(ProvenBatch::reference_block_num).chain(
                unauthenticated_note_proofs.values().map(|proof| proof.location().block_num()),
            ),
        );

        let account_witnesses =
            self.account_witnesses(batch_iterator.clone().flat_map(ProvenBatch::updated_accounts));

        let nullifier_proofs =
            self.nullifier_witnesses(batch_iterator.flat_map(ProvenBatch::created_nullifiers));

        BlockInputs::new(
            block_reference_block,
            chain_mmr,
            account_witnesses,
            nullifier_proofs,
            unauthenticated_note_proofs,
        )
    }

    // MODIFIERS
    // =========================================================================================

    /// Creates the next block or generates blocks up to the input number if specified.
    /// This will also make all the objects currently pending available for use.
    /// If `block_num` is `Some(number)`, blocks will be generated up to `number`.
    pub fn seal_block(&mut self, block_num: Option<u32>) -> ProvenBlock {
        let next_block_num =
            self.blocks.last().map_or(0, |b| b.header().block_num().child().as_u32());

        let target_block_num = block_num.unwrap_or(next_block_num);

        if target_block_num < next_block_num {
            panic!("Input block number should be higher than the last block number");
        }

        let mut last_block: Option<ProvenBlock> = None;

        for current_block_num in next_block_num..=target_block_num {
            for update in self.pending_objects.updated_accounts.iter() {
                self.accounts
                    .insert(update.account_id().into(), *update.final_state_commitment());

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

            // TODO: Implement nullifier tree reset once defined at the protocol level.
            for nullifier in self.pending_objects.created_nullifiers.iter() {
                self.nullifiers
                    .insert(nullifier.inner(), [current_block_num.into(), ZERO, ZERO, ZERO]);
            }
            let notes_tree = self.pending_objects.build_notes_tree();

            let version = 0;
            let previous = self.blocks.last();
            let peaks = self.chain.peaks();
            let chain_root: Digest = peaks.hash_peaks();
            let account_root = self.accounts.root();
            let prev_hash = previous.map_or(Digest::default(), |block| block.hash());
            let nullifier_root = self.nullifiers.root();
            let note_root = notes_tree.root();
            let timestamp = previous.map_or(TIMESTAMP_START_SECS, |block| {
                block.header().timestamp() + TIMESTAMP_STEP_SECS
            });
            let tx_hash = BlockHeader::compute_tx_commitment(
                self.pending_objects.included_transactions.clone().into_iter(),
            );

            let kernel_root = TransactionKernel::kernel_root();

            // TODO: Set `proof_hash` to the correct value once the kernel is available.
            let proof_hash = Digest::default();

            let header = BlockHeader::new(
                version,
                prev_hash,
                BlockNumber::from(current_block_num),
                chain_root,
                account_root,
                nullifier_root,
                note_root,
                tx_hash,
                kernel_root,
                proof_hash,
                timestamp,
            );

            let block = ProvenBlock::new_unchecked(
                header,
                self.pending_objects.updated_accounts.clone(),
                self.pending_objects.output_note_batches.clone(),
                self.pending_objects.created_nullifiers.clone(),
            );

            for (batch_index, note_batch) in
                self.pending_objects.output_note_batches.iter().enumerate()
            {
                for (note_index, note) in note_batch.iter() {
                    match note {
                        OutputNote::Full(note) => {
                            let block_note_index = BlockNoteIndex::new(batch_index, *note_index)
                                .expect(
                                "max batches in block and max notes in batches should be enforced",
                            );
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

            last_block = Some(block);
        }

        last_block.expect("There should be at least one block generated")
    }

    fn reset_pending(&mut self) {
        self.pending_objects = PendingObjects::new();
        self.removed_notes = vec![];
    }

    // ACCESSORS
    // =========================================================================================

    /// Returns a refernce to the current [`Mmr`] representing the blockchain.
    pub fn block_chain(&self) -> &Mmr {
        &self.chain
    }

    /// Gets the latest [ChainMmr].
    pub fn latest_chain_mmr(&self) -> ChainMmr {
        // We cannot pass the latest block as that would violate the condition in the transaction
        // inputs that the chain length of the mmr must match the number of the reference block.
        let block_headers = self.blocks.iter().map(|b| b.header()).take(self.blocks.len() - 1);

        ChainMmr::from_mmr(&self.chain, block_headers).unwrap()
    }

    /// Creates a new [`ChainMmr`] with all reference blocks in the given iterator except for the
    /// latest block header in the chain and returns that latest block header.
    ///
    /// The intended use is for the latest block header to become the reference block of a new
    /// transaction batch or block.
    pub fn latest_selective_chain_mmr(
        &self,
        reference_blocks: impl IntoIterator<Item = BlockNumber>,
    ) -> (BlockHeader, ChainMmr) {
        let latest_block_header = self.latest_block_header();
        // Deduplicate block numbers so each header will be included just once. This is required so
        // ChainMmr::from_mmr does not panic.
        let reference_blocks: BTreeSet<_> = reference_blocks.into_iter().collect();

        // Include all block headers of the reference blocks except the latest block.
        let block_headers: Vec<_> = reference_blocks
            .into_iter()
            .map(|block_ref_num| self.block_header(block_ref_num.as_usize()))
            .filter(|block_header| block_header.hash() != latest_block_header.hash())
            .collect();

        let chain_mmr = ChainMmr::from_mmr(&self.chain, block_headers).unwrap();

        (latest_block_header, chain_mmr)
    }

    /// Returns the witnesses for the provided account IDs of the current account tree.
    pub fn account_witnesses(
        &self,
        account_ids: impl IntoIterator<Item = AccountId>,
    ) -> BTreeMap<AccountId, AccountWitness> {
        let mut account_witnesses = BTreeMap::new();

        for account_id in account_ids {
            let proof = self.accounts.open(&account_id.into());
            account_witnesses.insert(account_id, AccountWitness::new(proof.value, proof.path));
        }

        account_witnesses
    }

    /// Returns the witnesses for the provided nullifiers of the current nullifier tree.
    pub fn nullifier_witnesses(
        &self,
        nullifiers: impl IntoIterator<Item = Nullifier>,
    ) -> BTreeMap<Nullifier, NullifierWitness> {
        let mut nullifier_proofs = BTreeMap::new();

        for nullifier in nullifiers {
            let proof = self.nullifiers.open(&nullifier.inner());
            nullifier_proofs.insert(nullifier, NullifierWitness::new(proof));
        }

        nullifier_proofs
    }

    /// Returns all note inclusion proofs for the provided notes, **if they are available for
    /// consumption**. Therefore, not all of the provided notes will be guaranteed to have an entry
    /// in the returned map.
    pub fn unauthenticated_note_proofs(
        &self,
        notes: impl IntoIterator<Item = NoteId>,
    ) -> BTreeMap<NoteId, NoteInclusionProof> {
        let mut proofs = BTreeMap::default();
        for note in notes {
            if let Some(input_note) = self.available_notes.get(&note) {
                proofs.insert(
                    note,
                    input_note
                        .proof()
                        .cloned()
                        .expect("all notes tracked by the chain are authenticated"),
                );
            }
        }

        proofs
    }

    /// Returns a reference to the latest [`BlockHeader`].
    pub fn latest_block_header(&self) -> BlockHeader {
        self.blocks[self.chain.forest() - 1].header()
    }

    /// Gets a reference to [BlockHeader] with `block_number`.
    pub fn block_header(&self, block_number: usize) -> BlockHeader {
        self.blocks[block_number].header()
    }

    /// Gets a reference to the nullifier tree.
    pub fn nullifiers(&self) -> &Smt {
        &self.nullifiers
    }

    /// Get the vector of IDs of the currently available notes.
    pub fn available_notes(&self) -> Vec<InputNote> {
        self.available_notes.values().cloned().collect()
    }

    /// Returns the map of note IDs to consumable input notes.
    pub fn available_notes_map(&self) -> &BTreeMap<NoteId, InputNote> {
        &self.available_notes
    }

    /// Returns a reference to the account identifed by the given account ID and panics if it does
    /// not exist.
    pub fn available_account(&self, account_id: AccountId) -> &Account {
        self.available_accounts
            .get(&account_id)
            .expect("account should be available")
            .account()
    }

    /// Get the reference to the accounts hash tree.
    pub fn accounts(&self) -> &SimpleSmt<ACCOUNT_TREE_DEPTH> {
        &self.accounts
    }
}

// HELPER TYPES
// ================================================================================================

/// Helper type for increased readability at call-sites. Indicates whether to build a new (nonce =
/// ZERO) or existing account (nonce = ONE).
pub enum AccountState {
    New,
    Exists,
}
