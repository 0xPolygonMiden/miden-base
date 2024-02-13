use core::fmt;

use miden_objects::{
    accounts::{Account, AccountId, AccountType, SlotItem},
    assets::Asset,
    crypto::merkle::{LeafIndex, Mmr, PartialMmr, SimpleSmt, Smt},
    notes::{Note, NoteLocation},
    transaction::{ChainMmr, InputNote},
    utils::collections::Vec,
    BlockHeader, Digest, Felt, FieldElement, Word, ACCOUNT_TREE_DEPTH, NOTE_TREE_DEPTH,
};
use rand::{Rng, SeedableRng};

use super::{
    block::mock_block_header,
    builders::{
        accountid_build_details, AccountBuilder, AccountIdBuilder, AccountStorageBuilder,
        FungibleAssetBuilder, NonFungibleAssetBuilder,
    },
    constants::DEFAULT_ACCOUNT_CODE,
};

/// Initial timestamp value
const TIMESTAMP_START: Felt = Felt::new(1693348223);
/// Timestamp of timestamp on each new block
const TIMESTAMP_STEP: Felt = Felt::new(10);

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Objects<R> {
    /// Holds the account and its corresponding seed.
    accounts: Vec<(Account, Word)>,
    fungible_faucets: Vec<(AccountId, FungibleAssetBuilder)>,
    nonfungible_faucets: Vec<(AccountId, NonFungibleAssetBuilder<R>)>,
    notes: Vec<Note>,
    recorded_notes: Vec<InputNote>,
    nullifiers: Vec<Digest>,
}

impl<R: Rng> Objects<R> {
    pub fn new() -> Self {
        Self {
            accounts: vec![],
            fungible_faucets: vec![],
            nonfungible_faucets: vec![],
            notes: vec![],
            recorded_notes: vec![],
            nullifiers: vec![],
        }
    }

    /// Update this instance with objects inserted in the chain.
    ///
    /// This method expects `pending` to be a list of objects in the pending block, and for
    /// this instance to be the set of objects added to the chain. Once the pending block is
    /// sealed and the auxiliary data is produced (i.e. the notes tree), this method can be
    /// called to 1. update the pending objects with the new data 2. move the objects to this
    /// container.
    pub fn update_with(
        &mut self,
        pending: &mut Objects<R>,
        header: BlockHeader,
        notes: &SimpleSmt<NOTE_TREE_DEPTH>,
    ) {
        self.accounts.append(&mut pending.accounts);
        self.fungible_faucets.append(&mut pending.fungible_faucets);
        self.nonfungible_faucets.append(&mut pending.nonfungible_faucets);

        let recorded_notes = pending.finalize_notes(header, notes);
        self.recorded_notes.extend(recorded_notes);
        pending.nullifiers.clear(); // nullifiers are saved in the nullifier TSTM
    }

    /// Creates a [SimpleSmt] tree from the `notes`.
    ///
    /// The root of the tree is a commitment to all notes created in the block. The commitment
    /// is not for all fields of the [Note] struct, but only for note metadata + core fields of
    /// a note (i.e., vault, inputs, script, and serial number).
    pub fn build_notes_tree(&self) -> SimpleSmt<NOTE_TREE_DEPTH> {
        let mut entries = Vec::with_capacity(self.notes.len() * 2);

        entries.extend(self.notes.iter().enumerate().map(|(index, note)| {
            let tree_index = (index * 2) as u64;
            (tree_index, note.id().into())
        }));
        entries.extend(self.notes.iter().enumerate().map(|(index, note)| {
            let tree_index = (index * 2 + 1) as u64;
            (tree_index, note.metadata().into())
        }));

        SimpleSmt::with_leaves(entries).unwrap()
    }

    /// Given the [BlockHeader] and its notedb's [SimpleSmt], set all the [Note]'s proof.
    ///
    /// Update the [Note]'s proof once the [BlockHeader] has been created.
    fn finalize_notes(
        &mut self,
        header: BlockHeader,
        notes: &SimpleSmt<NOTE_TREE_DEPTH>,
    ) -> Vec<InputNote> {
        self.notes
            .drain(..)
            .enumerate()
            .map(|(index, note)| {
                let auth_index = LeafIndex::new(index as u64).expect("index bigger than 2**20");
                let location = NoteLocation::new(header.block_num(), auth_index.value() as u32);
                InputNote::new(note.clone(), location, notes.open(&auth_index).path)
            })
            .collect::<Vec<_>>()
    }
}

