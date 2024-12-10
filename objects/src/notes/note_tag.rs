use core::{fmt, num::TryFromIntError};

use miden_crypto::Felt;

use super::{
    AccountId, ByteReader, ByteWriter, Deserializable, DeserializationError, NoteError, NoteType,
    Serializable,
};

// CONSTANTS
// ================================================================================================
const NETWORK_EXECUTION: u8 = 0;
const LOCAL_EXECUTION: u8 = 1;

// The 2 most significant bits are set to `0b11`
const LOCAL_EXECUTION_WITH_ALL_NOTE_TYPES_ALLOWED: u32 = 0xc000_0000;
// The 2 most significant bits are set to `0b10`
const PUBLIC_USECASE: u32 = 0x8000_0000;

/// [super::Note]'s execution mode hints.
///
/// The execution hints are _not_ enforced, therefore function only as hints. For example, if a
/// note's tag is created with the [NoteExecutionMode::Network], further validation is necessary to
/// check the account_id is known, that the account's state is on-chain, and the account is
/// controlled by the network.
///
/// The goal of the hint is to allow for a network node to quickly filter notes that are not
/// intended for network execution, and skip the validation steps mentioned above.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NoteExecutionMode {
    Network = NETWORK_EXECUTION,
    Local = LOCAL_EXECUTION,
}

// NOTE TAG
// ================================================================================================

/// [NoteTag]`s are best effort filters for notes registered with the network.
///
/// Tags are light-weight values used to speed up queries. The 2 most significant bits of the tags
/// have the following interpretation:
///
/// | Prefix | Execution hint | Target   | Allowed [NoteType] |
/// | ------ | :------------: | :------: | :----------------: |
/// | `0b00` | Network        | Specific | [NoteType::Public] |
/// | `0b01` | Network        | Use case | [NoteType::Public] |
/// | `0b10` | Local          | Any      | [NoteType::Public] |
/// | `0b11` | Local          | Any      | Any                |
///
/// Where:
///
/// - [`NoteExecutionMode`] is set to [`NoteExecutionMode::Network`] to hint a [`Note`](super::Note)
///   should be consumed by the network. These notes will be further validated and if possible
///   consumed by it.
/// - Target describes how to further interpret the bits in the tag. For tags with a specific
///   target, the rest of the tag is interpreted as an account_id. For use case values, the meaning
///   of the rest of the tag is not specified by the protocol and can be used by applications built
///   on top of the rollup.
///
/// The note type is the only value enforced by the protocol. The rationale is that any note
/// intended to be consumed by the network must be public to have all the details available. The
/// public note for local execution is intended to allow users to search for notes that can be
/// consumed right away, without requiring an off-band communication channel.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct NoteTag(u32);

impl NoteTag {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The exponent of the maximum allowed use case id. In other words, 2^exponent is the maximum
    /// allowed use case id.
    pub(crate) const MAX_USE_CASE_ID_EXPONENT: u8 = 14;

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new [NoteTag] instantiated from the specified account ID and execution mode.
    ///
    /// The tag is constructed as follows:
    ///
    /// - For local execution, the two most significant bits are set to `0b11`, which allows for any
    ///   note type to be used, the following 14 bits are set to the 14 most significant bits of the
    ///   account ID, and the remaining 16 bits are set to 0.
    /// - For network execution, the most significant bit is set to `0b0` and the remaining bits are
    ///   set to the 31 most significant bits of the account ID. Note that this results in the two
    ///   most significant bits of the tag being set to `0b00`, because the network execution
    ///   requires a public account which always have the high bit set to 0.
    ///
    /// # Errors
    ///
    /// This will return an error if the account_id is not for a public account and the execution
    /// hint is set to [NoteExecutionMode::Network].
    pub fn from_account_id(
        account_id: AccountId,
        execution: NoteExecutionMode,
    ) -> Result<Self, NoteError> {
        match execution {
            NoteExecutionMode::Local => {
                let first_felt_id: u64 = account_id.first_felt().into();
                // Consider the most significant bits to start at the 63rd bit, since the top bit is
                // always zero and doesn't add any value when matching account IDs.
                let mut high_bits = first_felt_id << 1;
                // Shift the high bits such that the most significant bits are in the range 0..30.
                // The two most significant bits are then zero.
                high_bits >>= 34;
                // Select the upper half of the u32 which contains the 14 most significant bits of
                // the account ID (without the top bit). The remaining bits will be zero.
                let high_bits = (high_bits as u32) & 0xffff0000;
                // Set the local execution tag.
                Ok(Self(high_bits | LOCAL_EXECUTION_WITH_ALL_NOTE_TYPES_ALLOWED))
            },
            NoteExecutionMode::Network => {
                if !account_id.is_public() {
                    Err(NoteError::NetworkExecutionRequiresOnChainAccount)
                } else {
                    let first_felt_id: u64 = account_id.first_felt().into();
                    // Select the 31 most significant bits of the account ID and shift them right by
                    // 1 bit.
                    // Note that we do not remove the top zero bit of the account ID since we can
                    // use it as part of the tag.
                    let high_bits = (first_felt_id >> 33) as u32;
                    // The tag will have the form [zero bit | 31 high bits of account ID]. Note that
                    // the second bit of the tag is guaranteed to be 0 because account IDs always
                    // start with 0.
                    Ok(Self(high_bits))
                }
            },
        }
    }

