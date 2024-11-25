use alloc::vec::Vec;

use vm_core::{Felt, Word};
use vm_processor::Digest;

use super::Hasher;
use crate::{
    accounts::{
        account_id::{
            FUNGIBLE_FAUCET, NON_FUNGIBLE_FAUCET, PRIVATE, PUBLIC, REGULAR_ACCOUNT_IMMUTABLE_CODE,
            REGULAR_ACCOUNT_UPDATABLE_CODE,
        },
        AccountStorageMode, AccountType,
    },
    AccountError,
};

// CONSTANTS
// ================================================================================================

const ACCOUNT_VERSION_MASK_SHIFT: u64 = 4;
const ACCOUNT_VERSION_MASK: u64 = 0b1111 << ACCOUNT_STORAGE_MASK_SHIFT;

const ACCOUNT_EPOCH_MASK_SHIFT: u64 = 48;
const ACCOUNT_EPOCH_MASK: u64 = 0xffff << ACCOUNT_EPOCH_MASK_SHIFT;

// The higher two bits of the least significant nibble determines the account storage mode
const ACCOUNT_STORAGE_MASK_SHIFT: u64 = 2;
const ACCOUNT_STORAGE_MASK: u64 = 0b11 << ACCOUNT_STORAGE_MASK_SHIFT;

// The lower two bits of the least significant nibble determine the account type.
const ACCOUNT_TYPE_MASK: u64 = 0b11;

/// # Layout
/// ```text
/// 1st felt: [zero bit | random (55 bits) | version (4 bits) | storage mode (2 bits) | type (2 bits)]
/// 2nd felt: [epoch (16 bits) | random (40 bits) | 8 zero bits]
/// ```
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AccountId2([Felt; 2]);

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
        Self(elements)
    }

    #[cfg(any(feature = "testing", test))]
    pub fn new_with_type_and_mode(
        mut bytes: [u8; 15],
        account_type: AccountType,
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
        account_type: AccountType,
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

    /// Returns true if an account with this ID is a public account.
    pub fn is_public(&self) -> bool {
        self.storage_mode() == AccountStorageMode::Public
    }

    pub fn storage_mode(&self) -> AccountStorageMode {
        extract_storage_mode(self.first_felt().as_int())
            .expect("account id should have been constructed with a valid storage mode")
    }

    pub fn version(&self) -> AccountVersion {
        extract_version(self.first_felt().as_int())
            .expect("account id should have been constructed with a valid version")
    }

    pub fn account_type(&self) -> AccountType {
        extract_type(self.first_felt().as_int())
    }

    pub fn epoch(&self) -> u16 {
        extract_epoch(self.second_felt().as_int())
    }

    fn first_felt(&self) -> Felt {
        self.0[0]
    }

    fn second_felt(&self) -> Felt {
        self.0[1]
    }
}

// CONVERSIONS TO ACCOUNT ID
// ================================================================================================

/// Returns an [AccountId] instantiated with the provided field elements.
///
/// TODO
fn account_id_from_felts(elements: [Felt; 2]) -> Result<AccountId2, AccountError> {
    validate_first_felt(elements[0])?;
    validate_second_felt(elements[1])?;

    Ok(AccountId2(elements))
}

pub(super) fn validate_first_felt(
    first_felt: Felt,
) -> Result<(AccountType, AccountStorageMode, AccountVersion), AccountError> {
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

fn extract_type(first_felt: u64) -> AccountType {
    let bits = first_felt & ACCOUNT_TYPE_MASK;
    match bits {
        REGULAR_ACCOUNT_UPDATABLE_CODE => AccountType::RegularAccountUpdatableCode,
        REGULAR_ACCOUNT_IMMUTABLE_CODE => AccountType::RegularAccountImmutableCode,
        FUNGIBLE_FAUCET => AccountType::FungibleFaucet,
        NON_FUNGIBLE_FAUCET => AccountType::NonFungibleFaucet,
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

impl TryFrom<[Felt; 2]> for AccountId2 {
    type Error = AccountError;

    /// TODO
    fn try_from(elements: [Felt; 2]) -> Result<Self, Self::Error> {
        account_id_from_felts(elements)
    }
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

#[cfg(test)]
mod tests {

    use vm_core::StarkField;

    use super::*;

    #[test]
    fn test_account_id_validation() {
        let felt_max: Felt = Felt::new(Felt::MODULUS);
        AccountId2::try_from([felt_max, felt_max]).unwrap_err();
    }

    #[test]
    fn test_account_id_from_seed_with_epoch() {
        // Precomputed seed.
        let valid_seed: [Felt; 4] = [
            Felt::new(13754904720699751090),
            Felt::new(13207074062734582735),
            Felt::new(457959651162721765),
            Felt::new(13059402505343003170),
        ];

        for epoch in [0, u16::MAX - 1, 5000] {
            let id = AccountId2::new(
                valid_seed,
                epoch,
                Digest::default(),
                Digest::default(),
                Digest::default(),
            )
            .unwrap();
            assert_eq!(id.epoch(), epoch);
        }
    }

    #[test]
    fn test_account_id() {
        let valid_second_felt = Felt::try_from(0xfffe_ffff_ffff_ff00u64).unwrap();
        let valid_first_felt = Felt::try_from(0x7fff_ffff_ffff_ff00u64).unwrap();

        let id1 = AccountId2::new_unchecked([valid_first_felt, valid_second_felt]);
        assert_eq!(id1.account_type(), AccountType::RegularAccountImmutableCode);
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
                AccountType::FungibleFaucet,
                AccountType::NonFungibleFaucet,
                AccountType::RegularAccountImmutableCode,
                AccountType::RegularAccountUpdatableCode,
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
}
