use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_crypto::{merkle::LeafIndex, utils::hex_to_bytes};
use vm_core::{
    utils::{ByteReader, Deserializable, Serializable},
    Felt, Word,
};
use vm_processor::{DeserializationError, Digest};

use super::Hasher;
use crate::{
    accounts::{
        account_id::{
            FUNGIBLE_FAUCET, NON_FUNGIBLE_FAUCET, PRIVATE, PUBLIC, REGULAR_ACCOUNT_IMMUTABLE_CODE,
            REGULAR_ACCOUNT_UPDATABLE_CODE,
        },
        AccountStorageMode, AccountType2,
    },
    AccountError, ACCOUNT_TREE_DEPTH,
};

// CONSTANTS
// ================================================================================================

const ACCOUNT_VERSION_MASK_SHIFT: u64 = 4;
const ACCOUNT_VERSION_MASK: u64 = 0b1111 << ACCOUNT_VERSION_MASK_SHIFT;

const ACCOUNT_EPOCH_MASK_SHIFT: u64 = 48;
const ACCOUNT_EPOCH_MASK: u64 = 0xffff << ACCOUNT_EPOCH_MASK_SHIFT;

// The higher two bits of the least significant nibble determines the account storage mode
const ACCOUNT_STORAGE_MASK_SHIFT: u64 = 2;
const ACCOUNT_STORAGE_MASK: u64 = 0b11 << ACCOUNT_STORAGE_MASK_SHIFT;

// The lower two bits of the least significant nibble determine the account type.
pub(super) const ACCOUNT_TYPE_MASK: u64 = 0b11;

/// # Layout
/// ```text
/// 1st felt: [zero bit | random (55 bits) | version (4 bits) | storage mode (2 bits) | type (2 bits)]
/// 2nd felt: [epoch (16 bits) | random (40 bits) | 8 zero bits]
/// ```
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AccountId2 {
    first_felt: Felt,
    second_felt: Felt,
}

impl AccountId2 {
    /// Specifies a minimum number of ones for a valid account ID.
    pub const MIN_ACCOUNT_ONES: u32 = 5;

    pub fn new(
        seed: Word,
        epoch: u16,
        code_commitment: Digest,
        storage_commitment: Digest,
        block_hash: Digest,
    ) -> Result<Self, AccountError> {
        let seed_digest = compute_digest(seed, code_commitment, storage_commitment, block_hash);

        let mut felts: [Felt; 2] = seed_digest.as_elements()[0..2]
            .try_into()
            .expect("we should have sliced off 2 elements");

        felts[1] = shape_second_felt(felts[1], epoch);

        account_id_from_felts(felts)
    }

    pub fn new_unchecked(elements: [Felt; 2]) -> Self {
        Self {
            first_felt: elements[0],
            second_felt: elements[1],
        }
    }

    #[cfg(any(feature = "testing", test))]
    pub fn new_with_type_and_mode(
        mut bytes: [u8; 15],
        account_type: AccountType2,
        storage_mode: AccountStorageMode,
    ) -> AccountId2 {
        let version = AccountVersion::VERSION_0_NUMBER;
        let low_nibble = (version << ACCOUNT_VERSION_MASK_SHIFT)
            | (storage_mode as u8) << ACCOUNT_STORAGE_MASK_SHIFT
            | (account_type as u8);

        // Set least significant byte.
        bytes[7] = low_nibble;

        // Clear most significant bit.
        bytes[0] &= 0b0111_1111;
        // Set five one bits to satisfy MIN_ACCOUNT_ONES.
        bytes[0] |= 0b0111_1100;

        let first_felt_bytes =
            bytes[0..8].try_into().expect("we should have sliced off exactly 8 bytes");
        let first_felt = Felt::try_from(u64::from_be_bytes(first_felt_bytes))
            .expect("should be a valid felt due to the most significant bit being zero");

        let mut second_felt_bytes = [0; 8];
        // Overwrite first 7 bytes, leaving the 8th byte 0 (which will be cleared by
        // shape_second_felt anyway).
        second_felt_bytes[..7].copy_from_slice(&bytes[8..]);
        // If the value is too large modular reduction is performed, which is fine here.
        let mut second_felt = Felt::new(u64::from_be_bytes(second_felt_bytes));

        second_felt = shape_second_felt(second_felt, 0);

        let account_id = account_id_from_felts([first_felt, second_felt])
            .expect("we should have shaped the felts to produce a valid id");

        debug_assert_eq!(account_id.account_type(), account_type);
        debug_assert_eq!(account_id.storage_mode(), storage_mode);

        account_id
    }

