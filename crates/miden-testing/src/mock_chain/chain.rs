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
    MAX_BATCHES_PER_BLOCK, MAX_OUTPUT_NOTES_PER_BATCH, NoteError, ProposedBatchError,
    ProposedBlockError,
    account::{
        Account, AccountBuilder, AccountHeader, AccountId, AccountIdAnchor, AccountType,
        StorageSlot, delta::AccountUpdateDetails,
    },
    asset::{Asset, TokenSymbol},
    batch::{ProposedBatch, ProvenBatch},
    block::{
        AccountTree, AccountWitness, BlockAccountUpdate, BlockHeader, BlockInputs, BlockNoteTree,
        BlockNumber, Blockchain, NullifierTree, NullifierWitness, ProposedBlock, ProvenBlock,
    },
    crypto::merkle::SmtProof,
    note::{Note, NoteHeader, NoteId, NoteInclusionProof, NoteType, Nullifier},
    testing::account_code::DEFAULT_AUTH_SCRIPT,
    transaction::{
        ExecutedTransaction, ForeignAccountInputs, InputNote, InputNotes,
        OrderedTransactionHeaders, OutputNote, PartialBlockchain, ProvenTransaction,
        TransactionHeader, TransactionId, TransactionInputs, TransactionScript,
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
    updated_accounts: BTreeMap<AccountId, BlockAccountUpdate>,

    /// Note batches created in transactions in the block.
    output_notes: Vec<OutputNote>,

    /// Nullifiers produced in transactions in the block.
    created_nullifiers: Vec<Nullifier>,

    /// Transaction IDs added to the block.
    /// TODO: Remove or use in pending objects batch.
    included_transactions: Vec<(TransactionId, AccountId)>,
}

impl PendingObjects {
    pub fn new() -> PendingObjects {
        PendingObjects {
            updated_accounts: BTreeMap::new(),
            output_notes: vec![],
            created_nullifiers: vec![],
            included_transactions: vec![],
        }
    }

    /// Returns `true` if there are no pending objects, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.updated_accounts.is_empty()
            && self.output_notes.is_empty()
            && self.created_nullifiers.is_empty()
            && self.included_transactions.is_empty()
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

    /// Tree containing all nullifiers.
    nullifier_tree: NullifierTree,

    /// Tree containing the state commitments of all accounts.
    account_tree: AccountTree,

    /// Objects that have not yet been finalized.
    ///
    /// These will become available once the block is sealed.
    ///
    /// Note:
    /// - The [Note]s in this container do not have the `proof` set.
    pending_objects: PendingObjects,

    /// Transactions that have been submitted to the chain but have not yet been included in a
    /// block.
    pending_transactions: Vec<ExecutedTransaction>,

    /// NoteID |-> MockChainNote mapping to simplify note retrieval.
    committed_notes: BTreeMap<NoteId, MockChainNote>,

    /// AccountId |-> MockAccount mapping to simplify transaction creation.
    committed_accounts: BTreeMap<AccountId, MockAccount>,

    // The RNG used to generate note serial numbers, account seeds or cryptographic keys.
    rng: ChaCha20Rng,
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

    /// Creates a new `MockChain` with an empty genesis block.
    pub fn new() -> Self {
        Self::with_accounts(&[])
    }

