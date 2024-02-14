#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

pub mod accounts;
pub mod assets;
pub mod block;
pub mod notes;
pub mod transaction;

mod errors;

// RE-EXPORTS
// ================================================================================================

pub use block::BlockHeader;
pub use errors::{
    AccountDeltaError, AccountError, AssetError, AssetVaultError, ChainMmrError, NoteError,
    TransactionInputError, TransactionOutputError, TransactionScriptError,
};
pub use miden_crypto::hash::rpo::{Rpo256 as Hasher, RpoDigest as Digest};
pub use vm_core::{Felt, FieldElement, StarkField, Word, EMPTY_WORD, ONE, WORD_SIZE, ZERO};

pub mod assembly {
    pub use assembly::{
        ast::{AstSerdeOptions, ModuleAst, ProgramAst},
        Assembler, AssemblyContext, AssemblyError, Library, LibraryNamespace, LibraryPath,
        MaslLibrary, Version,
    };
}

pub mod crypto {
    pub use miden_crypto::{dsa, hash, merkle, rand, utils};
}

pub mod utils {
    pub use miden_crypto::utils::{bytes_to_hex_string, format, hex_to_bytes, vec, HexParseError};
    pub use vm_core::utils::{collections, group_slice_elements, string, IntoBytes};

    pub mod serde {
        pub use miden_crypto::utils::{
            ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
        };
    }
}

pub mod vm {
    pub use miden_verifier::ExecutionProof;
    pub use vm_core::{code_blocks::CodeBlock, Program, ProgramInfo};
    pub use vm_processor::{AdviceInputs, StackInputs, StackOutputs};
}

// CONSTANTS
// ================================================================================================

/// Depth of the account database tree.
pub const ACCOUNT_TREE_DEPTH: u8 = 64;

/// The depth of the Merkle tree used to commit to notes produced in a block.
pub const NOTE_TREE_DEPTH: u8 = 20;

/// The maximum number of assets that can be stored in a single note.
pub const MAX_ASSETS_PER_NOTE: usize = 256;

/// The maximum number of inputs that can accompany a single note.
///
/// The value is set to 128 so that it can be represented using as a single byte while being
/// evenly divisible by 8.
pub const MAX_INPUTS_PER_NOTE: usize = 128;

/// The maximum number of notes that can be consumed by a single transaction.
pub const MAX_INPUT_NOTES_PER_TX: usize = 1023;

/// The maximum number of new notes created by a single transaction.
pub const MAX_OUTPUT_NOTES_PER_TX: usize = 4096;

/// The minimum proof security level used by the Miden prover & verifier.
pub const MIN_PROOF_SECURITY_LEVEL: u32 = 96;