    pub fn get_account_seed(
        init_seed: [u8; 32],
        account_type: AccountType2,
        storage_mode: AccountStorageMode,
        version: AccountVersion,
        code_commitment: Digest,
        storage_commitment: Digest,
        block_hash: Digest,
    ) -> Result<Word, AccountError> {
        crate::accounts::seed2::get_account_seed(
            init_seed,
            account_type,
            storage_mode,
            version,
            code_commitment,
            storage_commitment,
            block_hash,
        )
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    pub fn account_type(&self) -> AccountType2 {
        extract_type(self.first_felt().as_int())
    }

    /// Returns true if an account with this ID is a faucet (can issue assets).
    pub fn is_faucet(&self) -> bool {
        self.account_type().is_faucet()
    }

    /// Returns true if an account with this ID is a regular account.
    pub fn is_regular_account(&self) -> bool {
        self.account_type().is_regular_account()
    }

    pub fn storage_mode(&self) -> AccountStorageMode {
        extract_storage_mode(self.first_felt().as_int())
            .expect("account id should have been constructed with a valid storage mode")
    }

    /// Returns true if an account with this ID is a public account.
    pub fn is_public(&self) -> bool {
        self.storage_mode() == AccountStorageMode::Public
    }

    pub fn version(&self) -> AccountVersion {
        extract_version(self.first_felt().as_int())
            .expect("account id should have been constructed with a valid version")
    }

    pub fn epoch(&self) -> u16 {
        extract_epoch(self.second_felt().as_int())
    }

    /// Creates an Account Id from a hex string. Assumes the string starts with "0x" and
    /// that the hexadecimal characters are big-endian encoded.
    pub fn from_hex(hex_str: &str) -> Result<AccountId2, AccountError> {
        hex_to_bytes(hex_str).map_err(AccountError::AccountIdHexParseError).and_then(
            |mut bytes: [u8; 15]| {
                // TryFrom<[u8; 15]> expects little-endian order, so we need to convert the
                // bytes representation from big endian to little endian by reversing.
                bytes.reverse();
                AccountId2::try_from(bytes)
            },
        )
    }

    /// Returns a big-endian, hex-encoded string of length 32, including the `0x` prefix, so it
    /// encodes 15 bytes.
    pub fn to_hex(&self) -> String {
        format!("0x{:016x}{:014x}", self.first_felt().as_int(), self.second_felt().as_int())
    }

    fn first_felt(&self) -> Felt {
        self.first_felt
    }

    fn second_felt(&self) -> Felt {
        self.second_felt
    }
}

// CONVERSIONS FROM ACCOUNT ID
// ================================================================================================

impl From<AccountId2> for [Felt; 2] {
    fn from(id: AccountId2) -> Self {
        [id.first_felt, id.second_felt]
    }
}

impl From<AccountId2> for [u8; 15] {
    fn from(id: AccountId2) -> Self {
        let mut result = [0_u8; 15];
        result[..7].copy_from_slice(&id.second_felt().as_int().to_le_bytes()[..7]);
        result[7..].copy_from_slice(&id.first_felt().as_int().to_le_bytes());
        result
    }
}

impl From<AccountId2> for u128 {
    fn from(id: AccountId2) -> Self {
        let mut le_bytes = [0_u8; 16];
        le_bytes[..8].copy_from_slice(&id.second_felt().as_int().to_le_bytes());
        le_bytes[8..].copy_from_slice(&id.first_felt().as_int().to_le_bytes());
        u128::from_le_bytes(le_bytes)
    }
}

/// Account IDs are used as indexes in the account database, which is a tree of depth 64.
impl From<AccountId2> for LeafIndex<ACCOUNT_TREE_DEPTH> {
    fn from(id: AccountId2) -> Self {
        LeafIndex::new_max_depth(id.first_felt().as_int())
    }
}

// CONVERSIONS TO ACCOUNT ID
// ================================================================================================

impl TryFrom<[Felt; 2]> for AccountId2 {
    type Error = AccountError;