    /// Creates a new `MockChain` with a genesis block containing the provided accounts.
    pub fn with_accounts(accounts: &[Account]) -> Self {
        let (genesis_block, account_tree) =
            create_genesis_state(accounts.iter().cloned()).expect("TODO: turn into error");

        let mut chain = MockChain {
            chain: Blockchain::default(),
            blocks: vec![],
            nullifier_tree: NullifierTree::default(),
            account_tree,
            pending_objects: PendingObjects::new(),
            pending_transactions: Vec::new(),
            committed_notes: BTreeMap::new(),
            committed_accounts: BTreeMap::new(),
            // Initialize RNG with default seed.
            rng: ChaCha20Rng::from_seed(Default::default()),
        };

        // We do not have to apply the tree changes, because the account tree is already initialized
        // and the nullifier tree is empty at genesis.
        chain
            .apply_block(genesis_block)
            .context("failed to apply genesis block")
            .unwrap();

        debug_assert_eq!(chain.blocks.len(), 1);
        debug_assert_eq!(chain.account_tree.num_accounts(), accounts.len());
        debug_assert_eq!(chain.committed_accounts.len(), accounts.len());
        for added_account in accounts {
            debug_assert_eq!(
                chain.account_tree.get(added_account.id()),
                added_account.commitment()
            );
            debug_assert_eq!(
                chain.committed_account(added_account.id()).commitment(),
                added_account.commitment(),
            );
        }

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

        self.pending_transactions.push(transaction.clone());

        account
    }

    /// Adds an [OutputNote] to the pending objects.
    /// A block has to be created to finalize the new entity.
    pub fn add_pending_note(&mut self, note: OutputNote) {
        self.pending_objects.output_notes.push(note);
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
        .expect("failed to create ProvenBatch")
    }

    /// Proposes a new block from the provided batches with the given timestamp and returns it.
    ///
    /// This method does not modify the chain state.
    pub fn propose_block_at<I>(
        &self,
        batches: impl IntoIterator<Item = ProvenBatch, IntoIter = I>,
        timestamp: u32,
    ) -> Result<ProposedBlock, ProposedBlockError>
    where
        I: Iterator<Item = ProvenBatch> + Clone,
    {
        let batches: Vec<_> = batches.into_iter().collect();
        let block_inputs = self.get_block_inputs(batches.iter());

        let proposed_block = ProposedBlock::new_at(block_inputs, batches, timestamp)?;

        Ok(proposed_block)
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
        // We can't access system time because the testing feature does not depend on std at this
        // time. So we use the minimally correct next timestamp.
        let timestamp = self.latest_block_header().timestamp() + 1;

        self.propose_block_at(batches, timestamp)
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

        self.committed_accounts
            .insert(account.id(), MockAccount::new(account.clone(), None, authenticator));

        MockFungibleFaucet::new(account)
    }

    /// Adds the [`AccountComponent`](miden_objects::account::AccountComponent) corresponding to
    /// `auth_method` to the account in the builder and builds a new or existing account
    /// depending on `account_state`.
    ///
    /// This account is added to the available accounts and are immediately available without having
    /// to seal a block.
    // TODO: Rename to add_pending_account_from_builder
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
            let epoch_block_number = self.latest_block_header().epoch_block_num();
            let account_id_anchor =
                self.blocks.get(epoch_block_number.as_usize()).unwrap().header();
            account_builder =
                account_builder.anchor(AccountIdAnchor::try_from(account_id_anchor).unwrap());

