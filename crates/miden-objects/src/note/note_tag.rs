use core::fmt;

use miden_crypto::Felt;

use super::{
    AccountId, ByteReader, ByteWriter, Deserializable, DeserializationError, NoteError, NoteType,
    Serializable,
};

// CONSTANTS
// ================================================================================================
const NETWORK_EXECUTION: u8 = 0;
const LOCAL_EXECUTION: u8 = 1;

// The 2 most significant bits are set to `0b00`.
const NETWORK_ACCOUNT: u32 = 0;
// The 2 most significant bits are set to `0b01`.
const NETWORK_PUBLIC_USECASE: u32 = 0x4000_0000;
// The 2 most significant bits are set to `0b10`.
const LOCAL_PUBLIC_USECASE: u32 = 0x8000_0000;
// The 2 most significant bits are set to `0b11`.
const LOCAL_ANY: u32 = 0xc000_0000;

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
/// | Prefix | Name                   | [`NoteExecutionMode`] | Target                   | Allowed [`NoteType`] |
/// | :----: | :--------------------: | :-------------------: | :----------------------: | :------------------: |
/// | `0b00` | `NetworkAccount`       | Network               | Network Account          | [`NoteType::Public`] |
/// | `0b01` | `NetworkPublicUseCase` | Network               | Use case                 | [`NoteType::Public`] |
/// | `0b10` | `LocalPublicUseCase`   | Local                 | Use case                 | [`NoteType::Public`] |
/// | `0b11` | `LocalAny`             | Local                 | Any                      | Any                  |
///
/// Where:
///
/// - [`NoteExecutionMode`] is set to [`NoteExecutionMode::Network`] to hint a [`Note`](super::Note)
///   should be consumed by the network. These notes will be further validated and if possible
///   consumed by it.
/// - Target describes how to further interpret the bits in the tag.
///   - For tags with an account target, the rest of the tag is interpreted as a partial
///     [`AccountId`]. For network accounts these are the first 30 bits of the ID while for local
///     account targets, the first 14 bits are used - a trade-off between privacy and uniqueness.
///   - For use case values, the meaning of the rest of the tag is not specified by the protocol and
///     can be used by applications built on top of the rollup.
///
/// The note type is the only value enforced by the protocol. The rationale is that any note
/// intended to be consumed by the network must be public to have all the details available. The
/// public note for local execution is intended to allow users to search for notes that can be
/// consumed right away, without requiring an off-band communication channel.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum NoteTag {
    /// Represents a tag for a note intended for network execution, targeted at a network account.
    /// The note must be public.
    NetworkAccount(u32),
    /// Represents a tag for a note intended for network execution for a public use case. The note
    /// must be public.
    NetworkPublicUseCase(u16, u16),
    /// Represents a tag for a note intended for local execution for a public use case. The note
    /// must be public.
    LocalPublicUseCase(u16, u16),
    /// Represents a tag for a note intended for local execution.
    ///
    /// This is used for two purposes:
    /// - A private use case note can be private/public/encrypted.
    /// - A note targeted at any type of account.
    ///
    /// In all cases, the note can be of any type.
    LocalAny(u32),
}

impl NoteTag {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The exponent of the maximum allowed use case id. In other words, 2^exponent is the maximum
    /// allowed use case id.
    pub(crate) const MAX_USE_CASE_ID_EXPONENT: u8 = 14;

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new [NoteTag::NetworkAccount] or [NoteTag::LocalAny] instantiated from the
    /// specified account ID and execution mode.
    ///
    /// The tag is constructed as follows:
    ///
    /// - For local execution, the two most significant bits are set to `0b11`, which allows for any
    ///   note type to be used. The following 14 bits are set to the most significant bits of the
    ///   account ID, and the remaining 16 bits are set to 0.
    /// - For network execution, the most significant bits are set to `0b00` and the remaining bits
    ///   are set to the 30 most significant bits of the account ID.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NoteExecutionMode::Network`] is provided but the storage mode of the account_id is not
    ///   [`AccountStorageMode::Public`](crate::account::AccountStorageMode::Public) and the
    ///   [`NetworkAccount`](crate::account::NetworkAccount) configuration is enabled.
    pub fn from_account_id(
        account_id: AccountId,
        execution: NoteExecutionMode,
    ) -> Result<Self, NoteError> {
        match execution {
            NoteExecutionMode::Local => {
                let prefix_id: u64 = account_id.prefix().into();

                // Shift the high bits of the account ID such that they are layed out as:
                // [34 zero bits | remaining high bits (30 bits)].
                let high_bits = prefix_id >> 34;

                // This is equivalent to the following layout, interpreted as a u32:
                // [2 zero bits | remaining high bits (30 bits)].
                let high_bits = high_bits as u32;

                // Select the upper half of the u32 which then contains the 14 most significant bits
                // of the account ID, i.e.:
                // [2 zero bits | remaining high bits (14 bits) | 16 zero bits].
                let high_bits = high_bits & 0xffff0000;

                // Set the local execution tag in the two most significant bits.
                Ok(Self::LocalAny(LOCAL_ANY | high_bits))
            },
            NoteExecutionMode::Network => {
                if !account_id.is_network() {
                    Err(NoteError::NetworkExecutionRequiresNetworkAccount)
                } else {
                    let prefix_id: u64 = account_id.prefix().into();

                    // Shift the high bits of the account ID such that they are layed out as:
                    // [34 zero bits | remaining high bits (30 bits)].
                    let high_bits = prefix_id >> 34;

                    // This is equivalent to the following layout, interpreted as a u32:
                    // [2 zero bits | remaining high bits (30 bits)].
                    // The two most significant zero bits match the tag we need for network
                    // execution.
                    Ok(Self::NetworkAccount(high_bits as u32))
                }
            },
        }
    }

