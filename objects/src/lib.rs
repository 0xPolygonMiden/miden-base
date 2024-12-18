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
    /// Expects a '0x' prefixed hex string followed by up to 64 hex digits.
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
                // This digit's nibble offset within the felt. We need to invert the nibbles per
                // byte for endianess reasons i.e. ABCD -> BADC.
                let inibble = if i % 2 == 0 { (i + 1) % 16 } else { (i - 1) % 16 };

                // SAFETY: u8 cast to u64 is safe. We cannot use u64::from in const context so we
                // are forced to cast.
                let value = (parse_hex_digit(hex_bytes[i]) as u64) << (inibble * 4);
                felts[i / 2 / 8] += value;

                i += 1;
            }

            // Ensure each felt is within bounds as `Felt::new` silently wraps around.
            // This matches the behaviour of `Digest::try_from(String)`.
            let mut i = 0;
            while i < felts.len() {
                use $crate::StarkField;
                if felts[i] > $crate::Felt::MODULUS {
                    panic!("Felt overflow");
                }
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
        #[rstest::rstest]
        #[case::missing_prefix("1234")]
        #[case::invalid_character("1234567890abcdefg")]
        #[case::too_long("0xx00000000000000000000000000000000000000000000000000000000000000001")]
        #[case::overflow_felt0(
            "0xffffffffffffffff000000000000000000000000000000000000000000000000"
        )]
        #[case::overflow_felt1(
            "0x0000000000000000ffffffffffffffff00000000000000000000000000000000"
        )]
        #[case::overflow_felt2(
            "0x00000000000000000000000000000000ffffffffffffffff0000000000000000"
        )]
        #[case::overflow_felt3(
            "0x000000000000000000000000000000000000000000000000ffffffffffffffff"
        )]
        #[should_panic]
        fn digest_macro_invalid(#[case] bad_input: &str) {
            digest!(bad_input);
        }

        #[rstest::rstest]
        #[case::each_digit("0x1234567890abcdef")]
        #[case::empty("0x")]
        #[case::zero("0x0")]
        #[case::zero_full("0x0000000000000000000000000000000000000000000000000000000000000000")]
        #[case::one_lsb("0x1")]
        #[case::one_msb("0x0000000000000000000000000000000000000000000000000000000000000001")]
        #[case::one_partial("0x0001")]
        #[case::odd("0x123")]
        #[case::even("0x1234")]
        #[case::touch_each_felt(
            "0x00000000000123450000000000067890000000000000abcd00000000000000ef"
        )]
        #[case::unique_felt("0x111111111111111155555555555555559999999999999999cccccccccccccccc")]
        #[case::digits_on_repeat(
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        )]
        fn digest_macro(#[case] input: &str) {
            let uut = digest!(input);

            // Right pad to 64 hex digits (66 including prefix). This is required by the
            // Digest::try_from(String) implementation.
            let padded_input = format!("{input:<66}").replace(" ", "0");
            let expected = crate::Digest::try_from(std::dbg!(padded_input)).unwrap();

            assert_eq!(uut, expected);
        }
    }
}

pub mod vm {
    pub use miden_verifier::ExecutionProof;
    pub use vm_core::{AdviceMap, Program, ProgramInfo};
    pub use vm_processor::{AdviceInputs, RowIndex, StackInputs, StackOutputs};
}
