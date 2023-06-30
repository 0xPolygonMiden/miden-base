pub use crypto::{
    hash::rpo::{Rpo256 as Hasher, RpoDigest as Digest},
    merkle::{MerkleStore, NodeIndex, SimpleSmt},
    FieldElement, StarkField, ONE, ZERO,
};
pub use miden_lib::{memory, MidenLib};
pub use miden_objects::{
    assets::{Asset, FungibleAsset, NonFungibleAsset, NonFungibleAssetDetails},
    notes::{Note, NoteInclusionProof, NoteScript, NoteVault, NOTE_LEAF_DEPTH, NOTE_TREE_DEPTH},
    transaction::{ExecutedTransaction, PreparedTransaction, ProvenTransaction},
    Account, AccountCode, AccountId, AccountStorage, AccountType, AccountVault, BlockHeader,
    ChainMmr, StorageItem,
};
pub use processor::{
    math::Felt, AdviceInputs, AdviceProvider, ExecutionError, MemAdviceProvider, Process, Program,
    StackInputs, Word,
};

#[cfg(not(target_family = "wasm"))]
pub mod data;