    /// Returns a new [NoteTag] instantiated for a custom use case which requires a public note.
    ///
    /// The public use_case tag requires a [NoteType::Public] note.
    ///
    /// The two high bits are set to the `b10` or `b01` depending on the execution hint, the next 14
    /// bits are set to the `use_case_id`, and the low 16 bits are set to `payload`.
    ///
    /// # Errors
    ///
    /// - If `use_case_id` is larger than or equal to $2^{14}$.
    pub fn for_public_use_case(
        use_case_id: u16,
        payload: u16,
        execution: NoteExecutionMode,
    ) -> Result<Self, NoteError> {
        if (use_case_id >> 14) != 0 {
            return Err(NoteError::NoteTagUseCaseTooLarge(use_case_id));
        }

        let execution_bits = match execution {
            NoteExecutionMode::Local => PUBLIC_USECASE, // high bits set to `0b10`
            NoteExecutionMode::Network => 0x40000000,   // high bits set to `0b01`
        };

        let use_case_bits = (use_case_id as u32) << 16;
        let payload_bits = payload as u32;

        Ok(Self(execution_bits | use_case_bits | payload_bits))
    }

    /// Returns a new [NoteTag] instantiated for a custom local use case.
    ///
    /// The local use_case tag is the only tag type that allows for [NoteType::Private] notes.
    ///
    /// The two high bits are set to the `b11`, the next 14 bits are set to the `use_case_id`, and
    /// the low 16 bits are set to `payload`.
    ///
    /// # Errors
    ///
    /// - If `use_case_id` is larger than or equal to 2^14.
    pub fn for_local_use_case(use_case_id: u16, payload: u16) -> Result<Self, NoteError> {
        if (use_case_id >> NoteTag::MAX_USE_CASE_ID_EXPONENT) != 0 {
            return Err(NoteError::NoteTagUseCaseTooLarge(use_case_id));
        }

        let execution_bits = LOCAL_EXECUTION_WITH_ALL_NOTE_TYPES_ALLOWED;
        let use_case_bits = (use_case_id as u32) << 16;
        let payload_bits = payload as u32;

        Ok(Self(execution_bits | use_case_bits | payload_bits))
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns true if the note is intended for execution by a specific account.
    ///
    /// A note is intended for execution by a single account if the first two bits are zeros
    pub fn is_single_target(&self) -> bool {
        let first_2_bit = self.0 >> 30;
        first_2_bit == 0b00
    }

    /// Returns note execution mode defined by this tag.
    ///
    /// If the most significant bit of the tag is 0 the note is intended for local execution;
    /// otherwise, the note is intended for network execution.
    pub fn execution_hint(&self) -> NoteExecutionMode {
        let first_bit = self.0 >> 31;

        if first_bit == (LOCAL_EXECUTION as u32) {
            NoteExecutionMode::Local
        } else {
            NoteExecutionMode::Network
        }
    }

    /// Returns the inner u32 value of this tag.
    pub fn inner(&self) -> u32 {
        self.0
    }

    // UTILITY METHODS
    // --------------------------------------------------------------------------------------------

    /// Returns an error if this tag is not consistent with the specified note type, and self
    /// otherwise.
    pub fn validate(&self, note_type: NoteType) -> Result<Self, NoteError> {
        if self.execution_hint() == NoteExecutionMode::Network && note_type != NoteType::Public {
            return Err(NoteError::NetworkExecutionRequiresPublicNote(note_type));
        }

        let is_public_use_case = (self.0 & 0xc0000000) == PUBLIC_USECASE;
        if is_public_use_case && note_type != NoteType::Public {
            Err(NoteError::PublicUseCaseRequiresPublicNote(note_type))
        } else {
            Ok(*self)
        }
    }
}

impl fmt::Display for NoteTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// CONVERSIONS INTO NOTE TAG
// ================================================================================================

impl From<u32> for NoteTag {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl TryFrom<u64> for NoteTag {
    type Error = TryFromIntError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Ok(Self(value.try_into()?))
    }
}

impl TryFrom<Felt> for NoteTag {
    type Error = TryFromIntError;

