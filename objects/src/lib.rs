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
    /// Expects a '0x' prefixed hex string followed by upto 64 hex digits.
    #[macro_export]
    macro_rules! digest {
        ($hex:expr) => {{
            const fn parse_hex_digit(digit: u8) -> u8 {
                match digit {
                    b'0'..=b'9' => digit - b'0',
                    b'A'..=b'F' => digit - b'A' + 0x0a,
                    b'a'..=b'f' => digit - b'a' + 0x0a,
                    _ => panic!("Invalid hex character"),
                }
            }

            // Enforce and skip the '0x' prefix.
            let hex_bytes = match $hex.as_bytes() {
                [b'0', b'x', rest @ ..] => rest,
                _ => panic!(r#"Hex string must have a "0x" prefix"#),
            };

            if hex_bytes.len() > 64 {
                panic!("Hex string has more than 64 characters");
            }

            // Aggregate each byte into the appropriate felt value. Doing it this way also allows
            // for variable string lengths.
            let mut felts = [0u64; 4];
            let mut i = 0;
            // We are forced to use a while loop because the others aren't supported in const
            // context.
            while i < hex_bytes.len() {
                // A hex-value is two characters per byte, and we need to reverse index to account
                // for LE -> BE.
                let ibyte = hex_bytes.len() - 1 - i;
                let ifelt = felts.len() - 1 - (i / 2 / 8);

                // This digit's nibble offset within the felt.
                let inibble = i % (2 * 8);

                // SAFETY: u8 cast to u64 is safe. We cannot use u64::from in const context so we
                // are forced to cast.
                let value = (parse_hex_digit(hex_bytes[ibyte]) as u64) << (inibble * 4);
                felts[ifelt] += value;

                i += 1;
            }

            $crate::Digest::new([
                $crate::Felt::new(felts[0]),
                $crate::Felt::new(felts[1]),
                $crate::Felt::new(felts[2]),
                $crate::Felt::new(felts[3]),
            ])
        }};
    }

    #[cfg(test)]
    mod tests {
        #[test]
        #[should_panic]
        fn digest_macro_missing_prefix() {
            digest!("1234");
        }

        #[rstest::rstest]
        #[case::each_digit("0x1234567890abcdef", [0, 0, 0, 0x1234567890abcdef])]
        #[case::empty("0x", Default::default())]
        #[case::zero("0x0", Default::default())]
        #[case::zero_full(
            "0x0000000000000000000000000000000000000000000000000000000000000000",
            Default::default()
        )]
        #[case::one("0x1", [0, 0, 0, 1])]
        #[case::one_full("0x0000000000000000000000000000000000000000000000000000000000000001", [0, 0, 0, 1])]
        #[case::one_partial("0x0001", [0, 0, 0, 1])]
        #[case::odd("0x123", [0, 0, 0, 0x123])]
        #[case::even("0x1234", [0, 0, 0, 0x1234])]
        #[case::touch_each_felt(
            "0x00000000000123450000000000067890000000000000abcd00000000000000ef",
            [0x12345, 0x67890, 0xabcd, 0xef]
        )]
        #[case::digits_on_repeat(
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            [0x1234567890abcdef, 0x1234567890abcdef, 0x1234567890abcdef, 0x1234567890abcdef]
        )]
        #[case::max(
            "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            [u64::MAX, u64::MAX, u64::MAX, u64::MAX]
        )]
        fn digest_macro(#[case] hex_str: &str, #[case] expected: [u64; 4]) {
            use crate::{Digest, Felt};

            let uut = digest!(hex_str);

            let expected = Digest::new([
                Felt::new(expected[0]),
                Felt::new(expected[1]),
                Felt::new(expected[2]),
                Felt::new(expected[3]),
            ]);

            assert_eq!(uut, expected);
        }
    }
}

pub mod vm {
    pub use miden_verifier::ExecutionProof;
    pub use vm_core::{Program, ProgramInfo};
    pub use vm_processor::{AdviceInputs, AdviceMap, RowIndex, StackInputs, StackOutputs};
}
