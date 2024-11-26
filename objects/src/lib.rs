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

    /// Construct a new `Digest` from a hex value.
    ///
    /// Marco supports hex strings of length 18, 34, 50 and 66 (with prefix).
    #[macro_export]
    macro_rules! digest {
        ($hex:expr) => {{
            // hex prefix offset
            const START: usize = 2;

            const fn parse_hex_digit(digit: u8) -> u8 {
                match digit {
                    b'0'..=b'9' => digit - b'0',
                    b'A'..=b'F' => digit - b'A' + 10,
                    b'a'..=b'f' => digit - b'a' + 10,
                    _ => panic!("Invalid hex character"),
                }
            }

            // Returns a byte array of the u64 value decoded from the hex byte array.
            //
            // Where:
            // - global_index is the index of the hex digit byte in the hex_bytes array
            // - upper_limit is the index at which the bytes of the current u64 value end
            const fn decode_u64_bytes(
                mut global_index: usize,
                hex_bytes: &[u8],
                upper_limit: usize,
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

            match hex_bytes.len() {
                18 => {
                    let v1 = u64::from_le_bytes(decode_u64_bytes(0, hex_bytes, 16));

                    Digest::new([Felt::new(v1), Felt::new(0u64), Felt::new(0u64), Felt::new(0u64)])
                },
                34 => {
                    let v1 = u64::from_le_bytes(decode_u64_bytes(0, hex_bytes, 16));
                    let v2 = u64::from_le_bytes(decode_u64_bytes(16, hex_bytes, 32));

                    Digest::new([Felt::new(v1), Felt::new(v2), Felt::new(0u64), Felt::new(0u64)])
                },
                50 => {
                    let v1 = u64::from_le_bytes(decode_u64_bytes(0, hex_bytes, 16));
                    let v2 = u64::from_le_bytes(decode_u64_bytes(16, hex_bytes, 32));
                    let v3 = u64::from_le_bytes(decode_u64_bytes(32, hex_bytes, 48));

                    Digest::new([Felt::new(v1), Felt::new(v2), Felt::new(v3), Felt::new(0u64)])
                },
                66 => {
                    let v1 = u64::from_le_bytes(decode_u64_bytes(0, hex_bytes, 16));
                    let v2 = u64::from_le_bytes(decode_u64_bytes(16, hex_bytes, 32));
                    let v3 = u64::from_le_bytes(decode_u64_bytes(32, hex_bytes, 48));
                    let v4 = u64::from_le_bytes(decode_u64_bytes(48, hex_bytes, 64));

                    Digest::new([Felt::new(v1), Felt::new(v2), Felt::new(v3), Felt::new(v4)])
                },
                _ => panic!("Hex string has invalid length"),
            }
        }};
    }

    /// Test the correctness of the `digest!` macro for every supported hex string length.
    #[test]
    fn test_digest_macro() {
        use crate::{Digest, Felt};

        let digest_18_hex = "0x8B5563E13FE8135D";
        let expected =
            Digest::try_from("0x8B5563E13FE8135D000000000000000000000000000000000000000000000000")
                .unwrap();
        let result_18 = digest!(digest_18_hex);
        assert_eq!(expected, result_18);

        let digest_34_hex = "0x64FA911355C7818D58E3EE5BC6D1452D";
        let expected =
            Digest::try_from("0x64FA911355C7818D58E3EE5BC6D1452D00000000000000000000000000000000")
                .unwrap();
        let result_34 = digest!(digest_34_hex);
        assert_eq!(expected, result_34);

        let digest_50_hex = "0x12613C3D43EE1A4C7B528B8AB68A23A31A45336DAFDEDAC9";
        let expected =
            Digest::try_from("0x12613C3D43EE1A4C7B528B8AB68A23A31A45336DAFDEDAC90000000000000000")
                .unwrap();
        let result_50 = digest!(digest_50_hex);
        assert_eq!(expected, result_50);

        let digest_66_hex = "0x7F75ED9C5826FCAF77A5B227D66D6A874003027009783494C55FE741E87D89BC";
        let expected =
            Digest::try_from("0x7F75ED9C5826FCAF77A5B227D66D6A874003027009783494C55FE741E87D89BC")
                .unwrap();
        let result_66 = digest!(digest_66_hex);
        assert_eq!(expected, result_66);
    }
}

pub mod vm {
    pub use miden_verifier::ExecutionProof;
    pub use vm_core::{Program, ProgramInfo};
    pub use vm_processor::{AdviceInputs, AdviceMap, RowIndex, StackInputs, StackOutputs};
}