    /// Returns a new [`NoteTag::NetworkPublicUseCase`] or [`NoteTag::LocalPublicUseCase`]
    /// instantiated for a custom use case which requires a public note.
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

        match execution {
            NoteExecutionMode::Network => {
                let use_case_bits = (NETWORK_PUBLIC_USECASE >> 16) as u16 | use_case_id;
                Ok(Self::NetworkPublicUseCase(use_case_bits, payload))
            },
            NoteExecutionMode::Local => {
                let use_case_bits = (LOCAL_PUBLIC_USECASE >> 16) as u16 | use_case_id;
                Ok(Self::LocalPublicUseCase(use_case_bits, payload))
            },
        }
    }

    /// Returns a new [`NoteTag::LocalAny`] instantiated for a custom local use case.
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

        let use_case_bits = (use_case_id as u32) << 16;
        let payload_bits = payload as u32;

        Ok(Self::LocalAny(LOCAL_ANY | use_case_bits | payload_bits))
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns true if the note is intended for execution by a specific account, i.e.
    /// [`NoteTag::NetworkAccount`]
    pub fn is_single_target(&self) -> bool {
        matches!(self, NoteTag::NetworkAccount(_))
    }

    /// Returns note execution mode defined by this tag.
    ///
    /// If the most significant bit of the tag is 0 the note is intended for local execution;
    /// otherwise, the note is intended for network execution.
    pub fn execution_mode(&self) -> NoteExecutionMode {
        match self {
            NoteTag::NetworkAccount(_) | NoteTag::NetworkPublicUseCase(..) => {
                NoteExecutionMode::Network
            },
            NoteTag::LocalAny(_) | NoteTag::LocalPublicUseCase(..) => NoteExecutionMode::Local,
        }
    }

    /// Returns the inner u32 value of this tag.
    pub fn as_u32(&self) -> u32 {
        match self {
            NoteTag::NetworkAccount(tag) => *tag,
            NoteTag::NetworkPublicUseCase(use_case_bits, payload_bits) => {
                (*use_case_bits as u32) << 16 | *payload_bits as u32
            },
            NoteTag::LocalAny(tag) => *tag,
            NoteTag::LocalPublicUseCase(use_case_bits, payload_bits) => {
                (*use_case_bits as u32) << 16 | *payload_bits as u32
            },
        }
    }

    // UTILITY METHODS
    // --------------------------------------------------------------------------------------------

    /// Returns an error if this tag is not consistent with the specified note type, and self
    /// otherwise.
    pub fn validate(&self, note_type: NoteType) -> Result<Self, NoteError> {
        if self.execution_mode() == NoteExecutionMode::Network && note_type != NoteType::Public {
            return Err(NoteError::NetworkExecutionRequiresPublicNote(note_type));
        }

        // If the use case is public, the note must be public as well.
        if self.is_public_use_case() && note_type != NoteType::Public {
            Err(NoteError::PublicUseCaseRequiresPublicNote(note_type))
        } else {
            Ok(*self)
        }
    }

    fn is_public_use_case(&self) -> bool {
        matches!(self, NoteTag::NetworkPublicUseCase(_, _) | NoteTag::LocalPublicUseCase(_, _))
    }
}

