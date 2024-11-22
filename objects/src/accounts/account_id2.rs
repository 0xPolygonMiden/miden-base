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

const ACCOUNT_EPOCH_MASK_SHIFT: u64 = 8;
const ACCOUNT_EPOCH_MASK: u64 = 0xffff << ACCOUNT_EPOCH_MASK_SHIFT;

// The higher two bits of the least significant nibble determines the account storage mode
const ACCOUNT_STORAGE_MASK_SHIFT: u64 = 2;
const ACCOUNT_STORAGE_MASK: u64 = 0b11 << ACCOUNT_STORAGE_MASK_SHIFT;

// The lower two bits of the least significant nibble determine the account type.
const ACCOUNT_TYPE_MASK: u64 = 0b11;

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

        // Manipulate second felt to meet requirements of the ID.
        // Set epoch.
        let mut second_felt = felts[1].as_int();
        let epoch = (epoch as u64) << ACCOUNT_EPOCH_MASK_SHIFT;
        second_felt &= epoch;
        second_felt |= epoch;

        // Set high bit and lower 8 bits to zero.
        second_felt &= 0x7fff_ffff_ffff_ff00;

        felts[1] = Felt::new(second_felt);

        account_id_from_felts(felts)
    }

    pub fn new_unchecked(elements: [Felt; 2]) -> Self {
        Self(elements)
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
        return Err(AccountError::AssumptionViolated("TODO: Make proper error".into()));
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

    // Validate high bit of second felt is zero.
    if second_felt >> 63 != 0 {
        return Err(AccountError::AssumptionViolated("TODO: Make proper error".into()));
    }

    // Validate lower 8 bits of second felt are zero.
    if second_felt & 0xff != 0 {
        return Err(AccountError::AssumptionViolated("TODO: Make proper error".into()));
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

        for epoch in [0, u16::MAX, 5000] {
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
        let valid_second_felt = Felt::new(0x7fff_ffff_ffff_ff00);
        let valid_first_felt_high_bits: u64 = 0x7fff_ffff_ffff_ff00;

        let first_felt_1 = Felt::new(valid_first_felt_high_bits);
        let id1 = AccountId2::new_unchecked([first_felt_1, valid_second_felt]);
        assert_eq!(id1.account_type(), AccountType::RegularAccountImmutableCode);
        assert_eq!(id1.storage_mode(), AccountStorageMode::Public);
        assert_eq!(id1.version(), AccountVersion::VERSION_0);
        assert_eq!(id1.epoch(), u16::MAX);
    }
}
