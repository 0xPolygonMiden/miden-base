#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

pub mod accounts;

pub mod assets;
pub mod notes;

pub mod block;
pub use block::BlockHeader;

pub mod transaction;

mod errors;
pub use errors::{
    AccountDeltaError, AccountError, AssetError, ChainMmrError, NoteError, TransactionError,
    TransactionResultError, TransactionWitnessError,
};
// RE-EXPORTS
// ================================================================================================
pub use miden_crypto::hash::rpo::{Rpo256 as Hasher, RpoDigest as Digest};
pub use vm_core::{Felt, FieldElement, StarkField, Word, EMPTY_WORD, ONE, WORD_SIZE, ZERO};

pub mod assembly {
    pub use assembly::{
        ast::{AstSerdeOptions, ModuleAst, ProgramAst},
        Assembler, AssemblyContext, AssemblyError,
    };
}

pub mod crypto {
    pub use miden_crypto::{dsa, merkle, utils};
}

pub mod utils {
    pub use miden_crypto::utils::{format, vec};
    pub use vm_core::utils::{collections, string, IntoBytes};

    pub mod serde {
        pub use miden_crypto::utils::{
            ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
        };
    }
}

pub mod vm {
    pub use vm_core::{code_blocks::CodeBlock, Program};
    pub use vm_processor::{AdviceInputs, StackInputs};
}
