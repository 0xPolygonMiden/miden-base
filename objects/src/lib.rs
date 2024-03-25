#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod accounts;
pub mod assets;
pub mod block;
pub mod notes;
pub mod transaction;

mod constants;
mod errors;

// RE-EXPORTS
// ================================================================================================

pub use block::BlockHeader;
pub use constants::*;
pub use errors::{
    AccountDeltaError, AccountError, AssetError, AssetVaultError, ChainMmrError, NoteError,
    ProvenTransactionError, TransactionInputError, TransactionOutputError, TransactionScriptError,
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
    pub use vm_core::utils::*;

    pub mod serde {
        pub use miden_crypto::utils::{
            ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
        };
    }
}

pub mod vm {
    pub use miden_verifier::ExecutionProof;
    pub use vm_core::{code_blocks::CodeBlock, Program, ProgramInfo};
    pub use vm_processor::{AdviceInputs, AdviceMap, StackInputs, StackOutputs};
}
