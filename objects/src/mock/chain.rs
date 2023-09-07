use crate::{
    mock::mock_block_header,
    notes::{Note, NoteInclusionProof, NOTE_LEAF_DEPTH, NOTE_TREE_DEPTH},
    ChainMmr, Felt, Vec,
};
use crypto::merkle::SimpleSmt;
use miden_core::{crypto::merkle::NodeIndex, FieldElement};

pub fn mock_chain_data(consumed_notes: &mut [Note]) -> ChainMmr {
    let mut note_trees = Vec::new();

    // TODO: Consider how to better represent note authentication data.
    // we use the index for both the block number and the leaf index in the note tree
    for (index, note) in consumed_notes.iter().enumerate() {
        let tree_index = 2 * index;
        let smt_entries = vec![
            (tree_index as u64, note.hash().into()),
            ((tree_index + 1) as u64, note.metadata().into()),
        ];
        let smt = SimpleSmt::with_leaves(NOTE_LEAF_DEPTH, smt_entries).unwrap();
        note_trees.push(smt);
    }

    let mut note_tree_iter = note_trees.iter();

    // create a dummy chain of block headers
    let block_chain = vec![
        mock_block_header(Felt::ZERO, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(Felt::ONE, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(Felt::new(2), None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(Felt::new(3), None, note_tree_iter.next().map(|x| x.root()), &[]),
    ];

    // instantiate and populate MMR
    let mut chain_mmr = ChainMmr::default();
    for block_header in block_chain.iter() {
        chain_mmr.mmr_mut().add(block_header.hash())
    }

    // set origin for consumed notes using chain and block data
    for (index, note) in consumed_notes.iter_mut().enumerate() {
        let block_header = &block_chain[index];
        let auth_index = NodeIndex::new(NOTE_TREE_DEPTH, index as u64).unwrap();
        note.set_proof(
            NoteInclusionProof::new(
                block_header.block_num(),
                block_header.sub_hash(),
                block_header.note_root(),
                index as u64,
                note_trees[index].get_path(auth_index).unwrap(),
            )
            .unwrap(),
        );
    }

    chain_mmr
}

#[cfg(feature = "mock")]
mod mock {
    use crate::{
        assets::Asset,
        builder::{
            AccountBuilder, AccountIdBuilder, FungibleAssetBuilder, NonFungibleAssetBuilder,
            DEFAULT_ACCOUNT_CODE,
        },
        notes::{Note, NoteInclusionProof, NOTE_LEAF_DEPTH, NOTE_TREE_DEPTH},
        Account, AccountError, AccountId, AccountType, AssetError, BlockHeader, ChainMmr, Felt,
        StorageItem, Vec,
    };
    use core::fmt;
    use crypto::{
        hash::rpo::RpoDigest as Digest,
        merkle::{MerkleError, NodeIndex, SimpleSmt, TieredSmt},
    };
    use miden_core::{FieldElement, StarkField};
    use rand::{Rng, SeedableRng};

    /// Initial timestamp value
    const TIMESTAMP_START: Felt = Felt::new(1693348223);
    /// Timestep of timestamp on each new block
    const TIMESTAMP_STEP: Felt = Felt::new(10);

    #[derive(Default, Debug, Clone)]
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct Objects<R: Rng> {
        accounts: Vec<Account>,
        fungible_faucets: Vec<(AccountId, FungibleAssetBuilder)>,
        nonfungible_faucets: Vec<(AccountId, NonFungibleAssetBuilder<R>)>,
        notes: Vec<Note>,
        nullifiers: Vec<Digest>,
    }

    impl<R: Rng> Objects<R> {
        pub fn new() -> Self {
            Self {
                accounts: vec![],
                fungible_faucets: vec![],
                nonfungible_faucets: vec![],
                notes: vec![],
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
            notes: &SimpleSmt,
        ) {
            self.accounts.extend(pending.accounts.drain(..));
            self.fungible_faucets.extend(pending.fungible_faucets.drain(..));
            self.nonfungible_faucets.extend(pending.nonfungible_faucets.drain(..));

            pending.set_notes_proofs(header, &notes);
            self.notes.extend(pending.notes.drain(..));
            pending.nullifiers.clear(); // nullifiers are saved in the nullifier TSTM
        }

        /// Creates a [SimpleSmt] tree from the `notes`.
        ///
        /// The root of the tree is a commitment to all notes created in the block. The commitment
        /// is not for all fields of the [Note] struct, but only for note metadata + core fields of
        /// a note (i.e., vault, inputs, script, and serial number).
        pub fn build_notes_tree(&self) -> Result<SimpleSmt, MerkleError> {
            let mut entries = Vec::with_capacity(self.notes.len() * 2);

            entries.extend(self.notes.iter().enumerate().map(|(index, note)| {
                let tree_index = (index * 2) as u64;
                (tree_index, note.hash().into())
            }));
            entries.extend(self.notes.iter().enumerate().map(|(index, note)| {
                let tree_index = (index * 2 + 1) as u64;
                (tree_index, note.metadata().into())
            }));

            SimpleSmt::with_leaves(NOTE_LEAF_DEPTH, entries)
        }

        /// Given the [BlockHeader] and its notedb's [SimpleSmt], set all the [Note]'s proof.
        ///
        /// Update the [Note]'s proof once the [BlockHeader] has been created.
        fn set_notes_proofs(&mut self, header: BlockHeader, notes: &SimpleSmt) {
            self.notes.iter_mut().enumerate().for_each(|(index, note)| {
                let auth_index =
                    NodeIndex::new(NOTE_TREE_DEPTH, index as u64).expect("index bigger than 2**20");
                let note_path =
                    notes.get_path(auth_index).expect("auth_index outside of SimpleSmt range");
                note.set_proof(
                    NoteInclusionProof::new(
                        header.block_num(),
                        header.sub_hash(),
                        header.note_root(),
                        index as u64,
                        note_path,
                    )
                    .expect("Invalid data provided to proof constructor"),
                );
            });
        }
    }

    /// Structure chain data, used to build necessary openings and to construct [BlockHeader]s.
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct MockChain<R: Rng + SeedableRng> {
        /// An append-only structure used to represent the history of blocks produced for this chain.
        chain: ChainMmr,

        /// History of produced blocks.
        blocks: Vec<BlockHeader>,

        /// Tree containing the latest `Nullifier`'s tree.
        nullifiers: TieredSmt,

        /// Tree containing the latest hash of each account.
        // TODO: change this to a TieredSmt with 64bit keys.
        accounts: SimpleSmt,

        /// RNG used to seed builders.
        ///
        /// This is used to seed the [AccountBuilder] and the [NonFungibleAssetBuilder].
        rng: R,

        /// Builder for new [AccountId]s of faucets.
        account_id_builder: AccountIdBuilder<R>,

        /// Objects that have been created and commited to a block.
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
        #[cfg_attr(feature = "serde", serde(skip))]
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

        pub fn new(mut rng: R) -> Result<Self, rand::Error> {
            let account_rng = R::from_rng(&mut rng).expect("rng seeding failed");
            let account_id_builder = AccountIdBuilder::new(account_rng);
            Ok(Self {
                chain: ChainMmr::default(),
                blocks: vec![],
                nullifiers: TieredSmt::default(),
                accounts: SimpleSmt::new(64).expect("depth too big for SimpleSmt"),
                rng,
                account_id_builder,
                objects: Objects::new(),
                pending_objects: Objects::new(),
            })
        }

        // MODIFIERS
        // ----------------------------------------------------------------------------------------

        /// Creates the next block.
        ///
        /// This will also make all the objects currently pending available for use.
        pub fn seal_block(&mut self) -> Result<BlockHeader, MerkleError> {
            let block_num: u64 = self.blocks.len().try_into().expect("usize to u64 failed");
            let block_num: Felt = block_num.into();

            for account in self.pending_objects.accounts.iter() {
                let id: Felt = account.id().into();
                self.accounts.update_leaf(id.as_int(), account.hash().into())?;
            }
            for account in self.objects.accounts.iter() {
                let id: Felt = account.id().into();
                self.accounts.update_leaf(id.as_int(), account.hash().into())?;
            }

            // TODO:
            // - resetting the nullifier tree once defined at the protocol level.
            // - insering only nullifier from transactions included in the batches, once the batch
            // kernel has been implemented.
            for nullifier in self.pending_objects.nullifiers.iter() {
                self.nullifiers
                    .insert(*nullifier, [block_num, Felt::ZERO, Felt::ZERO, Felt::ZERO]);
            }
            let notes = self.pending_objects.build_notes_tree()?;

            let previous = self.blocks.last();
            let peaks = self.chain.mmr().accumulator();
            let chain_root: Digest = peaks.hash_peaks().into();
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
            self.chain.mmr_mut().add(header.hash());
            self.objects.update_with(&mut self.pending_objects, header, &notes);

            Ok(header)
        }

        /// Creates a [Account] and add to the list of pending objects.
        pub fn new_account<C, S, A>(
            &mut self,
            code: C,
            storage: S,
            assets: A,
            immutable: Immutable,
            on_chain: OnChain,
        ) -> Result<AccountId, AccountError>
        where
            C: AsRef<str>,
            S: IntoIterator<Item = StorageItem>,
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
                .build()?;
            let account_id = account.id();
            self.pending_objects.accounts.push(account);
            Ok(account_id)
        }

        pub fn new_basic_wallet(&mut self) -> Result<AccountId, AccountError> {
            let rng = R::from_rng(&mut self.rng).expect("rng seeding failed");
            let account = AccountBuilder::new(rng)
                .account_type(AccountType::RegularAccountUpdatableCode)
                .on_chain(true)
                .code(DEFAULT_ACCOUNT_CODE)
                .build()?;
            let account_id = account.id();
            self.pending_objects.accounts.push(account);
            Ok(account_id)
        }

        /// Creates a [AccountId] with type [AccountType::FungibleFaucet] and add to the list of
        /// pending objects.
        pub fn new_fungible_faucet<C: AsRef<str>>(
            &mut self,
            on_chain: OnChain,
            code: C,
            storage_root: Digest,
        ) -> Result<AccountId, AccountError> {
            let faucet_id = self
                .account_id_builder
                .account_type(AccountType::FungibleFaucet)
                .on_chain(on_chain == OnChain::Yes)
                .code(code)
                .storage_root(storage_root)
                .build()?;
            let builder = FungibleAssetBuilder::new(faucet_id)
                .expect("builder was not configured to create fungible faucets");
            self.pending_objects.fungible_faucets.push((faucet_id, builder));
            Ok(faucet_id)
        }

        /// Creates a [AccountId] with type [AccountType::NonFungibleFaucet] and add to the list of
        /// pending objects.
        pub fn new_nonfungible_faucet<C: AsRef<str>>(
            &mut self,
            on_chain: OnChain,
            code: C,
            storage_root: Digest,
        ) -> Result<AccountId, AccountError> {
            let faucet_id = self
                .account_id_builder
                .account_type(AccountType::NonFungibleFaucet)
                .on_chain(on_chain == OnChain::Yes)
                .code(code)
                .storage_root(storage_root)
                .build()?;
            let rng = R::from_rng(&mut self.rng).expect("rng seeding failed");
            let builder = NonFungibleAssetBuilder::new(faucet_id, rng)
                .expect("builder was not configured to build nonfungible faucets");
            self.pending_objects.nonfungible_faucets.push((faucet_id, builder));
            Ok(faucet_id)
        }

        /// Creates [FungibleAsset] from the fungible faucet at position `faucet_pos`.
        pub fn new_fungible_asset(
            &mut self,
            faucet_pos: usize,
            amount: u64,
        ) -> Result<Asset, AssetError> {
            self.objects.fungible_faucets[faucet_pos]
                .1
                .amount(amount)?
                .build()
                .map(|v| v.into())
        }

        /// Creates [NonFungibleAsset] from the nonfungible faucet at position `faucet_pos`.
        pub fn new_nonfungible_asset(&mut self, faucet_pos: usize) -> Result<Asset, AssetError> {
            self.objects.nonfungible_faucets[faucet_pos].1.build().map(|v| v.into())
        }

        fn check_nullifier_unknown(&self, nullifier: Digest) -> Result<(), MockError> {
            if self.pending_objects.nullifiers.iter().find(|e| **e == nullifier).is_some() {
                return Err(MockError::DuplicatedNullifier);
            }

            if self.nullifiers.get_value(nullifier) != TieredSmt::EMPTY_VALUE {
                return Err(MockError::DuplicatedNullifier);
            }

            Ok(())
        }

        /// Mark a [Note] as produced by inserting into the block.
        pub fn add_note(&mut self, note: Note) -> Result<(), MockError> {
            if self.pending_objects.notes.iter().find(|e| e.hash() == note.hash()).is_some() {
                return Err(MockError::DuplicatedNote);
            }

            // The check below works because the notes can not be added directly to the
            // [BlockHeader], so we don't have to iterate over the known headers and check for
            // inclusion proofs.
            if self.objects.notes.iter().find(|e| e.hash() == note.hash()).is_some() {
                return Err(MockError::DuplicatedNote);
            }

            self.check_nullifier_unknown(note.nullifier())?;
            self.pending_objects.notes.push(note);
            Ok(())
        }

        /// Mark a [Note] as consumed by inserting its nullifier into the block.
        pub fn add_nullifier(&mut self, nullifier: Digest) -> Result<(), MockError> {
            self.check_nullifier_unknown(nullifier)?;
            self.pending_objects.nullifiers.push(nullifier);
            Ok(())
        }

        // ACESSORS
        // ----------------------------------------------------------------------------------------

        /// Get a reference to the latest [ChainMmr].
        pub fn chain(&self) -> &ChainMmr {
            &self.chain
        }

        /// Get a reference to [BlockHeader] with `block_number`.
        pub fn blockheader(&self, block_number: usize) -> &BlockHeader {
            &self.blocks[block_number]
        }

        /// Get a reference to the nullifier tree.
        pub fn nullifiers(&self) -> &TieredSmt {
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
            &mut self.objects.accounts[pos]
        }

        // Get the nth Note
        pub fn note(&self, pos: usize) -> &Note {
            &self.objects.notes[pos]
        }
    }
}

#[cfg(feature = "mock")]
pub use mock::*;