    fn try_from(value: Felt) -> Result<Self, Self::Error> {
        Ok(Self(value.as_int().try_into()?))
    }
}

// CONVERSIONS FROM NOTE TAG
// ================================================================================================

impl From<NoteTag> for u32 {
    fn from(value: NoteTag) -> Self {
        value.0
    }
}

impl From<NoteTag> for u64 {
    fn from(value: NoteTag) -> Self {
        value.0 as u64
    }
}

impl From<NoteTag> for Felt {
    fn from(value: NoteTag) -> Self {
        Felt::from(value.0)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteTag {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.0.write_into(target);
    }
}

impl Deserializable for NoteTag {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let tag = u32::read_from(source)?;
        Ok(Self(tag))
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use super::{NoteExecutionMode, NoteTag};
    use crate::{
        accounts::{
            account_id::testing::{
                ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
                ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
                ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
                ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1,
                ACCOUNT_ID_OFF_CHAIN_SENDER, ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
                ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2,
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2, ACCOUNT_ID_SENDER,
            },
            AccountId,
        },
        notes::NoteType,
        NoteError,
    };

    #[test]
    fn test_from_account_id() {
        let off_chain_accounts = [
            AccountId::try_from(ACCOUNT_ID_SENDER).unwrap(),
            AccountId::try_from(ACCOUNT_ID_OFF_CHAIN_SENDER).unwrap(),
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN).unwrap(),
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap(),
            AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap(),
        ];
        let on_chain_accounts = [
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap(),
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2).unwrap(),
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap(),
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2).unwrap(),
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap(),
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap(),
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap(),
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3).unwrap(),
            AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN).unwrap(),
            AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap(),
        ];

        for off_chain in off_chain_accounts {
            assert_matches!(
                NoteTag::from_account_id(off_chain, NoteExecutionMode::Network).unwrap_err(),
                NoteError::NetworkExecutionRequiresOnChainAccount,
                "Tag generation must fail if network execution and off-chain account id are mixed"
            )
        }

        for on_chain in on_chain_accounts {
            let tag = NoteTag::from_account_id(on_chain, NoteExecutionMode::Network)
                .expect("tag generation must work with network execution and on-chain account id");
            assert!(tag.is_single_target());
            assert_eq!(tag.execution_hint(), NoteExecutionMode::Network);

            tag.validate(NoteType::Public)
                .expect("network execution should require notes to be public");
            assert_matches!(
                tag.validate(NoteType::Private),
                Err(NoteError::NetworkExecutionRequiresPublicNote(NoteType::Private))
            );
            assert_matches!(
                tag.validate(NoteType::Encrypted),
                Err(NoteError::NetworkExecutionRequiresPublicNote(NoteType::Encrypted))
            );
        }

        for off_chain in off_chain_accounts {
            let tag = NoteTag::from_account_id(off_chain, NoteExecutionMode::Local)
                .expect("tag generation must work with local execution and off-chain account id");
            assert!(!tag.is_single_target());
            assert_eq!(tag.execution_hint(), NoteExecutionMode::Local);

            tag.validate(NoteType::Public)
                .expect("local execution should support public notes");
            tag.validate(NoteType::Private)
                .expect("local execution should support private notes");
            tag.validate(NoteType::Encrypted)
                .expect("local execution should support encrypted notes");
        }

        for on_chain in on_chain_accounts {
            let tag = NoteTag::from_account_id(on_chain, NoteExecutionMode::Local)
                .expect("Tag generation must work with local execution and on-chain account id");
            assert!(!tag.is_single_target());
            assert_eq!(tag.execution_hint(), NoteExecutionMode::Local);

            tag.validate(NoteType::Public)
                .expect("local execution should support public notes");
            tag.validate(NoteType::Private)
                .expect("local execution should support private notes");
            tag.validate(NoteType::Encrypted)
                .expect("local execution should support encrypted notes");
        }
    }

    #[test]
    fn test_from_account_id_values() {
        // Off-Chain Account ID with the 2nd and 16th most significant bit set.
        const OFF_CHAIN_INT: u128 = ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN
            | 0x4001_0000_0000_0000_0000_0000_0000_0000;
        let off_chain = AccountId::try_from(OFF_CHAIN_INT).unwrap();

        // On-Chain Account ID with the 2nd and 16th most significant bit set.
        let on_chain_int = ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN
            | 0x4001_0000_0000_0000_0000_0000_0000_0000;
        let on_chain = AccountId::try_from(on_chain_int).unwrap();

        assert_eq!(
            NoteTag::from_account_id(on_chain, NoteExecutionMode::Network).unwrap(),
            NoteTag(0b00100000_01010101_10000000_00000000)
        );
        assert_matches!(
            NoteTag::from_account_id(off_chain, NoteExecutionMode::Network),
            Err(NoteError::NetworkExecutionRequiresOnChainAccount)
        );

        // We expect the 16th most significant bit to be cut off.
        assert_eq!(
            NoteTag::from_account_id(off_chain, NoteExecutionMode::Local).unwrap(),
            NoteTag(0b11100000_01100110_00000000_00000000)
        );

        // We expect the 16th most significant bit to be cut off.
        assert_eq!(
            NoteTag::from_account_id(on_chain, NoteExecutionMode::Local).unwrap(),
            NoteTag(0b11100000_01010101_00000000_00000000)
        );
    }

    #[test]
    fn test_for_public_use_case() {
        // NETWORK
        // ----------------------------------------------------------------------------------------
        let tag = NoteTag::for_public_use_case(0b0, 0b0, NoteExecutionMode::Network).unwrap();
        assert_eq!(tag, NoteTag(0b01000000_00000000_00000000_00000000));

        tag.validate(NoteType::Public).unwrap();

        assert_matches!(
            tag.validate(NoteType::Private).unwrap_err(),
            NoteError::NetworkExecutionRequiresPublicNote(NoteType::Private)
        );
        assert_matches!(
            tag.validate(NoteType::Encrypted).unwrap_err(),
            NoteError::NetworkExecutionRequiresPublicNote(NoteType::Encrypted)
        );

        let tag = NoteTag::for_public_use_case(0b1, 0b0, NoteExecutionMode::Network).unwrap();
        assert_eq!(tag, NoteTag(0b01000000_00000001_00000000_00000000));

        let tag = NoteTag::for_public_use_case(0b0, 0b1, NoteExecutionMode::Network).unwrap();
        assert_eq!(tag, NoteTag(0b01000000_00000000_00000000_00000001));

        let tag = NoteTag::for_public_use_case(1 << 13, 0b0, NoteExecutionMode::Network).unwrap();
        assert_eq!(tag, NoteTag(0b01100000_00000000_00000000_00000000));

        // LOCAL
        // ----------------------------------------------------------------------------------------
        let tag = NoteTag::for_public_use_case(0b0, 0b0, NoteExecutionMode::Local).unwrap();
        assert_eq!(tag, NoteTag(0b10000000_00000000_00000000_00000000));

        tag.validate(NoteType::Public).unwrap();
        assert_matches!(
            tag.validate(NoteType::Private).unwrap_err(),
            NoteError::PublicUseCaseRequiresPublicNote(NoteType::Private)
        );
        assert_matches!(
            tag.validate(NoteType::Encrypted).unwrap_err(),
            NoteError::PublicUseCaseRequiresPublicNote(NoteType::Encrypted)
        );

        let tag = NoteTag::for_public_use_case(0b0, 0b1, NoteExecutionMode::Local).unwrap();
        assert_eq!(tag, NoteTag(0b10000000_00000000_00000000_00000001));

        let tag = NoteTag::for_public_use_case(0b1, 0b0, NoteExecutionMode::Local).unwrap();
        assert_eq!(tag, NoteTag(0b10000000_00000001_00000000_00000000));

        let tag = NoteTag::for_public_use_case(1 << 13, 0b0, NoteExecutionMode::Local).unwrap();
        assert_eq!(tag, NoteTag(0b10100000_00000000_00000000_00000000));

        assert_matches!(
          NoteTag::for_public_use_case(1 << 15, 0b0, NoteExecutionMode::Local).unwrap_err(),
          NoteError::NoteTagUseCaseTooLarge(use_case) if use_case == 1 << 15
        );
        assert_matches!(
          NoteTag::for_public_use_case(1 << 14, 0b0, NoteExecutionMode::Local).unwrap_err(),
          NoteError::NoteTagUseCaseTooLarge(use_case) if use_case == 1 << 14
        );
    }

    #[test]
    fn test_for_private_use_case() {
        let tag = NoteTag::for_local_use_case(0b0, 0b0).unwrap();
        assert_eq!(tag, NoteTag(0b11000000_00000000_00000000_00000000));

        tag.validate(NoteType::Public)
            .expect("local execution should support public notes");
        tag.validate(NoteType::Private)
            .expect("local execution should support private notes");
        tag.validate(NoteType::Encrypted)
            .expect("local execution should support encrypted notes");

        let tag = NoteTag::for_local_use_case(0b0, 0b1).unwrap();
        assert_eq!(tag, NoteTag(0b11000000_00000000_00000000_00000001));

        let tag = NoteTag::for_local_use_case(0b1, 0b0).unwrap();
        assert_eq!(tag, NoteTag(0b11000000_00000001_00000000_00000000));

        let tag = NoteTag::for_local_use_case(1 << 13, 0b0).unwrap();
        assert_eq!(tag, NoteTag(0b11100000_00000000_00000000_00000000));

        assert_matches!(
          NoteTag::for_local_use_case(1 << 15, 0b0).unwrap_err(),
          NoteError::NoteTagUseCaseTooLarge(use_case) if use_case == 1 << 15
        );
        assert_matches!(
          NoteTag::for_local_use_case(1 << 14, 0b0).unwrap_err(),
          NoteError::NoteTagUseCaseTooLarge(use_case) if use_case == 1 << 14
        );
    }

    /// Test for assumption built in the [NoteTag] encoding that only on-chain accounts have the
    /// highbit set to 0. If the account id encoding ever changes, the note tag needs to be
    /// adjusted.
    #[test]
    fn test_accounts_have_the_highbit_set_to_zero() {
        // Create a list of valid account ids with every combination of account types
        let accounts = [
            // ON-CHAIN
            // ---------------------------------------------------------------------------
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap(),
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2).unwrap(),
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap(),
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2).unwrap(),
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap(),
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap(),
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap(),
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3).unwrap(),
            AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN).unwrap(),
            AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap(),
            // OFF-CHAIN
            // --------------------------------------------------------------------------
            AccountId::try_from(ACCOUNT_ID_SENDER).unwrap(),
            AccountId::try_from(ACCOUNT_ID_OFF_CHAIN_SENDER).unwrap(),
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN).unwrap(),
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap(),
            AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap(),
        ];

        for account in accounts {
            let highbit = account.first_felt().as_int() >> 63;
            assert!(
                highbit == 0,
                "the account_id encoding changed, this breaks the assumptions built into the NoteTag"
            );
        }
    }
}
