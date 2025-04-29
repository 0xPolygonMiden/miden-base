use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

use anyhow::Context;
use miden_block_prover::LocalBlockProver;
use miden_lib::{
    account::{faucets::BasicFungibleFaucet, wallets::BasicWallet},
    note::{create_p2id_note, create_p2idr_note},
    transaction::{TransactionKernel, memory},
};
use miden_objects::{
    NoteError, ProposedBatchError, ProposedBlockError,
    account::{
        Account, AccountBuilder, AccountHeader, AccountId, AccountIdAnchor, AccountType,
        StorageSlot, delta::AccountUpdateDetails,
    },
    asset::{Asset, TokenSymbol},
    batch::{ProposedBatch, ProvenBatch},
    block::{
        AccountTree, AccountWitness, BlockAccountUpdate, BlockHeader, BlockInputs, BlockNoteIndex,
        BlockNoteTree, BlockNumber, Blockchain, NullifierTree, NullifierWitness, OutputNoteBatch,
        ProposedBlock, ProvenBlock,
    },
    crypto::merkle::SmtProof,
    note::{Note, NoteHeader, NoteId, NoteInclusionProof, NoteType, Nullifier},
    testing::account_code::DEFAULT_AUTH_SCRIPT,
    transaction::{
        ExecutedTransaction, ForeignAccountInputs, InputNote, InputNotes,
        OrderedTransactionHeaders, OutputNote, PartialBlockchain, ProvenTransaction,
        ToInputNoteCommitments, TransactionHeader, TransactionId, TransactionInputs,
        TransactionScript,
    },
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use vm_processor::{Digest, Felt, Word, ZERO, crypto::RpoRandomCoin};

use super::note::MockChainNote;
use crate::{
    Auth, MockFungibleFaucet, ProvenTransactionExt, TransactionContextBuilder,
    mock_chain::account::MockAccount,
};

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
/// # use miden_testing::{Auth, MockChain, TransactionContextBuilder};
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
/// mock_chain.seal_next_block();
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
/// # use miden_testing::{Auth, MockChain, TransactionContextBuilder};
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
    chain: Blockchain,

    /// History of produced blocks.
    blocks: Vec<ProvenBlock>,

    /// Tree containing the latest `Nullifier`'s tree.
    nullifiers: NullifierTree,

    /// Tree containing the latest state commitment of each account.
    account_tree: AccountTree,

    /// Objects that have not yet been finalized.
    ///
    /// These will become available once the block is sealed.
    ///
    /// Note:
    /// - The [Note]s in this container do not have the `proof` set.
    pending_objects: PendingObjects,

    /// NoteID |-> MockChainNote mapping to simplify note retrieval
    available_notes: BTreeMap<NoteId, MockChainNote>,

    /// TODO
    pending_transactions: Vec<ExecutedTransaction>,

    /// AccountId |-> Account mapping to simplify transaction creation
    available_accounts: BTreeMap<AccountId, MockAccount>,

    removed_notes: Vec<NoteId>,

    rng: ChaCha20Rng, // RNG field
}

impl Default for MockChain {
    fn default() -> Self {
        MockChain {
            chain: Blockchain::default(),
            blocks: vec![],
            nullifiers: NullifierTree::default(),
            account_tree: AccountTree::new(),
            pending_objects: PendingObjects::new(),
            pending_transactions: Vec::new(),
            available_notes: BTreeMap::new(),
            available_accounts: BTreeMap::new(),
            removed_notes: vec![],
            rng: ChaCha20Rng::from_seed(Default::default()), // Initialize RNG with default seed
        }
    }
}

impl MockChain {
    // CONSTANTS
    // ----------------------------------------------------------------------------------------

    /// The timestamp of the genesis of the chain, i.e. the timestamp of the first block, unless
    /// overwritten when calling [`Self::seal_block`]. Chosen as an easily readable number.
    pub const TIMESTAMP_START_SECS: u32 = 1700000000;

