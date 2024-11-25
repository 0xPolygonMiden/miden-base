#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod accounts;
pub mod assets;
pub mod batches;
pub mod block;
pub mod notes;
pub mod transaction;

#[cfg(any(feature = "testing", test))]
pub mod testing;

mod constants;
mod errors;

// RE-EXPORTS
// ================================================================================================

pub use block::BlockHeader;
pub use constants::*;
pub use errors::{
    AccountDeltaError, AccountError, AssetError, AssetVaultError, BlockError, ChainMmrError,
    NoteError, ProvenTransactionError, TransactionInputError, TransactionOutputError,
    TransactionScriptError,
};
pub use miden_crypto::hash::rpo::{Rpo256 as Hasher, RpoDigest as Digest};
pub use vm_core::{Felt, FieldElement, StarkField, Word, EMPTY_WORD, ONE, WORD_SIZE, ZERO};

pub mod assembly {
    pub use assembly::{
        mast, Assembler, AssemblyError, DefaultSourceManager, KernelLibrary, Library,
        LibraryNamespace, LibraryPath, SourceManager, Version,
    };
}

pub mod crypto {
    pub use miden_crypto::{dsa, hash, merkle, rand, utils};
}

pub mod utils {
    pub use miden_crypto::utils::{bytes_to_hex_string, collections, hex_to_bytes, HexParseError};
    pub use vm_core::utils::*;

    pub mod serde {
        pub use miden_crypto::utils::{
            ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
        };
    }

    /// Construct a new `Digest` from four `u64` values.
    #[macro_export]
    macro_rules! digest {
        ($a:expr, $b:expr, $c:expr, $d:expr) => {
            Digest::new([Felt::new($a), Felt::new($b), Felt::new($c), Felt::new($d)])
        };
    }

    /// Construct a new `Digest` from long hex value.
    #[macro_export]
    macro_rules! digest_from_hex {
        ($hex:expr) => {{
            // hex prefix offset
            const START: usize = 2;

            const fn parse_hex_digit(digit: u8) -> u8 {
                match digit {
                    b'0'..=b'9' => digit - b'0',
                    b'A'..=b'F' => digit - b'A' + 10,
                    b'a'..=b'f' => digit - b'a' + 10,
                    _ => panic!("Invalid hex letter"),
                }
            }

            // Returns a byte array of the u64 value decoded from the hex byte array.
            //
            // Where:
            // - global_index is the index of the hex digit byte in the hex_bytes array
            // - upper_limit is the index at which the bytes of the current u64 value end
            const fn decode_u64_bytes(
                mut global_index: usize,
                mut hex_bytes: &[u8],
                mut upper_limit: usize,
            ) -> [u8; 8] {
                let mut u64_bytes = [0u8; 8];

                let mut local_index = 0;
                while global_index < upper_limit {
                    let upper = parse_hex_digit(hex_bytes[START + global_index]);
                    let lower = parse_hex_digit(hex_bytes[START + global_index + 1]);
                    u64_bytes[local_index] = upper << 4 | lower;

                    local_index += 1;
                    global_index += 2;
                }

                u64_bytes
            }

            let hex_bytes = $hex.as_bytes();

            if hex_bytes[0] != b'0' || hex_bytes[1] != b'x' {
                panic!("Hex string should start with \"0x\" prefix");
            }
            if hex_bytes.len() != 66 {
                panic!("Hex string has invalid length");
            }

            let v1 = u64::from_le_bytes(decode_u64_bytes(0, hex_bytes, 16));
            let v2 = u64::from_le_bytes(decode_u64_bytes(16, hex_bytes, 32));
            let v3 = u64::from_le_bytes(decode_u64_bytes(32, hex_bytes, 48));
            let v4 = u64::from_le_bytes(decode_u64_bytes(48, hex_bytes, 64));

            Digest::new([Felt::new(v1), Felt::new(v2), Felt::new(v3), Felt::new(v4)])
        }};
    }
}

pub mod vm {
    pub use miden_verifier::ExecutionProof;
    pub use vm_core::{Program, ProgramInfo};
    pub use vm_processor::{AdviceInputs, AdviceMap, RowIndex, StackInputs, StackOutputs};
}