/// Structure chain data, used to build necessary openings and to construct [BlockHeader]s.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct MockChain<R> {
    /// An append-only structure used to represent the history of blocks produced for this chain.
    chain: Mmr,

    /// History of produced blocks.
    blocks: Vec<BlockHeader>,

    /// Tree containing the latest `Nullifier`'s tree.
    nullifiers: Smt,

    /// Tree containing the latest hash of each account.
    accounts: SimpleSmt<ACCOUNT_TREE_DEPTH>,

    /// RNG used to seed builders.
    ///
    /// This is used to seed the [AccountBuilder] and the [NonFungibleAssetBuilder].
    rng: R,

    /// Builder for new [AccountId]s of faucets.
    account_id_builder: AccountIdBuilder<R>,

    /// Objects that have been created and committed to a block.
    ///
    /// These can be used to perform additional operations on a block.
    ///
    /// Note:
    /// - The [Note]s in this container have the `proof` set.
    objects: Objects<R>,

    /// Objects that have been created and are waiting for a block.
    ///
    /// These objects will become available once the block is sealed.
    ///
    /// Note:
    /// - The [Note]s in this container do not have the `proof` set.
    pending_objects: Objects<R>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OnChain {
    No,
    Yes,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Immutable {
    No,
    Yes,
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

impl<R: Rng + SeedableRng> MockChain<R> {
    // CONSTRUCTORS
    // ----------------------------------------------------------------------------------------

    pub fn new(mut rng: R) -> Self {
        let account_rng = R::from_rng(&mut rng).expect("rng seeding failed");
        let account_id_builder = AccountIdBuilder::new(account_rng);
        Self {
            chain: Mmr::default(),
            blocks: vec![],
            nullifiers: Smt::default(),
            accounts: SimpleSmt::<ACCOUNT_TREE_DEPTH>::new().expect("depth too big for SimpleSmt"),
            rng,
            account_id_builder,
            objects: Objects::new(),
            pending_objects: Objects::new(),
        }
    }

    // BUILDERS
    // ----------------------------------------------------------------------------------------

    /// Creates an [Account] and add to the list of pending objects.
    pub fn build_account<C, S, A>(
        &mut self,
        code: C,
        storage: S,
        assets: A,
        immutable: Immutable,
        on_chain: OnChain,
    ) -> AccountId
    where
        C: AsRef<str>,
        S: IntoIterator<Item = SlotItem>,
        A: IntoIterator<Item = Asset>,
    {
        let account_type = match immutable {
            Immutable::Yes => AccountType::RegularAccountImmutableCode,
            Immutable::No => AccountType::RegularAccountUpdatableCode,
        };

        let on_chain_bool = match on_chain {
            OnChain::Yes => true,
            OnChain::No => false,
        };

        let storage = AccountStorageBuilder::new().add_items(storage).build();

        let (seed, _) = accountid_build_details(
            &mut self.rng,
            code.as_ref(),
            account_type,
            on_chain_bool,
            storage.root(),
        )
        .unwrap();

        let rng = R::from_rng(&mut self.rng).expect("rng seeding failed");
        let account = AccountBuilder::new(rng)
            .add_assets(assets)
            .account_type(account_type)
            .on_chain(on_chain == OnChain::Yes)
            .code(code)
            .with_seed_and_storage(seed, storage)
            .unwrap();
        let account_id = account.id();
        self.pending_objects.accounts.push((account, seed));
        account_id
    }

    /// Creates an [Account] using `seed` and add to the list of pending objects.
    pub fn build_account_with_seed<C, S, A>(
        &mut self,
        seed: Word,
        code: C,
        storage: S,
        assets: A,
        immutable: Immutable,
        on_chain: OnChain,
    ) -> AccountId
    where
        C: AsRef<str>,
        S: IntoIterator<Item = SlotItem>,
        A: IntoIterator<Item = Asset>,
    {
        let account_type = match immutable {
            Immutable::Yes => AccountType::RegularAccountImmutableCode,
            Immutable::No => AccountType::RegularAccountUpdatableCode,
        };

        let rng = R::from_rng(&mut self.rng).expect("rng seeding failed");
        let account = AccountBuilder::new(rng)
            .add_storage_items(storage)
            .add_assets(assets)
            .account_type(account_type)
            .on_chain(on_chain == OnChain::Yes)
            .code(code)
            .with_seed(seed)
            .unwrap();
        let account_id = account.id();
        self.pending_objects.accounts.push((account, seed));
        account_id
    }

    pub fn build_basic_wallet(&mut self) -> AccountId {
        let account_type = AccountType::RegularAccountUpdatableCode;
        let on_chain = true;
        let storage = AccountStorageBuilder::new().build();
        let (seed, _) = accountid_build_details(
            &mut self.rng,
            DEFAULT_ACCOUNT_CODE,
            account_type,
            on_chain,
            storage.root(),
        )
        .unwrap();
        let rng = R::from_rng(&mut self.rng).expect("rng seeding failed");
        let account = AccountBuilder::new(rng)
            .account_type(account_type)
            .on_chain(on_chain)
            .code(DEFAULT_ACCOUNT_CODE)
            .build()
            .unwrap();
        let account_id = account.id();
        self.pending_objects.accounts.push((account, seed));
        account_id
    }

    /// Creates a [AccountId] with type [AccountType::FungibleFaucet] and add to the list of
    /// pending objects.
    pub fn build_fungible_faucet<C: AsRef<str>>(
        &mut self,
        on_chain: OnChain,
        code: C,
        storage_root: Digest,
    ) -> AccountId {
        let faucet_id = self
            .account_id_builder
            .account_type(AccountType::FungibleFaucet)
            .on_chain(on_chain == OnChain::Yes)
            .code(code)
            .storage_root(storage_root)
            .build()
            .unwrap();
        let builder = FungibleAssetBuilder::new(faucet_id)
            .expect("builder was not configured to create fungible faucets");
        self.pending_objects.fungible_faucets.push((faucet_id, builder));
        faucet_id
    }

    /// Creates a [AccountId] with type [AccountType::FungibleFaucet] and add to the list of
    /// pending objects.
    pub fn build_fungible_faucet_with_seed<C: AsRef<str>>(
        &mut self,
        seed: Word,
        on_chain: OnChain,
        code: C,
        storage_root: Digest,
    ) -> AccountId {
        let faucet_id = self
            .account_id_builder
            .account_type(AccountType::FungibleFaucet)
            .on_chain(on_chain == OnChain::Yes)
            .code(code)
            .storage_root(storage_root)
            .with_seed(seed)
            .unwrap();
        let builder = FungibleAssetBuilder::new(faucet_id)
            .expect("builder was not configured to create fungible faucets");
        self.pending_objects.fungible_faucets.push((faucet_id, builder));
        faucet_id
    }

    /// Creates a [AccountId] with type [AccountType::NonFungibleFaucet] and add to the list of
    /// pending objects.
    pub fn build_nonfungible_faucet<C: AsRef<str>>(
        &mut self,
        on_chain: OnChain,
        code: C,
        storage_root: Digest,
    ) -> AccountId {
        let faucet_id = self
            .account_id_builder
            .account_type(AccountType::NonFungibleFaucet)
            .on_chain(on_chain == OnChain::Yes)
            .code(code)
            .storage_root(storage_root)
            .build()
            .unwrap();
        let rng = R::from_rng(&mut self.rng).expect("rng seeding failed");
        let builder = NonFungibleAssetBuilder::new(faucet_id, rng)
            .expect("builder was not configured to build nonfungible faucets");
        self.pending_objects.nonfungible_faucets.push((faucet_id, builder));
        faucet_id
    }

    /// Creates a [AccountId] with type [AccountType::NonFungibleFaucet] and add to the list of
    /// pending objects.
    pub fn build_nonfungible_faucet_with_seed<C: AsRef<str>>(
        &mut self,
        seed: Word,
        on_chain: OnChain,
        code: C,
        storage_root: Digest,
    ) -> AccountId {
        let faucet_id = self
            .account_id_builder
            .account_type(AccountType::NonFungibleFaucet)
            .on_chain(on_chain == OnChain::Yes)
            .code(code)
            .storage_root(storage_root)
            .with_seed(seed)
            .unwrap();
        let rng = R::from_rng(&mut self.rng).expect("rng seeding failed");
        let builder = NonFungibleAssetBuilder::new(faucet_id, rng)
            .expect("builder was not configured to build nonfungible faucets");
        self.pending_objects.nonfungible_faucets.push((faucet_id, builder));
        faucet_id
    }

    /// Creates [FungibleAsset] from the fungible faucet at position `faucet_pos`.
    pub fn build_fungible_asset(&mut self, faucet_pos: usize, amount: u64) -> Asset {
        self.objects.fungible_faucets[faucet_pos]
            .1
            .amount(amount)
            .unwrap()
            .build()
            .map(|v| v.into())
            .unwrap()
    }

    /// Creates [NonFungibleAsset] from the nonfungible faucet at position `faucet_pos`.
    pub fn build_nonfungible_asset(&mut self, faucet_pos: usize) -> Asset {
        self.objects.nonfungible_faucets[faucet_pos]
            .1
            .build()
            .map(|v| v.into())
            .unwrap()
    }

    fn check_nullifier_unknown(&self, nullifier: Digest) {
        assert!(self.pending_objects.nullifiers.iter().any(|e| *e == nullifier));
        assert!(self.nullifiers.get_value(&nullifier) != Smt::EMPTY_VALUE)
    }

    // MODIFIERS
    // ----------------------------------------------------------------------------------------

    /// Creates the next block.
    ///
    /// This will also make all the objects currently pending available for use.
    pub fn seal_block(&mut self) -> BlockHeader {
        let block_num: u32 = self.blocks.len().try_into().expect("usize to u32 failed");

        for (account, _seed) in self.pending_objects.accounts.iter() {
            self.accounts.insert(account.id().into(), account.hash().into());
        }
        for (account, _seed) in self.objects.accounts.iter() {
            self.accounts.insert(account.id().into(), account.hash().into());
        }

        // TODO:
        // - resetting the nullifier tree once defined at the protocol level.
        // - inserting only nullifier from transactions included in the batches, once the batch
        // kernel has been implemented.
        for nullifier in self.pending_objects.nullifiers.iter() {
            self.nullifiers
                .insert(*nullifier, [block_num.into(), Felt::ZERO, Felt::ZERO, Felt::ZERO]);
        }
        let notes = self.pending_objects.build_notes_tree();

        let previous = self.blocks.last();
        let peaks = self.chain.peaks(self.chain.forest()).unwrap();
        let chain_root: Digest = peaks.hash_peaks();
        let account_root = self.accounts.root();
        let prev_hash = previous.map_or(Digest::default(), |header| header.hash());
        let nullifier_root = self.nullifiers.root();
        let note_root = notes.root();
        let version = Felt::ZERO;
        let timestamp =
            previous.map_or(TIMESTAMP_START, |header| header.timestamp() + TIMESTAMP_STEP);

        // TODO: Set batch_root and proof_hash to the correct values once the kernel is
        // available.
        let batch_root = Digest::default();
        let proof_hash = Digest::default();

        let header = BlockHeader::new(
            prev_hash,
            block_num,
            chain_root,
            account_root,
            nullifier_root,
            note_root,
            batch_root,
            proof_hash,
            version,
            timestamp,
        );

        self.blocks.push(header);
        self.chain.add(header.hash());
        self.objects.update_with(&mut self.pending_objects, header, &notes);

        header
    }

    /// Mark a [Note] as produced by inserting into the block.
    pub fn add_note(&mut self, note: Note) -> Result<(), MockError> {
        if self.pending_objects.notes.iter().any(|e| e.id() == note.id()) {
            return Err(MockError::DuplicatedNote);
        }

        // The check below works because the notes can not be added directly to the
        // [BlockHeader], so we don't have to iterate over the known headers and check for
        // inclusion proofs.
        if self.objects.recorded_notes.iter().any(|e| e.id() == note.id()) {
            return Err(MockError::DuplicatedNote);
        }

        self.check_nullifier_unknown(note.nullifier().inner());
        self.pending_objects.notes.push(note);
        Ok(())
    }

    /// Mark a [Note] as consumed by inserting its nullifier into the block.
    pub fn add_nullifier(&mut self, nullifier: Digest) -> Result<(), MockError> {
        self.check_nullifier_unknown(nullifier);
        self.pending_objects.nullifiers.push(nullifier);
        Ok(())
    }

    /// Add a known [Account] to the mock chain.
    pub fn add_account(&mut self, account: Account, seed: Word) {
        assert!(
            !self.pending_objects.accounts.iter().any(|(a, _)| a.id() == account.id()),
            "Found duplicated AccountId"
        );
        self.pending_objects.accounts.push((account, seed));
    }

    // ACCESSORS
    // ----------------------------------------------------------------------------------------

    /// Get the latest [ChainMmr].
    pub fn chain(&self) -> ChainMmr {
        mmr_to_chain_mmr(&self.chain, &self.blocks)
    }

    /// Get a reference to [BlockHeader] with `block_number`.
    pub fn block_header(&self, block_number: usize) -> &BlockHeader {
        &self.blocks[block_number]
    }

    /// Get a reference to the nullifier tree.
    pub fn nullifiers(&self) -> &Smt {
        &self.nullifiers
    }

    /// Get the [AccountId] of the nth fungible faucet.
    pub fn fungible(&self, faucet_pos: usize) -> AccountId {
        self.objects.fungible_faucets[faucet_pos].0
    }

    /// Get the [AccountId] of the nth nonfungible faucet.
    pub fn nonfungible(&self, faucet_pos: usize) -> AccountId {
        self.objects.nonfungible_faucets[faucet_pos].0
    }

    /// Get a mutable reference to nth [Account].
    pub fn account_mut(&mut self, pos: usize) -> &mut Account {
        &mut self.objects.accounts[pos].0
    }

    /// Get the [Account]'s corresponding seed.
    pub fn account_seed(&mut self, pos: usize) -> Word {
        self.objects.accounts[pos].1
    }
}

// SERIALIZATION
// --------------------------------------------------------------------------------------------
#[cfg(feature = "serde")]
use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

#[cfg(feature = "serde")]
impl<R: serde::Serialize> MockChain<R> {
    pub fn to_file<T: AsRef<Path>>(self, path: T) -> io::Result<()> {
        let encoded = postcard::to_allocvec(&self).unwrap();
        File::create(path)?.write_all(&encoded)?;
        Ok(())
    }
}

#[cfg(feature = "serde")]
pub fn from_file<T, P>(path: P) -> io::Result<MockChain<T>>
where
    P: AsRef<Path>,
    MockChain<T>: serde::de::DeserializeOwned,
{
    let mut data = Vec::new();
    File::open(path)?.read_to_end(&mut data)?;
    let data: MockChain<T> = postcard::from_bytes(&data).unwrap();
    Ok(data)
}

pub fn mock_chain_data(consumed_notes: Vec<Note>) -> (ChainMmr, Vec<InputNote>) {
    let mut note_trees = Vec::new();

    // TODO: Consider how to better represent note authentication data.
    // we use the index for both the block number and the leaf index in the note tree
    for (index, note) in consumed_notes.iter().enumerate() {
        let smt_entries = vec![(index as u64, note.authentication_hash().into())];
        let smt = SimpleSmt::<NOTE_TREE_DEPTH>::with_leaves(smt_entries).unwrap();
        note_trees.push(smt);
    }

    let mut note_tree_iter = note_trees.iter();

    // create a dummy chain of block headers
    let block_chain = vec![
        mock_block_header(0, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(1, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(2, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(3, None, note_tree_iter.next().map(|x| x.root()), &[]),
    ];

    // instantiate and populate MMR
    let mut mmr = Mmr::default();
    for block_header in block_chain.iter() {
        mmr.add(block_header.hash())
    }
    let chain_mmr = mmr_to_chain_mmr(&mmr, &block_chain);

    // set origin for consumed notes using chain and block data
    let recorded_notes = consumed_notes
        .into_iter()
        .enumerate()
        .map(|(index, note)| {
            let block_header = &block_chain[index];
            let auth_index = LeafIndex::new(index as u64).unwrap();
            let location = NoteLocation::new(block_header.block_num(), auth_index.value() as u32);
            InputNote::new(note, location, note_trees[index].open(&auth_index).path)
        })
        .collect::<Vec<_>>();

    (chain_mmr, recorded_notes)
}

// HELPER FUNCTIONS
// ================================================================================================

/// Converts the MMR into partial MMR by copying all leaves from MMR to partial MMR.
fn mmr_to_chain_mmr(mmr: &Mmr, blocks: &[BlockHeader]) -> ChainMmr {
    let num_leaves = mmr.forest();
    let mut partial_mmr = PartialMmr::from_peaks(mmr.peaks(mmr.forest()).unwrap());

    for i in 0..num_leaves {
        let node = mmr.get(i).unwrap();
        let path = mmr.open(i, mmr.forest()).unwrap().merkle_path;
        partial_mmr.track(i, node, &path).unwrap();
    }

    ChainMmr::new(partial_mmr, blocks.to_vec()).unwrap()
}