impl fmt::Display for NoteTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_u32())
    }
}

// CONVERSIONS INTO NOTE TAG
// ================================================================================================

impl From<u32> for NoteTag {
    fn from(value: u32) -> Self {
        // Mask out the two most significant bits.
        match value & 0xc0000000 {
            NETWORK_ACCOUNT => Self::NetworkAccount(value),
            NETWORK_PUBLIC_USECASE => {
                Self::NetworkPublicUseCase((value >> 16) as u16, value as u16)
            },
            LOCAL_ANY => Self::LocalAny(value),
            LOCAL_PUBLIC_USECASE => Self::LocalPublicUseCase((value >> 16) as u16, value as u16),
            _ => {
                // This branch should be unreachable because `prefix` is derived from
                // the top 2 bits of a u32, which can only be 0, 1, 2, or 3.
                unreachable!("Invalid value encountered: {:b}", value);
            },
        }
    }
}

// CONVERSIONS FROM NOTE TAG
// ================================================================================================

impl From<NoteTag> for u32 {
    fn from(value: NoteTag) -> Self {
        value.as_u32()
    }
}

impl From<NoteTag> for Felt {
    fn from(value: NoteTag) -> Self {
        Felt::from(value.as_u32())
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteTag {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.as_u32().write_into(target);
    }
}

impl Deserializable for NoteTag {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let tag = u32::read_from(source)?;
        Ok(Self::from(tag))
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {

    use assert_matches::assert_matches;

    use super::{NoteExecutionMode, NoteTag};
    use crate::{
        NoteError,
        account::AccountId,
        note::{NoteType, note_tag::LOCAL_ANY},
        testing::account_id::{
            ACCOUNT_ID_NETWORK_FUNGIBLE_FAUCET, ACCOUNT_ID_NETWORK_NON_FUNGIBLE_FAUCET,
            ACCOUNT_ID_PRIVATE_FUNGIBLE_FAUCET, ACCOUNT_ID_PRIVATE_NON_FUNGIBLE_FAUCET,
            ACCOUNT_ID_PRIVATE_SENDER, ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET,
            ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1, ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_2,
            ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_3, ACCOUNT_ID_PUBLIC_NON_FUNGIBLE_FAUCET,
            ACCOUNT_ID_PUBLIC_NON_FUNGIBLE_FAUCET_1,
            ACCOUNT_ID_REGULAR_NETWORK_ACCOUNT_IMMUTABLE_CODE,
            ACCOUNT_ID_REGULAR_PRIVATE_ACCOUNT_UPDATABLE_CODE,
            ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE,
            ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE_2,
            ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_UPDATABLE_CODE,
            ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2, ACCOUNT_ID_SENDER,
        },
    };

    #[test]
    fn test_from_account_id() {
        let private_accounts = [
            AccountId::try_from(ACCOUNT_ID_SENDER).unwrap(),
            AccountId::try_from(ACCOUNT_ID_PRIVATE_SENDER).unwrap(),
            AccountId::try_from(ACCOUNT_ID_REGULAR_PRIVATE_ACCOUNT_UPDATABLE_CODE).unwrap(),
            AccountId::try_from(ACCOUNT_ID_PRIVATE_FUNGIBLE_FAUCET).unwrap(),
            AccountId::try_from(ACCOUNT_ID_PRIVATE_NON_FUNGIBLE_FAUCET).unwrap(),
        ];
        let public_accounts = [
            AccountId::try_from(ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE).unwrap(),
            AccountId::try_from(ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE_2).unwrap(),
            AccountId::try_from(ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_UPDATABLE_CODE).unwrap(),
            AccountId::try_from(ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2)
                .unwrap(),
            AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET).unwrap(),
            AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1).unwrap(),
            AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_2).unwrap(),
            AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_3).unwrap(),
            AccountId::try_from(ACCOUNT_ID_PUBLIC_NON_FUNGIBLE_FAUCET).unwrap(),
            AccountId::try_from(ACCOUNT_ID_PUBLIC_NON_FUNGIBLE_FAUCET_1).unwrap(),
        ];
        let network_accounts = [
            AccountId::try_from(ACCOUNT_ID_REGULAR_NETWORK_ACCOUNT_IMMUTABLE_CODE).unwrap(),
            AccountId::try_from(ACCOUNT_ID_NETWORK_FUNGIBLE_FAUCET).unwrap(),
            AccountId::try_from(ACCOUNT_ID_NETWORK_NON_FUNGIBLE_FAUCET).unwrap(),
        ];

        for account_id in private_accounts.into_iter().chain(public_accounts) {
            assert_matches!(
                NoteTag::from_account_id(account_id, NoteExecutionMode::Network).unwrap_err(),
                NoteError::NetworkExecutionRequiresNetworkAccount,
                "Tag generation must fail if network execution and private account ID are mixed"
            )
        }

        for account_id in network_accounts {
            let tag = NoteTag::from_account_id(account_id, NoteExecutionMode::Network)
                .expect("tag generation must work with network execution and network account ID");
            assert!(tag.is_single_target());
            assert_eq!(tag.execution_mode(), NoteExecutionMode::Network);

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

        for account_id in private_accounts {
            let tag = NoteTag::from_account_id(account_id, NoteExecutionMode::Local)
                .expect("tag generation must work with local execution and private account ID");
            assert!(!tag.is_single_target());
            assert_eq!(tag.execution_mode(), NoteExecutionMode::Local);

            tag.validate(NoteType::Public)
                .expect("local execution should support public notes");
            tag.validate(NoteType::Private)
                .expect("local execution should support private notes");
            tag.validate(NoteType::Encrypted)
                .expect("local execution should support encrypted notes");
        }

        for account_id in public_accounts {
            let tag = NoteTag::from_account_id(account_id, NoteExecutionMode::Local)
                .expect("Tag generation must work with local execution and public account ID");
            assert!(!tag.is_single_target());
            assert_eq!(tag.execution_mode(), NoteExecutionMode::Local);

            tag.validate(NoteType::Public)
                .expect("local execution should support public notes");
            tag.validate(NoteType::Private)
                .expect("local execution should support private notes");
            tag.validate(NoteType::Encrypted)
                .expect("local execution should support encrypted notes");
        }

        for account_id in network_accounts {
            let tag = NoteTag::from_account_id(account_id, NoteExecutionMode::Local)
                .expect("Tag generation must work with local execution and network account ID");
            assert!(!tag.is_single_target());
            assert_eq!(tag.execution_mode(), NoteExecutionMode::Local);

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
        /// Private Account ID with the following bit pattern in the first and second byte:
        /// 0b11001100_01010101
        ///   ^^^^^^^^ ^^^^^^  <- 14 bits of the local tag.
        const PRIVATE_ACCOUNT_INT: u128 = ACCOUNT_ID_REGULAR_PRIVATE_ACCOUNT_UPDATABLE_CODE
            | 0x0055_0000_0000_0000_0000_0000_0000_0000;
        let private_account_id = AccountId::try_from(PRIVATE_ACCOUNT_INT).unwrap();

        // Expected private tag with `NoteTag::LocalAny` prefix.
        let expected_private_local_tag = 0b11110011_00010101_00000000_00000000u32;

        /// Public Account ID with the following bit pattern in the first and second byte:
        /// 0b10101010_01010101_11001100_10101010
        ///   ^^^^^^^^ ^^^^^^  <- 14 bits of the local tag.
        const PUBLIC_ACCOUNT_INT: u128 = ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE
            | 0x0055_ccaa_0000_0000_0000_0000_0000_0000;
        let public_account_id = AccountId::try_from(PUBLIC_ACCOUNT_INT).unwrap();

        // Expected public tag with `NoteTag::LocalAny` prefix.
        let expected_public_local_tag = 0b11101010_10010101_00000000_00000000u32;

        /// Network Account ID with the following bit pattern in the first and second byte:
        /// 0b10101010_11001100_01110111_11000100
        ///   ^^^^^^^^ ^^^^^^^^ ^^^^^^^^ ^^^^^^  <- 30 bits of the network tag.
        ///   ^^^^^^^^ ^^^^^^  <- 14 bits of the local tag.
        ///
        /// Note that the 30th most significant bit is the enabled network flag.
        const NETWORK_ACCOUNT_INT: u128 = ACCOUNT_ID_REGULAR_NETWORK_ACCOUNT_IMMUTABLE_CODE
            | 0x00cc_77c0_0000_0000_0000_0000_0000_0000;
        let network_account_id = AccountId::try_from(NETWORK_ACCOUNT_INT).unwrap();

        // Expected network tag with `NoteTag::LocalAny` prefix.
        let expected_network_local_tag = 0b11101010_10110011_00000000_00000000u32;

        // Expected network tag with `NoteTag::NetworkAccount` for network execution.
        let expected_network_network_tag = 0b00101010_10110011_00011101_11110001u32;

        // Public and Private account modes (without network flag) with NoteExecutionMode::Network
        // should fail.
        // ----------------------------------------------------------------------------------------

        assert_matches!(
            NoteTag::from_account_id(private_account_id, NoteExecutionMode::Network),
            Err(NoteError::NetworkExecutionRequiresNetworkAccount)
        );
        assert_matches!(
            NoteTag::from_account_id(public_account_id, NoteExecutionMode::Network),
            Err(NoteError::NetworkExecutionRequiresNetworkAccount)
        );

        // NoteExecutionMode::Local
        // ----------------------------------------------------------------------------------------

        assert_eq!(
            NoteTag::from_account_id(private_account_id, NoteExecutionMode::Local)
                .unwrap()
                .as_u32(),
            expected_private_local_tag,
        );
        assert_eq!(
            NoteTag::from_account_id(public_account_id, NoteExecutionMode::Local)
                .unwrap()
                .as_u32(),
            expected_public_local_tag,
        );
        assert_eq!(
            NoteTag::from_account_id(network_account_id, NoteExecutionMode::Local)
                .unwrap()
                .as_u32(),
            expected_network_local_tag,
        );

        // NoteExecutionMode::Network
        // ----------------------------------------------------------------------------------------

        assert_eq!(
            NoteTag::from_account_id(network_account_id, NoteExecutionMode::Network)
                .unwrap()
                .as_u32(),
            expected_network_network_tag,
        );
    }

    #[test]
    fn test_for_public_use_case() {
        // NETWORK
        // ----------------------------------------------------------------------------------------
        let tag = NoteTag::for_public_use_case(0b0, 0b0, NoteExecutionMode::Network).unwrap();
        assert_eq!(tag.as_u32(), 0b01000000_00000000_00000000_00000000u32);

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
        assert_eq!(tag.as_u32(), 0b01000000_00000001_00000000_00000000u32);

        let tag = NoteTag::for_public_use_case(0b0, 0b1, NoteExecutionMode::Network).unwrap();
        assert_eq!(tag.as_u32(), 0b01000000_00000000_00000000_00000001u32);

        let tag = NoteTag::for_public_use_case(1 << 13, 0b0, NoteExecutionMode::Network).unwrap();
        assert_eq!(tag.as_u32(), 0b01100000_00000000_00000000_00000000u32);

        // LOCAL
        // ----------------------------------------------------------------------------------------
        let tag = NoteTag::for_public_use_case(0b0, 0b0, NoteExecutionMode::Local).unwrap();
        assert_eq!(tag.as_u32(), 0b10000000_00000000_00000000_00000000u32);

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
        assert_eq!(tag.as_u32(), 0b10000000_00000000_00000000_00000001u32);

        let tag = NoteTag::for_public_use_case(0b1, 0b0, NoteExecutionMode::Local).unwrap();
        assert_eq!(tag.as_u32(), 0b10000000_00000001_00000000_00000000u32);

        let tag = NoteTag::for_public_use_case(1 << 13, 0b0, NoteExecutionMode::Local).unwrap();
        assert_eq!(tag.as_u32(), 0b10100000_00000000_00000000_00000000u32);

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
        assert_eq!(
            tag.as_u32() >> 30,
            LOCAL_ANY >> 30,
            "local use case prefix should be local any"
        );
        assert_eq!(tag.as_u32(), 0b11000000_00000000_00000000_00000000u32);

        tag.validate(NoteType::Public)
            .expect("local execution should support public notes");
        tag.validate(NoteType::Private)
            .expect("local execution should support private notes");
        tag.validate(NoteType::Encrypted)
            .expect("local execution should support encrypted notes");

        let tag = NoteTag::for_local_use_case(0b0, 0b1).unwrap();
        assert_eq!(tag.as_u32(), 0b11000000_00000000_00000000_00000001u32);

        let tag = NoteTag::for_local_use_case(0b1, 0b0).unwrap();
        assert_eq!(tag.as_u32(), 0b11000000_00000001_00000000_00000000u32);

        let tag = NoteTag::for_local_use_case(1 << 13, 0b0).unwrap();
        assert_eq!(tag.as_u32(), 0b11100000_00000000_00000000_00000000u32);

        assert_matches!(
          NoteTag::for_local_use_case(1 << 15, 0b0).unwrap_err(),
          NoteError::NoteTagUseCaseTooLarge(use_case) if use_case == 1 << 15
        );
        assert_matches!(
          NoteTag::for_local_use_case(1 << 14, 0b0).unwrap_err(),
          NoteError::NoteTagUseCaseTooLarge(use_case) if use_case == 1 << 14
        );
    }
}