    /// Returns an [AccountId] instantiated with the provided field element.
    ///
    /// # Errors
    /// Returns an error if:
    /// - If there are fewer than [AccountId2::MIN_ACCOUNT_ONES] in the provided value.
    /// - If the provided value contains invalid account ID metadata (i.e., the first 4 bits).
    fn try_from(elements: [Felt; 2]) -> Result<Self, Self::Error> {
        account_id_from_felts(elements)
    }
}

impl TryFrom<[u8; 15]> for AccountId2 {
    type Error = AccountError;

    /// Converts a byte array in little-endian order to an [`AccountId`].
    fn try_from(bytes: [u8; 15]) -> Result<Self, Self::Error> {
        // This slice has 7 bytes, since the 8th byte will always be zero.
        let second_felt_slice = &bytes[..7];
        // This slice has 8 bytes.
        let first_felt_slice = &bytes[7..];

        let mut second_felt_bytes = [0; 8];
        second_felt_bytes[1..8].copy_from_slice(second_felt_slice);
        let second_felt = Felt::try_from(second_felt_bytes.as_slice())
            .map_err(AccountError::AccountIdInvalidFieldElement)?;

        let first_felt =
            Felt::try_from(first_felt_slice).map_err(AccountError::AccountIdInvalidFieldElement)?;

        Self::try_from([first_felt, second_felt])
    }
}

impl TryFrom<u128> for AccountId2 {
    type Error = AccountError;