            account_builder.build().map(|(account, seed)| (account, Some(seed))).unwrap()
        } else {
            account_builder.build_existing().map(|account| (account, None)).unwrap()
        };

        // Add account to the available accounts so transaction inputs can be retrieved via the mock
        // chain APIs.
        self.committed_accounts
            .insert(account.id(), MockAccount::new(account.clone(), seed, authenticator));

        // Only automatically add the account in the next block if it is supposed to already exist.
        // If it's a new account, it should be committed in other ways.
        if let AccountState::Exists = account_state {
            self.add_pending_account(account.clone());
        }

        account
    }

    /// Adds a new `Account` to the list of pending objects.
    ///
    /// A block has to be created to finalize the new entity.
    pub fn add_pending_account(&mut self, account: Account) {
        self.pending_objects.updated_accounts.insert(
            account.id(),
            BlockAccountUpdate::new(
                account.id(),
                account.commitment(),
                AccountUpdateDetails::New(account),
            ),
        );
    }

    /// Initializes a [TransactionContextBuilder].
    ///
    /// This initializes the builder with the correct [TransactionInputs] based on what is
    /// requested. The account's seed and authenticator are also introduced. Additionally, if
    /// the account is set to authenticate with [Auth::BasicAuth], the executed transaction
    /// script is defaulted to [DEFAULT_AUTH_SCRIPT].
    pub fn build_tx_context(
        &self,
        account_id: AccountId,
        note_ids: &[NoteId],
        unauthenticated_notes: &[Note],
    ) -> TransactionContextBuilder {
        let mock_account = self.committed_accounts.get(&account_id).unwrap().clone();

        let tx_inputs = self.get_transaction_inputs(
            mock_account.account().clone(),
            mock_account.seed().cloned(),
            note_ids,
            unauthenticated_notes,
        );

        let mut tx_context_builder = TransactionContextBuilder::new(mock_account.account().clone())
            .authenticator(mock_account.authenticator().cloned())
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
                .committed_notes
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
        let account = self.committed_account(account_id);

        let account_witness = self.account_tree().open(account_id);
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
    pub fn prove_next_block(&mut self) -> ProvenBlock {
        self.prove_block_inner(None).unwrap()
    }

    /// Proves the next block in the mock chain at the given timestamp.
    pub fn prove_next_block_at(&mut self, timestamp: u32) -> anyhow::Result<ProvenBlock> {
        self.prove_block_inner(Some(timestamp))
    }

    /// Proves new blocks until the block with the given target block number has been created.
    ///
    /// For example, if the latest block is `5` and this function is called with `10`, then blocks
    /// `6..=10` will be created and block 10 will be returned.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - the given block number is smaller or equal to the number of the latest block in the chain.
    pub fn prove_until_block(
        &mut self,
        target_block_num: impl Into<BlockNumber>,
    ) -> anyhow::Result<ProvenBlock> {
        let target_block_num = target_block_num.into();
        let latest_block_num = self.latest_block_header().block_num();
        assert!(
            target_block_num > latest_block_num,
            "target block number must be greater than the number of the latest block in the chain"
        );

        let mut last_block = None;
        for _ in latest_block_num.as_usize()..=target_block_num.as_usize() {
            last_block = Some(self.prove_next_block());
        }

        Ok(last_block.expect("at least one block should have been created"))
    }

    /// Creates a new block in the mock chain.
    ///
    /// This will make all the objects currently pending available for use.
    ///
    /// If `block_num` is `None`, the next block is created, otherwise all blocks from the next
    /// block up to and including `block_num` will be created.
    ///
    /// If a `timestamp` is provided, it will be set on the block with `block_num`.
    fn prove_block_inner(&mut self, timestamp: Option<u32>) -> anyhow::Result<ProvenBlock> {
        // Create batches from pending transactions.
        // ----------------------------------------------------------------------------------------

        let batches = self
            .pending_transactions_to_batches()
            .context("failed to convert pending transactions to batch")?;

        // Create block.
        // ----------------------------------------------------------------------------------------

        let block_timestamp =
            timestamp.unwrap_or(self.latest_block_header().timestamp() + Self::TIMESTAMP_STEP_SECS);

        let mut proven_block = self
            .propose_block_at(batches, block_timestamp)
            .context("failed to propose block")
            .and_then(|proposed_block| {
                self.prove_block(proposed_block)
                    .context("failed to prove proposed block into proven block")
            })?;

        // We apply the block tree updates here, so that add_pending_objects_to_block can easily
        // update the block header of this block with the pending accounts and nullifiers.
        self.apply_block_tree_updates(&proven_block)
            .context("failed to apply account and nullifier tree changes from block")?;

        if !self.pending_objects.is_empty() {
            self.add_pending_objects_to_block(&mut proven_block)
                .context("failed to add pending objects to block")?;
        }

        self.apply_block(proven_block.clone())
            .context("failed to apply proven block to chain state")?;

        Ok(proven_block)
    }

    /// Inserts the given block's account updates and created nullifiers into the account tree and
    /// nullifier tree, respectively.
    pub fn apply_block_tree_updates(&mut self, proven_block: &ProvenBlock) -> anyhow::Result<()> {
        for account_update in proven_block.updated_accounts() {
            self.account_tree
                .insert(account_update.account_id(), account_update.final_state_commitment())
                .context("failed to insert account update into account tree")?;
        }

        for nullifier in proven_block.created_nullifiers() {
            self.nullifier_tree
                .mark_spent(*nullifier, proven_block.header().block_num())
                .context("failed to mark block nullifier as spent")?;

            // TODO: Remove from self.available_notes.
        }

        Ok(())
    }

    /// Applies the given block to the chain state, which means:
    ///
    /// - Updated accounts from the block are updated in the available accounts.
    /// - Created notes are inserted into the available notes.
    /// - Consumed notes are removed from the available notes.
    /// - The block is appended to the [`BlockChain`] and the list of proven blocks.
    pub fn apply_block(&mut self, proven_block: ProvenBlock) -> anyhow::Result<()> {
        for account_update in proven_block.updated_accounts() {
            match account_update.details() {
                AccountUpdateDetails::New(account) => {
                    let committed_account =
                        self.committed_accounts.get(&account_update.account_id());
                    let authenticator =
                        committed_account.and_then(|account| account.authenticator());
                    let seed = committed_account.and_then(|account| account.seed());

                    self.committed_accounts.insert(
                        account.id(),
                        MockAccount::new(account.clone(), seed.cloned(), authenticator.cloned()),
                    );
                },
                AccountUpdateDetails::Delta(account_delta) => {
                    let committed_account =
                        self.committed_accounts.get_mut(&account_update.account_id()).ok_or_else(
                            || anyhow::anyhow!("account delta in block for non-existent account"),
                        )?;
                    committed_account
                        .apply_delta(account_delta)
                        .context("failed to apply account delta to committed account")?;
                },
                AccountUpdateDetails::Private => {
                    todo!("private accounts are not yet supported")
                },
            }
        }

        let notes_tree = proven_block.build_output_note_tree();
        for (block_note_index, created_note) in proven_block.output_notes() {
            let note_path = notes_tree.get_note_path(block_note_index);
            let note_inclusion_proof = NoteInclusionProof::new(
                proven_block.header().block_num(),
                block_note_index.leaf_index_value(),
                note_path,
            )
            .context("failed to construct note inclusion proof")?;

            if let OutputNote::Full(note) = created_note {
                self.committed_notes
                    .insert(note.id(), MockChainNote::Public(note.clone(), note_inclusion_proof));
            } else {
                self.committed_notes.insert(
                    created_note.id(),
                    MockChainNote::Private(
                        created_note.id(),
                        *created_note.metadata(),
                        note_inclusion_proof,
                    ),
                );
            }
        }

        debug_assert_eq!(
            self.chain.commitment(),
            proven_block.header().chain_commitment(),
            "current mock chain commitment and new block's chain commitment should match"
        );
        debug_assert_eq!(
            BlockNumber::from(self.chain.as_mmr().forest() as u32),
            proven_block.header().block_num(),
            "current mock chain length and new block's number should match"
        );

        self.chain.push(proven_block.header().commitment());
        self.blocks.push(proven_block);

        Ok(())
    }

    fn pending_transactions_to_batches(&mut self) -> anyhow::Result<Vec<ProvenBatch>> {
        // Batches must contian at least one transaction, so if there are no pending transactions,
        // return early.
        if self.pending_transactions.is_empty() {
            return Ok(vec![]);
        }

        let pending_transactions = core::mem::take(&mut self.pending_transactions);

        // TODO: Distribute the transactions into multiple batches if the transactions would not fit
        // into a single batch (according to max input notes, max output notes and max accounts).
        let proven_batch = self
            .propose_transaction_batch(pending_transactions.into_iter().map(|executed_tx| {
                // This essentially transforms an executed tx into a proven tx with a dummy proof.
                ProvenTransaction::from_executed_transaction_mocked(executed_tx)
            }))
            .map(|proposed_batch| self.prove_transaction_batch(proposed_batch))?;

        Ok(vec![proven_batch])
    }

    fn add_pending_objects_to_block(
        &mut self,
        proven_block: &mut ProvenBlock,
    ) -> anyhow::Result<()> {
        // Add pending accounts to block.
        let pending_account_updates = core::mem::take(&mut self.pending_objects.updated_accounts);

        let updated_accounts_block: BTreeSet<AccountId> = proven_block
            .updated_accounts()
            .iter()
            .map(|update| update.account_id())
            .collect();

        for (id, account_update) in pending_account_updates {
            if updated_accounts_block.contains(&id) {
                anyhow::bail!(
                    "account {id} is already modified through a transaction in the block so it cannot also be modified through pending objects"
                );
            }

            self.account_tree
                .insert(id, account_update.final_state_commitment())
                .context("failed to insert pending account into tree")?;

            proven_block.updated_accounts_mut().push(account_update);
        }

        // Add pending nullifiers to block.
        let pending_created_nullifiers =
            core::mem::take(&mut self.pending_objects.created_nullifiers);

        let created_nullifiers_block: BTreeSet<Nullifier> =
            proven_block.created_nullifiers().iter().copied().collect();

        for nullifier in pending_created_nullifiers {
            if created_nullifiers_block.contains(&nullifier) {
                anyhow::bail!(
                    "nullifier {nullifier} is already created by a transaction in the block so it cannot also be added through pending objects"
                );
            }

            self.nullifier_tree
                .mark_spent(nullifier, proven_block.header().block_num())
                .context("failed to insert pending nullifier into tree")?;

            proven_block.created_nullifiers_mut().push(nullifier);
        }

        // Add pending output notes to block.
        let output_notes_block: BTreeSet<NoteId> =
            proven_block.output_notes().map(|(_, output_note)| output_note.id()).collect();

        // We could distribute notes over multiple batches (if space is available), but most likely
        // one is sufficient.
        if self.pending_objects.output_notes.len() > MAX_OUTPUT_NOTES_PER_BATCH {
            anyhow::bail!(
                "cannot create more than {MAX_OUTPUT_NOTES_PER_BATCH} notes through pending objects"
            );
        }

        let mut pending_note_batch = Vec::with_capacity(self.pending_objects.output_notes.len());
        let pending_output_notes = core::mem::take(&mut self.pending_objects.output_notes);
        for (note_idx, output_note) in pending_output_notes.into_iter().enumerate() {
            if output_notes_block.contains(&output_note.id()) {
                anyhow::bail!(
                    "output note {} is already created by a transaction in the block so it cannot also be created through pending objects",
                    output_note.id()
                );
            }

            pending_note_batch.push((note_idx, output_note));
        }

        if (proven_block.output_note_batches().len() + 1) > MAX_BATCHES_PER_BLOCK {
            anyhow::bail!(
                "failed to add pending notes to block because max number of batches is already reached"
            )
        }

        proven_block.output_note_batches_mut().push(pending_note_batch);

        let updated_block_note_tree = proven_block.build_output_note_tree().root();

        // Update account tree and nullifier tree root in the block.
        let block_header = proven_block.header();
        let updated_header = BlockHeader::new(
            block_header.version(),
            block_header.prev_block_commitment(),
            block_header.block_num(),
            block_header.chain_commitment(),
            self.account_tree.root(),
            self.nullifier_tree.root(),
            updated_block_note_tree,
            block_header.tx_commitment(),
            block_header.tx_kernel_commitment(),
            block_header.proof_commitment(),
            block_header.timestamp(),
        );
        proven_block.set_block_header(updated_header);

        Ok(())
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
            let witness = self.nullifier_tree.open(&nullifier);
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
            if let Some(input_note) = self.committed_notes.get(&note) {
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
        &self.nullifier_tree
    }

    /// Returns the map of note IDs to committed notes.
    ///
    /// These notes are available for authenticated consumption.
    pub fn committed_notes(&self) -> &BTreeMap<NoteId, MockChainNote> {
        &self.committed_notes
    }

    /// Returns an [`InputNote`] for the given note ID. If the note does not exist or is not
    /// public, `None` is returned.
    pub fn get_public_note(&self, note_id: &NoteId) -> Option<InputNote> {
        let note = self.committed_notes.get(note_id)?;
        note.clone().try_into().ok()
    }

    /// Returns a reference to the account identifed by the given account ID and panics if it does
    /// not exist.
    pub fn committed_account(&self, account_id: AccountId) -> &Account {
        self.committed_accounts
            .get(&account_id)
            .expect("account should be available")
            .account()
    }

    /// Get the reference to the account tree.
    pub fn account_tree(&self) -> &AccountTree {
        &self.account_tree
    }
}

/// Creates the genesis state, consisting of a block containing the provided account updates and an
/// account tree with those accounts.
fn create_genesis_state(
    accounts: impl IntoIterator<Item = Account>,
) -> anyhow::Result<(ProvenBlock, AccountTree)> {
    let block_account_updates: Vec<BlockAccountUpdate> = accounts
        .into_iter()
        .map(|account| {
            BlockAccountUpdate::new(
                account.id(),
                account.commitment(),
                AccountUpdateDetails::New(account),
            )
        })
        .collect();

    let account_tree = AccountTree::with_entries(
        block_account_updates
            .iter()
            .map(|account| (account.account_id(), account.final_state_commitment())),
    )
    .context("failed to create genesis account tree")?;

    let output_note_batches = Vec::new();
    let created_nullifiers = Vec::new();
    let transactions = OrderedTransactionHeaders::new_unchecked(Vec::new());

    let version = 0;
    let prev_block_commitment = Digest::default();
    let block_num = BlockNumber::from(0u32);
    let chain_commitment = Blockchain::new().commitment();
    let account_root = account_tree.root();
    let nullifier_root = NullifierTree::new().root();
    let note_root = BlockNoteTree::empty().root();
    let tx_commitment = transactions.commitment();
    let tx_kernel_commitment = TransactionKernel::kernel_commitment();
    let proof_commitment = Digest::default();
    let timestamp = MockChain::TIMESTAMP_START_SECS;

    let header = BlockHeader::new(
        version,
        prev_block_commitment,
        block_num,
        chain_commitment,
        account_root,
        nullifier_root,
        note_root,
        tx_commitment,
        tx_kernel_commitment,
        proof_commitment,
        timestamp,
    );

    Ok((
        ProvenBlock::new_unchecked(
            header,
            block_account_updates,
            output_note_batches,
            created_nullifiers,
            transactions,
        ),
        account_tree,
    ))
}

impl Default for MockChain {
    fn default() -> Self {
        MockChain::new()
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

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use miden_objects::{
        account::{AccountStorage, AccountStorageMode},
        testing::account_component::AccountMockComponent,
    };

    use super::*;

    #[test]
    fn with_accounts() {
        let native_account = AccountBuilder::new([4; 32])
            .storage_mode(AccountStorageMode::Public)
            .with_component(
                AccountMockComponent::new_with_slots(
                    TransactionKernel::testing_assembler(),
                    vec![AccountStorage::mock_item_2().slot],
                )
                .unwrap(),
            )
            .build_existing()
            .unwrap();

        let mock_chain = MockChain::with_accounts(&[native_account.clone()]);

        assert_eq!(mock_chain.committed_account(native_account.id()), &native_account);

        // Check that transaction inputs retrieved from the chain are against the block header with
        // the current account tree root.
        let tx_context = mock_chain.build_tx_context(native_account.id(), &[], &[]).build();
        assert_eq!(tx_context.tx_inputs().block_header().block_num(), BlockNumber::from(0u32));
        assert_eq!(
            tx_context.tx_inputs().block_header().account_root(),
            mock_chain.account_tree.root()
        );
    }
}