    /// The number of seconds by which a block's timestamp increases over the previous block's
    /// timestamp, unless overwritten when calling [`Self::seal_block`].
    pub const TIMESTAMP_STEP_SECS: u32 = 10;

    // CONSTRUCTORS
    // ----------------------------------------------------------------------------------------

    /// Creates a new `MockChain`.
    pub fn empty() -> Self {
        MockChain::default()
    }

    /// Creates a new `MockChain` with two blocks.
    pub fn new() -> Self {
        let mut chain = MockChain::default();
        chain.seal_next_block();
        chain
    }

    /// Creates a new `MockChain` with two blocks and accounts in the genesis block.
    pub fn with_accounts(accounts: &[Account]) -> Self {
        let mut chain = MockChain::default();
        for account in accounts {
            chain.add_pending_account(account.clone());
            chain
                .available_accounts
                .insert(account.id(), MockAccount::new(account.clone(), None, None));
        }
        chain.seal_next_block();
        chain
    }

    /// Sets the seed for the internal RNG.
    pub fn set_rng_seed(&mut self, seed: [u8; 32]) {
        self.rng = ChaCha20Rng::from_seed(seed);
    }

    /// Applies the transaction, adding the entities to the mockchain.
    /// Returns the resulting state of the executing account after executing the transaction.
    pub fn submit_transaction(&mut self, transaction: &ExecutedTransaction) -> Account {
        let mut account = transaction.initial_account().clone();
        account.apply_delta(transaction.account_delta()).unwrap();

        // disregard private accounts, so it's easier to retrieve data
        let account_update_details = AccountUpdateDetails::New(account.clone());

        let block_account_update = BlockAccountUpdate::new(
            transaction.account_id(),
            account.commitment(),
            account_update_details,
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

    /// Adds an [OutputNote] to the pending objects.
    /// A block has to be created to finalize the new entity.
    pub fn add_pending_note(&mut self, note: OutputNote) {
        self.pending_objects.output_note_batches.push(vec![(0, note)]);
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

        self.add_pending_note(OutputNote::Full(note.clone()));

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

        let (batch_reference_block, partial_blockchain, unauthenticated_note_proofs) = self
            .get_batch_inputs(
                transactions.iter().map(|tx| tx.ref_block_num()),
                transactions
                    .iter()
                    .flat_map(|tx| tx.unauthenticated_notes().map(NoteHeader::id)),
            );

        ProposedBatch::new(
            transactions,
            batch_reference_block,
            partial_blockchain,
            unauthenticated_note_proofs,
        )
    }

    /// Mock-proves a proposed transaction batch from the provided [`ProposedBatch`] and returns it.
    ///
    /// This method does not modify the chain state.
    pub fn prove_transaction_batch(&self, proposed_batch: ProposedBatch) -> ProvenBatch {
        let (
            transactions,
            block_header,
            _partial_blockchain,
            _unauthenticated_note_proofs,
            id,
            account_updates,
            input_notes,
            output_notes,
            batch_expiration_block_num,
        ) = proposed_batch.into_parts();

        // SAFETY: This satisfies the requirements of the ordered tx headers.
        let tx_headers = OrderedTransactionHeaders::new_unchecked(
            transactions
                .iter()
                .map(AsRef::as_ref)
                .map(TransactionHeader::from)
                .collect::<Vec<_>>(),
        );

        ProvenBatch::new(
            id,
            block_header.commitment(),
            block_header.block_num(),
            account_updates,
            input_notes,
            output_notes,
            batch_expiration_block_num,
            tx_headers,
        )
        .expect("Failed to create ProvenBatch")
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

    pub fn prove_block(&self, proposed_block: ProposedBlock) -> anyhow::Result<ProvenBlock> {
        LocalBlockProver::new(0)
            .prove_without_batch_verification(proposed_block)
            .context("failed to prove proposed block into proven block")
    }

    // OTHER IMPLEMENTATIONS
    // ----------------------------------------------------------------------------------------

    /// Adds a new wallet with the specified authentication method and assets.
    pub fn add_new_wallet(&mut self, auth_method: Auth) -> Account {
        let account_builder = AccountBuilder::new(self.rng.random()).with_component(BasicWallet);

        self.add_from_account_builder(auth_method, account_builder, AccountState::New)
    }

    /// Adds an existing wallet (nonce == 1) with the specified authentication method and assets.
    pub fn add_existing_wallet(&mut self, auth_method: Auth, assets: Vec<Asset>) -> Account {
        let account_builder = Account::builder(self.rng.random())
            .with_component(BasicWallet)
            .with_assets(assets);

        self.add_from_account_builder(auth_method, account_builder, AccountState::Exists)
    }

    /// Adds a new faucet with the specified authentication method and metadata.
    pub fn add_new_faucet(
        &mut self,
        auth_method: Auth,
        token_symbol: &str,
        max_supply: u64,
    ) -> MockFungibleFaucet {
        let account_builder = AccountBuilder::new(self.rng.random())
            .account_type(AccountType::FungibleFaucet)
            .with_component(
                BasicFungibleFaucet::new(
                    TokenSymbol::new(token_symbol).unwrap(),
                    10,
                    max_supply.try_into().unwrap(),
                )
                .unwrap(),
            );

        MockFungibleFaucet::new(self.add_from_account_builder(
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
        let mut account_builder = AccountBuilder::new(self.rng.random())
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

        MockFungibleFaucet::new(account)
    }

    /// Adds the [`AccountComponent`](miden_objects::account::AccountComponent) corresponding to
    /// `auth_method` to the account in the builder and builds a new or existing account
    /// depending on `account_state`.
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
            let epoch_block_number = self
                .blocks
                .last()
                .expect("one block should always exist")
                .header()
                .epoch_block_num();
            let account_id_anchor =
                self.blocks.get(epoch_block_number.as_usize()).unwrap().header();
            account_builder =
                account_builder.anchor(AccountIdAnchor::try_from(account_id_anchor).unwrap());

            account_builder.build().map(|(account, seed)| (account, Some(seed))).unwrap()
        } else {
            account_builder.build_existing().map(|account| (account, None)).unwrap()
        };

        self.available_accounts
            .insert(account.id(), MockAccount::new(account.clone(), seed, authenticator));
        self.account_tree.insert(account.id(), account.commitment()).unwrap();

        account
    }

    /// Adds a new `Account` to the list of pending objects.
    /// A block has to be created to finalize the new entity.
    pub fn add_pending_account(&mut self, account: Account) {
        self.pending_objects.updated_accounts.push(BlockAccountUpdate::new(
            account.id(),
            account.commitment(),
            AccountUpdateDetails::New(account),
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
            mock_account.account().clone(),
            mock_account.seed().cloned(),
            note_ids,
            unauthenticated_notes,
        );

        let mut tx_context_builder = TransactionContextBuilder::new(mock_account.account().clone())
            .authenticator(mock_account.authenticator().clone())
            .account_seed(mock_account.seed().cloned())
            .tx_inputs(tx_inputs);

        if mock_account.authenticator().is_some() {
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
            let input_note: InputNote = self
                .available_notes
                .get(note)
                .expect("Note not found")
                .clone()
                .try_into()
                .expect("Note should be public");
            let note_block_num = input_note.location().unwrap().block_num();
            if note_block_num != block.header().block_num() {
                block_headers_map.insert(
                    note_block_num,
                    self.blocks.get(note_block_num.as_usize()).unwrap().header().clone(),
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
                    self.blocks.get(epoch_block_num.as_usize()).unwrap().header().clone(),
                );
            }
        }

        for note in unauthenticated_notes {
            input_notes.push(InputNote::Unauthenticated { note: note.clone() })
        }

        let block_headers = block_headers_map.values().cloned();
        let mmr = PartialBlockchain::from_blockchain(&self.chain, block_headers).unwrap();

        TransactionInputs::new(
            account,
            account_seed,
            block.header().clone(),
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
        unauthenticated_notes: impl Iterator<Item = NoteId>,
    ) -> (BlockHeader, PartialBlockchain, BTreeMap<NoteId, NoteInclusionProof>) {
        // Fetch note proofs for notes that exist in the chain.
        let unauthenticated_note_proofs = self.unauthenticated_note_proofs(unauthenticated_notes);

        // We also need to fetch block inclusion proofs for any of the blocks that contain
        // unauthenticated notes for which we want to prove inclusion.
        let required_blocks = tx_reference_blocks.into_iter().chain(
            unauthenticated_note_proofs
                .values()
                .map(|note_proof| note_proof.location().block_num()),
        );

        let (batch_reference_block, partial_block_chain) =
            self.latest_selective_partial_blockchain(required_blocks);

        (batch_reference_block, partial_block_chain, unauthenticated_note_proofs)
    }

    /// Gets foreign account inputs to execute FPI transactions.
    pub fn get_foreign_account_inputs(&self, account_id: AccountId) -> ForeignAccountInputs {
        let account = self.available_account(account_id);

        let account_witness = self.accounts().open(account_id);
        assert_eq!(account_witness.state_commitment(), account.commitment());

        let mut storage_map_proofs = vec![];
        for slot in account.storage().slots() {
            // if there are storage maps, we populate the merkle store and advice map
            if let StorageSlot::Map(map) = slot {
                let proofs: Vec<SmtProof> = map.entries().map(|(key, _)| map.open(key)).collect();
                storage_map_proofs.extend(proofs);
            }
        }

        ForeignAccountInputs::new(
            AccountHeader::from(account),
            account.storage().get_header(),
            account.code().clone(),
            account_witness,
            storage_map_proofs,
        )
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

        let (block_reference_block, partial_blockchain) = self.latest_selective_partial_blockchain(
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
            partial_blockchain,
            account_witnesses,
            nullifier_proofs,
            unauthenticated_note_proofs,
        )
    }

    // MODIFIERS
    // =========================================================================================

    /// Creates the next block in the mock chain.
    ///
    /// This will make all the objects currently pending available for use.
    pub fn seal_next_block(&mut self) -> ProvenBlock {
        self.seal_block(None, None)
    }

    /// Creates a new block in the mock chain.
    ///
    /// This will make all the objects currently pending available for use.
    ///
    /// If `block_num` is `None`, the next block is created, otherwise all blocks from the next
    /// block up to and including `block_num` will be created.
    ///
    /// If a `timestamp` is provided, it will be set on the block with `block_num`.
    pub fn seal_block(&mut self, block_num: Option<u32>, timestamp: Option<u32>) -> ProvenBlock {
        let next_block_num =
            self.blocks.last().map_or(0, |b| b.header().block_num().child().as_u32());

        let target_block_num = block_num.unwrap_or(next_block_num);

        assert!(
            target_block_num >= next_block_num,
            "target block number must be greater or equal to the number of the next block in the chain"
        );

        let pending_transactions = core::mem::take(&mut self.pending_transactions);

        // TODO: Split into multiple batches if num of pending transactions exceeds max txs per
        // batch.
        let proposed_batch = self
            .propose_transaction_batch(pending_transactions.into_iter().map(|executed_tx| {
                ProvenTransaction::from_executed_transaction_mocked(executed_tx)
            }))
            .map(|proposed_batch| self.prove_transaction_batch(proposed_batch))
            .unwrap();

        // TODO: Add pending objects into block.
        self.propose_block([proposed_batch])
            .map(|proposed_block| self.prove_block(proposed_block).unwrap())
            .unwrap();

        let mut last_block: Option<ProvenBlock> = None;

        for current_block_num in next_block_num..=target_block_num {
            for update in self.pending_objects.updated_accounts.iter() {
                self.account_tree
                    .insert(update.account_id(), update.final_state_commitment())
                    .unwrap();

                if let Some(mock_account) = self.available_accounts.get(&update.account_id()) {
                    let account = match update.details() {
                        AccountUpdateDetails::New(acc) => acc.clone(),
                        _ => panic!("The mockchain should have full account details"),
                    };
                    self.available_accounts.insert(
                        update.account_id(),
                        MockAccount::new(
                            account,
                            mock_account.seed().copied(),
                            mock_account.authenticator().clone(),
                        ),
                    );
                }
            }

            // TODO: Implement nullifier tree reset once defined at the protocol level.
            for nullifier in self.pending_objects.created_nullifiers.iter() {
                self.nullifiers
                    .mark_spent(*nullifier, BlockNumber::from(current_block_num))
                    .expect("nullifier should not already be spent");
            }
            let notes_tree = self.pending_objects.build_notes_tree();

            let version = 0;
            let previous = self.blocks.last();
            let peaks = self.chain.peaks();
            let chain_commitment: Digest = peaks.hash_peaks();
            let account_root = self.account_tree.root();
            let prev_block_commitment =
                previous.map_or(Digest::default(), |block| block.commitment());
            let nullifier_root = self.nullifiers.root();
            let note_root = notes_tree.root();

            let mut block_timestamp = previous.map_or(Self::TIMESTAMP_START_SECS, |block| {
                block.header().timestamp() + Self::TIMESTAMP_STEP_SECS
            });

            // Overwrite the block timestamp if we're building the target block.
            if current_block_num == target_block_num {
                if let Some(provided_timestamp) = timestamp {
                    if let Some(prev_block) = previous {
                        assert!(
                            provided_timestamp > prev_block.header().timestamp(),
                            "provided timestamp must be strictly greater than the previous block's timestamp"
                        );
                    }
                    block_timestamp = provided_timestamp;
                }
            }

            let tx_commitment = OrderedTransactionHeaders::compute_commitment(
                self.pending_objects.included_transactions.clone().into_iter(),
            );

            let tx_kernel_commitment = TransactionKernel::kernel_commitment();

            // TODO: Set `proof_commitment` to the correct value once the kernel is available.
            let proof_commitment = Digest::default();

            let header = BlockHeader::new(
                version,
                prev_block_commitment,
                BlockNumber::from(current_block_num),
                chain_commitment,
                account_root,
                nullifier_root,
                note_root,
                tx_commitment,
                tx_kernel_commitment,
                proof_commitment,
                block_timestamp,
            );

            let block = ProvenBlock::new_unchecked(
                header.clone(),
                self.pending_objects.updated_accounts.clone(),
                self.pending_objects.output_note_batches.clone(),
                self.pending_objects.created_nullifiers.clone(),
                // TODO: For now we can't easily compute the verified transactions of this block.
                // Let's do this as part of miden-base/#1224.
                OrderedTransactionHeaders::new_unchecked(vec![]),
            );

            for (batch_index, note_batch) in
                self.pending_objects.output_note_batches.iter().enumerate()
            {
                for (note_index, note) in note_batch.iter() {
                    let block_note_index = BlockNoteIndex::new(batch_index, *note_index)
                        .expect("max batches in block and max notes in batches should be enforced");
                    let note_path = notes_tree.get_note_path(block_note_index);
                    let note_inclusion_proof = NoteInclusionProof::new(
                        block.header().block_num(),
                        block_note_index.leaf_index_value(),
                        note_path,
                    )
                    .unwrap();
                    if let OutputNote::Full(note) = note {
                        self.available_notes.insert(
                            note.id(),
                            MockChainNote::Public(note.clone(), note_inclusion_proof),
                        );
                    } else {
                        self.available_notes.insert(
                            note.id(),
                            MockChainNote::Private(
                                note.id(),
                                *note.metadata(),
                                note_inclusion_proof,
                            ),
                        );
                    }
                }
            }

            for removed_note in self.removed_notes.iter() {
                self.available_notes.remove(removed_note);
            }

            self.blocks.push(block.clone());
            self.chain.push(header.commitment());
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

    /// Returns a refernce to the current [`Blockchain`].
    pub fn block_chain(&self) -> &Blockchain {
        &self.chain
    }

    /// Gets the latest [PartialBlockchain].
    pub fn latest_partial_blockchain(&self) -> PartialBlockchain {
        // We have to exclude the latest block because we need to fetch the state of the chain at
        // that latest block, which does not include itself.
        let block_headers =
            self.blocks.iter().map(|b| b.header()).take(self.blocks.len() - 1).cloned();

        PartialBlockchain::from_blockchain(&self.chain, block_headers).unwrap()
    }

    /// Creates a new [`PartialBlockchain`] with all reference blocks in the given iterator except
    /// for the latest block header in the chain and returns that latest block header.
    ///
    /// The intended use is for the latest block header to become the reference block of a new
    /// transaction batch or block.
    pub fn latest_selective_partial_blockchain(
        &self,
        reference_blocks: impl IntoIterator<Item = BlockNumber>,
    ) -> (BlockHeader, PartialBlockchain) {
        let latest_block_header = self.latest_block_header().clone();
        // Deduplicate block numbers so each header will be included just once. This is required so
        // PartialBlockchain::from_blockchain does not panic.
        let reference_blocks: BTreeSet<_> = reference_blocks.into_iter().collect();

        // Include all block headers of the reference blocks except the latest block.
        let block_headers: Vec<_> = reference_blocks
            .into_iter()
            .map(|block_ref_num| self.block_header(block_ref_num.as_usize()))
            .filter(|block_header| block_header.commitment() != latest_block_header.commitment())
            .collect();

        let partial_blockchain =
            PartialBlockchain::from_blockchain(&self.chain, block_headers).unwrap();

        (latest_block_header, partial_blockchain)
    }

    /// Returns the witnesses for the provided account IDs of the current account tree.
    pub fn account_witnesses(
        &self,
        account_ids: impl IntoIterator<Item = AccountId>,
    ) -> BTreeMap<AccountId, AccountWitness> {
        let mut account_witnesses = BTreeMap::new();

        for account_id in account_ids {
            let witness = self.account_tree.open(account_id);
            account_witnesses.insert(account_id, witness);
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
            let witness = self.nullifiers.open(&nullifier);
            nullifier_proofs.insert(nullifier, witness);
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
                proofs.insert(note, input_note.inclusion_proof().clone());
            }
        }

        proofs
    }

    /// Returns a reference to the latest [`BlockHeader`].
    pub fn latest_block_header(&self) -> BlockHeader {
        let chain_tip =
            self.chain.chain_tip().expect("chain should contain at least the genesis block");
        self.blocks[chain_tip.as_usize()].header().clone()
    }

    /// Gets a reference to [BlockHeader] with `block_number`.
    pub fn block_header(&self, block_number: usize) -> BlockHeader {
        self.blocks[block_number].header().clone()
    }

    /// Returns a reference to the tracked proven blocks.
    pub fn proven_blocks(&self) -> &[ProvenBlock] {
        &self.blocks
    }

    /// Gets a reference to the nullifier tree.
    pub fn nullifiers(&self) -> &NullifierTree {
        &self.nullifiers
    }

    /// Get the vector of IDs of the currently available notes.
    pub fn available_notes(&self) -> Vec<MockChainNote> {
        self.available_notes.values().cloned().collect()
    }

    /// Returns the map of note IDs to consumable notes.
    pub fn available_notes_map(&self) -> &BTreeMap<NoteId, MockChainNote> {
        &self.available_notes
    }

    /// Returns an [`InputNote`] for the given note ID. If the note does not exist or is not
    /// public, `None` is returned.
    pub fn get_public_note(&self, note_id: &NoteId) -> Option<InputNote> {
        let note = self.available_notes.get(note_id)?;
        note.clone().try_into().ok()
    }

    /// Returns a reference to the account identifed by the given account ID and panics if it does
    /// not exist.
    pub fn available_account(&self, account_id: AccountId) -> &Account {
        self.available_accounts
            .get(&account_id)
            .expect("account should be available")
            .account()
    }

    /// Get the reference to the account tree.
    pub fn accounts(&self) -> &AccountTree {
        &self.account_tree
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