    fn try_from(int: u128) -> Result<Self, Self::Error> {
        let bytes: [u8; 15] = int.to_le_bytes()[1..16]
            .try_into()
            .expect("we should have sliced off exactly 15 bytes");
        Self::try_from(bytes)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountId2 {
    fn write_into<W: miden_crypto::utils::ByteWriter>(&self, target: &mut W) {
        let bytes: [u8; 15] = (*self).into();
        bytes.write_into(target);
    }

    fn get_size_hint(&self) -> usize {
        // TODO: Turn into constant?
        15
    }
}

impl Deserializable for AccountId2 {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        <[u8; 15]>::read_from(source)?
            .try_into()
            .map_err(|err: AccountError| DeserializationError::InvalidValue(err.to_string()))
    }
}

/// Returns an [AccountId] instantiated with the provided field elements.
///
/// TODO
fn account_id_from_felts(elements: [Felt; 2]) -> Result<AccountId2, AccountError> {
    validate_first_felt(elements[0])?;
    validate_second_felt(elements[1])?;

    Ok(AccountId2 {
        first_felt: elements[0],
        second_felt: elements[1],
    })
}

pub(super) fn validate_first_felt(
    first_felt: Felt,
) -> Result<(AccountType2, AccountStorageMode, AccountVersion), AccountError> {
    let first_felt = first_felt.as_int();

    // Validate min account ones.
    // TODO: Describe why we only count ones on first felt.
    let ones_count = first_felt.count_ones();
    if ones_count < AccountId2::MIN_ACCOUNT_ONES {
        return Err(AccountError::AccountIdTooFewOnes(ones_count));
    }

    // Validate high bit of first felt is zero.
    if first_felt >> 63 != 0 {
        return Err(AccountError::AssumptionViolated(
            "TODO: Make proper error: first felt high bit must be zero".into(),
        ));
    }

    // Validate storage bits.
    let storage_mode = extract_storage_mode(first_felt)?;

    // Validate version bits.
    let version = extract_version(first_felt)?;

    let account_type = extract_type(first_felt);

    Ok((account_type, storage_mode, version))
}

fn validate_second_felt(second_felt: Felt) -> Result<(), AccountError> {
    let second_felt = second_felt.as_int();

    // Validate lower 8 bits of second felt are zero.
    if second_felt & 0xff != 0 {
        return Err(AccountError::AssumptionViolated(
            "TODO: Make proper error: second felt lower 8 bits must be zero".into(),
        ));
    }

    Ok(())
}

fn extract_storage_mode(first_felt: u64) -> Result<AccountStorageMode, AccountError> {
    let bits = (first_felt & ACCOUNT_STORAGE_MASK) >> ACCOUNT_STORAGE_MASK_SHIFT;
    match bits {
        PUBLIC => Ok(AccountStorageMode::Public),
        PRIVATE => Ok(AccountStorageMode::Private),
        _ => Err(AccountError::InvalidAccountStorageMode(format!("0b{bits:b}"))),
    }
}

fn extract_version(first_felt: u64) -> Result<AccountVersion, AccountError> {
    let bits = (first_felt & ACCOUNT_VERSION_MASK) >> ACCOUNT_VERSION_MASK_SHIFT;
    let version = bits.try_into().expect("TODO");
    match version {
        AccountVersion::VERSION_0_NUMBER => Ok(AccountVersion::VERSION_0),
        other => Err(AccountError::AssumptionViolated(format!(
            "TODO: Error. Unexpected version {other}"
        ))),
    }
}

fn extract_type(first_felt: u64) -> AccountType2 {
    let bits = first_felt & ACCOUNT_TYPE_MASK;
    match bits {
        REGULAR_ACCOUNT_UPDATABLE_CODE => AccountType2::RegularAccountUpdatableCode,
        REGULAR_ACCOUNT_IMMUTABLE_CODE => AccountType2::RegularAccountImmutableCode,
        FUNGIBLE_FAUCET => AccountType2::FungibleFaucet,
        NON_FUNGIBLE_FAUCET => AccountType2::NonFungibleFaucet,
        _ => {
            // account_type mask contains only 2bits, there are 4 options total.
            unreachable!()
        },
    }
}

fn extract_epoch(second_felt: u64) -> u16 {
    ((second_felt & ACCOUNT_EPOCH_MASK) >> ACCOUNT_EPOCH_MASK_SHIFT) as u16
}

// Shapes the second felt so it meets the requirements of the [`AccountId2`].
fn shape_second_felt(second_felt: Felt, epoch: u16) -> Felt {
    if epoch == u16::MAX {
        unimplemented!("TODO: Return error");
    }

    // Set epoch.
    let mut second_felt = second_felt.as_int();
    let epoch = (epoch as u64) << ACCOUNT_EPOCH_MASK_SHIFT;
    second_felt &= epoch;
    second_felt |= epoch;

    // Set lower 8 bits to zero.
    second_felt &= 0xffff_ffff_ffff_ff00;

    Felt::try_from(second_felt).expect("felt should still be valid")
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct AccountVersion(u8);

impl AccountVersion {
    const VERSION_0_NUMBER: u8 = 0;
    pub const VERSION_0: AccountVersion = AccountVersion(Self::VERSION_0_NUMBER);

    pub const fn version_num(&self) -> u8 {
        self.0
    }
}

/// Returns the digest of two hashing permutations over the seed, code commitment, storage
/// commitment and padding.
pub(super) fn compute_digest(
    seed: Word,
    code_commitment: Digest,
    storage_commitment: Digest,
    block_hash: Digest,
) -> Digest {
    let mut elements = Vec::with_capacity(16);
    elements.extend(seed);
    elements.extend(*code_commitment);
    elements.extend(*storage_commitment);
    elements.extend(*block_hash);
    Hasher::hash_elements(&elements)
}

// TESTING
// ================================================================================================

#[cfg(any(feature = "testing", test))]
pub mod testing {
    use super::{AccountStorageMode, AccountType2, ACCOUNT_STORAGE_MASK_SHIFT};

    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    // REGULAR ACCOUNTS - OFF-CHAIN
    pub const ACCOUNT_ID_SENDER: u128 = account_id(
        AccountType2::RegularAccountImmutableCode,
        AccountStorageMode::Private,
        0b0001_1111,
    );
    pub const ACCOUNT_ID_OFF_CHAIN_SENDER: u128 = account_id(
        AccountType2::RegularAccountImmutableCode,
        AccountStorageMode::Private,
        0b0010_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN: u128 = account_id(
        AccountType2::RegularAccountUpdatableCode,
        AccountStorageMode::Private,
        0b0011_1111,
    );
    // REGULAR ACCOUNTS - ON-CHAIN
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN: u128 = account_id(
        AccountType2::RegularAccountImmutableCode,
        AccountStorageMode::Public,
        0b0001_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2: u128 = account_id(
        AccountType2::RegularAccountImmutableCode,
        AccountStorageMode::Public,
        0b0010_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u128 = account_id(
        AccountType2::RegularAccountUpdatableCode,
        AccountStorageMode::Public,
        0b0011_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2: u128 = account_id(
        AccountType2::RegularAccountUpdatableCode,
        AccountStorageMode::Public,
        0b0100_1111,
    );

    // FUNGIBLE TOKENS - OFF-CHAIN
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN: u128 =
        account_id(AccountType2::FungibleFaucet, AccountStorageMode::Private, 0b0001_1111);
    // FUNGIBLE TOKENS - ON-CHAIN
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u128 =
        account_id(AccountType2::FungibleFaucet, AccountStorageMode::Public, 0b0001_1111);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1: u128 =
        account_id(AccountType2::FungibleFaucet, AccountStorageMode::Public, 0b0010_1111);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2: u128 =
        account_id(AccountType2::FungibleFaucet, AccountStorageMode::Public, 0b0011_1111);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3: u128 =
        account_id(AccountType2::FungibleFaucet, AccountStorageMode::Public, 0b0100_1111);

    // NON-FUNGIBLE TOKENS - OFF-CHAIN
    pub const ACCOUNT_ID_INSUFFICIENT_ONES: u128 =
        account_id(AccountType2::NonFungibleFaucet, AccountStorageMode::Private, 0b0000_0000); // invalid
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN: u128 =
        account_id(AccountType2::NonFungibleFaucet, AccountStorageMode::Private, 0b0001_1111);
    // NON-FUNGIBLE TOKENS - ON-CHAIN
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN: u128 =
        account_id(AccountType2::NonFungibleFaucet, AccountStorageMode::Public, 0b0010_1111);
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1: u128 =
        account_id(AccountType2::NonFungibleFaucet, AccountStorageMode::Public, 0b0011_1111);

    // UTILITIES
    // --------------------------------------------------------------------------------------------

    pub const fn account_id(
        account_type: AccountType2,
        storage_mode: AccountStorageMode,
        random: u32,
    ) -> u128 {
        let mut id = 0;

        id |= account_type as u128;
        id |= (storage_mode as u128) << ACCOUNT_STORAGE_MASK_SHIFT;
        // Shift the random part of the ID so we don't overwrite the metadata.
        id |= (random as u128) << 8;

        // Shifts in zeroes from the right so the second felt will be entirely 0.
        id << 64
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {

    use vm_core::StarkField;

    use super::*;
    use crate::accounts::testing::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
        ACCOUNT_ID_OFF_CHAIN_SENDER, ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
    };

    #[test]
    fn test_account_id_validation() {
        let felt_max: Felt = Felt::new(Felt::MODULUS);
        AccountId2::try_from([felt_max, felt_max]).unwrap_err();
    }

    #[test]
    fn test_account_id_from_seed_with_epoch() {
        let code_commitment: Digest = Digest::default();
        let storage_commitment: Digest = Digest::default();
        let block_hash: Digest = Digest::default();

        let seed = AccountId2::get_account_seed(
            [10; 32],
            AccountType2::FungibleFaucet,
            AccountStorageMode::Public,
            AccountVersion::VERSION_0,
            code_commitment,
            storage_commitment,
            block_hash,
        )
        .unwrap();

        for epoch in [0, u16::MAX - 1, 5000] {
            let id = AccountId2::new(seed, epoch, code_commitment, storage_commitment, block_hash)
                .unwrap();
            assert_eq!(id.epoch(), epoch);
        }
    }

    #[test]
    fn test_account_id() {
        let valid_second_felt = Felt::try_from(0xfffe_ffff_ffff_ff00u64).unwrap();
        let valid_first_felt = Felt::try_from(0x7fff_ffff_ffff_ff00u64).unwrap();

        let id1 = AccountId2::new_unchecked([valid_first_felt, valid_second_felt]);
        assert_eq!(id1.account_type(), AccountType2::RegularAccountImmutableCode);
        assert_eq!(id1.storage_mode(), AccountStorageMode::Public);
        assert_eq!(id1.version(), AccountVersion::VERSION_0);
        assert_eq!(id1.epoch(), u16::MAX - 1);
    }

    #[test]
    fn account_id_construction() {
        // Use the highest possible input to check if the constructed id is a valid Felt in that
        // scenario.
        // Use the lowest possible input to check whether the constructor satisfies
        // MIN_ACCOUNT_ONES.
        for input in [[0xff; 15], [0; 15]] {
            for account_type in [
                AccountType2::FungibleFaucet,
                AccountType2::NonFungibleFaucet,
                AccountType2::RegularAccountImmutableCode,
                AccountType2::RegularAccountUpdatableCode,
            ] {
                for storage_mode in [AccountStorageMode::Private, AccountStorageMode::Public] {
                    let id = AccountId2::new_with_type_and_mode(input, account_type, storage_mode);
                    assert_eq!(id.account_type(), account_type);
                    assert_eq!(id.storage_mode(), storage_mode);
                    assert_eq!(id.epoch(), 0);
                    // TODO: Do a serialization roundtrip to ensure validity.
                    // AccountId2::read_from_bytes(&id.to_bytes()).unwrap();
                }
            }
        }
    }

    // CONVERSION TESTS
    // ================================================================================================

    #[test]
    fn test_account_id_conversion_roundtrip() {
        for account_id in [
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
            ACCOUNT_ID_OFF_CHAIN_SENDER,
        ] {
            let id = AccountId2::try_from(account_id).expect("account ID should be valid");
            assert_eq!(id, AccountId2::from_hex(&id.to_hex()).unwrap());
            assert_eq!(id, AccountId2::try_from(<[u8; 15]>::from(id)).unwrap());
            assert_eq!(id, AccountId2::try_from(u128::from(id)).unwrap());
            assert_eq!(account_id, u128::from(id));
        }
    }

    #[test]
    fn test_account_id_account_type() {
        let account_id = AccountId2::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN)
            .expect("valid account ID");

        let account_type: AccountType2 = ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.into();
        assert_eq!(account_type, account_id.account_type());

        let account_id = AccountId2::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN)
            .expect("valid account ID");
        let account_type: AccountType2 = ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN.into();
        assert_eq!(account_type, account_id.account_type());

        let account_id =
            AccountId2::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).expect("valid account ID");
        let account_type: AccountType2 = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.into();
        assert_eq!(account_type, account_id.account_type());

        let account_id = AccountId2::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN)
            .expect("valid account ID");
        let account_type: AccountType2 = ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN.into();
        assert_eq!(account_type, account_id.account_type());
    }

    #[test]
    fn test_account_id_tag_identifiers() {
        let account_id = AccountId2::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN)
            .expect("valid account ID");
        assert!(account_id.is_regular_account());
        assert_eq!(account_id.account_type(), AccountType2::RegularAccountImmutableCode);
        assert!(account_id.is_public());

        let account_id = AccountId2::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN)
            .expect("valid account ID");
        assert!(account_id.is_regular_account());
        assert_eq!(account_id.account_type(), AccountType2::RegularAccountUpdatableCode);
        assert!(!account_id.is_public());

        let account_id =
            AccountId2::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).expect("valid account ID");
        assert!(account_id.is_faucet());
        assert_eq!(account_id.account_type(), AccountType2::FungibleFaucet);
        assert!(account_id.is_public());

        let account_id = AccountId2::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN)
            .expect("valid account ID");
        assert!(account_id.is_faucet());
        assert_eq!(account_id.account_type(), AccountType2::NonFungibleFaucet);
        assert!(!account_id.is_public());
    }

    /// The following test ensure there is a bit available to identify an account as a faucet or
    /// normal.
    #[test]
    fn test_account_id_faucet_bit() {
        const ACCOUNT_IS_FAUCET_MASK: u64 = 0b10;

        // faucets have a bit set
        assert_ne!((FUNGIBLE_FAUCET) & ACCOUNT_IS_FAUCET_MASK, 0);
        assert_ne!((NON_FUNGIBLE_FAUCET) & ACCOUNT_IS_FAUCET_MASK, 0);

        // normal accounts do not have the faucet bit set
        assert_eq!((REGULAR_ACCOUNT_IMMUTABLE_CODE) & ACCOUNT_IS_FAUCET_MASK, 0);
        assert_eq!((REGULAR_ACCOUNT_UPDATABLE_CODE) & ACCOUNT_IS_FAUCET_MASK, 0);
    }
}
